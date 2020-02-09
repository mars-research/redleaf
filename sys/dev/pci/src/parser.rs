#![feature(alloc)]

use crate::pci::{Pci, PciBar, PciHeader, PciHeaderError};
use syscalls::PciResource;
use console::println;
use alloc::format;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use alloc::vec::Vec;
use spin::Mutex;
use pci_driver::PciClass;

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

fn print_header(bus_num: u8,
                        dev_num: u8, func_num: u8, header: &PciHeader) {
    let raw_class: u8 = header.class().into();
    let mut string = format!("PCI {:>02X}/{:>02X}/{:>02X} {:>04X}:{:>04X} {:>02X}.{:>02X}.{:>02X}.{:>02X} {:?}",
                             bus_num, dev_num, func_num, header.vendor_id(), header.device_id(), raw_class,
                             header.subclass(), header.interface(), header.revision(), header.class());

    let pci_device = PciDevice { vendor_id: header.vendor_id(), device_id: header.device_id() };

    match header.class() {
        PciClass::Storage => match header.subclass() {
            0x01 => {
                string.push_str(" IDE");
            },
            0x06 => {
                string.push_str(" SATA");
            },
            _ => ()
        },
        PciClass::SerialBus => match header.subclass() {
            0x03 => match header.interface() {
                0x00 => {
                    string.push_str(" UHCI");
                },
                0x10 => {
                    string.push_str(" OHCI");
                },
                0x20 => {
                    string.push_str(" EHCI");
                },
                0x30 => {
                    string.push_str(" XHCI");
                },
                _ => ()
            },
            _ => ()
        },
        _ => ()
    }


    for (i, bar) in header.bars().iter().enumerate() {
        if !bar.is_none() {
            string.push_str(&format!(" {}={}", i, bar));
        } else {
            string.push_str(&format!(" {}=NULL", i));
        }
    }

    println!("{}", string);
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
                        print_header(bus.num, dev.num, func_num, &header);
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
