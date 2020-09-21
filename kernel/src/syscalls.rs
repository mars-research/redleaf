use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::{VSpace, ResourceType};
use crate::arch::vspace::MapAction;
use crate::memory::{paddr_to_kernel_vaddr, VSPACE};
use x86::bits64::paging::BASE_PAGE_SIZE;
use alloc::boxed::Box; 
use spin::Mutex;
use alloc::sync::Arc; 
use crate::domain::domain::{Domain}; 
use crate::round_up;
use crate::thread;
use platform::PciBarAddr;
use crate::kbd::{KBDCTRL};
use pc_keyboard::{DecodedKey};
use crate::unwind::{register_cont, unwind};
use syscalls::Continuation; 

//extern crate syscalls; 


//pub static BOOT_SYSCALL: BootSyscall = BootSyscall {
//    sys_boot_syscall,
//};

//// AB: I was not able to pass Box<dyn Syscall> as an argument 
//// to user_ep() (maybe it's possible, I didn't have time to 
//// figure it out
//pub fn sys_boot_syscall() -> Box<dyn Syscall> {
//    let pdom = BOOTING_DOMAIN.replace(None);
//
//    enable_irq(); 
//    return pdom.unwrap();
//}

pub struct PDomain {
    domain: Arc<Mutex<Domain>>
}

impl PDomain {
    pub const fn new(dom: Arc<Mutex<Domain>>) -> PDomain {
        PDomain {
            domain: dom,
        }
    }
    
    fn create_domain_thread(&self, name: &str, func: extern fn()) -> Box<dyn syscalls::Thread>  {

        println!("sys_create_thread"); 
        let pt = create_thread(name, func);

        let t = pt.thread.clone(); 
    
        let mut d = self.domain.lock();
        d.add_thread(t);
        pt.thread.lock().current_domain_id = d.id;

        println!("Created thread {} for domain {}", pt.thread.lock().name, d.name); 
        pt   
    }
}

impl syscalls::Domain for PDomain {
    fn get_domain_id(&self) -> u64 {
        disable_irq();
        let domain_id = {
            self.domain.lock().id
        };
        enable_irq();
        domain_id
    }
}

impl syscalls::Syscall for PDomain {

    // Print a string 
    fn sys_print(&self, s: &str) {
        disable_irq();
        print!("{}", s);
        enable_irq(); 
    }
    
    // Print a string and a newline
    fn sys_println(&self, s: &str) {
        disable_irq();
        usrprintln!("{}", s);
        enable_irq(); 
    }

    // Get physical CPU id number (we use it for print) 
    fn sys_cpuid(&self) -> u32 {
        disable_irq();
        let cpuid = crate::console::cpuid();
        enable_irq();
        cpuid
    }

