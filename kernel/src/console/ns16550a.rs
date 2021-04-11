
//! A simple poll-only NS16550A driver.
//!
//! May be used in MMIO mode or Port IO mode (X86 only). Initializes the
//! serial to 38400 baud. Does not do funny stuff with the bytes you send
//! (*cough* uart_16550 *cough*).
//!
//! `MmioSerial` and `PioSerial` (X86 only) have the same interface.
//! Some quick hints for the impatient:
//! - On `qemu-system-riscv64 -machine virt` the MMIO base of
//! the serial port is 0x1000_0000.
//! - On X86, the Port IO base of COM1 is 0x3F8.
//!
//! The MMIO remmants were from when Ace was a hobbyist kernel
//! also targeting RISC-V (and also GBA!)
//!
//! # Example
//! ```
//! let serial = unsafe { MmioSerial::new(0x1000_0000) };
//! serial.init();
//! writeln!(serial, "meow");
//! serial.putc('?' as u8);
//! let cat = serial.getc();
//! ```
//!
//! References:
//! - https://osblog.stephenmarz.com/ch2.html
//! - https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/uart.c
//! - http://byterunner.com/16550.htm

use core::fmt::{Write, Result as FmtResult};
/// Receive Holding Register
const RHR: usize = 0;

/// Transmit Holding Register
const THR: usize = 0;

/// Interrupt Enable Register
const IER: usize = 1;

/// FIFO Control Register
const FCR: usize = 2;

// /// Interrupt Status Register
// const ISR: usize = 2;

/// Line Control Register
const LCR: usize = 3;

/// Line Status Register
const LSR: usize = 5;

// Shared logic
macro_rules! impl_serial {
    ($name:ident, $write:ident, $read:ident) => {

pub struct $name {
    base: usize,
}

impl $name {
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            base: base as usize,
        }
    }

    /// Initializes the serial port.
    ///
    /// Currently hardcoded to do 38400 8N1 for QEMU :/
    pub fn init(&self) {
        unsafe {
            // Disable interrupts
            $write(self.base, IER, 0x00);

            // Enable DLAB to set baud rate
            $write(self.base, LCR, 0x80);

            // Set 38400 baud
            $write(self.base, 0, 0x03);
            $write(self.base, 1, 0x00);

            // Disable DLAB, and 8N1
            $write(self.base, LCR, 0x03);

            // Reset and enable FIFOs
            $write(self.base, FCR, 0x07);
        }
    }

    /// Puts a byte to the serial port.
    pub fn putc(&mut self, c: u8) {
        unsafe {
            while $read(self.base, LSR) & (1 << 5) == 0 {
            }

            $write(self.base, THR, c);
        }
    }

    /// Reads a byte from the serial port.
    ///
    /// Returns `None` if there the input buffer is empty.
    pub fn getc(&mut self) -> Option<u8> {
        unsafe {
            if $read(self.base, LSR) & 0x1 == 1 {
                Some($read(self.base, RHR))
            } else {
                None
            }
        }
    }

    /// Reads a byte from the serial port (blocking).
    pub fn getc_blocking(&self) -> u8 {
        unsafe {
            while $read(self.base, LSR) & 0x1 == 0 {
            }

            $read(self.base, RHR)
        }
    }
}

impl Write for $name {
    fn write_str(&mut self, s: &str) -> FmtResult {
        for byte in s.as_bytes() {
            self.putc(*byte);
        }
        Ok(())
    }
}

    };
}

/*
// MMIO-specific
impl_serial!(MmioSerial, mmio_write, mmio_read);
​
#[allow(dead_code)]
#[inline(always)]
unsafe fn mmio_write(base: usize, reg: usize, value: u8) {
    let ptr = (base + reg) as *mut u8;
    core::ptr::write_volatile(ptr, value);
}
​
#[allow(dead_code)]
#[inline(always)]
unsafe fn mmio_read(base: usize, reg: usize) -> u8 {
    let ptr = (base + reg) as *mut u8;
    core::ptr::read_volatile(ptr)
}
*/

// PIO-specific
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl_serial!(PioSerial, pio_write, pio_read);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
unsafe fn pio_write(base: usize, reg: usize, value: u8) {
    llvm_asm!("out dx, al" :: "{dx}"(base + reg), "{al}"(value) :: "intel", "volatile");
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
unsafe fn pio_read(base: usize, reg: usize) -> u8 {
    let mut result: u8;
    llvm_asm!("in al, dx" : "={al}"(result) : "{dx}"(base + reg) :: "intel", "volatile");
    result
}