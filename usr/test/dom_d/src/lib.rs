#![no_std]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use rref::{RRef, RRefDeque};
use alloc::vec::Vec;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_c: Box<dyn usr::dom_c::DomC>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("Init domain D");
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
