#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;

use console::println;

use core::panic::PanicInfo;

use interface::rref::RRef;

use interface::rpc::RpcResult;

struct DomC {}

impl DomC {
    fn new() -> Self {
        Self {}
    }
}

impl interface::dom_c::DomC for DomC {
    fn no_arg(&self) -> RpcResult<()> {
        Ok(())
    }

    fn one_arg(&self, x: usize) -> RpcResult<usize> {
        #[cfg(feature = "unwind")]
        {
            let start = libtime::get_rdtsc();
            assert!((start & 0x100) != 0x100);
        }
        Ok(x + 1)
    }

    fn one_rref(&self, mut x: RRef<usize>) -> RpcResult<RRef<usize>> {
        *x += 1;
        Ok(x)
    }

    fn init_dom_c(&self, c: Box<dyn interface::dom_c::DomC>) -> RpcResult<()> {
        Ok(())
    }
}

pub fn main() -> Box<dyn interface::dom_c::DomC> {
    println!("Init domain C");

    Box::new(DomC::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain C panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    libsyscalls::syscalls::sys_test_unwind();
    loop {}
}
