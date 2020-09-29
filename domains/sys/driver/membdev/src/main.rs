#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    maybe_uninit_extra,
)]

use alloc::boxed::Box;
use core::panic::PanicInfo;

use syscalls::{Syscall, Heap};
use usr::bdev::BDev;

extern crate alloc;
extern crate malloc;




#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn Heap + Send + Sync>,
            mut memdisk: &'static mut [u8]) -> Box<dyn BDev> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    #[cfg(feature = "default-memdisk")]
    if memdisk.len() == 0 {
        console::println!("an empty memdisk is passed into memdisk. the default memdisk is now being used");
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
