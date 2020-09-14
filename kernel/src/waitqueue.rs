use core::cell::RefCell;
//use alloc::rc::Rc;
use alloc::sync::Arc; 
use spin::Mutex;
use crate::thread::Thread;

/// Per-CPU queues of interrupt threads
#[thread_local]
static INTERRUPT_WAIT_QUEUES: RefCell<InterruptWaitQueues> 
            = RefCell::new(InterruptWaitQueues::new());

pub const MAX_INT: usize = 256; 

/// Interrupt wait queues are local to CPU
struct InterruptWaitQueues {
    queues: [Option<Arc<Mutex<Thread>>>; MAX_INT]
}

impl InterruptWaitQueues {
    const fn new() -> InterruptWaitQueues {
        InterruptWaitQueues {
            queues: [None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None,
                     None, None, None, None, None, None, None, None]
        }
    }
}

impl InterruptWaitQueues {

    fn add_thread(&mut self, queue: usize, thread: Arc<Mutex<Thread>>) {
        let previous_head = self.queues[queue].take();

        if let Some(node) = previous_head {
            thread.lock().next_iwq = Some(node);
        } else {
            thread.lock().next_iwq = None; 
        }

        self.queues[queue] = Some(thread);
    }

    fn signal_threads(&mut self, queue: usize) {

        loop {
            let previous_head = self.queues[queue].take();

            if let Some(thread) = previous_head {
                trace_wq!("signal interrupt threads: int: {} thread {}", 
                    queue, thread.lock().name);
                self.queues[queue] = thread.lock().next_iwq.take();
                thread.lock().state = crate::thread::ThreadState::Runnable;
            } else {
                break;
            }

        };

        //crate::thread::do_yield()

    }

}

pub fn add_interrupt_thread(queue: usize, thread: Arc<Mutex<Thread>>) {
     INTERRUPT_WAIT_QUEUES.borrow_mut().add_thread(queue, thread);
}

pub fn signal_interrupt_threads(queue: usize) {
     INTERRUPT_WAIT_QUEUES.borrow_mut().signal_threads(queue);
}

