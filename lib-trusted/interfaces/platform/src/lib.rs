#![no_std]

mod bar_addr;

pub use bar_addr::*;
pub type MmioAddr = PciBarAddr;
