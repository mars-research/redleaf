// AB: for now lets use a global lock, we'll get rid of it later
//pub static CONTEXT_SWITCH_LOCK: AtomicBool = AtomicBool::new(false);

use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::cell::RefCell;
//use alloc::rc::Rc;
use crate::halt;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use crate::domain::domain::{Domain, KERNEL_DOMAIN}; 
use crate::tls::cpuid; 
use core::sync::atomic::{AtomicU64, Ordering};
use crate::memory::VSPACE;
use crate::arch::memory::{BASE_PAGE_SIZE, PAddr};
use core::alloc::Layout;
use crate::memory::buddy::BUDDY;
use crate::memory::{PhysicalAllocator, Frame};

/// This should be a cryptographically secure number, for now 
/// just sequential ID
static THREAD_ID: AtomicU64 = AtomicU64::new(0);

const MAX_PRIO: usize = 15;
const MAX_CPUS: usize = 64;
const NULL_RETURN_MARKER: usize = 0x0000_0000;

/// Per-CPU scheduler
#[thread_local]
static SCHED: RefCell<Scheduler> = RefCell::new(Scheduler::new()); 

/// Per-CPU current thread
#[thread_local]
static CURRENT: RefCell<Option<Arc<Mutex<Thread>>>> = RefCell::new(None); 

static mut REBALANCE_FLAGS: RebalanceFlags = RebalanceFlags::new();
static REBALANCE_QUEUES: Mutex<RebalanceQueues> = Mutex::new(RebalanceQueues::new());

type Priority = usize;
pub type Link = Option<Arc<Mutex<Thread>>>;


#[repr(align(64))]
struct RebalanceFlag {
    rebalance: bool,
}

impl RebalanceFlag {
    const fn new() -> RebalanceFlag {
        RebalanceFlag { rebalance: false }
    }
}

struct RebalanceFlags {
    flags: [RebalanceFlag; MAX_CPUS],
}

// AB: I need this nested data structure hoping that 
// it will ensure cache-line alignment
impl RebalanceFlags {
    const fn new() -> RebalanceFlags {
        RebalanceFlags {
            flags : [RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(),
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), 
                     RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new(), RebalanceFlag::new()],
        }
    }
}

struct RebalanceQueues {
    queues: [Link; MAX_CPUS],
}

unsafe impl Sync for RebalanceQueues {} 
unsafe impl Send for RebalanceQueues {} 

impl RebalanceQueues {
    const fn new() -> RebalanceQueues {
        RebalanceQueues {
            queues: [None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None,
                     None, None, None, None, 
                     None, None, None, None],
        }
    }
}

fn rb_push_thread(queue: usize, thread: Arc<Mutex<Thread>>) {
    let mut rb_lock = REBALANCE_QUEUES.lock();

    let previous_head = rb_lock.queues[queue].take();

    if let Some(node) = previous_head {
        thread.lock().next = Some(node);
    } else {
        thread.lock().next = None; 
    }
    rb_lock.queues[queue] = Some(thread);
}


fn rb_pop_thread(queue: usize) -> Option<Arc<Mutex<Thread>>> {
    let mut rb_lock = REBALANCE_QUEUES.lock(); 
    let previous_head = rb_lock.queues[queue].take();

    if let Some(node) = previous_head {
        rb_lock.queues[queue] = node.lock().next.take();
        return Some(node);
    } else {
        return None;
    }
}

fn rb_queue_signal(queue: usize) {
    println!("cpu({}): rb queue signal, queue:{}", cpuid(), queue);
    unsafe {
        REBALANCE_FLAGS.flags[queue].rebalance = true; 
    };
}

fn rb_queue_clear_signal(queue: usize) {
    println!("cpu({}): rb clear signal, queue:{}", cpuid(), queue);
    unsafe {
        REBALANCE_FLAGS.flags[queue].rebalance = false; 
    };
}

fn rb_check_signal(queue: usize) -> bool {
    unsafe {
        REBALANCE_FLAGS.flags[queue].rebalance 
    }
}

