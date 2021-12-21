#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use console::println;
use core::panic::PanicInfo;
use interface::bdev::NvmeBDev;
use syscalls::{Heap, Syscall};

pub fn main(mut nvme: Box<dyn NvmeBDev>) {
    println!("Init domain benchnvme_inside (╯°□°）╯︵ ┻━┻ ^_^ ʕ·͡ᴥ·ʔ (҂◡_◡) ᕤ (┬┬﹏┬┬) ノ┬─┬ノ ︵ ( \\o°o)\\");

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
