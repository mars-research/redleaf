use core::ptr::{read_volatile, write_volatile};
use core::mem::MaybeUninit;
use core::ops::{BitAnd, BitOr, Not};

use super::io::Io;

#[repr(packed)]
pub struct Mmio<T> {
    value: MaybeUninit<T>,
}

impl<T> Mmio<T> {
    /// Create a new Mmio without initializing
    pub fn new() -> Self {
        Mmio {
            value: MaybeUninit::uninit()
        }
    }
}

impl<T> Io for Mmio<T> where T: Copy + PartialEq + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T> {
    type Value = T;

    fn read(&self) -> T {
        unsafe { read_volatile(self.value.as_ptr()) }
    }

    fn write(&mut self, value: T) {
        unsafe { write_volatile(self.value.as_mut_ptr(), value) };
    }
}
