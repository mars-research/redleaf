use syscalls::PciResource;
use syscalls::PciBar;
use alloc::boxed::Box;
use crate::memory::VSPACE;
use crate::arch::vspace::MapAction;
use crate::arch::memory::PAddr;

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
        use crate::dev::ixgbe::Bar;
        use crate::dev::ahci::AhciBar;

        crate::interrupt::disable_irq();

        let bar = match pci_device {
            pci_driver::PciDrivers::IxgbeDriver => {
                let ref mut vspace = *VSPACE.lock();

                // identity map the bar region
                vspace.map_identity(PAddr::from(base), PAddr::from(base + size as u64),
                                     MapAction::ReadWriteKernelNoCache);

                pci_driver::BarRegions::Ixgbe(Box::new(Bar::new(base as usize, size)))
            },
            pci_driver::PciDrivers::AhciDriver => {
                let ref mut vspace = *VSPACE.lock();

                // identity map the bar region
                vspace.map_identity(PAddr::from(base), PAddr::from(base + size as u64),
                                     MapAction::ReadWriteKernelNoCache);

                pci_driver::BarRegions::Ahci(Box::new(AhciBar::new(base, size)))
            },
        };

        crate::interrupt::enable_irq();

        bar
    }
}
