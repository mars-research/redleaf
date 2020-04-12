use env_logger;
use libc;
use rand;
use spin::Mutex;
use std::alloc;
use std::alloc::Layout;
use std::mem::{size_of, transmute};
use std::prelude::v1::*;

// The types we want to test:
use super::{ObjectPage, PageProvider, SCAllocator, ZoneAllocator, BASE_PAGE_SIZE};

use test::Bencher;

/// Page allocator based on mmap/munmap system calls for backing slab memory.
struct MmapPageProvider {
    currently_allocated: usize,
}

impl MmapPageProvider {
    pub fn new() -> MmapPageProvider {
        MmapPageProvider {
            currently_allocated: 0,
        }
    }
}

impl MmapPageProvider {
    pub fn currently_allocated(&self) -> usize {
        self.currently_allocated
    }
}

impl<'a> PageProvider<'a> for MmapPageProvider {
    /// Allocates a new ObjectPage from the system.
    ///
    /// Uses `mmap` to map a page and casts it to a ObjectPage.
    fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
        let mut addr: libc::c_void = libc::c_void::__variant1;
        let len: libc::size_t = BASE_PAGE_SIZE;
        let prot = libc::PROT_READ | libc::PROT_WRITE;
        let flags = libc::MAP_PRIVATE | libc::MAP_ANON;
        let fd = -1;
        let offset = 0;
        let r = unsafe { libc::mmap(&mut addr, len as libc::size_t, prot, flags, fd, offset) };
        if r == libc::MAP_FAILED {
            return None;
        } else {
            let slab_page: &'a mut ObjectPage = unsafe { transmute(r as usize) };
            self.currently_allocated += 1;
            return Some(slab_page);
        }
    }

    /// Release a ObjectPage back to the system.slab_page
    ///
    /// Uses `munmap` to release the page back to the OS.
    fn release_page(&mut self, p: &'a mut ObjectPage<'a>) {
        let addr: *mut libc::c_void = unsafe { transmute(p) };
        let len: libc::size_t = BASE_PAGE_SIZE;
        let r = unsafe { libc::munmap(addr, len) };
        if r != 0 {
            panic!("munmap failed!");
        }
        self.currently_allocated -= 1;
    }
}

#[test]
fn check_size() {
    assert!(
        BASE_PAGE_SIZE as usize == size_of::<ObjectPage>(),
        "ObjectPage should be exactly the size of a single page."
    );
}

#[test]
fn test_mmap_allocator() {
    let mut mmap = MmapPageProvider::new();
    match mmap.allocate_page() {
        Some(sp) => {
            assert!(!sp.is_full(), "Got empty slab");
            mmap.release_page(sp)
        }
        None => panic!("failed to allocate ObjectPage"),
    }
}

#[test]
fn check_sizes() {
    assert_eq!(size_of::<ObjectPage>(), BASE_PAGE_SIZE);
}

macro_rules! test_sc_allocation {
    ($test:ident, $size:expr, $alignment:expr, $allocations:expr) => {
        #[test]
        fn $test() {
            let mut mmap = Mutex::new(MmapPageProvider::new());

            {
                let mut sa: SCAllocator = SCAllocator::new($size, &mut mmap);
                let alignment = $alignment;

                let mut objects: Vec<*mut u8> = Vec::new();
                let mut vec: Vec<(usize, &mut [usize; $size / 8])> = Vec::new();
                let layout = Layout::from_size_align($size, alignment).unwrap();

                for _ in 0..$allocations {
                    let ptr = sa.allocate(layout);
                    if ptr.is_null() {
                        panic!("OOM is unlikely.");
                    } else {
                        unsafe { vec.push((rand::random::<usize>(), transmute(ptr))) };
                        objects.push(ptr)
                    }
                }

                // Write the objects with a random pattern
                for item in vec.iter_mut() {
                    let (pattern, ref mut obj) = *item;
                    assert!(obj.len() == $size / 8);
                    for i in 0..obj.len() {
                        obj[i] = pattern;
                    }
                }

                for item in vec.iter() {
                    let (pattern, ref obj) = *item;
                    for i in 0..obj.len() {
                        assert!(
                            (obj[i]) == pattern,
                            "No two allocations point to the same memory."
                        );
                    }
                }

                // Make sure we can correctly deallocate:
                let pages_allocated = sa.slabs.elements;

                // Deallocate all the objects
                for item in objects.iter_mut() {
                    sa.deallocate(*item, layout);
                }

                objects.clear();

                // then allocate everything again,
                for idx in 0..$allocations {
                    let ptr = sa.allocate(layout);
                    if ptr.is_null() {
                        panic!("OOM is unlikely.");
                    } else {
                        unsafe { vec.push((rand::random::<usize>(), transmute(ptr))) };
                        objects.push(ptr)
                    }
                }

                // and make sure we do not request more pages than what we had previously
                // println!("{} {}", pages_allocated, sa.slabs.elements);
                assert!(
                    pages_allocated == sa.slabs.elements,
                    "Did not use more memory for 2nd allocation run."
                );

                // Deallocate everything once more
                for item in objects.iter_mut() {
                    sa.deallocate(*item, layout);
                }
            }

            // Check that we released everything to our page allocator:
            let pager = mmap.lock();
            assert!(
                pager.currently_allocated() == 1,
                "Released all but one pages to underlying memory manager."
            );
        }
    };
}

