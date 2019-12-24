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
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread, sys_yield};
use console::println;

extern fn foo() {
   loop {
        println!("User init thread"); 
        sys_yield(); 
   }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    
    //let b = Box::new(4);
    //let r = sys_alloc();
    let mut v1: Vec<u64> = Vec::with_capacity(1024);
    for i in 0..2048 {
        v1.push(i);
    }

    println!("{} {} {}", "init", "userland", 1);

    println!("init userland print works");
    let t = sys_create_thread("init_thread", foo); 
    t.set_affinity(10); 
    //println!("thread:{}", t);
    drop(t); 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
