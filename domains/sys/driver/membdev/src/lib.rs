#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    maybe_uninit_extra
)]

use alloc::boxed::Box;
use core::panic::PanicInfo;

use interface::bdev::BDev;
use syscalls::{Heap, Syscall};

extern crate alloc;
extern crate malloc;

pub fn main(mut memdisk: &'static mut [u8]) -> Box<dyn BDev> {
    #[cfg(feature = "default-memdisk")]
    if memdisk.is_empty() {
        console::println!(
            "an empty memdisk is passed into memdisk. the default memdisk is now being used"
        );
        memdisk = unsafe { libmembdev::get_memdisk() };
    }

    Box::new(libmembdev::MemBDev::new(memdisk))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    console::println!("membdev panicked: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    libsyscalls::syscalls::sys_test_unwind();
    loop {}
}
