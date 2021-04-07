#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;

#[macro_use]
use alloc::boxed::Box;

use core::panic::PanicInfo;

use syscalls::{Heap, Syscall};
use interface::rv6::Rv6;
use usrlib::println;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone_rv6().unwrap());
    println!("Starting rv6 benchnet with args: {}", args);

    let mut nvme = rv6.as_nvme().unwrap();

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(
            &mut *nvme, 4096, /*is_write=*/ true, /*is_random=*/ false,
        );
    }

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(
            &mut *nvme, 4096, /*is_write=*/ false, /*is_random=*/ false,
        );
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnvme panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
