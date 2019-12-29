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
    panic_info_message,
    const_vec_new
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
extern crate spin;
#[macro_use]
extern crate lazy_static;
extern crate syscalls;
extern crate tls;

use alloc::boxed::Box;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_alloc, sys_create_thread, sys_println};
use syscalls::Syscall;

mod bcache;
mod block;
mod directory;
mod fcntl;
mod file;
mod fs;
mod log;
mod params;
mod sysfile;

struct VFS {}

impl VFS {
    fn new() -> VFS {
        VFS{}
    }
}

impl syscalls::VFS for VFS {}


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, bdev: Box<dyn syscalls::BDev>) -> Box<dyn syscalls::VFS> {
    libsyscalls::syscalls::init(s);


    println!("init xv6 filesystem");

    Box::new(VFS::new()) 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
