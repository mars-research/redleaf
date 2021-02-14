#![no_std]
#![feature(
    llvm_asm,
    allocator_api,
)]

#[macro_use]
extern crate bitflags;
extern crate byteorder;
extern crate alloc;

mod header;

use pci_driver::{PciClass, DeviceBarRegions, PciDrivers};
use alloc::vec::Vec;
use libsyscalls::errors::Result;
use libsyscalls::errors::{Error, EINVAL, ENODEV};
use libsyscalls::syscalls::{sys_mmap, init_mmap};
use platform::PciBarAddr;
use header::{PciHeaderType, read_config_space, PciDeviceHeader};
use console::println;
use byteorder::{LittleEndian, ByteOrder};

const PCI_ADDR_PORT: u16 = 0xCF8;
const PCI_DATA_PORT: u16 = 0xCFC;

const BASE_PAGE_SIZE: u32 = 4096;

macro_rules! round_up {
    ($num:expr, $s:expr) => {
        (($num + $s - 1) / $s) * $s
    };
}

macro_rules! is_page_aligned {
    ($num:expr) => {
        $num % BASE_PAGE_SIZE as u64 == 0
    };
}

#[derive(Debug)]
pub struct PciAddress {
    bus: u16,
    dev: u8,
    func: u8, 
}

impl PciAddress {
    fn new(bus: u16, dev: u8, func: u8) -> Result<PciAddress> {
        if bus < 256 && dev < 32 && func < 8 {
            Ok(PciAddress {
                bus,
                dev,
                func,
            })
        } else {
            Err(Error::new(EINVAL))
        }
    }
}

#[derive(Debug)]
pub struct PciDevice {
    pci_addr: PciAddress,
    pci_hdr: PciDeviceHeader,
}

impl PciDevice {
    pub unsafe fn new(pci_addr: PciAddress, pci_hdr: PciDeviceHeader) -> PciDevice {
        PciDevice {
            pci_addr,
            pci_hdr,
        }
    }

    pub fn get_bar(&self, idx: usize, dev_type: PciDrivers) -> DeviceBarRegions {
        if let Some(bar_addr) = self.pci_hdr.get_bar(idx) {
            unsafe {
              println!("Mapping bar region {:x} {:x}", bar_addr.get_base(), bar_addr.get_size());
            }
            map_bar_region(bar_addr);

            match dev_type {
                PciDrivers::IxgbeDriver => { return DeviceBarRegions::Ixgbe(*bar_addr); },
                PciDrivers::NvmeDriver => { return DeviceBarRegions::Nvme(*bar_addr); },
                PciDrivers::AhciDriver => { return DeviceBarRegions::Ahci(*bar_addr); },
                _ => { return DeviceBarRegions::None; },
            }
        } else {
            return DeviceBarRegions::None;
        }
    }

    pub fn class(&self) -> PciClass {
        self.pci_hdr.class()
    }

    pub fn subclass(&self) -> u8 {
        self.pci_hdr.subclass()
    }

    pub fn vendor_id(&self) -> u16 {
        self.pci_hdr.vendor_id()
    }

    pub fn device_id(&self) -> u16 {
        self.pci_hdr.device_id()
    }
}


pub fn get_config(bus: u16, dev: u8, func: u8) -> Result<PciDevice> {
    
    // Check if the pci address is valid
    let pci_addr = match PciAddress::new(bus, dev, func) {
        Err(e) => return Err(e),
        Ok(a) => a,
    };

    match read_config_space(&pci_addr) {
        Ok(hdr) => unsafe { Ok(PciDevice::new(pci_addr, hdr)) },
        _ => Err(Error::new(ENODEV)),
    }
}

fn pci_read_bars(pci: &PciAddress, hdr_type: PciHeaderType) -> Vec<Option<PciBarAddr>> {
    let mut bar_vec: Vec<Option<PciBarAddr>> = Vec::new();

    // Baraddresses start from offset 0x10 in the config space and there can be a 2-6 bars each
    // 32-bit in size depending on the PCI device type
    let start = 0x10;
    let mut end: u8 = 0;
    match hdr_type & PciHeaderType::HEADER_TYPE {
        PciHeaderType::GENERAL => {
            end = start + 6 * 4;
        }

        PciHeaderType::PCITOPCI => {
            end = start + 2 * 4;
        }

        _ => (end = start),
    }

    for off in (start..end).step_by(4) {
        let mut addr = pci_read(pci, off);
        if addr & 0xFFFF_FFFC == 0 {
            bar_vec.push(None)
        } else if addr & 1 == 0 {
            addr &= 0xFFFF_FFF0;
            // Write all 1's to the pci config space
            pci_write(pci, off, 0xFFFF_FFFF);
            // Read it back, unmask, add 1 to determine size
            let size = round_up!((!pci_read(pci, off)) + 1, BASE_PAGE_SIZE);
            // Restore the original bar address
            pci_write(pci, off, addr);
            unsafe { bar_vec.push(Some(PciBarAddr::new(addr & 0xFFFF_FFF0, size as usize))); }
            println!("BarAddr {:x} size {:x}", addr, size);
        } else {
            // Write all 1's to the pci config space
            pci_write(pci, off, 0xFFFF_FFFF);
            // Read it back, unmask, add 1 to determine size
            let size = (!pci_read(pci, off)) + 1;
            // Restore the original bar address
            pci_write(pci, off, addr);
            unsafe { bar_vec.push(Some(PciBarAddr::new(addr & 0xFFFC, size as usize))); }
            println!("Bar I/O Addr {:x} size {:x}", addr, size);
        }
    }
    bar_vec
}

fn pci_read_range(pci: &PciAddress, offset: u8, len: u8) -> Vec<u8> {
    assert!(len > 3 && len % 4 == 0);
    let mut ret = Vec::with_capacity(len as usize);
    let results = (offset..offset + len).step_by(4).fold(Vec::new(), |mut acc, offset| {
        let val = pci_read(pci, offset);
        acc.push(val);
        acc
    });
    unsafe {
        ret.set_len(len as usize);
    }
    LittleEndian::write_u32_into(&*results, &mut ret);
    ret
}

fn pci_read(pci: &PciAddress, offset: u8) -> u32 {
    let address = 0x80000000 | ((pci.bus as u32) << 16) | ((pci.dev as u32) << 11) | ((pci.func as u32) << 8) | ((offset as u32) & 0xFC);
    let value: u32;
    unsafe {
        llvm_asm!("mov dx, $2
          out dx, eax
          mov dx, $3
          in eax, dx"
          : "={eax}"(value) : "{eax}"(address), "r"(PCI_ADDR_PORT), "r"(PCI_DATA_PORT) : "dx" : "intel", "volatile");
    }
    value
}

fn pci_write(pci: &PciAddress, offset: u8, value: u32) {
    let address = 0x80000000 | ((pci.bus as u32) << 16) | ((pci.dev as u32) << 11) | ((pci.func as u32) << 8) | ((offset as u32) & 0xFC);

    unsafe {
        llvm_asm!("mov dx, $1
          out dx, eax"
          : : "{eax}"(address), "r"(PCI_ADDR_PORT) : "dx" : "intel", "volatile");
        llvm_asm!("mov dx, $1
          out dx, eax"
          : : "{eax}"(value), "r"(PCI_DATA_PORT) : "dx" : "intel", "volatile");
    }
}

fn map_bar_region(bar: &PciBarAddr) {
    sys_mmap(bar);
}
