#![no_std]
extern crate memcpy;

use core::panic::PanicInfo;

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}