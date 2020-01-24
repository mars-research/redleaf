#![no_std]
use core::fmt::{Write};
use spin::Mutex;
use libsyscalls::syscalls::sys_print;

pub static CONSOLE: Mutex<Console> = Mutex::new(Console {});

pub struct Console {
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        sys_print(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("cpu({}):{}\n", libsyscalls::syscalls::sys_cpuid(), format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    CONSOLE.lock().write_fmt(args).unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
