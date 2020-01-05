use x86::io::{outb, inb};



pub struct SerialPort {
    base_port: u16
}

impl SerialPort {
    pub const fn new(base_port: u16) -> SerialPort {
        SerialPort {
            base_port
        }
    }

    pub fn init(&self) {
        unsafe {
            outb(self.base_port+1, 0x00); // Disable interrupts
            outb(self.base_port+3, 0x80); // Set baud rate divisor
            outb(self.base_port+0, 0x01); // Set baud rate to 115200 baud
            outb(self.base_port+1, 0x00); // 
            outb(self.base_port+3, 0x03); // 8 bits, no parity, one stop bit
            outb(self.base_port+2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
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

use core::fmt::{Write};

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

use spin::Mutex;

pub static mut EMERGENCY_SERIAL1: SerialPort = SerialPort::new(0x3F8);

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        unsafe {
            let serial_port = SerialPort::new(0x3F8);
            serial_port.init();
            Mutex::new(serial_port)
        }
    };
}


