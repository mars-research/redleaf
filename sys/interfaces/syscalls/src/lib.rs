#![no_std]
#![feature(associated_type_defaults)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::collections::VecDeque;

use spin::{MutexGuard};
use core::alloc::Layout;
extern crate platform;
use platform::PciBarAddr;
use pc_keyboard::{DecodedKey};

pub mod errors;

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_println(&self, s: &str);
    fn sys_cpuid(&self) -> u32;
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>;
    fn sys_current_thread(&self) -> Box<dyn Thread>;
    fn sys_get_current_domain_id(&self) -> u64;
    unsafe fn sys_update_current_domain_id(&self, new_domain_id: u64) -> u64;
    fn sys_alloc(&self) -> *mut u8;
    fn sys_free(&self, p: *mut u8);
    fn sys_alloc_huge(&self, sz: u64) -> *mut u8;
    fn sys_free_huge(&self, p: *mut u8);
    fn sys_backtrace(&self);
    fn sys_dummy(&self);
    // call this one to read a character from keyboard
    fn sys_readch_kbd(&self) -> Result<Option<DecodedKey>, &'static str>; 
    fn sys_make_condvar(&self) -> CondVarPtr;
}

#[derive(Clone,Copy,Debug)]
pub enum ThreadState {
    Runnable = 1,
    Paused = 2,
    Waiting = 3, 
}

/// RedLeaf thread interface
pub trait Thread : Send {
    fn get_id(&self) -> u64;
    fn set_affinity(&self, affinity: u64);
    fn set_priority(&self, prio: u64);
    fn set_state(&self, state: ThreadState);
    fn sleep(&self, guard: MutexGuard<()>);
}

/// RedLeaf Domain interface
pub trait Domain {
    fn get_domain_id(&self) -> u64;
}

/// Shared heap interface
pub trait Heap {
    unsafe fn alloc(&self, domain_id: u64, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, domain_id: u64, ptr: *mut u8, layout: Layout);
    unsafe fn change_domain(&self, from_domain_id: u64, to_domain_id: u64, ptr: *mut u8, layout: Layout);
}

pub static IRQ_TIMER: u8 = 32;

pub trait Interrupt {
    // Recieve an interrupt
    fn sys_recv_int(&self, int: u8);
    fn int_clone(&self) -> Box<dyn Interrupt>;
}

pub trait Mmap {
    // Map bar region
    fn sys_mmap(&self, bar_addr: &PciBarAddr);
}

pub trait CondVar {
    // Atomically goes to sleep and release the guard
    fn sleep<'a>(&self, guard: MutexGuard<'a, ()>);
    // Wakes up one sleeping thread
    fn wakeup(&self);
}
pub type CondVarPtr = Box<dyn CondVar + Send + Sync>;
