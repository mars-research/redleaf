use core::marker::PhantomData;
use core::ops::Deref;
use core::{mem, slice};
use num_traits::int::PrimInt;

// A RawMemorySpace behaves roughly like a Rust pointer.
#[derive(Debug)]
pub struct RawMemoryRegion<T> {
    // Private fields
    start: usize,
    length: usize,
    cur: usize,
    _phantom: PhantomData<T>,
}

impl<T> RawMemoryRegion<T> {
    pub unsafe fn new(start: usize, length: usize) -> RawMemoryRegion<T> {
        RawMemoryRegion {
            start,
            length,
            cur: start,
            _phantom: PhantomData,
        }
    }

    pub fn offset(&self, offset: isize) -> Option<RawMemoryRegion<T>> {
        let istart = self.start as isize;
        let icur = self.cur as isize;
        let tsize: isize = mem::size_of::<T>() as isize;
        let new_cur = (icur + offset) as usize;
        let new_offset = icur + offset + tsize - istart;

        if new_cur < self.start || new_offset > (self.length as isize) {
            None
        } else {
            Some(RawMemoryRegion {
                start: self.start,
                length: self.length,
                cur: new_cur,
                _phantom: PhantomData,
            })
        }
    }
}

// Primitive integer types
impl<T: PrimInt> Deref for RawMemoryRegion<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.cur as *const T) }
    }
}

impl<T: PrimInt> RawMemoryRegion<T> {
    pub fn as_slice(&self) -> &[T] {
        let tsize: usize = mem::size_of::<T>();
        let count = self.length / tsize;
        unsafe { slice::from_raw_parts(self.start as *const T, count) }
    }

    pub fn as_slice_mut(&self) -> &mut [T] {
        let tsize: usize = mem::size_of::<T>();
        let count = self.length / tsize;
        unsafe { slice::from_raw_parts_mut(self.start as *mut T, count) }
    }
}
