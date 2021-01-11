#![no_std]
#![no_main]
extern crate alloc;
use alloc::boxed::Box;
use core::panic::PanicInfo;
use usr::tpm::TpmDev;

use console::println;
use libsyscalls;
use libtpm::*;

use syscalls::{Heap, Syscall};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    tpm: Box<dyn TpmDev + Send + Sync>,
) {
    libsyscalls::syscalls::init(s);

    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

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
