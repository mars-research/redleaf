#![no_std]
//#![no_main]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;

#[macro_use]
mod console;

use core::panic::PanicInfo;

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello, World (vga and serial)!");

    //use core::fmt::Write;
    //vga::WRITER.lock().write_str("Hello again").unwrap();
    //write!(vga::WRITER.lock(), ", some numbers: {} {}", 42, 1.337).unwrap();
    
    loop {}
}

