pub use crate::bar::PciBar;
pub use crate::bus::{PciBus, PciBusIter};
pub use crate::class::PciClass;
pub use crate::dev::{PciDev, PciDevIter};
pub use crate::func::PciFunc;
pub use crate::header::{PciHeader, PciHeaderError, PciHeaderType};
use syscalls::PciResource;

pub struct Pci<'a> {
    pci_config: &'a dyn PciResource,
}

impl<'a> Pci<'a> {
    pub fn new(pci_config: &'a dyn PciResource) -> Self {
        Pci {
            pci_config
        }
    }

    pub fn buses<'pci>(&'pci self) -> PciIter<'pci> {
        PciIter::new(self)
    }

    pub fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
        self.pci_config.read(bus, dev, func, offset)
    }

    pub fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
        self.pci_config.write(bus, dev, func, offset, value)
    }
}

pub struct PciIter<'pci> {
    pci: &'pci Pci<'pci>,
    num: u32
}

impl<'pci> PciIter<'pci> {
    pub fn new(pci: &'pci Pci) -> Self {
        PciIter {
            pci: pci,
            num: 0
        }
    }
}

impl<'pci> Iterator for PciIter<'pci> {
    type Item = PciBus<'pci>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.num < 255 { /* TODO: Do not ignore 0xFF bus */
            let bus = PciBus {
                pci: self.pci,
                num: self.num as u8
            };
            self.num += 1;
            Some(bus)
        } else {
            None
        }
    }
}