test_sc_allocation!(test_sc_allocation512_size8_alignment1, 8, 1, 512);
test_sc_allocation!(test_sc_allocation4096_size8_alignment8, 8, 8, 4096);
test_sc_allocation!(test_sc_allocation500_size8_alignment64, 8, 64, 500);
test_sc_allocation!(test_sc_allocation4096_size12_alignment1, 12, 1, 4096);
test_sc_allocation!(test_sc_allocation4096_size13_alignment1, 13, 1, 4096);
test_sc_allocation!(test_sc_allocation2000_size14_alignment1, 14, 1, 2000);
test_sc_allocation!(test_sc_allocation4096_size15_alignment1, 15, 1, 4096);
test_sc_allocation!(test_sc_allocation8000_size16_alignment1, 16, 1, 8000);
test_sc_allocation!(test_sc_allocation1024_size24_alignment1, 24, 1, 1024);
test_sc_allocation!(test_sc_allocation3090_size32_alignment1, 32, 1, 3090);
test_sc_allocation!(test_sc_allocation4096_size64_alignment1, 64, 1, 4096);
test_sc_allocation!(test_sc_allocation1000_size512_alignment1, 512, 1, 1000);
test_sc_allocation!(test_sc_allocation4096_size1024_alignment1, 1024, 1, 4096);
test_sc_allocation!(test_sc_allocation10_size2048_alignment1, 2048, 1, 10);
test_sc_allocation!(test_sc_allocation10000_size512_alignment1, 512, 1, 10000);

#[test]
#[should_panic]
fn invalid_alignment() {
    let layout = Layout::from_size_align(10, 3).unwrap();
}

#[test]
fn test_readme() {
    let object_size = 12;
    let alignment = 4;
    let mmap = Mutex::new(MmapPageProvider::new());
    let mut zone = ZoneAllocator::new(&mmap);

    unsafe {
        let layout = Layout::from_size_align(object_size, alignment).unwrap();
        let allocated = zone.allocate(layout);
        assert!(!allocated.is_null());
        zone.deallocate(allocated, layout);
    }
}

#[test]
fn test_bug1() {
    let _ = env_logger::init();

    let mut mmap = Mutex::new(MmapPageProvider::new());
    let mut sa: SCAllocator = SCAllocator::new(8, &mut mmap);
    sa.refill_slab(1);

    let ptr1 = sa.allocate(Layout::from_size_align(1, 1).unwrap());
    let ptr2 = sa.allocate(Layout::from_size_align(2, 1).unwrap());
    sa.deallocate(ptr1, Layout::from_size_align(1, 1).unwrap());
    let ptr3 = sa.allocate(Layout::from_size_align(4, 1).unwrap());
    sa.deallocate(ptr2, Layout::from_size_align(2, 1).unwrap());
}

#[test]
fn test_readme2() {
    let object_size = 10;
    let alignment = 8;
    let layout = Layout::from_size_align(object_size, alignment).unwrap();
    let mut mmap = Mutex::new(MmapPageProvider::new());
    let mut sa: SCAllocator = SCAllocator::new(object_size, &mut mmap);
    sa.allocate(layout);
}

#[bench]
fn bench_allocate(b: &mut Bencher) {
    let mut mmap = Mutex::new(MmapPageProvider::new());
    let mut sa: SCAllocator = SCAllocator::new(8, &mut mmap);
    let layout = Layout::from_size_align(8, 1).unwrap();
    sa.refill_slab(1);
    b.iter(|| {
        let ptr = sa.allocate(layout);
        assert!(!ptr.is_null());
        sa.deallocate(ptr, layout);
    });
}

#[bench]
fn bench_allocate_big(b: &mut Bencher) {
    let mut mmap = Mutex::new(MmapPageProvider::new());
    let mut sa: SCAllocator = SCAllocator::new(512, &mut mmap);
    sa.refill_slab(1);
    let layout = Layout::from_size_align(512, 1).unwrap();
    b.iter(|| {
        let ptr = sa.allocate(layout);
        assert!(!ptr.is_null());
        sa.deallocate(ptr, layout);
    });
}

#[bench]
fn compare_vs_alloc(b: &mut Bencher) {
    let layout = Layout::from_size_align(8, 1).unwrap();
    b.iter(|| unsafe {
        let ptr = alloc::alloc(layout);
        alloc::dealloc(ptr, layout);
    });
}
