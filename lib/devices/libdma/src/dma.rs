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

use core::{mem};
use core::ops::{Deref, DerefMut};
use alloc::boxed::Box;

use libsyscalls::errors::Result;

pub struct Dma<T> {
    value: Box<T>,
}

impl<T> Dma<T> {
    pub fn new(value: T) -> Result<Dma<T>> {
        Ok(Dma {
            value: Box::new(value),
        })
    }

    pub unsafe fn zeroed() -> Result<Dma<T>> {
        Ok(Dma {
            value: Box::new(mem::zeroed()),
        })
    }

    pub fn physical(&self) -> usize {
        let tr: &T = &self.value;
        tr as *const _ as usize
    }

    pub fn from_box(b: Box<T>) -> Dma<T> {
        Dma {
            value: b,
        }
    }
}

impl<T> Deref for Dma<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Dma<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

/*
impl<T> Drop for Dma<T> {
    fn drop(&mut self) {
        unsafe { drop(ptr::read(self.virt)); }
        let _ = unsafe { crate::physunmap(self.virt as usize) };
    }
}
*/
