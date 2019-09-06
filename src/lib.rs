#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(asm)]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;

#[macro_use]
mod console;
mod interrupt;
pub mod banner;
pub mod gdt;

use core::panic::PanicInfo;

#[no_mangle]
pub static mut others_stack: u64 = 0;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    banner::boot_banner();


    gdt::init();
    interrupt::init_idt();

    interrupt::init_irqs();
    x86_64::instructions::interrupts::enable();

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3(); 
     
    println!("boot ok");

    halt();
}

#[no_mangle]
pub extern "C" fn rust_main_others() -> ! {

    gdt::init();
    interrupt::init_idt();

    interrupt::init_irqs();
    x86_64::instructions::interrupts::enable();

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3(); 
     
    println!("booted another CPU ok");

    halt();
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
