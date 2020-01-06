#[macro_use]
mod serial;
mod vga; 

use core::fmt::{Write};
use crate::console::vga::WRITER;
use crate::console::serial::{SERIAL1, EMERGENCY_SERIAL1};

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
                return; 
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

// The debug version
#[cfg(trace_sched)]
macro_rules! trace_sched {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(trace_sched))]
macro_rules! trace_sched {
    ($( $args:expr ),*) => {()}
}

// The debug version
#[cfg(trace_wq)]
macro_rules! trace_wq {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(trace_wq))]
macro_rules! trace_wq {
    ($( $args:expr ),*) => {()}
}

// The debug version
#[cfg(feature="trace_alloc")]
macro_rules! trace_alloc {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(feature="trace_alloc"))]
macro_rules! trace_alloc {
    ($( $args:expr ),*) => {()}
}


