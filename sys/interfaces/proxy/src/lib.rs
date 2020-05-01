#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Domain};
use usr::{bdev};
use create::{CreatePCI, CreateAHCI, CreateMemBDev, CreateIxgbe, CreateXv6FS, CreateXv6Usr, CreateXv6};

pub trait CreateProxy {
    fn create_domain_proxy(
        &self,
        create_pci: Arc<dyn CreatePCI>,
        create_ahci: Arc<dyn CreateAHCI>,
        create_membdev: Arc<dyn CreateMemBDev>,
        create_ixgbe: Arc<dyn CreateIxgbe>,
        create_xv6fs: Arc<dyn CreateXv6FS>,
        create_xv6usr: Arc<dyn CreateXv6Usr + Send + Sync>,
        create_xv6: Arc<dyn CreateXv6>) -> (Box<dyn Domain>, Arc<dyn Proxy>);
}

pub trait Proxy: CreatePCI + CreateAHCI + CreateIxgbe + CreateXv6FS + CreateXv6Usr + CreateXv6 {
    // necessary because rust doesn't support trait object upcasting
    fn as_create_pci(&self) -> Arc<dyn CreatePCI>;
    fn as_create_ahci(&self) -> Arc<dyn CreateAHCI>;
    fn as_create_membdev(&self) -> Arc<dyn CreateMemBDev>;
    fn as_create_ixgbe(&self) -> Arc<dyn CreateIxgbe>;
    fn as_create_xv6fs(&self) -> Arc<dyn CreateXv6FS>;
    fn as_create_xv6usr(&self) -> Arc<dyn CreateXv6Usr + Send + Sync>;
    fn as_create_xv6(&self) -> Arc<dyn CreateXv6>;
}
