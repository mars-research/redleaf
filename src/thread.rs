// AB: for now lets use a global lock, we'll get rid of it later
//pub static CONTEXT_SWITCH_LOCK: AtomicBool = AtomicBool::new(false);

use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::cell::RefCell;

const MAX_PRIO: usize = 15;

enum ThreadState {
    Running = 0,
    Runnable = 1,
    Paused = 2, 
}

const STACK_SIZE_IN_LINES: usize = 4096 * 2;

struct Stack {
    mem: [usize; STACK_SIZE_IN_LINES],
}

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

type Priority = usize;

type Link = Option<Box<Thread>>;

pub struct Thread {
    name: String,
    state: ThreadState, 
    priority: Priority, 
    context: Context,
    stack: RefCell<Box<Stack>>,
    // Next thread in the scheduling queue
    next: Link,
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

impl Stack {

    pub fn new() -> Stack {
        Stack{mem: [0; STACK_SIZE_IN_LINES]}
    }

}

impl Context {

    pub fn new() -> Context {
        Context{ r15: 0, r14: 0, r13:0, r12:0, r11:0, rbx:0, rbp:0, rsp:0, rflags:0 }
    }
}

impl  Thread {
/*
    /// Spawn a context from a function.
    pub fn spawn(&mut self, func: extern fn()) -> Result<&Arc<RwLock<Context>>> {
        let context_lock = self.new_context()?;
        {
            let mut context = context_lock.write();
            let mut fx = unsafe { Box::from_raw(crate::ALLOCATOR.alloc(Layout::from_size_align_unchecked(512, 16)) as *mut [u8; 512]) };
            for b in fx.iter_mut() {
                *b = 0;
            }
            let mut stack = vec![0; 65_536].into_boxed_slice();
            let offset = stack.len() - mem::size_of::<usize>();
            unsafe {
                let offset = stack.len() - mem::size_of::<usize>();
                let func_ptr = stack.as_mut_ptr().offset(offset as isize);
                *(func_ptr as *mut usize) = func as usize;
            }
            context.arch.set_page_table(unsafe { paging::ActivePageTable::new().address() });
            context.arch.set_fx(fx.as_ptr() as usize);
            context.arch.set_stack(stack.as_ptr() as usize + offset);
            context.kfx = Some(fx);
            context.kstack = Some(stack);
        }
        Ok(context_lock)
    }

*/
    
    fn init_stack(&mut self, func: extern fn()) {
       
        /* die() takes one argument lets pass it via r15 and prey */
        self.context.r15 = func as usize;

        /* push die() on the stack where the switch will pick 
         * it up with the ret instruction */
        let mut s = &mut **self.stack.borrow_mut(); 
        s.mem[s.mem.len() - 1] = die as usize;

        /* set the stack pointer to point to die() */
        //self.context.rsp = s.mem[s.mem.len() - 1].as_ptr(); 
        self.context.rsp = &(s.mem[s.mem.len() - 1]) as *const usize as usize;
    }

