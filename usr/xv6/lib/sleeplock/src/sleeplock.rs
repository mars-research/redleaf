use spin::Mutex;
use alloc::sync::Arc;
use crate::sleep::{sys_sleep, sys_wakeup, WaitQueue};

pub struct SleepLock {
    locked: Mutex<bool>,
    wait_queue: Mutex<WaitQueue>,
}

impl SleepLock {
    pub fn new() -> SleepLock {
        SleepLock {
            locked: Mutex::new(false),
            wait_queue: Mutex::new(WaitQueue::new()),
        }
    }

    pub fn acquire(&self) {
        loop {
            let mut locked_guard = self.locked.lock();
            if *locked_guard == false {
                drop(locked_guard);
                sys_sleep(&self.wait_queue);
                continue;
            }
            *locked_guard = true;
            drop(locked_guard);
            break;
        }
    }

    pub fn release(&self) {
        let mut locked_guard = self.locked.lock();
        *locked_guard = false;
        sys_wakeup(&self.wait_queue);
        drop(locked_guard);
    }

    // TODO: implement fn holding(&self) -> Bool
}
