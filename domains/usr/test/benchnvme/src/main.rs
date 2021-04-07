#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;

use console::println;

use core::panic::PanicInfo;

use interface::bdev::NvmeBDev;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    mut nvme: Box<dyn NvmeBDev>,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain benchnet_inside");

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
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
