#![no_std]
extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use libsyscalls;
use usr::tpm::TpmDev;
use libtpm::*;

use syscalls::{Syscall, Heap};
use core::panic::PanicInfo;
use console::println;

const DOMAIN_NAME: &str = "testtpm";

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, tpm: Box<dyn TpmDev + Send + Sync>) {
    libsyscalls::syscalls::init(s);

    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Initalizing domain: {}", DOMAIN_NAME);

    // Add TPM2 functions here!
    // tpm_test_read_pcr(tpm);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain {} panic: {:?}", DOMAIN_NAME, info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
