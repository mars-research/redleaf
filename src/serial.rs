pub struct SerialPort {
    base_port: u16
}

use x86::io::{outb, inb};

impl SerialPort {
    pub const unsafe fn new(base_port: u16) -> SerialPort {
        SerialPort {
            base_port
        }
    }

    pub fn init(&self) {
        unsafe {
            outb(self.base_port+1, 0x00); // Disable interrupts
            outb(self.base_port+3, 0x00); // Set baud rate divisor
            outb(self.base_port+0, 0x00); // Set baud rate to 38400 baud
            outb(self.base_port+1, 0x00); // 
            outb(self.base_port+3, 0x00); // 8 bits, no parity, one stop bit
            outb(self.base_port+2, 0x00); // Enable FIFO, clear them, with 14-byte threshold
            outb(self.base_port+4, 0x00); // Enable IRQs, RTS/DSR set
            outb(self.base_port+1, 0x00); // Disable Interrupts
        }
    }

pub fn get_lsts(&self) -> u8 {
    unsafe {
        inb(self.base_port + 5) // line status register is on port 5.
    }
}

pub fn send(&self, data: u8) {
    unsafe {
        match data {
            8 | 0x7F => {
                while (!self.get_lsts() & 1) == 0 {}
                outb(self.base_port, 8);
                while (!self.get_lsts() & 1) == 0 {}
                outb(self.base_port, b' ');
                while (!self.get_lsts() & 1) == 0 {}
                outb(self.base_port, 8);
            }
            _ => {
                while (!self.get_lsts() & 1) == 0 {}
                outb(self.base_port, data);
            }
        }
    }
}

}

use core::fmt::{Write, Result};

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

use spin::Mutex;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        unsafe {
            let mut serial_port = SerialPort::new(0x3F8);
            serial_port.init();
            Mutex::new(serial_port)
        }
    };
}

pub fn print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1
        .lock()
        .write_fmt(args)
        .expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! prints {
    ($($arg:tt)*) => {
        $crate::serial::print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! printsln {
    () => (prints!("\n"));
    ($fmt:expr) => (prints!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (prints!(concat!($fmt, "\n"), $($arg)*));
}
