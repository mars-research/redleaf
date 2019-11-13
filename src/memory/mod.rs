use core::fmt;
use x86::bits64::paging;
use core::alloc::Layout;
use core::mem::transmute;
use slabmalloc::{ObjectPage};
use crate::memory::buddy::BUDDY;

pub use self::buddy::BuddyFrameAllocator as PhysicalMemoryAllocator;
pub use crate::arch::memory::{paddr_to_kernel_vaddr, PAddr, VAddr, BASE_PAGE_SIZE};

pub mod buddy;

pub trait PhysicalAllocator {
    fn init(&mut self) {}

    unsafe fn add_memory(&mut self, _region: Frame) -> bool {
        false
    }

    unsafe fn allocate(&mut self, _layout: Layout) -> Option<Frame> {
        None
    }

    unsafe fn deallocate(&mut self, _frame: Frame, _layout: Layout) {}

    fn print_info(&self) {}
}

/// Physical region of memory.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Frame {
    pub base: PAddr,
    pub size: usize,
}

impl Frame {
    pub const fn new(base: PAddr, size: usize) -> Frame {
        Frame {
            base: base,
            size: size,
        }
    }

    #[allow(unused)]
    const fn empty() -> Frame {
        Frame {
            base: PAddr::zero(),
            size: 0,
        }
    }

    unsafe fn as_mut_slice<T>(&mut self) -> Option<&mut [T]> {
        if self.size % core::mem::size_of::<T>() == 0 {
            Some(core::slice::from_raw_parts_mut(
                self.kernel_vaddr().as_mut_ptr::<T>(),
                self.size / core::mem::size_of::<T>(),
            ))
        } else {
            None
        }
    }

    #[allow(unused)]
    unsafe fn as_slice<T>(&self) -> Option<&[T]> {
        if self.size % core::mem::size_of::<T>() == 0 {
            Some(core::slice::from_raw_parts(
                self.kernel_vaddr().as_mut_ptr::<T>(),
                self.size / core::mem::size_of::<T>(),
            ))
        } else {
            None
        }
    }

    unsafe fn fill<T: Copy>(&mut self, pattern: T) -> bool {
        self.as_mut_slice::<T>().map_or(false, |obj| {
            for i in 0..obj.len() {
                obj[i] = pattern;
            }
            true
        })
    }

    /// Size of the region (in 4K pages).
    pub fn base_pages(&self) -> usize {
        self.size / BASE_PAGE_SIZE
    }

    /// Size of the region (in bytes).
    pub fn size(&self) -> usize {
        self.size
    }

    pub unsafe fn zero(&mut self) {
        self.fill(0);
    }

    /// The kernel virtual address for this region.
    pub fn kernel_vaddr(&self) -> VAddr {
        paddr_to_kernel_vaddr(self.base)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Frame {{ 0x{:x} -- 0x{:x} (size = {}, pages = {} }}",
            self.base,
            self.base + self.size,
            self.size,
            self.base_pages()
        )
    }
}

pub struct BespinSlabsProvider;

unsafe impl Send for BespinSlabsProvider {}
unsafe impl Sync for BespinSlabsProvider {}

impl BespinSlabsProvider {
    pub const fn new() -> BespinSlabsProvider {
        BespinSlabsProvider
    }
}

impl<'a> BespinSlabsProvider {
    fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
        let mut f: Option<Frame> = None;
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            f = unsafe {
                fmanager.allocate(
                    Layout::new::<paging::Page>()
                        .align_to(BASE_PAGE_SIZE)
                        .unwrap(),
                )
            };
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }

        f.map(|mut frame| unsafe {
            frame.zero();
            println!("slabmalloc allocate frame.base = {:x}", frame.base);
            let sp: &'a mut ObjectPage = transmute(paddr_to_kernel_vaddr(frame.base));
            sp
        })
    }

    fn release_page(&mut self, _p: &'a mut ObjectPage<'a>) {
        println!("TODO!");
    }
}
