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
use rref::RRef;

struct DomA {}

impl DomA {
    fn new() -> Self {
        Self {
        }
    }
}

impl usr::dom_a::DomA for DomA {
    fn ping_pong(&self, buffer: RRef<[u8; 1024]>) -> RRef<[u8; 1024]> {
        println!("[dom_a]: ping pong");
        buffer
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::dom_a::DomA> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("In domain A");

    Box::new(DomA::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain A panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
