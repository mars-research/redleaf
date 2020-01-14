#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;

use core::panic::PanicInfo;
use syscalls::Syscall;
use libsyscalls::syscalls::{sys_println, sys_alloc};
use usr::xv6::Xv6;
use console::println;
use alloc::boxed::Box;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, xv6: Box<dyn Xv6>) {
    libsyscalls::syscalls::init(s);

    sys_println("xv6 shell domain");


}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
