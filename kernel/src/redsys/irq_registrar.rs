use super::resources::IRQ;
use crate::drivers::Driver;
use crate::interrupt::IRQManager;
use alloc::sync::Arc;
use core::mem::transmute;
use spin::Mutex;

pub struct IRQRegistrar<T: Driver + Send + 'static> {
    // Private fields
    _driver: Arc<Mutex<T>>,
    _irqManager: Arc<Mutex<IRQManager>>,
}

impl<T: Driver + Send + 'static> IRQRegistrar<T> {
    pub unsafe fn new(
        driver: Arc<Mutex<T>>,
        irqManager: Arc<Mutex<IRQManager>>,
    ) -> IRQRegistrar<T> {
        IRQRegistrar {
            _driver: driver,
            _irqManager: irqManager,
        }
    }

    pub fn request_irq(&self, irq: u8, callback: fn(&mut T)) -> Result<IRQ, &'static str> {
        let tcallback = unsafe { transmute::<fn(&mut T), fn(&mut (dyn Driver + Send))>(callback) };

        let mut guard = self._irqManager.lock();
        let result = unsafe { guard.register(self._driver.clone(), irq, tcallback) };

        result
    }

    pub fn request_any_irq(&self, callback: fn(&mut T)) -> Result<IRQ, &'static str> {
        let tcallback = unsafe { transmute::<fn(&mut T), fn(&mut (dyn Driver + Send))>(callback) };

        let mut guard = self._irqManager.lock();
        match guard.get_free_irq() {
            Some(irq) => unsafe { guard.register(self._driver.clone(), irq, tcallback) },
            None => Err("No free IRQ available"),
        }
    }
}
