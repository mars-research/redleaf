use crate::sync::SpinMutex;
use crate::sync::SpinMutexGuard;

pub struct SleepMutex<T> {
    intr_mutex: SpinMutex<()>,
    m: SpinMutex<T>,
    cv: syscalls::CondVarPtr,
}

impl<T> SleepMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            intr_mutex: SpinMutex::new(()),
            m: SpinMutex::new(data),
            cv: crate::syscalls::sys_make_condvar(),
        }
    }

    pub fn lock<'a>(&'a self) -> SleepMutexGuard<'a, T> {
        let mut intr_guard = self.intr_mutex.lock();

        loop {
            match self.m.try_lock() {
                None => {
                    self.cv.sleep(intr_guard);
                    intr_guard = self.intr_mutex.lock();
                },
                Some(guard) => {
                    drop(intr_guard);
                    return SleepMutexGuard::new(&self, guard);
                },  
            }
        }  
    }
}

pub struct SleepMutexGuard<'a, T> {
    mutex: &'a SleepMutex<T>,
    guard: SpinMutexGuard<'a, T>,
}

impl<'a, T> SleepMutexGuard<'a, T> {
    fn new(mutex: &'a SleepMutex<T>, guard: SpinMutexGuard<'a, T>) -> Self {
        Self {
            mutex,
            guard,
        }
    }
}

impl<'a, T> core::ops::Deref for SleepMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> core::ops::Drop for SleepMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.cv.wakeup();
    }
}
