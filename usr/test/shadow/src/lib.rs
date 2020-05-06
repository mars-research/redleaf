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

struct Shadow {
    dom_c: Box<dyn usr::dom_c::DomC>
}

impl Shadow {
    fn new(dom_c: Box<dyn usr::dom_c::DomC>) -> Self {
        Self {
            dom_c,
        }
    }
}

impl usr::dom_c::DomC for Shadow {
    fn foo(&mut self, x: usize) -> usize {
        self.dom_c.foo(x)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_c: Box<dyn usr::dom_c::DomC>) -> Box<dyn usr::dom_c::DomC> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("Init shadow domain");

    Box::new(Shadow::new(dom_c))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
