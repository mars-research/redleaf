//! A minimal example that implements the GlobalAlloc trait.

extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};
use core::mem::transmute;
use core::ptr::{self, NonNull};
use slabmalloc::*;
use spin::Mutex;
use syscalls::syscalls::{sys_alloc, sys_free, sys_alloc_huge, sys_free_huge};

/// SLAB_ALLOC is set as the system's default allocator, it's implementation follows below.
///
/// It's a ZoneAllocator wrapped inside a Mutex.
#[global_allocator]
static SLAB_ALLOC: SafeZoneAllocator = SafeZoneAllocator(Mutex::new(ZoneAllocator::new()));

/// To use a ZoneAlloactor we require a lower-level allocator
/// (not provided by this crate) that can supply the allocator
/// with backing memory for `LargeObjectPage` and `ObjectPage` structs.
///
/// In our dummy implementation we just rely on the OS system allocator `alloc::System`.
struct Pager;

impl Pager {
    const BASE_PAGE_SIZE: usize = 4096;
    const LARGE_PAGE_SIZE: usize = 2 * 2 * 1024;

    /// Allocates a given `page_size`.
    fn alloc_page(&mut self, _page_size: usize) -> Option<*mut u8> {
        let r = sys_alloc();

        if r as u64 != 0 {
            Some(r)
        } else {
            None
        }
    }

    /// De-allocates a given `page_size`.
    fn dealloc_page(&mut self, ptr: *mut u8, page_size: usize) {
        let _layout = match page_size {
            Pager::BASE_PAGE_SIZE => {
                Layout::from_size_align(Pager::BASE_PAGE_SIZE, Pager::BASE_PAGE_SIZE).unwrap()
            }
            Pager::LARGE_PAGE_SIZE => {
                Layout::from_size_align(Pager::LARGE_PAGE_SIZE, Pager::LARGE_PAGE_SIZE).unwrap()
            }
            _ => unreachable!("invalid page-size supplied"),
        };

        sys_free(ptr);
    }

    /// Allocates a new ObjectPage from the System.
    fn allocate_page(&mut self) -> Option<&'static mut ObjectPage<'static>> {
        self.alloc_page(Pager::BASE_PAGE_SIZE)
            .map(|r| unsafe { transmute(r as usize) })
    }

    /// Release a ObjectPage back to the System.
    #[allow(unused)]
    fn release_page(&mut self, p: &'static mut ObjectPage<'static>) {
        self.dealloc_page(p as *const ObjectPage as *mut u8, Pager::BASE_PAGE_SIZE);
    }

    /// Allocates a new LargeObjectPage from the system.
    fn allocate_large_page(&mut self) -> Option<&'static mut LargeObjectPage<'static>> {
        self.alloc_page(Pager::LARGE_PAGE_SIZE)
            .map(|r| unsafe { transmute(r as usize) })
    }

    /// Release a LargeObjectPage back to the System.
    #[allow(unused)]
    fn release_large_page(&mut self, p: &'static mut LargeObjectPage<'static>) {
        self.dealloc_page(
            p as *const LargeObjectPage as *mut u8,
            Pager::LARGE_PAGE_SIZE,
        );
    }
}

/// A pager for GlobalAlloc.
static mut PAGER: Pager = Pager;

/// A SafeZoneAllocator that wraps the ZoneAllocator in a Mutex.
///
/// Note: This is not very scalable since we use a single big lock
/// around the allocator. There are better ways make the ZoneAllocator
/// thread-safe directly, but they are not implemented yet.
pub struct SafeZoneAllocator(Mutex<ZoneAllocator<'static>>);

unsafe impl GlobalAlloc for SafeZoneAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match layout.size() {
            Pager::BASE_PAGE_SIZE => {
                // Best to use the underlying backend directly to allocate pages
                // to avoid fragmentation
                PAGER.allocate_page().expect("Can't allocate page?") as *mut _ as *mut u8
            }
            Pager::LARGE_PAGE_SIZE => {
                // Best to use the underlying backend directly to allocate large
                // to avoid fragmentation
                PAGER.allocate_large_page().expect("Can't allocate page?") as *mut _ as *mut u8
            }
            sz => {
                sys_alloc_huge(sz as u64)
            }
            /*0..=ZoneAllocator::MAX_ALLOC_SIZE => {
                let mut zone_allocator = self.0.lock();
                match zone_allocator.allocate(layout) {
                    Ok(nptr) => nptr.as_ptr(),
                    Err(AllocationError::OutOfMemory) => {
                        if layout.size() <= ZoneAllocator::MAX_BASE_ALLOC_SIZE {
                            PAGER.allocate_page().map_or(ptr::null_mut(), |page| {
                                zone_allocator
                                    .refill(layout, page)
                                    .expect("Could not refill?");
                                zone_allocator
                                    .allocate(layout)
                                    .expect("Should succeed after refill")
                                    .as_ptr()
                            })
                        } else {
                            // layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE
                            PAGER
                                .allocate_large_page()
                                .map_or(ptr::null_mut(), |large_page| {
                                    zone_allocator
                                        .refill_large(layout, large_page)
                                        .expect("Could not refill?");
                                    zone_allocator
                                        .allocate(layout)
                                        .expect("Should succeed after refill")
                                        .as_ptr()
                                })
                        }
                    }
                    Err(AllocationError::InvalidLayout) => panic!("Can't allocate this size"),
                }
            }*/
            _ => unimplemented!("Can't handle it, probably needs another allocator."),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match layout.size() {
            Pager::BASE_PAGE_SIZE => Pager.dealloc_page(ptr, Pager::BASE_PAGE_SIZE),
            Pager::LARGE_PAGE_SIZE => Pager.dealloc_page(ptr, Pager::LARGE_PAGE_SIZE),

            sz => sys_free_huge(ptr),
            /*0..=ZoneAllocator::MAX_ALLOC_SIZE => {
                if let Some(nptr) = NonNull::new(ptr) {
                    self.0
                        .lock()
                        .deallocate(nptr, layout)
                        .expect("Couldn't deallocate");
                } else {
                    // Nothing to do (don't dealloc null pointers).
                }

                // An proper reclamation strategy could be implemented here
                // to release empty pages back from the ZoneAllocator to the PAGER
            }*/
            _ => unimplemented!("Can't handle it, probably needs another allocator."),
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
