//! A slab allocator implementation for small objects
//! (< architecture page size).
//!
//! The organization is as follows (top-down):
//!
//!  * A `ZoneAllocator` manages many `SCAllocator` and can
//!    satisfy requests for different allocation sizes.
//!  * A `SCAllocator` allocates objects of exactly one size.
//!    It holds its data in a ObjectPageList.
//!  * A `ObjectPage` contains allocated objects and associated meta-data.
//!  * A `PageProvider` is provided by the client and used by the
//!    SCAllocator to allocate ObjectPage.
//!
#![allow(unused_features, dead_code, unused_variables)]
#![cfg_attr(feature = "unstable", feature(const_fn))]
#![cfg_attr(test, feature(prelude_import, test, raw, libc, c_void_variant))]
#![no_std]
#![crate_type = "lib"]

extern crate spin;
#[macro_use]
extern crate log;

#[cfg(test)]
extern crate env_logger;

#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
extern crate libc;
#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate test;
#[cfg(test)]
mod tests;

use core::alloc::{GlobalAlloc, Layout};
use core::fmt;
use core::mem;
use core::ptr;
use spin::Mutex;

#[cfg(target_arch = "x86_64")]
const CACHE_LINE_SIZE: usize = 64;

#[cfg(target_arch = "x86_64")]
const BASE_PAGE_SIZE: usize = 4096;

#[cfg(target_arch = "x86_64")]
type VAddr = usize;

const MAX_SIZE_CLASSES: usize = 10;

pub struct SafeZoneAllocator(Mutex<ZoneAllocator<'static>>);

impl SafeZoneAllocator {
    #[cfg(feature = "unstable")]
    pub const fn new(provider: &'static Mutex<dyn PageProvider>) -> SafeZoneAllocator {
        SafeZoneAllocator(Mutex::new(ZoneAllocator::new(provider)))
    }
    #[cfg(not(feature = "unstable"))]
    pub fn new(provider: &'static Mutex<dyn PageProvider>) -> SafeZoneAllocator {
        SafeZoneAllocator(Mutex::new(ZoneAllocator::new(provider)))
    }
}

unsafe impl GlobalAlloc for SafeZoneAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align().is_power_of_two());
        self.0.lock().allocate(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        //let ptr = NonNull::new_unchecked(ptr);
        self.0.lock().deallocate(ptr, layout);
    }
}

/// The memory backing as used by the SCAllocator.
///
/// A client that wants to use the zone or size class allocators
/// has to provide this interface and stick an implementation of it
/// into every SCAllocator.
pub trait PageProvider<'a>: Send {
    fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>>;
    fn release_page(&mut self, &'a mut ObjectPage<'a>);
}

/// A zone allocator.
///
/// Has a bunch of size class allocators and can serve
/// allocation requests for many different (MAX_SIZE_CLASSES) object sizes
/// (by selecting the right slab allocator).
pub struct ZoneAllocator<'a> {
    pager: &'a Mutex<dyn PageProvider<'a>>,
    slabs: [SCAllocator<'a>; MAX_SIZE_CLASSES],
}

impl<'a> ZoneAllocator<'a> {
    pub const MAX_ALLOC_SIZE: usize = 4032;

