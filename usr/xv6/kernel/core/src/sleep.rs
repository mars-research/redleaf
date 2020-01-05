use libsyscalls::syscalls::{sys_yield, sys_current_thread};
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::boxed::Box;
use spin::Mutex;
use syscalls::Thread;

pub struct WaitQueue {
    threads: Vec<Box<dyn Thread>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            threads: Vec::new(),
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

    let thread = sys_current_thread();

    wait_queue_guard.push_thread(thread);

    drop(wait_queue_guard);
    // TODO: in between these two lines, a wakeup could be called, which this thread would miss
    // TODO: 'thread' variable above is moved into waitqueue, so we have to call sys_current_thread() again
    sys_current_thread().set_state(syscalls::ThreadState::Waiting);

    sys_yield();
}

pub fn sys_wakeup(wait_queue: &Mutex<WaitQueue>) {
    let mut wait_queue_guard = wait_queue.lock();

    while let Some(thread) = wait_queue_guard.pop_thread() {
        thread.set_state(syscalls::ThreadState::Runnable);
    }

    drop(wait_queue_guard);
}
