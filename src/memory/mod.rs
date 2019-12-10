use core::fmt;
use x86::bits64::paging;
use core::mem::transmute;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use slabmalloc::{ObjectPage, PageProvider, ZoneAllocator};
use crate::memory::buddy::BUDDY;
use log::{debug, warn, trace, info};

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
                trace!("slabmalloc allocate frame.base = {:x}", frame.base);
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
        trace!("alloc layout={:?}", layout);
        if layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE {
            let ptr = self.0.lock().allocate(layout);
            trace!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
            ptr
        } else {
            let mut ptr = core::ptr::null_mut();

            if let Some(ref mut fmanager) = *BUDDY.lock() {
                let mut f = fmanager.allocate(layout);
                ptr = f.map_or(core::ptr::null_mut(), |mut region| {
                    region.zero();
                    region.kernel_vaddr().as_mut_ptr()
                });
                trace!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        trace!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
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

use crate::arch::vspace::{VSpace, MapAction};
use x86::controlregs;

fn text_start() -> u64 {
    extern {
        static __text_start: u8;
    }
    unsafe {
        & __text_start as *const _ as u64
    }
}

fn text_end() -> u64 {
    extern {
        static __text_end: u8;
    }
    unsafe {
        & __text_end as *const _ as u64
    }
}

fn rodata_start() -> u64 {
    extern {
        static __rodata_start: u8;
    }
    unsafe {
        & __rodata_start as *const _ as u64
    }
}

fn rodata_end() -> u64 {
    extern {
        static __rodata_end: u8;
    }
    unsafe {
        & __rodata_end as *const _ as u64
    }
}

fn data_start() -> u64 {
    extern {
        static __data_start: u8;
    }
    unsafe {
        & __data_start as *const _ as u64
    }
}

fn data_end() -> u64 {
    extern {
        static __data_end: u8;
    }
    unsafe {
        & __data_end as *const _ as u64
    }
}

fn bss_start() -> u64 {
    extern {
        static __bss_start: u8;
    }
    unsafe {
        & __bss_start as *const _ as u64
    }
}

fn bss_end() -> u64 {
    extern {
        static __bss_end: u8;
    }
    unsafe {
        & __bss_end as *const _ as u64
    }
}

fn tdata_start() -> u64 {
    extern {
        static __tdata_start: u8;
    }
    unsafe {
        & __tdata_start as *const _ as u64
    }
}

fn tdata_end() -> u64 {
    extern {
        static __tdata_end: u8;
    }
    unsafe {
        & __tdata_end as *const _ as u64
    }
}

fn kernel_end() -> u64 {
    extern {
        /// The starting byte of the thread data segment
        static __end: u8;
    }

    unsafe{
        & __end as *const _ as u64
    }
}
const KERNEL_BASE: u64 = 0x100000;
const MAP_BASE: u64 = 0x0;

pub fn construct_pt() {
    let mut vspace = VSpace::new();

    // Map RWX as some code is copied to 0x7000 for ap start
    vspace.map_generic(VAddr::from(MAP_BASE),
                (PAddr::from(MAP_BASE),
                KERNEL_BASE as usize - MAP_BASE as usize),
                MapAction::ReadWriteExecuteKernel);

    // Map kernel sections with appropriate permission bits
    vspace.map_generic(VAddr::from(KERNEL_BASE),
                (PAddr::from(KERNEL_BASE),
                text_end() as usize - KERNEL_BASE as usize),
                MapAction::ReadWriteExecuteKernel);

    vspace.map_generic(VAddr::from(rodata_start()),
                (PAddr::from(rodata_start()),
                rodata_end() as usize - rodata_start() as usize),
                MapAction::ReadKernel);

    vspace.map_generic(VAddr::from(data_start()),
                (PAddr::from(data_start()),
                data_end() as usize - data_start() as usize),
                MapAction::ReadWriteExecuteKernel);

    vspace.map_generic(VAddr::from(bss_start()),
                (PAddr::from(bss_start()),
                bss_end() as usize - bss_start() as usize),
                MapAction::ReadWriteExecuteKernel);

    vspace.map_generic(VAddr::from(tdata_start()),
                (PAddr::from(tdata_start()),
                kernel_end() as usize - tdata_start() as usize),
                MapAction::ReadWriteExecuteKernel);

    let mut frame = {
        if let Some(ref mut fmanager) = *BUDDY.lock() {
            fmanager.get_region()
        } else {
            panic!("__rust_allocate: buddy not initialized");
        }
    };

    // Map the regions held by buddy allocator
    vspace.map_identity(
        frame.base,
        frame.base + frame.size as u64,
        MapAction::ReadWriteExecuteKernel,
    );

    // Map LAPIC regions
    vspace.map_identity(
        PAddr(0xfec00000u64),
        PAddr(0xfec00000u64 + BASE_PAGE_SIZE as u64),
        MapAction::ReadWriteExecuteKernel,
    );

    vspace.map_identity(
        PAddr(0xfee00000u64),
        PAddr(0xfee00000u64 + BASE_PAGE_SIZE as u64),
        MapAction::ReadWriteExecuteKernel,
    );

    println!("pml4_vaddr {:x}", vspace.pml4_address());
    unsafe {
        println!("=> Switching to new PageTable!");
        controlregs::cr3_write(vspace.pml4_address().into());
        println!("Flushing TLB");
        x86::tlb::flush_all();
    }

    // We need the memory pointed to by vspace after exiting the scope
    core::mem::forget(vspace);
}
