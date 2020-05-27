#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;

#[macro_use]
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::panic::PanicInfo;

use syscalls::{Syscall, Heap};
use usrlib::{println, print};
use usr::xv6::Xv6;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone());
    println!("Starting rv6 benchnet with args: {}", args);

    let mut nvme = rv6.as_nvme();

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(&mut *nvme, 4096,
                                    /*is_write=*/true, /*is_random=*/false);
    }

    for _ in 0..=6 {
        let _ = libbenchnvme::run_blocktest_rref(&mut *nvme, 4096,
                                    /*is_write=*/false, /*is_random=*/false);
    }
}


// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnvme panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