/// Move thread to another CPU, affinity is CPU number for now
// We push thread on the rebalance queue (at the moment it's not 
// on the scheduling queue of this CPU), and signal rebalance request
// for the target CPU
fn rebalance_thread(t: Arc<Mutex<Thread>>) {
    // AB: TODO: treat affinity in a standard way as a bitmask
    // not as CPU number, yes I'm vomiting too
    let cpu_id = t.lock().affinity;
    
    rb_push_thread(cpu_id as usize, t);
    rb_queue_signal(cpu_id as usize); 
}

#[derive(Clone,Copy,Debug)]
pub enum ThreadState {
    Running = 0,
    Runnable = 1,
    Paused = 2,
    Waiting = 3, 
}

// AB: Watch out! if you change format of this line 
// you need to update the grep arguments in checkstack.mk
// Right now we have it as: 
//    grep "^pub const STACK_SIZE_IN_PAGES"
pub const STACK_SIZE_IN_PAGES: usize  = 4096;

pub struct Context {
  r15: usize,
  r14: usize,
  r13: usize, 
  r12: usize,
  r11: usize, 
  rbx: usize, 
  rbp: usize,  
  rsp: usize,
  rflags: usize,
}

pub struct Thread {
    pub id: u64, 
    pub name: String,
    pub state: ThreadState, 
    priority: Priority,
    affinity: u64,
    rebalance: bool, 
    context: Context,
    stack: *mut u64,
    domain: Option<Arc<Mutex<Domain>>>,
    // Next thread in the scheduling queue
    next: Link,
    // Next thread on the domain list 
    pub next_domain: Option<Arc<Mutex<Thread>>>,
    // Next thread on the interrupt wait queue list 
    pub next_iwq: Option<Arc<Mutex<Thread>>>,

}

struct SchedulerQueue {
    highest: Priority,
    prio_queues: [Link; MAX_PRIO + 1],
}

pub struct Scheduler {
    active: bool,
    active_queue: SchedulerQueue,
    passive_queue: SchedulerQueue,
}


impl Context {

    pub fn new() -> Context {
        Context{ r15: 0, r14: 0, r13:0, r12:0, r11:0, rbx:0, rbp:0, rsp:0, rflags:0 }
    }
}

pub unsafe fn alloc_stack() -> *mut u8 { 

    let layout = Layout::from_size_align(STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE, BASE_PAGE_SIZE).unwrap();

    let mut frame: Frame = Frame::new(PAddr::from(0), 0);

    if let Some(ref mut fmanager) = *BUDDY.lock() {
        unsafe {
            frame = fmanager.allocate(layout).unwrap()
        };
    };

    {
        let ref mut vspace = *VSPACE.lock();
        vspace.set_guard_page(frame.kernel_vaddr());
    }


    let stack_u8 = frame.kernel_vaddr().as_mut_ptr::<u8>();
    stack_u8
}

impl  Thread {
  
    fn init_stack(&mut self, func: extern fn()) {
       
        /* AB: XXX: die() takes one argument lets pass it via r15 and hope 
         * it will stay there */
        self.context.r15 = func as usize;

        let stack_u8 = unsafe { alloc_stack() };

        /* push 0x0 as the return address for die() so the backtrace 
         * terminates correctly */
        unsafe {
            let null_return = stack_u8.offset((STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE - core::mem::size_of::<*const usize>()) as isize) as *mut usize;
            *null_return = NULL_RETURN_MARKER; 
        };

        /* push die() on the stack where the switch will pick 
         * it up with the ret instruction */
        let die_return = unsafe {
            stack_u8.offset((STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE - 2*core::mem::size_of::<*const usize>()) as isize) as *mut usize
        };

        unsafe { 
            *die_return = die as usize; 
        }
        self.stack = stack_u8 as *mut u64;

        /* set the stack pointer to point to die() */
        self.context.rsp = die_return as usize;
    }

    pub fn new(name: &str, func: extern fn()) -> Thread  {
        let mut t = Thread {
            id: THREAD_ID.fetch_add(1, Ordering::SeqCst),
            name: name.to_string(),
            state: ThreadState::Runnable, 
            priority: 0,
            affinity: 0,
            rebalance: false,
            context: Context::new(),
            stack: 0 as *mut _, 
            domain: None, 
            next: None, 
            next_domain: None,
            next_iwq: None, 
        };

        t.init_stack(func);

        t
    }
}