    #[cfg(feature = "unstable")]
    pub const fn new(pager: &'a Mutex<dyn PageProvider<'a>>) -> ZoneAllocator<'a> {
        ZoneAllocator {
            pager: pager,
            slabs: [
                SCAllocator::new(8, pager),
                SCAllocator::new(16, pager),
                SCAllocator::new(32, pager),
                SCAllocator::new(64, pager),
                SCAllocator::new(128, pager),
                SCAllocator::new(256, pager),
                SCAllocator::new(512, pager),
                SCAllocator::new(1024, pager),
                SCAllocator::new(2048, pager),
                SCAllocator::new(4032, pager),
            ],
        }
    }
    #[cfg(not(feature = "unstable"))]
    pub fn new(pager: &'a Mutex<dyn PageProvider<'a>>) -> ZoneAllocator<'a> {
        ZoneAllocator {
            pager: pager,
            slabs: [
                SCAllocator::new(8, pager),
                SCAllocator::new(16, pager),
                SCAllocator::new(32, pager),
                SCAllocator::new(64, pager),
                SCAllocator::new(128, pager),
                SCAllocator::new(256, pager),
                SCAllocator::new(512, pager),
                SCAllocator::new(1024, pager),
                SCAllocator::new(2048, pager),
                SCAllocator::new(4032, pager),
            ],
        }
    }

    /// Return maximum size an object of size `current_size` can use.
    ///
    /// Used to optimize `realloc`.
    fn get_max_size(current_size: usize) -> Option<usize> {
        match current_size {
            0..=8 => Some(8),
            9..=16 => Some(16),
            17..=32 => Some(32),
            33..=64 => Some(64),
            65..=128 => Some(128),
            129..=256 => Some(256),
            257..=512 => Some(512),
            513..=1024 => Some(1024),
            1025..=2048 => Some(2048),
            2049..=4032 => Some(4032),
            _ => None,
        }
    }

    /// Figure out index into zone array to get the correct slab allocator for that size.
    fn get_slab_idx(requested_size: usize) -> Option<usize> {
        match requested_size {
            0..=8 => Some(0),
            9..=16 => Some(1),
            17..=32 => Some(2),
            33..=64 => Some(3),
            65..=128 => Some(4),
            129..=256 => Some(5),
            257..=512 => Some(6),
            513..=1024 => Some(7),
            1025..=2048 => Some(8),
            2049..=4032 => Some(9),
            _ => None,
        }
    }

    /// Tries to locate a slab allocator.
    ///
    /// Returns either a index into the slab array or None in case
    /// the requested allocation size can not be satisfied by
    /// any of the available slabs.
    fn try_acquire_slab(&mut self, size: usize) -> Option<usize> {
        ZoneAllocator::get_slab_idx(size).map(|idx| {
            if self.slabs[idx].size == 0 {
                self.slabs[idx].size = size;
            }
            idx
        })
    }

    /// Refills the SCAllocator in slabs at `idx` with a ObjectPage.
    ///
    /// # TODO
    ///  * Panics in case we're OOM (should probably return error).
    fn refill_slab_allocator<'b>(&'b mut self, idx: usize) {
        match self.pager.lock().allocate_page() {
            Some(new_head) => self.slabs[idx].insert_slab(new_head),
            None => panic!("OOM"),
        };
    }

    /// Allocate a pointer to a block of memory of size `size` with alignment `align`.
    ///
    /// Can return None in case the zone allocator can not satisfy the allocation
    /// of the requested size or if we do not have enough memory.
    /// In case we are out of memory we try to refill the slab using our local pager
    /// and re-try the allocation request once more before we give up.
    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        match self.try_acquire_slab(layout.size()) {
            Some(idx) => {
                let mut p = self.slabs[idx].allocate(layout);
                if p.is_null() {
                    self.refill_slab_allocator(idx);
                    p = self.slabs[idx].allocate(layout);
                }
                p
            }
            None => ptr::null_mut(),
        }
    }

    /// Deallocates a pointer to a block of memory previously allocated by `allocate`.
    ///
    /// # Arguments
    ///  * `ptr` - Address of the memory location to free.
    ///  * `old_size` - Size of the block.
    ///  * `align` - Alignment of the block.
    ///
    pub unsafe fn deallocate<'b>(&'b mut self, ptr: *mut u8, layout: Layout) {
        match self.try_acquire_slab(layout.size()) {
            Some(idx) => self.slabs[idx].deallocate(ptr, layout),
            None => panic!(
                "Unable to find slab allocator for size ({}) with ptr {:?}.",
                layout.size(),
                ptr
            ),
        }
    }

    unsafe fn copy(dest: *mut u8, src: *const u8, n: usize) {
        let mut i = 0;
        while i < n {
            *dest.offset(i as isize) = *src.offset(i as isize);
            i += 1;
        }
    }

    /*pub unsafe fn reallocate<'b>(&'b mut self, ptr: *mut u8, old_size: usize, size: usize, align: usize) -> Option<*mut u8> {
        // Return immediately in case we can still fit the new request in the current buffer
        match ZoneAllocator::get_max_size(old_size) {
            Some(max_size) => {
                if max_size >= size {
                    return Some(ptr);
                }
                ()
            },
            None => ()
        };

        // Otherwise allocate, copy, free:
        self.allocate(size, align).map(|new| {
            ZoneAllocator::copy(new, ptr, old_size);
            self.deallocate(NonNull::new_unchecked(ptr as *mut u8), old_size, align);
            new
        })
    }*/
}

/// A list of ObjectPage.
struct ObjectPageList<'a> {
    /// Points to the head of the list.
    head: Option<&'a mut ObjectPage<'a>>,
    /// Number of elements in the list.
    pub elements: usize,
}

