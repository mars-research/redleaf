#[macro_use]
mod serial;
mod vga; 

use x86::cpuid::CpuId;
use core::fmt::{Write};
use crate::console::vga::WRITER;
use crate::console::serial::{SERIAL1, EMERGENCY_SERIAL1};

pub static mut IN_A_CRASH: bool = false; 

pub fn unlock_console() {
    unsafe {
        IN_A_CRASH = true;
    };
}

pub fn cpuid() -> u32 {
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    cpu_id
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("cpu({}):{}\n", crate::console::cpuid(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! usrprintln {
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
        crate::interrupt::disable_irq();

        unlock_console(); 

        println!("Interrupts are enabled"); 
        x86_64::instructions::interrupts::int3();
    }

    // We don't need interrupts off any more, inside the 
    // kernel interrupts are off all the time
    WRITER.lock().write_fmt(args).unwrap();
    SERIAL1.lock().write_fmt(args).unwrap(); 
}

// The debug version
#[cfg(feature = "trace_sched")]
macro_rules! trace_sched {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(feature = "trace_sched"))]
macro_rules! trace_sched {
    ($( $args:expr ),*) => {()}
}

// The debug version
#[cfg(feature = "trace_wq")]
macro_rules! trace_wq {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(feature = "trace_wq"))]
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

// The debug version
#[cfg(feature = "trace_vspace")]
macro_rules! trace_vspace {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[cfg(not(feature = "trace_vspace"))]
macro_rules! trace_vspace {
    ($( $args:expr ),*) => {()}
}


