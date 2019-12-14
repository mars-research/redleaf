#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

use spin::Once;
use syscalls::syscalls::{Syscall};
use core::panic::PanicInfo;

static SYSCALL: Once<Syscall> = Once::new();

#[no_mangle]
pub fn init(s: Syscall) {
    SYSCALL.call_once(|| s);
    (s.sys_print)("init userland");
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
