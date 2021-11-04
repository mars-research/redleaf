#![no_std]
#![no_main]
#![feature(untagged_unions)]
#![forbid(unsafe_code)]

extern crate alloc;
extern crate malloc;

mod parser;

use crate::parser::PCI_DEVICES;

use alloc::boxed::Box;
use console::println;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_backtrace, sys_println};
use syscalls::{Heap, Syscall};
use interface::rpc::RpcResult;
use interface::error::{Result, ErrorKind};

use pci_driver::{PciClass, PciDriver};

use pcidevice::PciDevice;

#[derive(Clone)]
struct PCI {}

impl PCI {
    fn new() -> PCI {
        PCI {}
    }
}

impl interface::pci::PCI for PCI {
    fn pci_register_driver(
        &self,
        pci_driver: &mut dyn PciDriver,
        bar_index: usize,
        class: Option<(PciClass, u8)>,
    ) -> RpcResult<Result<()>> {
        Ok(|| -> Result<()> {
            println!("Register driver called");
            let vendor_id = pci_driver.get_vid();
            let device_id = pci_driver.get_did();
            // match vid, dev_id with the registered pci devices we have and
            // typecast the barregion to the appropriate one for this device
            let pci_devs = &*PCI_DEVICES.lock();
            let pci_dev = match class {
                Some((class, subclass)) => pci_devs
                    .iter()
                    .filter(|header| header.class() == class && header.subclass() == subclass)
                    .next()
                    .ok_or(ErrorKind::InvalidPciClass),
                None => pci_devs
                    .iter()
                    .filter(|header| header.vendor_id() == vendor_id && header.device_id() == device_id)
                    .next()
                    .ok_or(ErrorKind::InvalidPciDeviceID),
            };
            let pci_dev = pci_dev?;

            // TODO: dont panic here
            let bar = pci_dev.get_bar(bar_index, pci_driver.get_driver_type());

            pci_driver.probe(bar);
            Ok(())
        }())
    }

    fn pci_clone(&self) -> RpcResult<Box<dyn interface::pci::PCI>> {
        Ok(Box::new((*self).clone()))
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    m: Box<dyn syscalls::Mmap + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
) -> Box<dyn interface::pci::PCI> {
    libsyscalls::syscalls::init(s);

    libsyscalls::syscalls::init_mmap(m);

    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

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
