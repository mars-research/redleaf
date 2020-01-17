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

fn handle_parsed_header(pci: &Pci, bus_num: u8,
                        dev_num: u8, func_num: u8, header: PciHeader) {
    let raw_class: u8 = header.class().into();
    let mut string = format!("PCI {:>02X}/{:>02X}/{:>02X} {:>04X}:{:>04X} {:>02X}.{:>02X}.{:>02X}.{:>02X} {:?}",
                             bus_num, dev_num, func_num, header.vendor_id(), header.device_id(), raw_class,
                             header.subclass(), header.interface(), header.revision(), header.class());

    let pci_device = PciDevice { vendor_id: header.vendor_id(), device_id: header.device_id() };

    PCI_MAP.lock().insert(pci_device, Vec::new());

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


    if let Some(bar_vec) = PCI_MAP.lock().get_mut(&pci_device) {
        for (i, bar) in header.bars().iter().enumerate() {
            if !bar.is_none() {
                bar_vec.push(Some(*bar));
                string.push_str(&format!(" {}={}", i, bar));
            } else {
                bar_vec.push(None);
                string.push_str(&format!(" {}=NULL", i));
            }
        }
    }

    #[cfg(feature = "c220g2_ixgbe")]
    {
        // Cloudlab has dual port ixgbe devices and the we need to attach our driver
        // to the second device.
        let mut hmap = PCI_MAP.lock();
        if let Some(bar_vec) = hmap.get_mut(&pci_device) {
            if bar_vec[0] == PciBar::Memory(0xc7900000) {
                    hmap.remove(&pci_device);
            }
        }
    }

    string.push('\n');

    println!("{}", string);
/*
    for driver in config.drivers.iter() {
        if let Some(class) = driver.class {
            if class != raw_class { continue; }
        }

        if let Some(subclass) = driver.subclass {
            if subclass != header.subclass() { continue; }
        }

        if let Some(interface) = driver.interface {
            if interface != header.interface() { continue; }
        }

        if let Some(ref ids) = driver.ids {
            let mut device_found = false;
            for (vendor, devices) in ids {
                let vendor_without_prefix = vendor.trim_start_matches("0x");
                let vendor = i64::from_str_radix(vendor_without_prefix, 16).unwrap() as u16;

                if vendor != header.vendor_id() { continue; }

                for device in devices {
                    if *device == header.device_id() {
                        device_found = true;
                        break;
                    }
                }
            }
            if !device_found { continue; }
        } else {
            if let Some(vendor) = driver.vendor {
                if vendor != header.vendor_id() { continue; }
            }

            if let Some(device) = driver.device {
                if device != header.device_id() { continue; }
            }
        }

        if let Some(ref device_id_range) = driver.device_id_range {
            if header.device_id() < device_id_range.start  ||
               device_id_range.end <= header.device_id() { continue; }
        }

        if let Some(ref args) = driver.command {
            // Enable bus mastering, memory space, and I/O space
            unsafe {
                let mut data = pci.read(bus_num, dev_num, func_num, 0x04);
                data |= 7;
                pci.write(bus_num, dev_num, func_num, 0x04, data);
            }

            // Set IRQ line to 9 if not set
            let mut irq;
            unsafe {
                let mut data = pci.read(bus_num, dev_num, func_num, 0x3C);
                irq = (data & 0xFF) as u8;
                if irq == 0xFF {
                    irq = 9;
                }
                data = (data & 0xFFFFFF00) | irq as u32;
                pci.write(bus_num, dev_num, func_num, 0x3C, data);
            }

            // Find BAR sizes
            let mut bars = [PciBar::None; 6];
            let mut bar_sizes = [0; 6];
            unsafe {
                let count = match header.header_type() {
                    PciHeaderType::GENERAL => 6,
                    PciHeaderType::PCITOPCI => 2,
                    _ => 0,
                };

                for i in 0..count {
                    bars[i] = header.get_bar(i);

                    let offset = 0x10 + (i as u8) * 4;

                    let original = pci.read(bus_num, dev_num, func_num, offset);
                    pci.write(bus_num, dev_num, func_num, offset, 0xFFFFFFFF);

                    let new = pci.read(bus_num, dev_num, func_num, offset);
                    pci.write(bus_num, dev_num, func_num, offset, original);

                    let masked = if new & 1 == 1 {
                        new & 0xFFFFFFFC
                    } else {
                        new & 0xFFFFFFF0
                    };

                    let size = !masked + 1;
                    bar_sizes[i] = if size <= 1 {
                        0
                    } else {
                        size
                    };
                }
            }

            // TODO: find a better way to pass the header data down to the
            // device driver, making passing the capabilities list etc
            // posible.
            let mut args = args.iter();
            if let Some(program) = args.next() {
                let mut command = Command::new(program);
                for arg in args {
                    let arg = match arg.as_str() {
                        "$BUS" => format!("{:>02X}", bus_num),
                        "$DEV" => format!("{:>02X}", dev_num),
                        "$FUNC" => format!("{:>02X}", func_num),
                        "$NAME" => format!("pci-{:>02X}.{:>02X}.{:>02X}", bus_num, dev_num, func_num),
                        "$BAR0" => format!("{}", bars[0]),
                        "$BAR1" => format!("{}", bars[1]),
                        "$BAR2" => format!("{}", bars[2]),
                        "$BAR3" => format!("{}", bars[3]),
                        "$BAR4" => format!("{}", bars[4]),
                        "$BAR5" => format!("{}", bars[5]),
                        "$BARSIZE0" => format!("{:>08X}", bar_sizes[0]),
                        "$BARSIZE1" => format!("{:>08X}", bar_sizes[1]),
                        "$BARSIZE2" => format!("{:>08X}", bar_sizes[2]),
                        "$BARSIZE3" => format!("{:>08X}", bar_sizes[3]),
                        "$BARSIZE4" => format!("{:>08X}", bar_sizes[4]),
                        "$BARSIZE5" => format!("{:>08X}", bar_sizes[5]),
                        "$IRQ" => format!("{}", irq),
                        "$VENID" => format!("{:>04X}", header.vendor_id()),
                        "$DEVID" => format!("{:>04X}", header.device_id()),
                        _ => arg.clone()
                    };
                    command.arg(&arg);
                }

                println!("PCID SPAWN {:?}", command);
                match command.spawn() {
                    Ok(mut child) => match child.wait() {
                        Ok(_status) => (), //println!("pcid: waited for {}: {:?}", line, status.code()),
                        Err(err) => println!("pcid: failed to wait for {:?}: {}", command, err)
                    },
                    Err(err) => println!("pcid: failed to execute {:?}: {}", command, err)
                }
            }
        }
    }
*/
}

pub fn scan_pci_devs(pci_resource: &dyn PciResource) {
    let pci = Pci::new(pci_resource);
    for bus in pci.buses() {
        for dev in bus.devs() {
            for func in dev.funcs() {
                // do stuff here
                let func_num = func.num;
                match PciHeader::from_reader(func) {
                    Ok(header) => {
                        handle_parsed_header(&pci, bus.num, dev.num, func_num, header);
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