impl<'a> ObjectPageList<'a> {
    #[cfg(feature = "unstable")]
    const fn new() -> ObjectPageList<'a> {
        ObjectPageList {
            head: None,
            elements: 0,
        }
    }
    #[cfg(not(feature = "unstable"))]
    fn new() -> ObjectPageList<'a> {
        ObjectPageList {
            head: None,
            elements: 0,
        }
    }

    fn iter_mut<'b>(&'b mut self) -> ObjectPageIterMut<'a> {
        let m = match self.head {
            None => Rawlink::none(),
            Some(ref mut m) => Rawlink::some(*m),
        };
        ObjectPageIterMut { head: m }
    }

    /// Inserts `new_head` at the front of the list.
    fn insert_front<'b>(&'b mut self, mut new_head: &'a mut ObjectPage<'a>) {
        match self.head {
            None => {
                new_head.prev = Rawlink::none();
                self.head = Some(new_head);
            }
            Some(ref mut head) => {
                new_head.prev = Rawlink::none();
                head.prev = Rawlink::some(new_head);
                mem::swap(head, &mut new_head);
                head.next = Rawlink::some(new_head);
            }
        }

        self.elements += 1;
    }

    /// Removes `slab_page` from the list.
    fn remove_from_list<'b, 'c>(&'b mut self, slab_page: &'c mut ObjectPage<'a>) {
        unsafe {
            match slab_page.prev.resolve_mut() {
                None => {
                    self.head = slab_page.next.resolve_mut();
                }
                Some(prev) => {
                    prev.next = match slab_page.next.resolve_mut() {
                        None => Rawlink::none(),
                        Some(next) => Rawlink::some(next),
                    };
                }
            }

            match slab_page.next.resolve_mut() {
                None => (),
                Some(next) => {
                    next.prev = match slab_page.prev.resolve_mut() {
                        None => Rawlink::none(),
                        Some(prev) => Rawlink::some(prev),
                    };
                }
            }
        }

        self.elements -= 1;
    }

    /// Does the list contain `s`?
    fn has_objectpage<'b>(&'b mut self, s: &'a ObjectPage<'a>) -> bool {
        for slab_page in self.iter_mut() {
            if slab_page as *const ObjectPage == s as *const ObjectPage {
                return true;
            }
        }

        false
    }
}

/// Iterate over all the pages inside a slab allocator
struct ObjectPageIterMut<'a> {
    head: Rawlink<ObjectPage<'a>>,
}

impl<'a> Iterator for ObjectPageIterMut<'a> {
    type Item = &'a mut ObjectPage<'a>;

    #[inline]
    fn next(&mut self) -> Option<&'a mut ObjectPage<'a>> {
        unsafe {
            self.head.resolve_mut().map(|next| {
                self.head = match next.next.resolve_mut() {
                    None => Rawlink::none(),
                    Some(ref mut sp) => Rawlink::some(*sp),
                };
                next
            })
        }
    }
}

/// A slab allocator allocates elements of a fixed size.
///
/// It has a list of ObjectPage stored inside `slabs` from which
/// it allocates memory.
pub struct SCAllocator<'a> {
    /// Allocation size.
    size: usize,
    /// Memory backing store, to request new ObjectPage.
    pager: &'a Mutex<dyn PageProvider<'a>>,
    /// List of ObjectPage.
    slabs: ObjectPageList<'a>,
}

#[test]
pub fn iter_empty_list() {
    let mut new_head1: ObjectPage = Default::default();
    let mut l = ObjectPageList::new();
    l.insert_front(&mut new_head1);
    for p in l.iter_mut() {}
}

