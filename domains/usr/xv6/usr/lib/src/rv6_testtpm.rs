#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::{eprintln, println};
use alloc::boxed::Box;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(rv6: Box<dyn interface::rv6::Rv6>, args: &str) {
    println!("Starting rv6 testtpm with args: {}", args);

    libbenchtpm::test_tpm(&*rv6.get_usrtpm().unwrap());
}
