use core::{mem, ptr};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::memory::VSPACE;
use crate::arch::memory::{BASE_PAGE_SIZE, PAddr};
use core::alloc::Layout;
use crate::memory::buddy::BUDDY;
use crate::memory::{PhysicalAllocator, Frame};


#[thread_local]
static mut THIS_CPU_ID: usize = 0;

static mut KERNEL_PER_CPU_AREA: *mut usize = 0x0 as *mut usize;
static mut KERNEL_PER_CPU_AREA_SIZE: usize = 0x0;

pub unsafe fn set_cpuid(id: usize) {
    unsafe {
        THIS_CPU_ID = id; 
    }

    println!("set cpu id:{}", id); 
}

pub fn cpuid() -> usize {
    unsafe {
        THIS_CPU_ID
    }
}

pub unsafe fn init_per_cpu_area(max_cpus: u32) {
    extern {
        /// The starting byte of the thread data segment
        static mut __tdata_start: u8;
        /// The ending byte of the thread BSS segment
        static mut __tbss_end: u8;
    }

    println!("Init per-CPU area");

    KERNEL_PER_CPU_AREA_SIZE = & __tbss_end as *const _ as usize - & __tdata_start as *const _ as usize;
    //KERNEL_PER_CPU_AREA =
    //            alloc::alloc::alloc(
    //                Layout::from_size_align_unchecked(KERNEL_PER_CPU_AREA_SIZE*(max_cpus as usize), 4096)) as *mut usize;

    let layout = Layout::from_size_align(KERNEL_PER_CPU_AREA_SIZE * (max_cpus as usize), BASE_PAGE_SIZE).unwrap();

    let mut frame: Frame = Frame::new(PAddr::from(0), 0);

    if let Some(ref mut fmanager) = *BUDDY.lock() {
        unsafe {
            frame = fmanager.allocate(layout).unwrap()
        };
    };

    KERNEL_PER_CPU_AREA = frame.kernel_vaddr().as_mut_ptr::<usize>();

    println!("KERNEL_PER_CPU_AREA: {:?}, KERNEL_PER_CPU_AREA_SIZE:{}", 
        KERNEL_PER_CPU_AREA,  KERNEL_PER_CPU_AREA_SIZE); 
}

/// Copy tdata, clear tbss, set TCB self pointer
pub unsafe fn init_per_cpu_vars(cpu_id: u32) -> usize {
     extern {
        /// The starting byte of the thread data segment
        static mut __tdata_start: u8;
        /// The ending byte of the thread data segment
        static mut __tdata_end: u8;
        /// The starting byte of the thread BSS segment
        static mut __tbss_start: u8;
        /// The ending byte of the thread BSS segment
        static mut __tbss_end: u8;
    }

   
    let tcb_offset;
    {
        let tbss_offset = & __tbss_start as *const _ as usize - & __tdata_start as *const _ as usize;

        let start = KERNEL_PER_CPU_AREA as usize + KERNEL_PER_CPU_AREA_SIZE * (cpu_id as usize);
        let end = start + KERNEL_PER_CPU_AREA_SIZE;
        tcb_offset = end - mem::size_of::<usize>();

        // Copy per-CPU data
        ptr::copy(& __tdata_start as *const u8, start as *mut u8, tbss_offset);
        // Set per-CPU BSS to 0
        ptr::write_bytes((start + tbss_offset) as *mut u8, 0, KERNEL_PER_CPU_AREA_SIZE - tbss_offset);

        *(tcb_offset as *mut usize) = end;
    }
    tcb_offset
}


