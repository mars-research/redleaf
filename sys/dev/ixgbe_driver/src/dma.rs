use core::{mem, ptr};
use core::ops::{Deref, DerefMut};
use alloc::boxed::Box;
use crate::Result;

pub struct Dma<T> {
    value: Box<T>,
}

impl<T> Dma<T> {
    pub fn new(value: T) -> Result<Dma<T>> {
        Ok(Dma {
            value: Box::new(value),
        })
    }

    pub fn zeroed() -> Result<Dma<T>> {
        Ok(Dma {
            value: Box::new(unsafe { mem::zeroed() }),
            // value: unsafe { mem::zeroed() },
        })
    }

    pub fn physical(&self) -> usize {
        let tr: &T = &self.value;
        tr as *const _ as usize
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
