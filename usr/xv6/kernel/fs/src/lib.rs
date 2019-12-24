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
    panic_info_message
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
extern crate spin;
#[macro_use]
extern crate lazy_static;
extern crate syscalls;

use alloc::boxed::Box;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_alloc, sys_create_thread, sys_println};
use syscalls::syscalls::Syscall;

mod bcache;
mod block;
mod directory;
mod fcntl;
mod file;
mod fs;
mod log;
mod params;
mod sysfile;

extern "C" fn foo() {}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    //let b = Box::new(4);
    //let r = sys_alloc();
    let mut v1: Vec<u64> = Vec::with_capacity(1024);
    for i in 0..2048 {
        v1.push(i);
    }

    sys_println("init xv6 filesystem");

    let t = sys_create_thread("trait_test", foo);
    t.set_affinity(10);
    //println!("thread:{}", t);
    drop(t);
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
