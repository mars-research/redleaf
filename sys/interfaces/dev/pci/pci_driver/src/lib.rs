#![no_std]

mod pci_class;

pub use pci_class::PciClass;

extern crate alloc;
use alloc::boxed::Box;
use ixgbe::IxgbeBarRegion;
use ahci::AhciBarRegion;
use platform::PciBarAddr;

pub trait PciDriver {
    fn probe(&mut self, bar_region: DeviceBarRegions);
    fn get_vid(&self) -> u16;
    fn get_did(&self) -> u16;
    fn get_driver_type(&self) -> PciDrivers;
}

pub enum BarRegions {
    Ahci(Box <dyn AhciBarRegion>),
    Ixgbe(Box <dyn IxgbeBarRegion>),
    None
}

pub enum DeviceBarRegions {
    Ahci(PciBarAddr),
    Ixgbe(PciBarAddr),
    None
}

#[derive(Copy, Clone, Debug)]
pub enum PciDrivers {
    IxgbeDriver,
    AhciDriver,
}

#[derive(Copy, Clone, Debug)]
pub enum PciDeviceMatcher {
    DeviceId((u16, u16)),
    Class((u8, u8)),
}
