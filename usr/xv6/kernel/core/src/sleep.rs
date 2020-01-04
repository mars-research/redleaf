use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_recv_int, sys_get_thread_id};
use syscalls::Syscall;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

pub struct WaitQueue {
    queue: Vec<u64>, // thread_ids
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            queue: Vec::new(),
        }
    }

    pub fn push_thread(&mut self, thread_id: u64) {
        self.queue.push_back(thread_id);
    }
}

pub fn sys_sleep(wait_queue: Arc<Mutex<WaitQueue>>) {
    let wait_queue_guard = wait_queue.lock();

    let thread_id = sys_get_thread_id();
    wait_queue_guard.push_thread(thread_id);

    let thread = sys_current_thread();
    thread.set_state(syscalls::ThreadState::Waiting);

    drop(wait_queue_guard);

    sys_yield();
}

pub fn sys_wakeup(wait_queue: Arc<Mutex<WaitQueue>>) {
    let wait_queue_guard = wait_queue.lock();

    for thread_id in wait_queue_guard.queue {
        // let thread = sys_get_thread(thread_id);
        // thread.set_state(syscalls::ThreadState::Runnable);
    }

    wait_queue_guard.threads.clear();

    drop(wait_queue_guard);
}
