//! A buddy allocator for managing physical memory.
//!
//! Some of this code was inspired by
//! https://crates.io/crates/alloc_buddy_simple (Apache2 / MIT License)
//!
//! See also
//!   * https://en.wikipedia.org/wiki/Buddy_memory_allocation
//!

use core::alloc::Layout;
use core::cmp::{max, min};
use core::ptr;

use crate::prelude::*;

use log::{trace, info};
use super::{Frame, PAddr, PhysicalAllocator, VAddr};
use crate::arch::memory::{kernel_vaddr_to_paddr, BASE_PAGE_SIZE};
use spin::Mutex;

/// A free block in our heap.
pub struct FreeBlock {
    /// The next block in the free list, or NULL if this is the final
    /// block.
    next: *mut FreeBlock,
}

impl FreeBlock {
    /// Construct a `FreeBlock` header pointing at `next`.
    fn new(next: *mut FreeBlock) -> FreeBlock {
        FreeBlock { next }
    }
}

pub static BUDDY: Mutex<Option<BuddyFrameAllocator>> = Mutex::new(None);

/// The interface to a heap.  This data structure is stored _outside_ the
/// heap somewhere, because every single byte of our heap is potentially
/// available for allocation.
pub struct BuddyFrameAllocator {
    /// The physical region managed by this allocator. Its base must be aligned on a
    /// `MIN_HEAP_ALIGN` boundary.
    region: Frame,

    /// The free lists for our heap.  The list at `free_lists[0]` contains
    /// the smallest block size we can allocate, and the list at the end
    /// can only contain a single free block the size of our entire heap,
    /// and only when no memory is allocated.
    free_lists: [*mut FreeBlock; 27],

    /// Our minimum block size.
    min_block_size: usize,

    /// The log base 2 of our min block size.
    min_block_size_log2: u8,
}

unsafe impl Send for BuddyFrameAllocator {}

impl PhysicalAllocator for BuddyFrameAllocator {
    unsafe fn add_memory(&mut self, region: Frame) -> bool {
        if self.region.base.as_u64() == 0 {
            let size = region.size.next_power_of_two() >> 1;
            self.region.size = region.size;
            let order = self
                .layout_to_order(Layout::from_size_align_unchecked(size, 1))
                .expect("Failed to calculate order for root heap block");
            println!("order = {} size = {}", order, region.size);
            self.region.base = region.base;
            self.free_list_insert(order, region.kernel_vaddr().as_mut_ptr::<FreeBlock>());
            true
        } else {
            false
        }
    }

    /// Allocate a block of physical memory large enough to contain `size` bytes,
    /// and aligned on `align`.
    ///
    /// Returns None in case the request can not be satisfied.
    ///
    /// All allocated Frames must be passed to `deallocate` with the same
    /// `size` and `align` parameter.
    unsafe fn allocate(&mut self, layout: Layout) -> Option<Frame> {
        trace!("buddy allocate {:?}", layout);
        // Figure out which order block we need.
        if let Some(order_needed) = self.layout_to_order(layout) {
            // Start with the smallest acceptable block size, and search
            // upwards until we reach blocks the size of the entire heap.
            for order in order_needed..self.free_lists.len() {
                // Do we have a block of this size?
                if let Some(block) = self.free_list_pop(order) {
                    // If the block is too big, break it up.  This leaves
                    // the address unchanged, because we always allocate at
                    // the head of a block.
                    if order > order_needed {
                        self.split_free_block(block, order, order_needed);
                    }

                    return Some(Frame::new(
                        PAddr::from(kernel_vaddr_to_paddr(VAddr::from(block as usize))),
                        self.order_to_size(order_needed),
                    ));
                }
            }
            None
        } else {
            trace!("Allocation size too big for request {:?}", layout);
            None
        }
    }

    /// Deallocate a block allocated using `allocate`.
    /// Layout value must match the value passed to
    /// `allocate`.
    unsafe fn deallocate(&mut self, frame: Frame, layout: Layout) {
        trace!("buddy deallocate {:?} {:?}", frame, layout);
        let initial_order = self
            .layout_to_order(layout)
            .expect("Tried to dispose of invalid block");

        // See if we can merge block with it's neighbouring buddy.
        // If so merge and continue walking up until done.
        //
        // `block` is the biggest merged block we have so far.
        let mut block = frame.kernel_vaddr().as_mut_ptr::<FreeBlock>();
        for order in initial_order..self.free_lists.len() {
            // Would this block have a buddy?
            if let Some(buddy) = self.buddy(order, block) {
                // Is this block's buddy free?
                if self.free_list_remove(order, buddy) {
                    // Merge them!  The lower address of the two is the
                    // newly-merged block.  Then we want to try again.
                    block = min(block, buddy);
                    continue;
                }
            }

            // If we reach here, we didn't find a buddy block of this size,
            // so take what we've got and mark it as free.
            self.free_list_insert(order, block);
            return;
        }
    }

