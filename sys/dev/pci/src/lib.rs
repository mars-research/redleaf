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
use syscalls::{Syscall, PciResource, PciBar};
use libsyscalls::syscalls::{sys_println};
use alloc::boxed::Box;
use crate::parser::{PciDevice, PCI_MAP};
use console::println;
use spin::Once;

#[derive(Clone)]
struct PCI {}

static PCI_BAR: Once<Box<dyn PciBar + Send + Sync>> = Once::new();

impl PCI {
    fn new() -> PCI {
        PCI{}
    }
}

impl syscalls::PCI for PCI {

    //-> bar_regions::BarRegions
    fn pci_register_driver(&self, pci_driver: &mut dyn pci_driver::PciDriver) {
        let vendor_id = pci_driver.get_vid();
        let device_id = pci_driver.get_did();
        // match vid, dev_id with the registered pci devices we have and
        // typecast the barregion to the appropriate one for this device
        let pci_dev = PciDevice::new(vendor_id, device_id);
        if let Some(bars) = PCI_MAP.lock().get(&pci_dev) {
            println!("Device found {:x?} {:?}", pci_dev, bars[0]);
            let bar0 = match bars[0] {
                bar::PciBar::Memory(addr) => addr,
                bar::PciBar::Port(port) => port as u32,
                _ => 0 as u32,
            };
            let pci_bar = PCI_BAR.r#try().expect("System call interface is not initialized.");
            let bar_region = pci_bar.get_bar_region(bar0 as u64, 512 * 1024 as usize, pci_driver.get_driver_type());
            pci_driver.probe(bar_region);
        };
    }

    fn pci_clone(&self) -> Box<dyn syscalls::PCI> {
        Box::new((*self).clone())
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
                pci_resource: Box<dyn PciResource>,
                pci_bar: Box<dyn PciBar + Send + Sync>) -> Box<dyn syscalls::PCI> {

    libsyscalls::syscalls::init(s);

    sys_println("init: starting PCI domain");

    parser::scan_pci_devs(pci_resource.as_ref());

    PCI_BAR.call_once(|| pci_bar);
    Box::new(PCI::new()) 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
