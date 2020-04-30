use syscalls::PciResource;
use console::println;
use alloc::format;

use lazy_static::lazy_static;
use alloc::vec::Vec;
use spin::Mutex;
use pci_driver::PciClass;

// Import from a safe interface
use pcidevice::{PciDevice};

lazy_static! {
    pub static ref PCI_DEVICES: Mutex<Vec<PciDevice>> = {
        Mutex::new(Vec::new())
    };
}

/*
fn print_header(bus_num: u8,
                        dev_num: u8, func_num: u8, header: &PciHeader) {
    let raw_class: u8 = header.class().into();
    let mut string = format!("PCI {:>02X}/{:>02X}/{:>02X} {:>04X}:{:>04X} {:>02X}.{:>02X}.{:>02X}.{:>02X} {:?}",
                             bus_num, dev_num, func_num, header.vendor_id(), header.device_id(), raw_class,
                             header.subclass(), header.interface(), header.revision(), header.class());

    string.push_str(&format!(" Command {:08X} status {:08X}", header.command(), header.status()));

    let _pci_device = PciDevice { vendor_id: header.vendor_id(), device_id: header.device_id() };

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
}*/

pub fn scan_pci_devs() {
    let mut pci_devices = PCI_DEVICES.lock();
    for bus in 0..=255 {
        for dev in 0..32 {
            for func in 0..8 {
                match pcidevice::get_config(bus, dev, func) {
                    Ok(pci_dev) => {
                        //print_header(bus.num, dev.num, func_num, &header);
                        pci_devices.push(pci_dev);
                    }
                    Err(_) => {}
                }
            }
        }
    }
}
