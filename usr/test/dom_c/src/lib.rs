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

struct DomC {
    x: usize,
}

impl DomC {
    fn new() -> Self {
        Self {
            x: 0,
        }
    }
}

impl usr::dom_c::DomC for DomC {
    fn foo(&mut self, x: usize) -> usize {
        self.x += x;
        self.x
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::dom_c::DomC> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("Init domain C");

    Box::new(DomC::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain C panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
