#![feature(alloc)]

use crate::pci::{Pci, PciBar, PciClass, PciHeader, PciHeaderError};
use syscalls::PciResource;
use console::println;
use alloc::format;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use alloc::vec::Vec;
use spin::Mutex;

lazy_static! {
    pub static ref PCI_DEVICES: Mutex<Vec<PciHeader>> = {
        Mutex::new(Vec::new())
    };
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct PciDevice {
    vendor_id: u16,
    device_id: u16,
}

impl PciDevice {
    pub fn new(vendor_id: u16, device_id: u16) -> PciDevice {
        PciDevice { vendor_id, device_id }
    }
}

pub fn scan_pci_devs(pci_resource: &dyn PciResource) {
    let pci = Pci::new(pci_resource);
    let mut pci_devices = PCI_DEVICES.lock();
    for bus in pci.buses() {
        for dev in bus.devs() {
            for func in dev.funcs() {
                // do stuff here
                let func_num = func.num;
                match PciHeader::from_reader(func) {
                    Ok(header) => {
                        #[cfg(feature = "c220g2_ixgbe")]
                        {
                            // Cloudlab has dual port ixgbe devices and the we need to attach our driver
                            // to the second device.
                            if header.get_bar(0) == PciBar::Memory(0xc7900000) {
                                continue;
                            }
                        }
                        pci_devices.push(header);
                    }
                    Err(PciHeaderError::NoDevice) => {},
                    Err(PciHeaderError::UnknownHeaderType(id)) => {
                        //println!("pcid: unknown header type: {}", id);
                    }
                }
            }
        }
    }
}