impl<'a> SCAllocator<'a> {
    /// Create a new SCAllocator.
    #[cfg(feature = "unstable")]
    pub const fn new(size: usize, pager: &'a Mutex<dyn PageProvider<'a>>) -> SCAllocator<'a> {
        // const_assert!(size < (BASE_PAGE_SIZE as usize - CACHE_LINE_SIZE);
        SCAllocator {
            size: size,
            pager: pager,
            slabs: ObjectPageList::new(),
        }
    }
    /// Create a new SCAllocator.
    #[cfg(not(feature = "unstable"))]
    pub fn new(size: usize, pager: &'a Mutex<PageProvider<'a>>) -> SCAllocator<'a> {
        // const_assert!(size < (BASE_PAGE_SIZE as usize - CACHE_LINE_SIZE);
        SCAllocator {
            size: size,
            pager: pager,
            slabs: ObjectPageList::new(),
        }
    }

    /// Return object size of this allocator.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Try to allocate a new ObjectPage and insert it.
    ///
    /// # TODO
    ///  * Amount is currently ignored.
    ///  * Panics on OOM (should return error!)
    fn refill_slab<'b>(&'b mut self, amount: usize) {
        let mut pager = self.pager.lock();
        for i in 0..amount {
            match pager.allocate_page() {
                Some(new_head) => {
                    self.insert_slab(new_head);
                }
                None => panic!("OOM"),
            }
        }
    }

    /// Add a new ObjectPage.
    pub fn insert_slab<'b>(&'b mut self, new_head: &'a mut ObjectPage<'a>) {
        self.slabs.insert_front(new_head);
    }

    /// Tries to allocate a block of memory with respect to the `alignment`.
    ///
    /// Only searches within already allocated slab pages.
    fn try_allocate_from_pagelist<'b>(&'b mut self, layout: Layout) -> *mut u8 {
        let size = self.size;
        for (idx, slab_page) in self.slabs.iter_mut().enumerate() {
            let ptr = slab_page.allocate(layout);
            if !ptr.is_null() {
                return ptr;
            } else {
                continue;
            }
        }

        ptr::null_mut()
    }

    /// Allocates a block of memory with respect to `alignment`.
    ///
    /// In case of failure will try to grow the slab allocator by requesting
    /// additional pages and re-try the allocation once more before we give up.
    pub fn allocate<'b>(&'b mut self, layout: Layout) -> *mut u8 {
        trace!(
            "SCAllocator({}) is trying to allocate {:?}",
            self.size,
            layout
        );
        assert!(layout.size() <= self.size);
        assert!(self.size <= (BASE_PAGE_SIZE as usize - CACHE_LINE_SIZE));
        let new_layout = unsafe { Layout::from_size_align_unchecked(self.size, layout.align()) };
        assert!(new_layout.size() >= layout.size());

        let ptr = self.try_allocate_from_pagelist(new_layout);
        if ptr.is_null() {
            self.refill_slab(1);
            return self.try_allocate_from_pagelist(layout);
        }

        trace!(
            "SCAllocator({}) allocated ptr=0x{:x}",
            self.size,
            ptr as usize
        );
        return ptr;
    }

    /// Deallocates a previously allocated block.
    ///
    /// # Bug
    /// This never releases memory in case the ObjectPage are provided by the zone.
    pub fn deallocate<'b>(&'b mut self, ptr: *mut u8, layout: Layout) {
        trace!(
            "SCAllocator({}) is trying to deallocate ptr = 0x{:x} layout={:?}",
            self.size,
            ptr as usize,
            layout
        );
        assert!(layout.size() <= self.size);

        let page = (ptr as usize) & !(BASE_PAGE_SIZE - 1) as usize;
        let slab_page = unsafe { mem::transmute::<VAddr, &'a mut ObjectPage>(page) };

        assert!(self.size <= (BASE_PAGE_SIZE as usize - CACHE_LINE_SIZE));
        let new_layout = unsafe { Layout::from_size_align_unchecked(self.size, layout.align()) };

        slab_page.deallocate(ptr, new_layout);

        // Drop page in case it is empty and not the last
        if slab_page.is_empty() && self.slabs.elements > 1 {
            self.slabs.remove_from_list(slab_page);
            let mut pager = self.pager.lock();
            pager.release_page(slab_page);
        }
    }
}

/// Holds allocated data.
///
/// Objects life within data and meta tracks the objects status.
/// Currently, `bitfield`, `next` and `prev` pointer should fit inside
/// a single cache-line.
#[repr(packed)]
pub struct ObjectPage<'a> {
    /// Holds memory objects.
    data: [u8; 4096 - 64],

    /// Next element in list (used by `ObjectPageList`).
    next: Rawlink<ObjectPage<'a>>,
    prev: Rawlink<ObjectPage<'a>>,

    /// A bit-field to track free/allocated memory within `data`.
    ///
    /// # Notes
    /// * With only 48 bits we do waste some space at the end of every page for 8 bytes allocations.
    ///   but 12 bytes on-wards is okay.
    bitfield: [u64; 6],
}

impl<'a> Default for ObjectPage<'a> {
    fn default() -> ObjectPage<'a> {
        unsafe { mem::zeroed() }
    }
}

unsafe impl<'a> Send for ObjectPage<'a> {}
unsafe impl<'a> Sync for ObjectPage<'a> {}

impl<'a> fmt::Debug for ObjectPage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ObjectPage")
    }
}

