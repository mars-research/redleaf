#[macro_use]
mod serial;
mod vga; 

use core::fmt::{Write};
use crate::console::vga::WRITER;
use crate::console::serial::SERIAL1;

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
	use x86_64::instructions::interrupts;

	interrupts::without_interrupts(|| {
    	WRITER.lock().write_fmt(args).unwrap();
	    SERIAL1.lock().write_fmt(args).unwrap(); 
	});
}
