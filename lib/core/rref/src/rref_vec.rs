// although unsafe function's don't need unsafe blocks, it helps readability
#![allow(unused_unsafe)]
use crate::traits::{RRefable, TypeIdentifiable, CustomCleanup};
use crate::rref::RRef;

use alloc::boxed::Box;
use core::ops::{Deref, DerefMut, Drop};
use core::alloc::Layout;
use spin::Once;



pub struct RRefVec<T> where T: 'static + RRefable + Copy {
    data: RRef<T>,
    size: usize,
}

unsafe impl<T: RRefable + Copy> RRefable for RRefVec<T> {}
unsafe impl<T: RRefable + Copy> Send for RRefVec<T> where T: Send {}

impl<T: RRefable + Copy + TypeIdentifiable> RRefVec<T> where T: Copy {
    pub fn new(initial_value: T, size: usize) -> Self {
        let layout = Layout::array::<T>(size).unwrap();
        let data = unsafe { RRef::new_with_layout(initial_value, layout) };
        let mut vec = Self {
            data,
            size
        };
        for e in vec.as_mut_slice() {
            *e = initial_value;
        }
        vec
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data.ptr_mut(), self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.data.ptr_mut(), self.size) }
    }
}

impl<T: RRefable + Copy> Drop for RRefVec<T> {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl<T: 'static + RRefable + Copy> CustomCleanup for RRefVec<T> {
    fn cleanup(&mut self) {
        self.data.cleanup()
    }
}

impl<T: RRefable + Copy> Deref for RRefVec<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T: RRefable + Copy> DerefMut for RRefVec<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}
