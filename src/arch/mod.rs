pub mod memory;
pub mod vspace;

use crate::memory::{Frame};
use crate::arch::memory::{PAddr, BASE_PAGE_SIZE};
use crate::multibootv2::BootInformation;
use crate::memory::PhysicalAllocator;
use crate::memory::buddy::BUDDY;

const KERNEL_START: u64 = 0x10_0000;

pub fn kernel_end() -> u64 {
    extern {
        /// The starting byte of the thread data segment
        static __end: u8;
    }

    unsafe{
        & __end as *const _ as u64
    }
}

pub static mut KERNEL_END: u64 = 0;

pub fn init_buddy(bootinfo: &BootInformation) {
   // Find the physical memory regions available and add them to the physical memory manager
    crate::memory::buddy::BuddyFrameAllocator::init();
    println!("Finding RAM regions");
    if let Some(memory_map_tag) = bootinfo.memory_map_tag() {
        for region in memory_map_tag.memory_areas() {
            println!("{:x?}", region);
            if region.typ() == 1 {
                let mut base = region.start_address();
                let mut size: usize = region.size() as usize;
                let kernel_end = unsafe { KERNEL_END };

                if base >= KERNEL_START && base < kernel_end {
                    base = kernel_end;
                } 


                // TODO BAD: We can only add one region to the buddy allocator, so we need
                // to pick a big one weee
                if (base >= 1_0000_0000 as u64) && size > BASE_PAGE_SIZE && size >= 0x1_0000_0000 {
                    // downsize the region to 4GiB
                    size = 4 * 0x1_0000_0000 as usize;
                    println!("region.base = {:#x} region.size = {:#x}", base, size);
                    unsafe {
                        let f = Frame::new(PAddr::from(base), size);
                        if let Some(ref mut fmanager) = *BUDDY.lock() {
                            if fmanager.add_memory(f) {
                                println!("Added base={:#x} size={:#x}", base, size);
                            } else {
                                println!("Unable to add base={:#x} size={:#x}", base, size)
                            }
                        } else {
                            panic!("__rust_allocate: buddy not initialized");
                        }
                    }
                } else {
                    println!("Ignore memory region at {:?}", region);
                }
            }
        }
    }
    println!("added memory regions");
}
