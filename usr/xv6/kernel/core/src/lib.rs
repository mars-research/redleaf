#![no_std]
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
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_create_thread, sys_yield};
use console::println;

struct Xv6Syscalls {}

impl Xv6Syscalls {
    fn new() -> Xv6Syscalls {
        Xv6Syscalls{}
    }
}

impl syscalls::Xv6 for Xv6Syscalls {}

extern fn xv6_kernel_test_th() {
   loop {
        println!("xv6_kernel_test_th"); 
        sys_yield(); 
   }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
            create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
            bdev: Box<dyn syscalls::BDev>) {
    libsyscalls::syscalls::init(s);

    println!("init xv6/core");

    let t = sys_create_thread("xv6_kernel_test_th", xv6_kernel_test_th); 
    t.set_affinity(10); 
    
    let (dom_xv6fs, vfs)  = create_xv6fs.create_domain_xv6fs(bdev);
    
    let xv6 = Box::new(Xv6Syscalls::new()); 

    let dom_shell  = create_xv6usr.create_domain_xv6usr("shell", xv6);

    //println!("thread:{}", t);
    drop(t); 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
