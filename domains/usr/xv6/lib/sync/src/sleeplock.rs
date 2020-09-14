use spin::Mutex;

use crate::CondVar;

pub struct SleepLock {
    locked: Mutex<bool>,
    wait_queue: CondVar,
}

impl SleepLock {
    pub fn new() -> SleepLock {
        SleepLock {
            locked: Mutex::new(false),
            wait_queue: CondVar::new(),
        }
    }

    pub fn acquire(&self) {
        loop {
            let mut locked_guard = self.locked.lock();
            if *locked_guard == false {
                drop(locked_guard);
                self.wait_queue.sleep();
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
        self.wait_queue.wakeup();
        drop(locked_guard);
    }

    // TODO: implement fn holding(&self) -> Bool
}
