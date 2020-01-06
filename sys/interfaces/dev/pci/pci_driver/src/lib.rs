#![no_std]

extern crate alloc;
use alloc::boxed::Box;
use ixgbe::IxgbeBarRegion;
use ahci::AhciBarRegion;

pub trait PciDriver {
    fn probe(&mut self, bar_region: BarRegions);
    fn get_vid(&self) -> u16;
    fn get_did(&self) -> u16;
    fn get_driver_type(&self) -> PciDrivers;
}

pub enum BarRegions {
    Ahci(Box <dyn AhciBarRegion>),
    Ixgbe(Box <dyn IxgbeBarRegion>),
    None
}

#[derive(Copy, Clone, Debug)]
pub enum PciDrivers {
    IxgbeDriver,
    AhciDriver,
}
