pub mod memory;

use crate::memory::{Frame};
use crate::arch::memory::{PAddr, BASE_PAGE_SIZE};
use crate::multibootv2::BootInformation;
use crate::memory::PhysicalAllocator;
use crate::memory::buddy::BUDDY;

pub fn init_buddy(bootinfo: BootInformation) {
    // Find the physical memory regions available and add them to the physical memory manager
    crate::memory::buddy::BuddyFrameAllocator::init();
    println!("Finding RAM regions");
    if let Some(memory_map_tag) = bootinfo.memory_map_tag() {
        for region in memory_map_tag.memory_areas() {
            println!("{:?}", region);
            if region.typ() == 1 {
                let base = region.start_address();
                let size: usize = region.size() as usize;

                // TODO BAD: We can only add one region to the buddy allocator, so we need
                // to pick a big one weee
                if base >= 0x100000 && size > BASE_PAGE_SIZE && size > 49152000 {
                    println!("region.base = {:#x} region.size = {:#x}", base, size);
                    unsafe {
                        let mut f = Frame::new(PAddr::from(base), size);
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
