#![no_std]
#![no_main]
extern crate x86;

//#[macro_use]
//mod serial;
use core::panic::PanicInfo;

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    //printsln!("Hello, World!");
    loop {}
}

