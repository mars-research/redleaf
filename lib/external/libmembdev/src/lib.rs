#![no_std]

mod membdev;

pub use membdev::MemBDev;

extern crate alloc;
extern crate core;

extern "C" {
    fn _binary_build_fs_img_start();
    fn _binary_build_fs_img_end(); 
}

pub unsafe fn get_memdisk() -> &'static mut [u8] {
    let start = _binary_build_fs_img_start;
    let end = _binary_build_fs_img_end;
    let size = end as usize - start as usize;
    core::slice::from_raw_parts_mut(start as *mut u8, size)
} 
