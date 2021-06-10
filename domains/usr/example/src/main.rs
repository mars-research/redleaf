#![no_std]
#![no_main]
#![feature(
    global_asm,
    box_syntax,
    type_ascription,
)]

use interface::proxy;

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::panic::PanicInfo;
use interface::domain_create;
use libsyscalls;
use interface::rref;
use syscalls;

struct Example {}

impl interface::example::Example for Example {
    fn method(&self, a: u32) -> u8 {
        2
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
) -> Arc<dyn interface::example::Example> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    Arc::new(Example{})
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("example panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
