#![no_std]

mod pci_class;

pub use pci_class::PciClass;

extern crate alloc;
use ahci_regs::AhciBarRegion;
use alloc::boxed::Box;
use platform::PciBarAddr;

pub trait PciDriver {
    fn probe(&mut self, bar_region: DeviceBarRegions);
    fn get_vid(&self) -> u16;
    fn get_did(&self) -> u16;
    fn get_driver_type(&self) -> PciDrivers;
}

pub enum BarRegions {
    Ahci(Box<dyn AhciBarRegion>),
    // Ahci(PciBarAddr),
    None,
}

pub enum DeviceBarRegions {
    // Ahci(Box<dyn AhciBarRegion>),
    Ahci(PciBarAddr),
    Ixgbe(PciBarAddr),
    Nvme(PciBarAddr),
    None,
}

#[derive(Copy, Clone, Debug)]
pub enum PciDrivers {
    IxgbeDriver,
    AhciDriver,
    NvmeDriver,
}

#[derive(Copy, Clone, Debug)]
pub enum PciDeviceMatcher {
    DeviceId((u16, u16)),
    Class((u8, u8)),
}
