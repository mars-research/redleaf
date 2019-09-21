use core::fmt;
use x86::bits64::paging;
use core::mem::transmute;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use slabmalloc::{ObjectPage, PageProvider, ZoneAllocator};
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

pub trait PageTableProvider<'a> {
    fn allocate_pml4<'b>(&mut self) -> Option<&'b mut paging::PML4>;
    fn new_pdpt(&mut self) -> Option<paging::PML4Entry>;
    fn new_pd(&mut self) -> Option<paging::PDPTEntry>;
    fn new_pt(&mut self) -> Option<paging::PDEntry>;
    fn new_page(&mut self) -> Option<paging::PTEntry>;
}

#[allow(dead_code)]
pub struct BespinPageTableProvider;

impl BespinPageTableProvider {
    #[allow(dead_code)]
    pub const fn new() -> BespinPageTableProvider {
        BespinPageTableProvider
    }
}

impl<'a> PageTableProvider<'a> for BespinPageTableProvider {
    /// Allocate a PML4 table.
    fn allocate_pml4<'b>(&mut self) -> Option<&'b mut paging::PML4> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            unsafe {
                let f = fmanager.allocate(
                    Layout::new::<paging::Page>()
                        .align_to(BASE_PAGE_SIZE)
                        .unwrap(),
                );
                f.map(|frame| {
                    let pml4: &'b mut [paging::PML4Entry; 512] =
                        transmute(paddr_to_kernel_vaddr(frame.base));
                    pml4
                })
            }
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    }

    /// Allocate a new page directory and return a PML4 entry for it.
    fn new_pdpt(&mut self) -> Option<paging::PML4Entry> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            unsafe {
                fmanager
                    .allocate(
                        Layout::new::<paging::Page>()
                            .align_to(BASE_PAGE_SIZE)
                            .unwrap(),
                    )
                    .map(|frame| {
                        paging::PML4Entry::new(
                            frame.base,
                            paging::PML4Flags::P | paging::PML4Flags::RW | paging::PML4Flags::US,
                        )
                    })
            }
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    }

    /// Allocate a new page directory and return a pdpt entry for it.
    fn new_pd(&mut self) -> Option<paging::PDPTEntry> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            unsafe {
                fmanager
                    .allocate(
                        Layout::new::<paging::Page>()
                            .align_to(BASE_PAGE_SIZE)
                            .unwrap(),
                    )
                    .map(|frame| {
                        paging::PDPTEntry::new(
                            frame.base,
                            paging::PDPTFlags::P | paging::PDPTFlags::RW | paging::PDPTFlags::US,
                        )
                    })
            }
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    }

    /// Allocate a new page-directory and return a page directory entry for it.
    fn new_pt(&mut self) -> Option<paging::PDEntry> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            unsafe {
                fmanager
                    .allocate(
                        Layout::new::<paging::Page>()
                            .align_to(BASE_PAGE_SIZE)
                            .unwrap(),
                    )
                    .map(|frame| {
                        paging::PDEntry::new(
                            frame.base,
                            paging::PDFlags::P | paging::PDFlags::RW | paging::PDFlags::US,
                        )
                    })
            }
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    }

    /// Allocate a new (4KiB) page and map it.
    fn new_page(&mut self) -> Option<paging::PTEntry> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            unsafe {
                fmanager
                    .allocate(
                        Layout::new::<paging::Page>()
                            .align_to(BASE_PAGE_SIZE)
                            .unwrap(),
                    )
                    .map(|frame| {
                        paging::PTEntry::new(
                            frame.base,
                            paging::PTFlags::P | paging::PTFlags::RW | paging::PTFlags::US,
                        )
                    })
            }
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
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

impl<'a> PageProvider<'a> for BespinSlabsProvider {
    fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            let mut f = unsafe {
                fmanager.allocate(
                    Layout::new::<paging::Page>()
                        .align_to(BASE_PAGE_SIZE)
                        .unwrap(),
               )
            };
            f.map(|mut frame| unsafe {
                frame.zero();
                println!("slabmalloc allocate frame.base = {:x}", frame.base);
                let sp: &'a mut ObjectPage = transmute(paddr_to_kernel_vaddr(frame.base));
                sp
            })
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    }

    fn release_page(&mut self, _p: &'a mut ObjectPage<'a>) {
        println!("TODO!");
    }
}

#[allow(dead_code)]
static PAGER: Mutex<BespinSlabsProvider> = Mutex::new(BespinSlabsProvider::new());

#[allow(dead_code)]
pub struct SafeZoneAllocator(Mutex<ZoneAllocator<'static>>);

impl SafeZoneAllocator {
    pub const fn new(provider: &'static Mutex<PageProvider>) -> SafeZoneAllocator {
        SafeZoneAllocator(Mutex::new(ZoneAllocator::new(provider)))
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

unsafe impl GlobalAlloc for SafeZoneAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        println!("alloc layout={:?}", layout);
        if layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE {
            let ptr = self.0.lock().allocate(layout);
            println!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
            ptr
        } else {
            let mut ptr = core::ptr::null_mut();

            if let Some(ref mut fmanager) = *BUDDY.lock() {
                let mut f = fmanager.allocate(layout);
                ptr = f.map_or(core::ptr::null_mut(), |mut region| {
                    region.zero();
                    region.kernel_vaddr().as_mut_ptr()
                });
                println!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
                drop(fmanager);
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        println!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
        if layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE {
            //debug!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
            self.0.lock().deallocate(ptr, layout);
        } else {
            use crate::arch::memory::{kernel_vaddr_to_paddr, VAddr};
            if let Some(ref mut fmanager) = *BUDDY.lock() {
                fmanager.deallocate(
                    Frame::new(
                        kernel_vaddr_to_paddr(VAddr::from_u64(ptr as u64)),
                        layout.size(),
                    ),
                    layout,
                );
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
        }
    }
}

#[global_allocator]
static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&PAGER);
