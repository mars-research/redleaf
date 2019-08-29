#![no_std]
#![feature(abi_x86_interrupt)]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;

#[macro_use]
mod console;
mod interrupts; 

use core::panic::PanicInfo;
use crate::interrupts::init_idt;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Booting RedLeaf...");

    init_idt();

     // invoke a breakpoint exception
     x86_64::instructions::interrupts::int3(); 
     
    println!("boot ok");
    loop {}
}

