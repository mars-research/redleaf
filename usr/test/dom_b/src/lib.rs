#![no_std]
extern crate malloc;
extern crate alloc;
use syscalls::{Syscall, Heap};
use libsyscalls;
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use rref::RRef;
use usr::dom_a::DomA;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_a: Box<dyn DomA>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("In domain B");

    let mut buffer = RRef::<[u8; 1024]>::new([0;1024]);
    for i in 0..1024 {
        buffer[i] = (i % 256) as u8;
    }
    println!("before pingpong");
    println!("---------------");
    for i in 0..1024 {
        println!("buffer[{}]: {}", i, buffer[i]);
    }
    println!("---------------");
    buffer = dom_a.ping_pong(buffer);
    println!("after pingpong");
    println!("---------------");
    for i in 0..1024 {
        println!("buffer[{}]: {}", i, buffer[i]);
    }
    println!("---------------");
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain B panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