    fn sys_alloc(&self) -> *mut u8 {
        disable_irq();
        let paddr: PAddr = VSpace::allocate_one_page();
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        //println!("sys_alloc: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    fn sys_alloc_huge(&self, sz: u64) -> *mut u8 {
        let how_many = round_up!(sz as usize, BASE_PAGE_SIZE as usize) / BASE_PAGE_SIZE;
        disable_irq();
        let paddr: PAddr = VSpace::allocate_pages(how_many, ResourceType::Memory);
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        //println!("sys_alloc_huge: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    // todo: implement free!
    fn sys_free(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // todo: implement free!
    fn sys_free_huge(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // Yield to any thread
    fn sys_yield(&self) {
        disable_irq();
        trace_sched!("sys_yield"); 
        do_yield();
        enable_irq(); 
    }

    // Create a new thread
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn syscalls::Thread>  {
        disable_irq();
        let pt = self.create_domain_thread(name, func); 
        enable_irq();
        pt
    }

    fn sys_current_thread(&self) -> Box<dyn syscalls::Thread> {
        disable_irq();
        let current = crate::thread::get_current_pthread();
        enable_irq();
        current
    }

    fn sys_current_thread_id(&self) -> u64 {
        disable_irq();
        let thread_id = {
            // get domain id without locking the current thread
            let thread_option: &Option<Arc<Mutex<thread::Thread>>> = &thread::CURRENT.borrow();
            let thread_arc: &Arc<Mutex<thread::Thread>> = thread_option.as_ref().unwrap();
            let thread_mutex: &mut Mutex<thread::Thread> = unsafe {
                &mut *((&**thread_arc) as *const Mutex<thread::Thread> as *mut Mutex<thread::Thread>)
            };
            thread_mutex.get_mut().id
        };
        enable_irq();
        thread_id
    }

    fn sys_get_current_domain_id(&self) -> u64 {
        disable_irq();
        let domain_id = {
            // get domain id without locking the current thread
            let thread_option: &Option<Arc<Mutex<thread::Thread>>> = &thread::CURRENT.borrow();
            let thread_arc: &Arc<Mutex<thread::Thread>> = thread_option.as_ref().unwrap();
            let thread_mutex: &mut Mutex<thread::Thread> = unsafe {
                &mut *((&**thread_arc) as *const Mutex<thread::Thread> as *mut Mutex<thread::Thread>)
            };
            thread_mutex.get_mut().current_domain_id
        };
        enable_irq();
        domain_id
    }

    /* AB: XXX: move this syscall into a separate trait that is only 
     * accessible to proxy domain */
    unsafe fn sys_update_current_domain_id(&self, new_domain_id: u64) -> u64 {
        disable_irq();
        let mut old_domain_id = new_domain_id;
        {
            // swap domain id without locking the current thread
            let thread_option: &Option<Arc<Mutex<thread::Thread>>> = &thread::CURRENT.borrow();
            let thread_arc: &Arc<Mutex<thread::Thread>> = thread_option.as_ref().unwrap();
            let thread_mutex: &mut Mutex<thread::Thread> = unsafe {
                &mut *((&**thread_arc) as *const Mutex<thread::Thread> as *mut Mutex<thread::Thread>)
            };
            let mut thread = thread_mutex.get_mut();
            core::mem::swap(&mut thread.current_domain_id, &mut old_domain_id);
        }
        enable_irq();
        old_domain_id
    }


    /* AB: XXX: move this syscall into a separate trait that is only 
     * accessible to proxy domain */
    unsafe fn sys_register_cont(&self, cont: &Continuation) {
        disable_irq();
        register_cont(cont);
        enable_irq();
    }

    /* AB: XXX: Remove this system it's for testing only */
    fn sys_test_unwind(&self) {
        disable_irq();
        unwind();
        enable_irq();
    }



    fn sys_backtrace(&self) {
        use crate::panic::backtrace;
        disable_irq();
        backtrace();
        enable_irq();
    }

    fn sys_dummy(&self) {
        disable_irq();
        enable_irq();
    }

    fn sys_readch_kbd(&self) -> Result<Option<DecodedKey>, &'static str> {
        disable_irq();
        let rtn = KBDCTRL.lock().readch();
        enable_irq();
        rtn
    }

    fn sys_make_condvar(&self) -> syscalls::CondVarPtr {
        disable_irq();
        let rtn = crate::sync::condvar::make_condvar();
        enable_irq();
        rtn
    } 
}

#[derive(Clone)]
pub struct Interrupt {
}

impl Interrupt {
    pub const fn new() -> Interrupt {
        Interrupt {
        }
    }
}
 
impl syscalls::Interrupt for Interrupt {

    // Recieve an interrupt
    fn sys_recv_int(&self, int: u8) {
        disable_irq();
        if int as usize > crate::waitqueue::MAX_INT {
            println!("Interrupt {} doesn't exist", int); 
            enable_irq(); 
            return;
        }

        // take the thread off the scheduling queue
        // AB: XXX: for now just mark it as WAITING later we'll 
        // implement a real doubly-linked list and take it out
        let t = crate::thread::get_current_ref(); 
        t.lock().state = crate::thread::ThreadState::Waiting;

        crate::waitqueue::add_interrupt_thread(int as usize, t);
        
        do_yield();
        enable_irq();
    }

    fn int_clone(&self) -> Box<dyn syscalls::Interrupt> {
        Box::new((*self).clone())
    }


}

#[derive(Clone)]
pub struct Mmap {
}

impl Mmap {
    pub const fn new() -> Mmap {
        Mmap {
        }
    }
}
 
impl syscalls::Mmap for Mmap {

    // Recieve an interrupt
    fn sys_mmap(&self, bar_addr: &PciBarAddr) {
        disable_irq();

        let ref mut vspace = *VSPACE.lock();

        let base = unsafe { bar_addr.get_base() as u64 };
        let size = unsafe { bar_addr.get_size() };
        // identity map the bar region
        vspace.map_identity(PAddr::from(base), PAddr::from(base + size as u64),
                                        MapAction::ReadWriteKernelNoCache);
        println!("Mapping base {:x} size {:x}", base, size);
        enable_irq();
    }

/*    fn int_clone(&self) -> Box<dyn syscalls::Mmap> {
        Box::new((*self).clone())
    }*/
}