impl  SchedulerQueue {

    pub const fn new() -> SchedulerQueue {
        SchedulerQueue {
            highest: 0,
            prio_queues: [None, None, None, None, None, None, None, None,
                          None, None, None, None, None, None, None, None],
        }
    }

    fn push_thread(&mut self, queue: usize, thread: Arc<Mutex<Thread>>) {
        let previous_head = self.prio_queues[queue].take();

        if let Some(node) = previous_head {
            thread.lock().next = Some(node);
        } else {
            thread.lock().next = None; 
        }

        self.prio_queues[queue] = Some(thread);
    }

    pub fn pop_thread(&mut self, queue: usize) -> Option<Arc<Mutex<Thread>>> {
        let previous_head = self.prio_queues[queue].take();

        if let Some(node) = previous_head {
            self.prio_queues[queue] = node.lock().next.take();
            Some(node)
        } else {
            None
        }
    }

    // Add thread to the queue that matches thread's priority
    pub fn put_thread(&mut self, thread: Arc<Mutex<Thread>>) {
        let prio = thread.lock().priority;
   
        self.push_thread(prio, thread); 

        if self.highest < prio {
            trace_sched!("set highest priority to {}", prio);
            self.highest = prio
        }
    }

    
    // Try to get the thread with the highest priority
    pub fn get_highest(&mut self) -> Option<Arc<Mutex<Thread>>> {
        loop {
            match self.pop_thread(self.highest) {
                None => {
                    if self.highest == 0 {
                        return None;
                    }
                    self.highest -= 1;
                },
                Some(t) => {
                   
                    return Some(t);
                },
            }
        }
    }

}

impl  Scheduler {

    pub const fn new() -> Scheduler {
        Scheduler {
            active: true,
            active_queue: SchedulerQueue::new(),
            passive_queue: SchedulerQueue::new(),
        }
    }

    pub fn put_thread_in_passive(&mut self, thread: Arc<Mutex<Thread>>) {
        /* put thread in the currently passive queue */
        if !self.active {
            self.active_queue.put_thread(thread)
        } else {
            self.passive_queue.put_thread(thread)
        }
    }

    fn get_next_active(&mut self) -> Option<Arc<Mutex<Thread>>> {
        if self.active {
            //println!("get highest from active");
            self.active_queue.get_highest()
        } else {
            //println!("get highest from passive");
            self.passive_queue.get_highest()
        }
    }

    
    pub fn get_next(&mut self) -> Option<Arc<Mutex<Thread>>> {
        return self.get_next_active();
    }   

    // Flip active and passive queue making active queue passive
    pub fn flip_queues(&mut self) {
        //println!("flip queues");
        if self.active {
            self.active = false
        } else {
            self.active = true
        }
    }
    
    pub fn next(&mut self) -> Option<Arc<Mutex<Thread>>> {
        if let Some(t) = self.get_next() {
            return Some(t);
        }
        
        // No luck finding a thread in the active queue
        // flip active and passive queues and try again
        self.flip_queues();
        
        if let Some(t) = self.get_next() {
            return Some(t);
        }
       
        return None;
    }

    /// Process rebalance queue
    fn process_rb_queue(&mut self) {
        let cpu_id = cpuid(); 
        println!("cpu({}): process rb queue", cpuid());
        loop{
            if let Some(thread) = rb_pop_thread(cpu_id) {

                println!("cpu({}): found rb thread: {}", cpuid(), thread.lock().name);

                {
                    thread.lock().rebalance = false; 
                }

                self.put_thread_in_passive(thread); 
                continue;
            } 

            break;
        }
        rb_queue_clear_signal(cpu_id); 
     }
}


/// Just make sure die follows C calling convention
/// We don't really need it now as we pass the function pointer via r15
#[no_mangle] 
extern "C" fn die(/*func: extern fn()*/) {
    let func: extern fn();

    /* AB: XXX: We assume that the funciton pointer is still in r15 */
    unsafe{
        asm!("mov $0, r15" : "=r"(func) : : "memory" : "intel", "volatile");
    };

    println!("Starting new thread"); 

    // Enable interrupts before exiting to user
    enable_irq();
    func();
    disable_irq();
    
    loop {
        println!("waiting to be cleaned up"); 
        do_yield();
    };
}