impl<'a> ObjectPage<'a> {
    /// Tries to find a free block of memory that satisfies `alignment` requirement.
    ///
    /// # Notes
    /// * We pass size here to be able to calculate the resulting address within `data`.
    fn first_fit(&self, layout: Layout) -> Option<(usize, usize)> {
        unsafe {
            for (base_idx, b) in self.bitfield.iter().enumerate() {
                let bitval = *b;
                if bitval == u64::max_value() {
                    continue;
                } else {
                    let negated = !bitval;
                    let first_free = negated.trailing_zeros() as usize;
                    let idx: usize = base_idx * 64 + first_free;
                    let offset = idx * layout.size();

                    let offset_inside_data_area =
                        offset <= (BASE_PAGE_SIZE - CACHE_LINE_SIZE - layout.size());
                    if !offset_inside_data_area {
                        return None;
                    }

                    let addr: usize = ((self as *const ObjectPage) as usize) + offset;
                    let alignment_ok = addr % layout.align() == 0;
                    let block_is_free = bitval & (1 << first_free) == 0;
                    if alignment_ok && block_is_free {
                        return Some((idx, addr));
                    }
                }
            }
        }
        None
    }

    /// Check if the current `idx` is allocated.
    ///
    /// # Notes
    /// In case `idx` is 3 and allocation size of slab is
    /// 8. The corresponding object would start at &data + 3 * 8.
    fn is_allocated(&mut self, idx: usize) -> bool {
        let base_idx = idx / 64;
        let bit_idx = idx % 64;

        (self.bitfield[base_idx] & (1 << bit_idx)) > 0
    }

    /// Sets the bit number `idx` in the bit-field.
    fn set_bit(&mut self, idx: usize) {
        let base_idx = idx / 64;
        let bit_idx = idx % 64;
        self.bitfield[base_idx] |= 1 << bit_idx;
    }

    /// Clears bit number `idx` in the bit-field.
    fn clear_bit(&mut self, idx: usize) {
        let base_idx = idx / 64;
        let bit_idx = idx % 64;
        self.bitfield[base_idx] &= !(1 << bit_idx);
    }

    /// Deallocates a memory object within this page.
    fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        trace!(
            "ObjectPage deallocating ptr = 0x{:x} with {:?}",
            ptr as usize,
            layout
        );
        let page_offset = (ptr as usize) & 0xfff;
        assert!(page_offset % layout.size() == 0);
        let idx = page_offset / layout.size();
        assert!(
            self.is_allocated(idx),
            "ptr = 0x{:x} was not allocated",
            ptr as usize
        );
        self.clear_bit(idx);
    }

    /// Tries to allocate an object within this page.
    ///
    /// In case the Slab is full, returns None.
    fn allocate(&mut self, layout: Layout) -> *mut u8 {
        match self.first_fit(layout) {
            Some((idx, addr)) => {
                self.set_bit(idx);
                unsafe { mem::transmute::<usize, *mut u8>(addr) }
            }
            None => ptr::null_mut(),
        }
    }

    /// Checks if we can still allocate more objects within the page.
    fn is_full(&self) -> bool {
        unsafe {
            self.bitfield
                .iter()
                .filter(|&x| *x != u64::max_value())
                .count()
                == 0
        }
    }

    /// Checks if the page has currently no allocation.
    fn is_empty(&self) -> bool {
        unsafe { self.bitfield.iter().filter(|&x| *x > 0x0).count() == 0 }
    }
}

#[test]
pub fn check_first_fit() {
    let op: ObjectPage = Default::default();
    let layout = Layout::from_size_align(8, 8).unwrap();
    println!("{:?}", op.first_fit(layout));
}

/// Rawlink is a type like Option<T> but for holding a raw pointer
struct Rawlink<T> {
    p: *mut T,
}

impl<T> Default for Rawlink<T> {
    fn default() -> Self {
        Rawlink { p: ptr::null_mut() }
    }
}

impl<T> Rawlink<T> {
    /// Like Option::None for Rawlink
    fn none() -> Rawlink<T> {
        Rawlink { p: ptr::null_mut() }
    }

    /// Like Option::Some for Rawlink
    fn some(n: &mut T) -> Rawlink<T> {
        Rawlink { p: n }
    }

    /// Convert the `Rawlink` into an Option value
    ///
    /// **unsafe** because:
    ///
    /// - Dereference of raw pointer.
    /// - Returns reference of arbitrary lifetime.
    unsafe fn resolve<'a>(&self) -> Option<&'a T> {
        self.p.as_ref()
    }

    /// Convert the `Rawlink` into an Option value
    ///
    /// **unsafe** because:
    ///
    /// - Dereference of raw pointer.
    /// - Returns reference of arbitrary lifetime.
    unsafe fn resolve_mut<'a>(&mut self) -> Option<&'a mut T> {
        self.p.as_mut()
    }

    /// Return the `Rawlink` and replace with `Rawlink::none()`
    fn take(&mut self) -> Rawlink<T> {
        mem::replace(self, Rawlink::none())
    }
}
