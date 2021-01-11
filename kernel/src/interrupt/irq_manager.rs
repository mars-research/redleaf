use crate::drivers::Driver;
use crate::redsys::resources::IRQ;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::iter::repeat;
use core::marker::Send;
use spin::Mutex;

#[macro_use]

const IRQ_MAX: u8 = 255 - crate::interrupt::IRQ_OFFSET;
const IRQ_COUNT: u8 = IRQ_MAX + 1;

pub struct IRQManager {
    _irqmap: Vec<Vec<(Arc<Mutex<dyn Driver + Send>>, fn(&mut (dyn Driver + Send)))>>,
    _handle: Option<Arc<Mutex<IRQManager>>>,
}

impl IRQManager {
    pub fn new() -> IRQManager {
        IRQManager {
            _irqmap: repeat(Vec::new())
                .take(IRQ_COUNT as usize)
                .collect::<Vec<_>>(),
            _handle: None,
        }
    }

    pub fn set_manager_handle(&mut self, handle: Arc<Mutex<IRQManager>>) {
        self._handle = Some(handle);
    }

    pub unsafe fn register(
        &mut self,
        driver: Arc<Mutex<dyn Driver + Send>>,
        irq: u8,
        callback: fn(&mut (dyn Driver + Send)),
    ) -> Result<IRQ, &'static str> {
        if irq > IRQ_MAX {
            return Err("Invalid IRQ number");
        }

        for (rdriver, _callback) in &self._irqmap[irq as usize] {
            if Arc::ptr_eq(&rdriver, &driver) {
                return Err("IRQ already registered");
            }
        }
        self._irqmap[irq as usize].push((driver.clone(), callback));

        let irq = IRQ {};
        Ok(irq)
    }

    pub fn handle_irq(&mut self, irq: u8) {
        if irq > IRQ_MAX {
            panic!("Invalid IRQ number");
        }

        for (rdriver, callback) in &self._irqmap[irq as usize] {
            let mut guard = rdriver.lock();
            callback(&mut *guard);
        }
    }

    pub fn get_free_irq(&self) -> Option<u8> {
        // FIXME: Restrict the range

        for i in 0u8..=IRQ_MAX {
            if self._irqmap[i as usize].len() == 0 {
                return Some(i);
            }
        }
        None
    }
}
