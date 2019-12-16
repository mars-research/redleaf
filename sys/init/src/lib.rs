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
use console::println;

#[no_mangle]
pub fn init(s: Syscall) {
    syscalls::syscalls::init(s);
    sys_print("init userland");
    sys_print("init userland 2");
    sys_print("init userland 3");

    //println!("init userland print works {}", 4); 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
