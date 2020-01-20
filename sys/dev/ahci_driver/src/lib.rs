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
use libsyscalls::errors::Result;
use libsyscalls::syscalls::{sys_print, sys_alloc, sys_backtrace};
use console::println;
use pci_driver::{BarRegions, PciClass};
use ahci::AhciBarRegion;
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Once;
use byteorder::{LittleEndian, ByteOrder};

use core::iter::Iterator;

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
            // Dummy values. We will use class based matching
            // so vendor_id and device_id won't be used
            vendor_id: 0x1234,
            device_id: 0x1234,
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
        // Filter out all disk that already has an OS on it
        let have_magic_number: Vec<bool> = disks
                        .iter_mut()
                        .map(|d| {
                            let mut buf = [0u8; 512];
                            const MBR_MAGIC: u16 = 0xAA55;
                            d.read(0, &mut buf);
                            LittleEndian::read_u16(&buf[510..]) == MBR_MAGIC
                        })
                        .collect();
        let mut disks = disks
                        .into_iter()
                        .zip(have_magic_number)
                        .filter_map(|(d, has_magic_num)| {
                            if has_magic_num {
                                None
                            } else {
                                Some(d)
                            }
                        })
                        .collect();
        self.disks = RefCell::new(disks);

        for (i, disk) in self.disks.borrow_mut().iter_mut().enumerate() {
            println!("Disk {}: {}", i, disk.size());
        }

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

impl usr::bdev::BDev for Ahci {
    fn read(&self, block: u32, data: &mut [u8; 512]) {
        self.disks.borrow_mut()[0].read(block as u64, data);
    }
    fn read_contig(&self, block: u32, data: &mut [u8]) {
        self.disks.borrow_mut()[0].read(block as u64, data);
    }
    fn write(&self, block: u32, data: &[u8; 512]) {
        println!("WARNING: BDEV.write is currently disabled");
        // self.disks.borrow_mut()[0].write(block as u64, data);
    }

    fn submit(&self, block: u64, write: bool, buf: Box<[u8]>) -> Result<u32> {
        self.disks.borrow_mut()[0].submit(block, write, buf)
    }

    fn poll(&self, slot: u32) -> Result<Option<Box<[u8]>>> {
        self.disks.borrow_mut()[0].poll(slot)
    }
}


#[no_mangle]
pub fn ahci_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn usr::bdev::BDev> {
    libsyscalls::syscalls::init(s);

    let mut ahci = Ahci::new();
    if let Err(_) = pci.pci_register_driver(&mut ahci, /*ABAR index*/5, Some((PciClass::Storage, /*SATA*/0x06))) {
        println!("WARNING: Failed to register AHCI device");
    }

    let ahci: Box<dyn usr::bdev::BDev> = Box::new(ahci);

    // benchmark_ahci(&ahci, 256, 1);
    // benchmark_ahci_async(&ahci, 256, 1);
    // benchmark_ahci(&ahci, 8192, 8192);
    // benchmark_ahci_async(&ahci, 8192, 8192);
    // benchmark_ahci(&ahci, 8192 * 128, 8192);
    // benchmark_ahci_async(&ahci, 8192 * 128, 8192);
    // benchmark_ahci(&ahci, 32768, 32768);
    // benchmark_ahci(&ahci, 0xFFFF * 128, 0xFFFF);
    // benchmark_ahci_async(&ahci, 0xFFFF * 128, 0xFFFF);
    ahci
}

fn benchmark_ahci(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
    assert!(blocks_to_read % blocks_per_patch == 0);
    assert!(blocks_per_patch <= 0xFFFF);
    let mut buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];

    let start = libsyscalls::time::get_rdtsc();
    for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
        bdev.read_contig(i, &mut buf);
    }
    let end = libsyscalls::time::get_rdtsc();
    println!("AHCI benchmark: reading {} blocks, {} blocks at a time, takes {} cycles", blocks_to_read, blocks_per_patch, end - start);
}

fn benchmark_ahci_async(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
    println!("starting bencharl async {}", blocks_to_read);

    assert!(blocks_to_read % blocks_per_patch == 0);
    assert!(blocks_per_patch <= 0xFFFF);
    let mut buffers: Vec<Box<[u8]>> = Vec::new();
    for _ in 0..32 {
        let buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];
        buffers.push(buf.into_boxed_slice());
    }
    let mut pending = Vec::<u32>::new();

    let start = libsyscalls::time::get_rdtsc();
    for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
        while buffers.is_empty() {
            assert!(!pending.is_empty());
            pending = pending
                .into_iter()
                .filter(|slot|  {
                    if let Some(buf) = bdev.poll(*slot).unwrap() {
                        buffers.push(buf);
                        return false;
                    } else {
                        return true;
                    }
                })
                .collect();
        }

        pending.push(bdev.submit(i as u64, false, buffers.pop().unwrap()).unwrap());
    }

    for p in pending {
        while bdev.poll(p).unwrap().is_none() {
            // spin
        }
    }
    let end = libsyscalls::time::get_rdtsc();
    println!("AHCI async benchmark: reading {} blocks, {} blocks at a time, takes {} cycles", blocks_to_read, blocks_per_patch, end - start);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("panicked: {:?}", info);
    sys_backtrace();
    loop {}
}
