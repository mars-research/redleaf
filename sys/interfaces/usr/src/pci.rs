/// RedLeaf PCI bus driver interface
use alloc::boxed::Box;
use rref::{RRef, RRefDeque};
use pci_driver::{PciDriver, PciClass, BarRegions, PciDrivers};

pub trait PCI {
    fn pci_register_driver(&self, pci_driver: &mut dyn pci_driver::PciDriver, bar_index: usize, class: Option<(PciClass, u8)>) -> Result<(), ()>;
    /// Boxed trait objects cannot be cloned trivially!
    /// https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/6
    fn pci_clone(&self) -> Box<dyn PCI>;
}

pub trait PciResource {
    fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32;
    fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32);
}

pub trait PciBar {
    fn get_bar_region(&self, base: u64, size: usize,
                      pci_driver: pci_driver::PciDrivers) ->  pci_driver::BarRegions;
}
