#![no_std]

extern crate alloc;
use alloc::boxed::Box;

//#[derive(Copy, Clone)]
//pub struct BootSyscall {
//    pub sys_boot_syscall: fn() -> Box<dyn Syscall>,
//}

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_println(&self, s: &str);
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>;
    fn sys_alloc(&self) -> *mut u8;
    fn sys_free(&self, p: *mut u8);
    fn sys_alloc_huge(&self, sz: u64) -> *mut u8;
    fn sys_free_huge(&self, p: *mut u8);
}

pub trait Thread {
    fn set_affinity(&self, affinity: u64);
}

pub trait PCI {
}

pub trait VFS {
}

pub trait BDev {
}


