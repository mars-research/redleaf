use alloc::vec::Vec;


use core::ops::Deref;
use alloc::boxed::Box;
use spin::{Mutex, MutexGuard};
use syscalls::Thread;

pub struct CondVarInternal {
    threads: Mutex<Vec<Box<dyn Thread>>>,
    intr_mutex: Mutex<()>,
}

impl CondVarInternal {
    pub fn new() -> Self {
        Self {
            threads: Mutex::new(Vec::new()),
            intr_mutex: Mutex::new(()),
        }
    }
}

pub struct CondVar(CondVarInternal);

impl CondVar {
    fn new() -> Self {
        Self(CondVarInternal::new())
    }
}

impl Deref for CondVar {
    type Target = CondVarInternal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl syscalls::CondVar for CondVar {
    fn sleep<'a>(&self, guard: MutexGuard<'a, ()>) {
        let intr_guard = self.intr_mutex.lock();

        drop(guard);

        let mut threads_guard = self.threads.lock();
        threads_guard.push(crate::thread::get_current_pthread());
        drop(threads_guard);

        crate::thread::get_current_pthread().sleep(intr_guard);
    }
    
    fn wakeup(&self) {
        let intr_guard = self.intr_mutex.lock();
        let mut threads_guard = self.threads.lock();
    
        if let Some(thread) = threads_guard.pop() {
            thread.set_state(syscalls::ThreadState::Runnable);
        }
    
        drop(threads_guard);
        drop(intr_guard);
    }
}


pub fn make_condvar() -> syscalls::CondVarPtr {
    box CondVar::new()
}