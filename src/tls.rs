use core::{mem, ptr};


pub const KERNEL_PERCPU_OFFSET: usize = 0x400000;
pub const KERNEL_PERCPU_SIZE: usize = 0x100000;

/// Copy tdata, clear tbss, set TCB self pointer
pub unsafe fn init_tcb(cpu_id: u32) -> usize {
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
        let size = & __tbss_end as *const _ as usize - & __tdata_start as *const _ as usize;
        let tbss_offset = & __tbss_start as *const _ as usize - & __tdata_start as *const _ as usize;

        let start = KERNEL_PERCPU_OFFSET + KERNEL_PERCPU_SIZE * (cpu_id as usize);
        let end = start + size;
        tcb_offset = end - mem::size_of::<usize>();

        ptr::copy(& __tdata_start as *const u8, start as *mut u8, tbss_offset);
        ptr::write_bytes((start + tbss_offset) as *mut u8, 0, size - tbss_offset);

        *(tcb_offset as *mut usize) = end;
    }
    tcb_offset
}


