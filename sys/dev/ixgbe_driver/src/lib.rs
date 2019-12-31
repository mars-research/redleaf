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
    panic_info_message,
    maybe_uninit_extra
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall,PCI};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::println;
use pci_driver::BarRegions;
use ixgbe::IxgbeBarRegion;
use core::mem::MaybeUninit;

struct Ixgbe {
    vendor_id: u16,
    device_id: u16,
}

struct IxgbeBar<'a> {
    ixgbe_bar: &'a dyn IxgbeBarRegion,
}

impl Ixgbe {
    fn new() -> Ixgbe {
        Ixgbe {
            vendor_id: 0x8086,
            device_id: 0x154D,
        }
    }
}

static mut ixgbe_bar: MaybeUninit<IxgbeBar> = MaybeUninit::uninit();

impl<'a> IxgbeBar<'a> {
    fn new(bar: &'a dyn IxgbeBarRegion) -> IxgbeBar<'a> {
        IxgbeBar {
            ixgbe_bar: bar
        }
    }
}

impl syscalls::Net for Ixgbe {
}

impl pci_driver::PciDriver for Ixgbe {
    fn probe(&mut self, bar_region: BarRegions) {
        match bar_region {
            BarRegions::Ixgbe(bar) => {
                let ixgbebar = IxgbeBar::new(bar.as_ref());
                println!("got ixgbe bar region");
                unsafe {
                    ixgbe_bar.write(ixgbebar);
                }
            }
            _ => { println!("Got unknown bar region") }
        }
    }

    fn get_vid(&self) -> u16 {
        self.vendor_id
    }

    fn get_did(&self) -> u16 {
        self.device_id
    }
}

#[no_mangle]
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    pci.pci_register_driver(&mut ixgbe);
    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