/// Switch to the next context by restoring its stack and registers
#[cold]
#[inline(never)]
#[naked]
pub unsafe fn switch(prev: *mut Thread, next: *mut Thread) {
    //asm!("fxsave64 [$0]" : : "r"(self.fx) : "memory" : "intel", "volatile");
    //self.loadable = true;
    //if next.loadable {
    //    asm!("fxrstor64 [$0]" : : "r"(next.fx) : "memory" : "intel", "volatile");
    //}else{
    //    asm!("fninit" : : : "memory" : "intel", "volatile");
    //}

    //asm!("mov $0, cr3" : "=r"(self.cr3) : : "memory" : "intel", "volatile");
    //if next.cr3 != self.cr3 {
    //    asm!("mov cr3, $0" : : "r"(next.cr3) : "memory" : "intel", "volatile");
    //}

    asm!("pushfq ; pop $0" : "=r"((*prev).context.rflags) : : "memory" : "intel", "volatile");
    asm!("push $0 ; popfq" : : "r"((*next).context.rflags) : "memory" : "intel", "volatile");

    asm!("mov $0, rbx" : "=r"((*prev).context.rbx) : : "memory" : "intel", "volatile");
    asm!("mov rbx, $0" : : "r"((*next).context.rbx) : "memory" : "intel", "volatile");

    asm!("mov $0, r12" : "=r"((*prev).context.r12) : : "memory" : "intel", "volatile");
    asm!("mov r12, $0" : : "r"((*next).context.r12) : "memory" : "intel", "volatile");

    asm!("mov $0, r13" : "=r"((*prev).context.r13) : : "memory" : "intel", "volatile");
    asm!("mov r13, $0" : : "r"((*next).context.r13) : "memory" : "intel", "volatile");

    asm!("mov $0, r14" : "=r"((*prev).context.r14) : : "memory" : "intel", "volatile");
    asm!("mov r14, $0" : : "r"((*next).context.r14) : "memory" : "intel", "volatile");

    asm!("mov $0, r15" : "=r"((*prev).context.r15) : : "memory" : "intel", "volatile");
    asm!("mov r15, $0" : : "r"((*next).context.r15) : "memory" : "intel", "volatile");

    asm!("mov $0, rsp" : "=r"((*prev).context.rsp) : : "memory" : "intel", "volatile");
    asm!("mov rsp, $0" : : "r"((*next).context.rsp) : "memory" : "intel", "volatile");

    asm!("mov $0, rbp" : "=r"((*prev).context.rbp) : : "memory" : "intel", "volatile");
    asm!("mov rbp, $0" : : "r"((*next).context.rbp) : "memory" : "intel", "volatile");
}

fn set_current(t: Arc<Mutex<Thread>>) {
    CURRENT.replace(Some(t)); 
}

fn get_current() -> Option<Arc<Mutex<Thread>>> {
    CURRENT.replace(None)
}

/// Return rc into the current thread
pub fn get_current_ref() -> Arc<Mutex<Thread>> {

    let rc_t = CURRENT.borrow().as_ref().unwrap().clone(); 
    rc_t
}

/// Return domain of the current thread
fn get_domain_of_current() -> Arc<Mutex<Domain>> {

    let rc_t = CURRENT.borrow().as_ref().unwrap().clone(); 
    let arc_d = rc_t.lock().domain.as_ref().unwrap().clone();

    arc_d
}

pub fn get_current_pthread() -> Box<PThread> {
    Box::new(PThread::new(get_current_ref().clone()))
}

