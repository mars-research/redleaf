use crate::redsys::IRQRegistrar;

pub mod hello;
pub mod ide;
pub mod pfc;

pub trait Driver {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<Self>)
    where
        Self: Sized + Send;
}
