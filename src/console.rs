#[macro_use]
mod serial;
mod vga; 

use core::fmt::{Write};
use crate::console::vga::WRITER;
use crate::console::serial::{SERIAL1, EMERGENCY_SERIAL1};
use x86_64::instructions::interrupts;

pub static mut IN_A_CRASH: bool = false; 

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        if IN_A_CRASH {
                EMERGENCY_SERIAL1.write_fmt(args).unwrap();
        }
    }

    if x86_64::instructions::interrupts::are_enabled() {
        unsafe {
            IN_A_CRASH = true;
        };

        crate::interrupt::disable_irq();

        println!("Interrupts are enabled"); 
        x86_64::instructions::interrupts::int3();
    }

    // We don't need interrupts off any more, inside the 
    // kernel interrupts are off all the time
    WRITER.lock().write_fmt(args).unwrap();
    SERIAL1.lock().write_fmt(args).unwrap(); 
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::console::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _eprint(args: core::fmt::Arguments) {
    // We don't need interrupts off any more, inside the 
    // kernel interrupts are off all the time

    //WRITER.force_unlock().write_fmt(args).unwrap();
    //SERIAL1.force_unlock().write_fmt(args).unwrap();
}
