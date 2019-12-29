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

mod bar;
mod bus;
mod class;
mod dev;
mod func;
mod header;
mod pci;
mod parser;

use core::panic::PanicInfo;
use syscalls::{Syscall, PciResource};
use libsyscalls::syscalls::{sys_println};
use alloc::boxed::Box;

#[derive(Clone)]
struct PCI {}

impl PCI {
    fn new() -> PCI {
        PCI{}
    }
}

impl syscalls::PCI for PCI {

    //-> bar_regions::BarRegions
    fn pci_register_driver(&self, pci_driver: &dyn pci_driver::PciDriver) {
        let vid = pci_driver.get_vid();
        let did = pci_driver.get_did();
        // match vid, dev_id with the registered pci devices we have and
        // typecast the barregion to the appropriate one for this device
    }

    fn pci_clone(&self) -> Box<dyn syscalls::PCI> {
        Box::new((*self).clone())
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, PciResource: Box<dyn PciResource>) -> Box<dyn syscalls::PCI> {

    libsyscalls::syscalls::init(s);

    sys_println("init: starting PCI domain");

    parser::scan_pci_devs(PciResource.as_ref());

    Box::new(PCI::new()) 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
