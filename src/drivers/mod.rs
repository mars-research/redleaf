use crate::redsys::IRQRegistrar;

pub mod ide;
pub mod hello;

pub trait Driver {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<Self>) where Self: Sized + Send;
}
