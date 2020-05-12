#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

mod rv6_syscalls;
mod thread;

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use core::panic::PanicInfo;

use console::println;
use libsyscalls::syscalls::{sys_current_thread, sys_yield, sys_recv_int};
use rref;
use syscalls::{Syscall, Heap};
use usr_interface::bdev::BDev;
use usr_interface::xv6::{Xv6, Thread};
use usr_interface::vfs::{VFS, FileMode};


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn Heap + Send + Sync>,
            ints: Box<dyn syscalls::Interrupt + Send + Sync>,
            create_xv6fs: Arc<dyn create::CreateXv6FS>,
            create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
            bdev: Box<dyn BDev + Send + Sync>)
{
   
    libsyscalls::syscalls::init(s);
    libsyscalls::syscalls::init_interrupts(ints);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("init xv6/core");

    // Init fs
    let (_dom_xv6fs, fs)  = create_xv6fs.create_domain_xv6fs(bdev);
    // Init kernel
    let rv6 = box rv6_syscalls::Rv6Syscalls::new(create_xv6usr, fs.clone()); 

    rv6.sys_spawn_domain("/init", "/init", array_init::array_init(|_| None)).unwrap();
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6kernel panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
