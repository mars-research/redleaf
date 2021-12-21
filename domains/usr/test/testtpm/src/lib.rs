#![no_std]
#![no_main]
extern crate alloc;
use alloc::boxed::Box;
use core::panic::PanicInfo;
use interface::tpm::TpmDev;

use console::println;
use libsyscalls;
use libtpm::*;

use syscalls::{Heap, Syscall};

pub fn main(tpm: Box<dyn TpmDev + Send + Sync>) {
    println!("Initalizing domain: testtpm");

    // Add TPM2 functions here!
    // tpm_test_read_pcr(tpm);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain testtpm panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
