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
#![forbid(unsafe_code)]

extern crate malloc;
extern crate alloc;

mod parser;

use crate::parser::{PCI_DEVICES};

use core::panic::PanicInfo;
use syscalls::{Syscall, Heap, PciResource, PciBar};
use libsyscalls::syscalls::{sys_println, sys_backtrace, init_mmap};
use alloc::boxed::Box;
use console::println;
use spin::Once;
use rref;
use pci_driver::{PciDriver, PciClass};
use pcidevice::get_config;
use pcidevice::{PciDevice};

#[derive(Clone)]
struct PCI {}

impl PCI {
    fn new() -> PCI {
        PCI{}
    }
}

impl syscalls::PCI for PCI {
    fn pci_register_driver(&self, pci_driver: &mut dyn PciDriver, bar_index: usize, class: Option<(PciClass, u8)>) -> Result<(), ()> {
        println!("Register driver called");
        let vendor_id = pci_driver.get_vid();
        let device_id = pci_driver.get_did();
        // match vid, dev_id with the registered pci devices we have and
        // typecast the barregion to the appropriate one for this device
        let pci_devs = &*PCI_DEVICES.lock();
        let pci_dev: &PciDevice = match class {
            Some((class, subclass)) => {
                pci_devs
                .iter()
                .filter(|header| {
                    header.class() == class && header.subclass() == subclass
                }).next()
                .ok_or(())
            }, 
            None => {
                pci_devs
                .iter()
                .filter(|header| {
                    header.vendor_id() == vendor_id && header.device_id() == device_id
                }).next()
                .ok_or(())
            }
        }?;
        
        // TODO: dont panic here
        let bar = pci_dev.get_bar(bar_index, pci_driver.get_driver_type());

        pci_driver.probe(bar);

        Ok(())
    }

    fn pci_clone(&self) -> Box<dyn syscalls::PCI> {
        Box::new((*self).clone())
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            m: Box<dyn syscalls::Mmap + Send + Sync>,
            heap: Box<dyn Heap + Send + Sync>) -> Box<dyn syscalls::PCI> {

    libsyscalls::syscalls::init(s);

    libsyscalls::syscalls::init_mmap(m);

    rref::init(heap);

    sys_println("init: starting PCI domain");

    parser::scan_pci_devs();

    Box::new(PCI::new()) 
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("pci panicked: {:?}", info);
    sys_backtrace();
    loop {}
}