    fn print_info(&self) {
        info!("Found the following physical memory regions:");
        info!("{:?}", self.region);
    }
}

impl BuddyFrameAllocator {
    const MIN_HEAP_ALIGN: usize = BASE_PAGE_SIZE;

    pub fn init() {
        let buddy = BuddyFrameAllocator::new();
        *BUDDY.lock() = Some(buddy);
    }
    fn new() -> BuddyFrameAllocator {
        BuddyFrameAllocator {
            region: Frame {
                base: PAddr(0),
                size: 0,
            },
            free_lists: [
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            ],
            min_block_size: BASE_PAGE_SIZE,
            min_block_size_log2: 12,
        }
    }

    pub fn get_region(&self) -> Frame {
        self.region
    }

    /// Get block size for allocation request.
    fn allocation_size(&self, layout: Layout) -> Option<usize> {
        // Don't try to align more than our heap base alignment
        if layout.align() > BuddyFrameAllocator::MIN_HEAP_ALIGN {
            return None;
        }

        // We're automatically aligned to `size` because of how our heap is
        // sub-divided, but if we need a larger alignment, we can only do
        // it be allocating more memory.
        let mut size = max(layout.size(), layout.align());
        // We can't allocate blocks smaller than `min_block_size`.
        size = max(size, self.min_block_size);
        // Round up to the next power of two.
        size = size.next_power_of_two();

        // We can't allocate a block bigger than our heap.
        if size <= self.region.size {
            Some(size)
        } else {
            trace!("We can't allocate a block bigger than our heap.");
            None
        }
    }

    /// The "order" of an allocation is how many times we need to double
    /// `min_block_size` in order to get a large enough block, as well as
    /// the index we use into `free_lists`.
    fn layout_to_order(&self, layout: Layout) -> Option<usize> {
        self.allocation_size(layout)
            .map(|s| (s.log2() - self.min_block_size_log2) as usize)
    }

    /// Calculate size for a given order (2^order).
    fn order_to_size(&self, order: usize) -> usize {
        1 << (self.min_block_size_log2 as usize + order)
    }

    /// Return first block off the appropriate free list.
    unsafe fn free_list_pop(&mut self, order: usize) -> Option<*mut FreeBlock> {
        let candidate = self.free_lists[order];
        if candidate != ptr::null_mut() {
            self.free_lists[order] = (*candidate).next;
            Some(candidate as *mut FreeBlock)
        } else {
            None
        }
    }

    /// Insert block in the corresponding free list slot.
    unsafe fn free_list_insert(&mut self, order: usize, free_block_ptr: *mut FreeBlock) {
        assert!(!free_block_ptr.is_null());
        *free_block_ptr = FreeBlock::new(self.free_lists[order]);
        self.free_lists[order] = free_block_ptr;
    }

    /// Attempt to remove a block from our free list, returning true
    /// success, and false if the block wasn't on our free list.
    unsafe fn free_list_remove(&mut self, order: usize, block_ptr: *mut FreeBlock) -> bool {
        // `*checking` is the pointer we want to check, and `checking` is
        // the memory location we found it at, which we'll need if we want
        // to replace the value `*checking` with a new value.
        let mut checking: *mut *mut FreeBlock = &mut self.free_lists[order];

        while *checking != ptr::null_mut() {
            // Is this the pointer we want to remove from the free list?
            if *checking == block_ptr {
                // Remove block from list
                *checking = (*(*checking)).next;
                return true;
            }
            checking = &mut ((*(*checking)).next);
        }

        false
    }

    /// Split a `block` of order `order` down into a block of order
    /// `order_needed`, placing any unused chunks on the free list.
    unsafe fn split_free_block(
        &mut self,
        block: *mut FreeBlock,
        mut order: usize,
        order_needed: usize,
    ) {
        let mut size_to_split = self.order_to_size(order);

        // Progressively cut our block down to size.
        while order > order_needed {
            // Update our loop counters to describe a block half the size.
            size_to_split >>= 1;
            order -= 1;

            // Insert the "upper half" of the block into the free list.
            let split = (block as *mut u8).offset(size_to_split as isize);
            self.free_list_insert(order, split as *mut FreeBlock);
        }
    }

    /// Given a `block` with the specified `order`, find the block
    /// we could potentially merge it with.
    pub unsafe fn buddy(&self, order: usize, block: *mut FreeBlock) -> Option<*mut FreeBlock> {
        let relative: usize = (block as usize) - (self.region.kernel_vaddr().as_usize());
        let size = self.order_to_size(order);
        if size >= self.region.size as usize {
            // The main heap itself does not have a budy.
            None
        } else {
            // We can find our buddy by XOR'ing the right bit in our
            // offset from the base of the heap.
            Some(
                self.region
                    .kernel_vaddr()
                    .as_mut_ptr::<u8>()
                    .offset((relative ^ size) as isize) as *mut FreeBlock,
            )
        }
    }
}
