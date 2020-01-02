use syscalls::PciResource;
use syscalls::PciBar;
use alloc::boxed::Box;

#[derive(Copy, Clone)]
pub struct PciConfig {
    addr_port: u16, // 0xCF8
    data_port: u16, // 0xCFC
}

pub static PCI_RESOURCE: PciConfig = PciConfig {addr_port: 0xCF8, data_port: 0xCFC};

impl PciResource for PciConfig {

    fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
        let address = 0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
        let value: u32;
        unsafe {
            asm!("mov dx, $2
              out dx, eax
              mov dx, $3
              in eax, dx"
              : "={eax}"(value) : "{eax}"(address), "r"(self.addr_port), "r"(self.data_port) : "dx" : "intel", "volatile");
        }
        value
    }

    fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
        let address = 0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);

        unsafe {
            asm!("mov dx, $1
              out dx, eax"
              : : "{eax}"(address), "r"(self.addr_port) : "dx" : "intel", "volatile");
            asm!("mov dx, $1
              out dx, eax"
              : : "{eax}"(value), "r"(self.data_port) : "dx" : "intel", "volatile");
        }
    }
}

pub struct PciDevice {
}

impl PciDevice {
    pub fn new() -> PciDevice {
        PciDevice {}
    }
}

impl PciBar for PciDevice {
    fn get_bar_region(&self, base: u64, size: usize,
                        pci_device: pci_driver::PciDrivers) -> pci_driver::BarRegions {
        use crate::dev::ixgbe::IxgbeBar;
        match pci_device {
            IxgbeDriver => {
                pci_driver::BarRegions::Ixgbe(Box::new(IxgbeBar::new(base, size)))
            },
        }
    }
}