// Kicked from the timer IRQ
pub fn schedule() {

    //println!("Schedule"); 

    let mut s = SCHED.borrow_mut();

    // Process rebalance requests
    if rb_check_signal(cpuid()) {
        s.process_rb_queue(); 
    }

    let next_thread = loop {
        let next_thread = match s.next() {
            Some(t) => t,
            None => {
                // Nothing again, current is the only runnable thread, no need to
                // context switch
                trace_sched!("cpu({}): no runnable threads", cpuid());
                return; 
            }

        };

        // Need to rebalance this thread, send it to another CPU
        if next_thread.lock().rebalance {
            rebalance_thread(next_thread); 
            continue; 
        }

        {
            let state = next_thread.lock().state; 

            // The thread is not runnable, put it back into the passive queue
            match state {
                ThreadState::Waiting => {
                    s.put_thread_in_passive(next_thread.clone()); 
                    continue; 
                },
                _ => {}
            }
        }

        break next_thread;
    };

    let c = match get_current() {
        Some(t) => t,
        None => { return; } 
    };


    trace_sched!("cpu({}): switch to {}", cpuid(), next_thread.borrow().name); 

    // Make next thread current
    set_current(next_thread.clone()); 

    // put the old thread back in the scheduling queue
    s.put_thread_in_passive(c.clone());

    drop(s); 

    let prev = unsafe {
        core::mem::transmute::<*mut Thread, &mut Thread>(&mut *c.lock())
    }; 
    let next = unsafe {
       core::mem::transmute::<*mut Thread, &mut Thread>(&mut *next_thread.lock())
    }; 

    drop(c);
    drop(next_thread); 

    unsafe {
        switch(prev, next);
    }

}


// yield is a reserved keyword
pub fn do_yield() {
    trace_sched!("Yield"); 
    schedule();
}

pub extern fn idle() {
    halt(); 
}

pub fn create_thread (name: &str, func: extern fn()) -> Box<PThread> {
    let mut s = SCHED.borrow_mut();

    let t = Arc::new(Mutex::new(Thread::new(name, func)));
    let pt = Box::new(PThread::new(Arc::clone(&t)));

    s.put_thread_in_passive(t);
    return pt; 
}

pub struct PThread {
    pub thread: Arc<Mutex<Thread>>
}

impl PThread {
    pub const fn new(t:Arc<Mutex<Thread>>) -> PThread {
        PThread {
            thread: t,
        }
    }
}

impl syscalls::Thread for PThread {
    fn get_id(&self) -> u64 {
        disable_irq();
        let tid = {
            self.thread.lock().id
        };
        enable_irq();
        tid
    }

    fn set_affinity(&self, affinity: u64) {
        disable_irq(); 

        if affinity as usize >= crate::tls::active_cpus() {
            println!("Error: trying to set affinity:{} for {} but only {} cpus are active", 
                affinity, self.thread.lock().name, crate::tls::active_cpus());
            enable_irq();
            return; 
        }

        {
            let mut thread = self.thread.lock(); 
        
            println!("Setting affinity:{} for {}", affinity, thread.name);
            thread.affinity = affinity; 
            thread.rebalance = true; 

        }
        enable_irq(); 
    }

    fn set_priority(&self, prio: u64) {
        disable_irq(); 

        if prio as usize > MAX_PRIO {
            println!("Error: trying to set priority:{} for {} but MAX_PRIO is only {}", 
                prio, self.thread.lock().name, MAX_PRIO);
            enable_irq();
            return; 
        }

        {
            let mut thread = self.thread.lock(); 
        
            println!("Setting priority:{} for {}", prio, thread.name);
            thread.priority = prio as usize; 

        }
        enable_irq(); 
    }

    fn set_state(&self, state: syscalls::ThreadState) {
        disable_irq(); 

        {
            let mut thread = self.thread.lock(); 
        
            println!("Setting state:{:#?} for {}", state, thread.name);
            match state {
                syscalls::ThreadState::Waiting => {
                    thread.state = ThreadState::Waiting;
                },

                syscalls::ThreadState::Runnable => {
                    thread.state = ThreadState::Runnable; 
                },
                _ => {
                    println!("Can't set {:#?} state for {}", state, thread.name);
                }

            }
            drop(thread);

        }
        enable_irq(); 
    }

}

pub fn init_threads() {
    let idle = Arc::new(Mutex::new(Thread::new("idle", idle)));
    
    let kernel_domain = KERNEL_DOMAIN.r#try().expect("Kernel domain is not initialized");

    idle.lock().domain = Some(kernel_domain.clone());

    // Make idle the current thread
    set_current(idle);   
}

