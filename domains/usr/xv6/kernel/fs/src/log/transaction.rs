use alloc::sync::Arc;
use spin::Mutex;

use libsyscalls::sync::CondVar;

use crate::bcache::BufferGuard;
use crate::log::log::LogInternal;

pub struct Transaction {
    log: Arc<(Mutex<LogInternal>, CondVar)>,
}

impl Transaction {
    pub fn new(log: Arc<(Mutex<LogInternal>, CondVar)>) -> Self {
        let (log_internal, cv) = &*log;
        loop {
            let mut guard = log_internal.lock();
            if guard.try_begin_op() {
                break;
            }
            drop(guard);
            cv.sleep();
        }

        Self { log }
    }

    pub fn write(&mut self, buffer: &BufferGuard) {
        let (log_internal, _cv) = &*self.log;
        log_internal.lock().log_write(buffer);
    }
}

impl core::ops::Drop for Transaction {
    fn drop(&mut self) {
        let (log_internal, cv) = &*self.log;
        log_internal.lock().end_op();
        cv.wakeup();
    }
}
