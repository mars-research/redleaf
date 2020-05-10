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

struct DomC {}

impl DomC {
    fn new() -> Self {
        Self {}
    }
}

impl usr::dom_c::DomC for DomC {
    fn no_arg(&self) {}

    fn one_arg(&self, x: usize) -> Result<usize, i64> {
        #[cfg(feature = "unwind")]
        libsyscalls::syscalls::sys_test_unwind();
        Ok(x + 1)
    }

    fn one_rref(&self, mut x: RRef<usize>) -> RRef<usize> {
        *x += 1;
        x
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
