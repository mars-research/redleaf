// Adapted from the Redox OS Project:
//
//   Copyright (c) 2017 Redox OS Developers
//   
//   MIT License
//   
//   Permission is hereby granted, free of charge, to any person obtaining
//   a copy of this software and associated documentation files (the
//   "Software"), to deal in the Software without restriction, including
//   without limitation the rights to use, copy, modify, merge, publish,
//   distribute, sublicense, and/or sell copies of the Software, and to
//   permit persons to whom the Software is furnished to do so, subject to
//   the following conditions:
//   
//   The above copyright notice and this permission notice shall be
//   included in all copies or substantial portions of the Software.
//   
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
//   EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//   MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//   NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
//   LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
//   WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use core::ptr::{read_volatile, write_volatile};
use core::mem::MaybeUninit;
use core::ops::{BitAnd, BitOr, Not};

#[repr(packed)]
pub struct Mmio<T> {
    value: MaybeUninit<T>,
}

impl<T> core::fmt::Debug for Mmio<T> where T: Copy + PartialEq + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T> + core::fmt::Display {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "Mmio{{{}}}", self.read())
    }
}

impl<T> Mmio<T> {
    /// Create a new Mmio without initializing
    pub fn new() -> Self {
        Mmio {
            value: MaybeUninit::uninit()
        }
    }
}

impl<T> Mmio<T> where T: Copy + PartialEq + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T> {
    #[inline(always)]
    pub fn read(&self) -> T {
        unsafe { read_volatile(self.value.as_ptr()) }
    }

    #[inline(always)]
    pub fn write(&mut self, value: T) {
        unsafe { write_volatile(self.value.as_mut_ptr(), value) };
    }

    #[inline(always)]
    pub fn readf(&self, flags: T) -> bool  {
        (self.read() & flags) as T == flags
    }

    #[inline(always)]
    pub fn writef(&mut self, flags: T, value: bool) {
        let tmp: T = match value {
            true => self.read() | flags,
            false => self.read() & !flags,
        };
        self.write(tmp);
    }
}
