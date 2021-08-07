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

static mut SELF_BOX: Option<*const Box<dyn interface::dom_c::DomC>> = None;

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
        if *x <= 1 {
            *x = 42;
            Ok(x)
        } else {
            *x -= 1;
            unsafe {
                let stolen_box = SELF_BOX.as_ref().unwrap();
                stolen_box.as_ref().unwrap().one_rref(x)
            }
        }
    }

    fn init_dom_c(&self, c: *const Box<dyn interface::dom_c::DomC>) -> RpcResult<()> {
        unsafe {
            SELF_BOX.replace(c);
        }
        Ok(())
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
) -> Box<dyn interface::dom_c::DomC> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

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
