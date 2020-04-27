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
use rref;
use usr::dom_a::DomA;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_a: Box<dyn DomA>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("In domain B");
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain B panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
