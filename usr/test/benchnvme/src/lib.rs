#![no_std]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::{println, print};
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use alloc::vec::Vec;
use usr::bdev::NvmeBDev;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, mut nvme: Box<dyn NvmeBDev>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain benchnet_inside");

    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/false, /*is_random=*/false);
    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/false, /*is_random=*/false);

    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/false, /*is_random=*/true);
    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/false, /*is_random=*/true);


    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/true, /*is_random=*/false);
    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/true, /*is_random=*/false);

    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/true, /*is_random=*/true);
    libbenchnvme::run_blocktest_rref(&mut *nvme, 4096, /*is_write=*/true, /*is_random=*/true);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
