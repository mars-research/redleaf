// Adapted from Bespin OS code
use core::fmt;
use x86::bits64::paging;
use core::mem::transmute;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use slabmalloc::{Allocator, ZoneAllocator, AllocationError, ObjectPage, LargeObjectPage};
use crate::memory::buddy::{BUDDY, BuddyFrameAllocator};
use log::trace;
use crate::arch::KERNEL_END;
use core::ptr::NonNull;

pub use self::buddy::BuddyFrameAllocator as PhysicalMemoryAllocator;
pub use crate::arch::memory::{kernel_vaddr_to_paddr, paddr_to_kernel_vaddr, PAddr, VAddr, BASE_PAGE_SIZE};

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
            base,
            size,
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

#[allow(dead_code)]
pub struct SafeZoneAllocator {
    allocator: Mutex<ZoneAllocator<'static>>,

    // Should be dyn PhysicalAllocator, but let's not involve dynamic dispatch
    buddy: &'static Mutex<Option<BuddyFrameAllocator>>,
}

impl SafeZoneAllocator {
    pub const fn new(buddy: &'static Mutex<Option<BuddyFrameAllocator>>) -> SafeZoneAllocator {
        Self {
            allocator: Mutex::new(ZoneAllocator::new()),
            buddy,
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

unsafe impl GlobalAlloc for SafeZoneAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        trace!("alloc layout={:?}", layout);

        let mut buddy = self.buddy.lock();
        let mut buddy = buddy.as_mut()
            .expect("__rust_allocate: Buddy is not initialized");

        // Let's just pass most stuff to buddy I guess?
        match layout.size() {
            BASE_PAGE_SIZE => {
                // Use buddy directly
                let mut frame = buddy.allocate(layout)
                    .expect("__rust_allocate: Out of memory");

                frame.zero();
                frame.kernel_vaddr().as_mut_ptr()
            }
            0..=ZoneAllocator::MAX_ALLOC_SIZE => {
                // Ask zone allocator
                let mut zone_allocator = self.allocator.lock();
                match zone_allocator.allocate(layout) {
                    Ok(nptr) => nptr.as_ptr(),
                    Err(AllocationError::OutOfMemory) => {
                        // Allocator is hungry and needs new pages from
                        // the buddy :P

                        if layout.size() <= ZoneAllocator::MAX_BASE_ALLOC_SIZE {
                            let mut frame = buddy.allocate(layout)
                                .expect("__rust_allocate: Out of memory (ZoneAllocator)");

                            frame.zero();
                            let vframe: *mut u8 = frame.kernel_vaddr().as_mut_ptr();

                            zone_allocator.refill(layout, transmute(vframe as usize))
                                .expect("Failed to refill ZoneAllocator");
                        } else {
                            let huge_layout = layout.align_to(2 * 1024 * 1024)
                                .expect("Could not align?");
                            let mut frame = buddy.allocate(huge_layout)
                                .expect("__rust_allocate: Out of memory when allocating huge page (ZoneAllocator)");

                            frame.zero();
                            let vframe: *mut u8 = frame.kernel_vaddr().as_mut_ptr();
                            zone_allocator.refill_large(layout, transmute(vframe as usize))
                                .expect("Failed to refill ZoneAllocator with huge page");
                        }

                        // Let's try again
                        zone_allocator.allocate(layout)
                            .expect("Still failed to allocate after refill")
                            .as_ptr()
                    }
                    Err(AllocationError::InvalidLayout) => {
                        panic!("__rust_allocate: Invalid layout size (ZoneAllocator)");
                    }
                }
            }
            _ => {
                // Use buddy directly
                let mut frame = buddy.allocate(layout)
                    .expect("__rust_allocate: Out of memory");

                frame.zero();
                frame.kernel_vaddr().as_mut_ptr()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        trace!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
        let mut buddy = self.buddy.lock();
        let mut buddy = buddy.as_mut()
            .expect("__rust_allocate: Buddy is not initialized");

        let vaddr = VAddr::from_usize(ptr as usize);
        let frame = Frame::new(
            kernel_vaddr_to_paddr(vaddr),

            // As long as the layout is the same, we will be fine
            0,
        );

        // Let's just pass most stuff to buddy I guess?
        match layout.size() {
            BASE_PAGE_SIZE => {
                buddy.deallocate(frame, layout);
            }
            0..=ZoneAllocator::MAX_ALLOC_SIZE => {
                // Ask zone allocator
                if let Some(nptr) = NonNull::new(ptr) {
                    let mut zone_allocator = self.allocator.lock();
                    zone_allocator.deallocate(nptr, layout)
                        .expect("__rust_deallocate: Failed to deallocate (ZoneAllocator)");
                } else {
                }
            }
            _ => {
                // Use buddy directly
                buddy.deallocate(frame, layout);
            }
        }
    }
}

#[global_allocator]
pub static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&BUDDY);

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
const KERNEL_BASE: u64 = 0x10_0000;
const MAP_BASE: u64 = 0x0;

lazy_static! {
    pub static ref VSPACE: Mutex<VSpace> = Mutex::new(VSpace::new());
}

pub fn construct_pt() {
    
    {
        let ref mut vspace = *VSPACE.lock(); 

        // Map RWX as some code is copied to 0x7000 for ap start
        vspace.map_generic(VAddr::from(MAP_BASE),
                    (PAddr::from(MAP_BASE),
                    KERNEL_BASE as usize - MAP_BASE as usize),
                    MapAction::ReadWriteExecuteKernel).unwrap();

        // Map kernel sections with appropriate permission bits
        vspace.map_generic(VAddr::from(KERNEL_BASE),
                    (PAddr::from(KERNEL_BASE),
                    text_end() as usize - KERNEL_BASE as usize),
                    MapAction::ReadWriteExecuteKernel).unwrap();

        vspace.map_generic(VAddr::from(rodata_start()),
                    (PAddr::from(rodata_start()),
                    rodata_end() as usize - rodata_start() as usize),
                    MapAction::ReadKernel).unwrap();

        vspace.map_generic(VAddr::from(data_start()),
                    (PAddr::from(data_start()),
                    data_end() as usize - data_start() as usize),
                    MapAction::ReadWriteExecuteKernel).unwrap();

        vspace.map_generic(VAddr::from(bss_start()),
                    (PAddr::from(bss_start()),
                    bss_end() as usize - bss_start() as usize),
                    MapAction::ReadWriteExecuteKernel).unwrap();

        let kernel_end = unsafe { KERNEL_END };

        vspace.map_generic(VAddr::from(tdata_start()),
                    (PAddr::from(tdata_start()),
                    kernel_end as usize - tdata_start() as usize),
                    MapAction::ReadWriteExecuteKernel).unwrap();

        let frame = {
            if let Some(ref mut fmanager) = *BUDDY.lock() {
                fmanager.get_region()
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
        };

        assert!((frame.size % BASE_PAGE_SIZE) == 0);

        let num_4k_pages = frame.size / BASE_PAGE_SIZE;

        for page in 0..num_4k_pages {
            // Map the regions held by buddy allocator as 4k pages
            vspace.map_identity(
                frame.base + page * BASE_PAGE_SIZE,
                frame.base + (page + 1) * BASE_PAGE_SIZE,
                MapAction::ReadWriteExecuteKernel,
                );
        }

        // Map LAPIC regions
        vspace.map_identity(
            PAddr(0xfec0_0000u64),
            PAddr(0xfec0_0000u64 + BASE_PAGE_SIZE as u64),
            MapAction::ReadWriteExecuteKernel,
        );

        // Map TPM hardware region (5 pages) to our address space
        // From qemu doc at: https://www.qemu.org/docs/master/specs/tpm.html
        // The TIS interface makes a memory mapped IO region in the area 0xfed40000-0xfed44fff
        // available to the guest operating system.
        vspace.map_identity(
            PAddr(0xfed4_0000u64),
            PAddr(0xfed4_0000u64 + 5 * BASE_PAGE_SIZE as u64),
            MapAction::ReadWriteExecuteKernel,
        );

        vspace.map_identity(
            PAddr(0xfee0_0000u64),
            PAddr(0xfee0_0000u64 + BASE_PAGE_SIZE as u64),
            MapAction::ReadWriteExecuteKernel,
        );

        println!("pml4_vaddr {:x}", vspace.pml4_address());
        unsafe {
            println!("=> Switching to new PageTable!");
            controlregs::cr3_write(vspace.pml4_address().into());
        }
    }
    // We need the memory pointed to by vspace after exiting the scope
    // core::mem::forget(vspace);
}

pub fn construct_ap_pt() {
     {
        let ref mut vspace = *VSPACE.lock(); 
        println!("pml4_vaddr {:x}", vspace.pml4_address());
        unsafe {
            println!("=> Switching to new PageTable!");
            controlregs::cr3_write(vspace.pml4_address().into());
            println!("Flushing TLB");
            x86::tlb::flush_all();
        }
     }
} 

