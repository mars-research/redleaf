use spin::{Mutex, MutexGuard};

pub struct CondVar {
    intr_mutex: Mutex<()>,
    cv: syscalls::CondVarPtr,
}

impl CondVar {
    pub fn new() -> Self {
        Self {
            intr_mutex: Mutex::new(()),
            cv: crate::syscalls::sys_make_condvar(),
        }
    }

    pub fn sleep(&self) {
        let m = Mutex::new(());
        self.sleep_until(&m, |_| true);
    }

    pub fn sleep_until<'a, T, F>(&self, mutex: &'a Mutex<T>, pred: F) -> MutexGuard<'a, T> where F: Fn(&mut T) -> bool {
        let mut intr_guard = self.intr_mutex.lock();
        let mut data_guard = mutex.lock();
        while !pred(&mut data_guard) {
            // Goes to sleep
            drop(data_guard);
            self.cv.sleep(intr_guard); // Atomically releases the guard and goes to sleep

            // After being waken up
            intr_guard = self.intr_mutex.lock();
            data_guard = mutex.lock();
        }

        drop(intr_guard);
        data_guard
    }
    
    pub fn wakeup(&self) {
        self.cv.wakeup()
    }
}

