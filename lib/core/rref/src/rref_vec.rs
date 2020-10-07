// although unsafe function's don't need unsafe blocks, it helps readability
#![allow(unused_unsafe)]
use crate::traits::{RRefable, TypeIdentifiable, CustomCleanup};
use crate::rref::RRef;

use alloc::boxed::Box;
use core::ops::{Deref, DerefMut, Drop};
use core::alloc::Layout;
use core::mem::{MaybeUninit, ManuallyDrop};
use spin::Once;


/// `RRef`ed runtime constant size array.
/// This allow us to pass array across domains without having
/// its size being limited at complie time like in RRefArray.
/// 
/// Currently, it only support Copy types since we only need
/// it for passing byte arrays around. We will later merge it
/// with RRefArray when we have time.
pub struct RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable {
    data: RRef<T>,
    size: usize,
}

unsafe impl<T> RRefable for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {}
unsafe impl<T> Send for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {}

impl<T> RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {
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

    pub fn from_slice(slice: &[T]) -> Self {
        let size = slice.len();
        let layout = Layout::array::<T>(size).unwrap();
        let data = unsafe { RRef::new_with_layout(MaybeUninit::uninit().assume_init(), layout) };
        let mut vec = Self {
            data,
            size
        };
        for (dest, src) in vec.as_mut_slice().iter_mut().zip(slice) {
            *dest = *src;
        }
        vec
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(&*self.data, self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(&mut *self.data, self.size) }
    }

    pub fn move_to(&self, new_domain_id: u64) {
        self.data.move_to(new_domain_id);
    }

    pub fn borrow(&self) {
        self.data.borrow();
    }

    pub fn forfeit(&self) {
        self.data.forfeit();
    }
}

impl<T> Drop for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Drop every elements in the array and 
/// `RRef::cleanup` will drop the first element and deallocate the arry.
/// So our job here is to drop every element, besides the first element,
/// in the array.
impl<T> CustomCleanup for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {
    fn cleanup(&mut self) {
        assert!(self.size > 0);
        for e in self.as_mut_slice().iter_mut().skip(1) {
            e.cleanup();
        }
    }
}

impl<T> Deref for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for RRefVec<T> where T: 'static + RRefable + Copy + TypeIdentifiable  {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}
