#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

#[macro_use]
extern crate bitflags;
extern crate byteorder;
#[macro_use]
extern crate serde_derive;

extern crate malloc;
extern crate alloc;

mod ahcid;

use core::panic::PanicInfo;
use core::cell::RefCell;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_print, sys_alloc, sys_backtrace};
use console::println;
use pci_driver::BarRegions;
use ahci::AhciBarRegion;
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Once;

use self::ahcid::Disk;
use self::ahcid::hba::Hba;

struct Ahci {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    disks: RefCell<Vec<Box<dyn Disk>>>,
}

impl Ahci {
    fn new() -> Ahci {
        Ahci {
            vendor_id: 0x8086,
            device_id: 0x2922,
            driver: pci_driver::PciDrivers::AhciDriver,
            disks: RefCell::new(Vec::new()),
        }
    }
}

impl pci_driver::PciDriver for Ahci {
    fn probe(&mut self, bar_region: BarRegions) {
        println!("probe() called");

        let bar = match bar_region {
            BarRegions::Ahci(bar) => {
                bar
            }
            _ => { panic!("Got unknown BAR region"); }
        };

        println!("Initializing with base = {:x}", bar.get_base());

        let mut disks = self::ahcid::disks(bar);
        self.disks = RefCell::new(disks);

        println!("probe() finished");
    }

    fn get_vid(&self) -> u16 {
        self.vendor_id
    }

    fn get_did(&self) -> u16 {
        self.device_id
    }

    fn get_driver_type(&self) -> pci_driver::PciDrivers {
        self.driver
    }
}

impl syscalls::BDev for Ahci {
    fn read(&self, block: u32, data: &mut [u8; 512]) {
        self.disks.borrow_mut()[0].read(block as u64, data);
    }
    fn write(&self, block: u32, data: &[u8; 512]) {
        self.disks.borrow_mut()[0].write(block as u64, data);
    }
}


#[no_mangle]
pub fn ahci_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::BDev> {
    libsyscalls::syscalls::init(s);

    let mut ahci = Ahci::new();
    pci.pci_register_driver(&mut ahci, 5);

    let ahci: Box<dyn syscalls::BDev> = Box::new(ahci);
    benchmark_ahci(&ahci);
    ahci
}

fn benchmark_ahci(bdev: &Box<dyn syscalls::BDev>) {
    const BLOCKS_TO_READ: u32 = 100;
    let mut buf = [0 as u8; 512];

    let start = libsyscalls::time::get_rdtsc();
    for i in 0..BLOCKS_TO_READ {
        bdev.read(i, &mut buf);
    }
    let end = libsyscalls::time::get_rdtsc();
    println!("AHCI benchmark: reading {} blocks takes {} cycles", BLOCKS_TO_READ, end - start);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("panicked: {:?}", info);
    sys_backtrace();
    loop {}
}
