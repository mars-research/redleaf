#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    option_expect_none,
    untagged_unions
)]

pub mod ahci;
mod ata;
pub mod disk;
mod fis;
mod hba;
