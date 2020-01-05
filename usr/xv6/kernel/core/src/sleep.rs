use libsyscalls::syscalls::{sys_yield, sys_current_thread};
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::boxed::Box;
use spin::Mutex;
use syscalls::Thread;

pub struct WaitQueue {
    threads: Vec<Box<dyn Thread>>,
    intr_mutex: Arc<Mutex<()>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            threads: Vec::new(),
            intr_mutex: Arc::new(Mutex::new(())),
        }
    }

    pub fn push_thread(&mut self, thread: Box<dyn Thread>) {
        self.threads.push(thread);
    }

    pub fn pop_thread(&mut self) -> Option<Box<dyn Thread>> {
        self.threads.pop()
    }
}

pub fn sys_sleep(wait_queue: &Mutex<WaitQueue>) {
    let mut wait_queue_guard = wait_queue.lock();

    let intr_mutex = wait_queue_guard.intr_mutex.clone();
    let intr_guard = intr_mutex.lock();

    wait_queue_guard.push_thread(sys_current_thread());
    sys_current_thread().sleep(intr_guard);

    drop(wait_queue_guard);
}

pub fn sys_wakeup(wait_queue: &Mutex<WaitQueue>) {
    let mut wait_queue_guard = wait_queue.lock();

    let intr_mutex = wait_queue_guard.intr_mutex.clone();
    let intr_guard = intr_mutex.lock();

    while let Some(thread) = wait_queue_guard.pop_thread() {
        thread.set_state(syscalls::ThreadState::Runnable);
    }

    drop(intr_guard);
    drop(wait_queue_guard);
}