    pub fn new(name: &str, func: extern fn()) -> Thread  {
        let mut t = Thread {
            name: name.to_string(),
            state: ThreadState::Runnable, 
            priority: 0,
            context: Context::new(),
            stack: RefCell::new(Box::new(Stack::new())),
            next: None, 
        };

        t.init_stack(func);

        return t; 
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

    fn push_thread(&mut self, queue: usize, mut thread: Box<Thread>) {
        let previous_head = self.prio_queues[queue].take();

        if let Some(node) = previous_head {
            thread.next = Some(node);
        }
        self.prio_queues[queue] = Some(thread);
    }

    pub fn pop_thread(&mut self, queue: usize) -> Option<Box<Thread>> {
        let previous_head = self.prio_queues[queue].take();

        if let Some(mut node) = previous_head {
            self.prio_queues[queue] = node.next.take();
            Some(node)
        } else {
            None
        }
    }

    // Add thread to the queue that matches thread's priority
    pub fn put_thread(&mut self, mut thread: Box<Thread>) {
        let prio = thread.priority;
   
        self.push_thread(prio, thread); 

        if self.highest < prio {
            println!("set highest priority to {}", prio);
            self.highest = prio
        }
    }

    
    // Try to get the thread with the highest priority
    pub fn get_highest(&mut self) -> Option<Box<Thread>> {
        loop {
            match self.pop_thread(self.highest) {
                None => {
                    if self.highest == 0 {
                        return None;
                    }
                    self.highest += 1;
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

    pub fn put_thread(&mut self, mut thread: Box<Thread>) {
        /* put thread in the currently passive queue */
        if !self.active {
            self.active_queue.put_thread(thread)
        } else {
            self.passive_queue.put_thread(thread)
        }
    }

    fn get_next_active(&mut self) -> Option<Box<Thread>> {
        if self.active {
            //println!("get highest from active");
            self.active_queue.get_highest()
        } else {
            //println!("get highest from passive");
            self.passive_queue.get_highest()
        }
    }

    
    pub fn get_next(&mut self) -> Option<Box<Thread>> {
        return self.get_next_active();
    }   

    // Flip active and passive queue making active queue passive
    pub fn flip_queues(&mut self) {
        println!("flip queues");
        if self.active {
            self.active = false
        } else {
            self.active = true
        }
    }
    
    pub fn next(&mut self) -> Option<Box<Thread>> {
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
}


/// Just make sure die follows C calling convention
/// We don't really need it now as we pass the function pointer via r15
#[no_mangle] 
extern "C" fn die(/*func: extern fn()*/) {
    let func: extern fn();

    /* For now prey its still in r15 */
    unsafe{
        asm!("mov $0, r15" : "=r"(func) : : "memory" : "intel", "volatile");
    };

    println!("Starting new thread"); 

    // Enable interrupts so we get next scheduling tick
    x86_64::instructions::interrupts::enable();
    func();
    
    loop {
        println!("waiting to be cleaned up"); 
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

/* 
unsafe fn runnable(thread: &Thread) -> bool {
    thread.status == Status::Runnable
}

/// Do not call this while holding locks!
pub unsafe fn switch() -> bool {
    use core::ops::DerefMut;

    // Set the global lock to avoid the unsafe operations below from causing issues
    while arch::CONTEXT_SWITCH_LOCK.compare_and_swap(false, true, Ordering::SeqCst) {
        interrupt::pause();
    }

    let cpu_id = crate::cpu_id();

    let from_ptr;
    let mut to_ptr = 0 as *mut Context;
    {
        let contexts = contexts();
        {
            let context_lock = contexts
                .current()
                .expect("context::switch: not inside of context");
            let mut context = context_lock.write();
            from_ptr = context.deref_mut() as *mut Context;
        }

        for (_pid, context_lock) in contexts.iter() {
            let mut context = context_lock.write();
            update(&mut context, cpu_id);
        }

        for (pid, context_lock) in contexts.iter() {
            if *pid > (*from_ptr).id {
                let mut context = context_lock.write();
                if runnable(&mut context, cpu_id) {
                    to_ptr = context.deref_mut() as *mut Context;
                    break;
                }
            }
        }

        if to_ptr as usize == 0 {
            for (pid, context_lock) in contexts.iter() {
                if *pid < (*from_ptr).id {
                    let mut context = context_lock.write();
                    if runnable(&mut context, cpu_id) {
                        to_ptr = context.deref_mut() as *mut Context;
                        if (&mut *to_ptr).ksig.is_none() {
                            to_sig = context.pending.pop_front();
                        }
                        break;
                    }
                }
            }
        }
    };

    // Switch process states, TSS stack pointer, and store new context ID
    if to_ptr as usize != 0 {
        (&mut *from_ptr).state = Runnable;
        (&mut *to_ptr).state = Running;
        //if let Some(ref stack) = (*to_ptr).kstack {
        //    gdt::set_tss_stack(stack.as_ptr() as usize + stack.len());
        //}
        gdt::set_tcb((&mut *to_ptr).id.into());
        CONTEXT_ID.store((&mut *to_ptr).id, Ordering::SeqCst);
    }

    // Unset global lock before switch, as arch is only usable by the current CPU at this time
    arch::CONTEXT_SWITCH_LOCK.store(false, Ordering::SeqCst);

    if to_ptr as usize == 0 {
        // No target was found, return

        false
    } else {

        (&mut *from_ptr).arch.switch_to(&mut (&mut *to_ptr).arch);
        true
    }
}



*/
