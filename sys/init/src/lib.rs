#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions,
    naked_functions,
    panic_info_message
)]

use spin::Once;
use syscalls::syscalls::{Syscall};
use core::panic::PanicInfo;

static SYSCALL: Once<Syscall> = Once::new();

#[no_mangle]
pub extern fn init(s: Syscall) {
    SYSCALL.call_once(|| s);
    (s.sys_print)("init userland");
    foobar();
}

pub extern fn foobar() -> ! {
    loop {
        //(s.sys_print)("init");
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


