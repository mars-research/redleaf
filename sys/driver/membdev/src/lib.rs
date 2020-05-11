#![no_std]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message,
    maybe_uninit_extra
)]

mod membdev;

use alloc::boxed::Box;
use core::panic::PanicInfo;

use syscalls::{Syscall, Heap};
use usr::bdev::BDev;
use memcpy;

extern crate alloc;
extern crate malloc;



#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn Heap + Send + Sync>) -> Box<dyn BDev> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    Box::new(membdev::MemBDev::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    console::println!("membdev panicked: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    libsyscalls::syscalls::sys_test_unwind();
    loop {}
}
