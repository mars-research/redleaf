#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

use core::panic::PanicInfo;
use syscalls::syscalls::{Syscall, sys_print};

#[no_mangle]
pub fn init(s: Syscall) {
    syscalls::syscalls::init(s);
    sys_print("init userland");
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
