#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;

#[macro_use]
use alloc::boxed::Box;
use crate::println;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(rv6: Box<dyn Rv6>, args: &str) {
    println!("Starting rv6 benchnet with args: {}", args);

    let mut nvme = rv6.as_nvme().unwrap();

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(
            &mut *nvme, 4096, /*is_write=*/ true, /*is_random=*/ false,
        );
    }

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(
            &mut *nvme, 4096, /*is_write=*/ false, /*is_random=*/ false,
        );
    }
}
