#![no_std]
#![feature(associated_type_defaults)]

extern crate alloc;
use alloc::boxed::Box;
use core::panic::PanicInfo;
use core::any::TypeId;
use spin::{MutexGuard};
use core::alloc::Layout;
extern crate platform;
use platform::PciBarAddr;
use pc_keyboard::{DecodedKey};

/* AB: XXX: We should move this definition into a separate 
 * crate that deals with proxy syscalls (it's here to avoid the 
 * cyclic dependency unwind -> syscall (to call sys_register_cont), and 
 * syscall -> unwind (to get Continuation type definition 
 */
#[repr(C)]
#[derive(Copy,Clone, Debug)]
pub struct Continuation {
  pub func: u64,
  /* Caller saved registers (we need them since 
   * function arguments are passed in registers and 
   * we loose them for the restart */
  pub rax: u64,
  pub rcx: u64, 
  pub rdx: u64,
  pub rsi: u64,
  pub rdi: u64, 
  pub r8: u64, 
  pub r9: u64, 
  pub r10: u64,

  /* Callee saved registers */
  pub rflags: u64,
  pub r15: u64,
  pub r14: u64,
  pub r13: u64, 
  pub r12: u64,
  pub r11: u64, 
  pub rbx: u64, 
  pub rbp: u64,  
  pub rsp: u64,
}

impl Continuation {
    pub fn zeroed() -> Self {
        Self {
            func: 0,
            /* Caller saved registers (we need them since 
             * function arguments are passed in registers and 
             * we loose them for the restart */
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,

            /* Callee saved registers */
            rflags: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            rbx: 0,
            rbp: 0,
            rsp: 0,
        }
    }
}

pub mod errors;

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_println(&self, s: &str);
    fn sys_cpuid(&self) -> u32;
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>;
    fn sys_current_thread(&self) -> Box<dyn Thread>;
    fn sys_current_thread_id(&self) -> u64;
    fn sys_get_current_domain_id(&self) -> u64;
    unsafe fn sys_update_current_domain_id(&self, new_domain_id: u64) -> u64;
    unsafe fn sys_register_cont(&self, cont: &Continuation);
    unsafe fn sys_discard_cont(&self);
    fn sys_alloc(&self) -> *mut u8;
    fn sys_free(&self, p: *mut u8);
    fn sys_alloc_huge(&self, sz: u64) -> *mut u8;
    fn sys_free_huge(&self, p: *mut u8);
    fn sys_backtrace(&self);
    fn sys_dummy(&self);
    // call this one to read a character from keyboard
    fn sys_readch_kbd(&self) -> Result<Option<DecodedKey>, &'static str>; 
    fn sys_make_condvar(&self) -> CondVarPtr;

    fn sys_unwind(&self, cause: Option<UnwindCause>);

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
pub trait Domain: Send {
    fn get_domain_id(&self) -> u64;
}

/// Shared heap interface
#[derive(Copy, Clone)]
pub struct SharedHeapAllocation {
    pub value_pointer: *mut u8, // *mut T
    pub domain_id_pointer: *mut u64,
    pub borrow_count_pointer: *mut u64,
    pub layout: Layout,
    pub type_id: u64,
}

unsafe impl Send for SharedHeapAllocation {}

pub trait Heap {
    unsafe fn alloc(&self, layout: Layout, type_id: u64) -> Option<SharedHeapAllocation>;
    unsafe fn dealloc(&self, ptr: *mut u8);
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

pub enum UnwindCause {
    Panik,
}
