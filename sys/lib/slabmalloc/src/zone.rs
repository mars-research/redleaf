//! A ZoneAllocator to allocate arbitrary object sizes (up to `ZoneAllocator::MAX_ALLOC_SIZE`)
//!
//! The ZoneAllocator achieves this by having many `SCAllocator`

use crate::*;

/// Creates an instance of a zone, we do this in a macro because we
/// re-use the code in const and non-const functions
///
/// We can get rid of this once the const fn feature is fully stabilized.
macro_rules! new_zone {
    () => {
        ZoneAllocator {
            // TODO(perf): We should probably pick better classes
            // rather than powers-of-two (see SuperMalloc etc.)
            small_slabs: [
                SCAllocator::new(1 << 3),  // 8
                SCAllocator::new(1 << 4),  // 16
                SCAllocator::new(1 << 5),  // 32
                SCAllocator::new(1 << 6),  // 64
                SCAllocator::new(1 << 7),  // 128
                SCAllocator::new(1 << 8),  // 256
                SCAllocator::new(1 << 9),  // 512
                SCAllocator::new(1 << 10), // 1024 (TODO: maybe get rid of this class?)
                SCAllocator::new(1 << 11), // 2048 (TODO: maybe get rid of this class?)
                SCAllocator::new(4016), // 4016 (can't do 4096 because of metadata in ObjectPage)
            ],
            big_slabs: [
                SCAllocator::new(1 << 13), // 8192
                SCAllocator::new(1 << 14), // 16384
                SCAllocator::new(1 << 15), // 32767
                SCAllocator::new(1 << 16), // 65536
                SCAllocator::new(1 << 17), // 131072
            ],
        }
    };
}

/// A zone allocator for arbitrary sized allocations.
///
/// Has a bunch of `SCAllocator` and through that can serve allocation
/// requests for many different object sizes up to (MAX_SIZE_CLASSES) by selecting
/// the right `SCAllocator` for allocation and deallocation.
///
/// The allocator provides to refill functions `refill` and `refill_large`
/// to provide the underlying `SCAllocator` with more memory in case it runs out.
pub struct ZoneAllocator<'a> {
    small_slabs: [SCAllocator<'a, ObjectPage<'a>>; ZoneAllocator::MAX_BASE_SIZE_CLASSES],
    big_slabs: [SCAllocator<'a, LargeObjectPage<'a>>; ZoneAllocator::MAX_LARGE_SIZE_CLASSES],
}

impl<'a> Default for ZoneAllocator<'a> {
    fn default() -> ZoneAllocator<'a> {
        new_zone!()
    }
}

enum Slab {
    Base(usize),
    Large(usize),
    Unsupported,
}

impl<'a> ZoneAllocator<'a> {
    /// Maximum size that allocated within LargeObjectPages (2 MiB).
    /// This is also the maximum object size that this allocator can handle.
    pub const MAX_ALLOC_SIZE: usize = 1 << 17;

    /// Maximum size which is allocated with ObjectPages (4 KiB pages).
    ///
    /// e.g. this is 4 KiB - 80 bytes of meta-data.
    pub const MAX_BASE_ALLOC_SIZE: usize = 4016;

    /// How many allocators of type SCAllocator<ObjectPage> we have.
    const MAX_BASE_SIZE_CLASSES: usize = 10;

    /// How many allocators of type SCAllocator<LargeObjectPage> we have.
    const MAX_LARGE_SIZE_CLASSES: usize = 5;

    #[cfg(feature = "unstable")]
    pub const fn new() -> ZoneAllocator<'a> {
        new_zone!()
    }

    #[cfg(not(feature = "unstable"))]
    pub fn new() -> ZoneAllocator<'a> {
        new_zone!()
    }

    /// Return maximum size an object of size `current_size` can use.
    ///
    /// Used to optimize `realloc`.
    #[allow(dead_code)]
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
            2049..=4016 => Some(4016),
            4017..=8192 => Some(8192),
            8193..=16384 => Some(16384),
            16385..=32767 => Some(32767),
            32768..=65536 => Some(65536),
            65537..=131_072 => Some(131_072),
            _ => None,
        }
    }

    /// Figure out index into zone array to get the correct slab allocator for that size.
    fn get_slab(requested_size: usize) -> Slab {
        match requested_size {
            0..=8 => Slab::Base(0),
            9..=16 => Slab::Base(1),
            17..=32 => Slab::Base(2),
            33..=64 => Slab::Base(3),
            65..=128 => Slab::Base(4),
            129..=256 => Slab::Base(5),
            257..=512 => Slab::Base(6),
            513..=1024 => Slab::Base(7),
            1025..=2048 => Slab::Base(8),
            2049..=4016 => Slab::Base(9),
            4017..=8192 => Slab::Large(0),
            8193..=16384 => Slab::Large(1),
            16385..=32767 => Slab::Large(2),
            32768..=65536 => Slab::Large(3),
            65537..=131_072 => Slab::Large(4),
            _ => Slab::Unsupported,
        }
    }

    /// Refills the SCAllocator for a given Layout with an ObjectPage.
    ///
    /// # Safety
    /// ObjectPage needs to be emtpy etc.
    pub unsafe fn refill(
        &mut self,
        layout: Layout,
        new_page: &'a mut ObjectPage<'a>,
    ) -> Result<(), AllocationError> {
        match ZoneAllocator::get_slab(layout.size()) {
            Slab::Base(idx) => {
                self.small_slabs[idx].refill(new_page);
                Ok(())
            }
            Slab::Large(_idx) => Err(AllocationError::InvalidLayout),
            Slab::Unsupported => Err(AllocationError::InvalidLayout),
        }
    }

    /// Refills the SCAllocator for a given Layout with an ObjectPage.
    ///
    /// # Safety
    /// ObjectPage needs to be emtpy etc.
    pub unsafe fn refill_large(
        &mut self,
        layout: Layout,
        new_page: &'a mut LargeObjectPage<'a>,
    ) -> Result<(), AllocationError> {
        match ZoneAllocator::get_slab(layout.size()) {
            Slab::Base(_idx) => Err(AllocationError::InvalidLayout),
            Slab::Large(idx) => {
                self.big_slabs[idx].refill(new_page);
                Ok(())
            }
            Slab::Unsupported => Err(AllocationError::InvalidLayout),
        }
    }

    /// Allocate a pointer to a block of memory described by `layout`.
    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocationError> {
        match ZoneAllocator::get_slab(layout.size()) {
            Slab::Base(idx) => self.small_slabs[idx].allocate(layout),
            Slab::Large(idx) => self.big_slabs[idx].allocate(layout),
            Slab::Unsupported => Err(AllocationError::InvalidLayout),
        }
    }

    /// Deallocates a pointer to a block of memory, which was
    /// previously allocated by `allocate`.
    ///
    /// # Arguments
    ///  * `ptr` - Address of the memory location to free.
    ///  * `layout` - Memory layout of the block pointed to by `ptr`.
    pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) -> Result<(), AllocationError> {
        match ZoneAllocator::get_slab(layout.size()) {
            Slab::Base(idx) => self.small_slabs[idx].deallocate(ptr, layout),
            Slab::Large(idx) => self.big_slabs[idx].deallocate(ptr, layout),
            Slab::Unsupported => Err(AllocationError::InvalidLayout),
        }
    }
}
