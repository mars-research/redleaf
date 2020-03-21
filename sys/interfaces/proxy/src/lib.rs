#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Domain};
use usr::{bdev};
use create::{CreatePCI, CreateAHCI, CreateIxgbe, CreateXv6FS, CreateXv6Usr, CreateXv6};

pub trait CreateProxy {
    fn create_domain_proxy(
        &self,
        create_pci: Box<dyn CreatePCI>,
        create_ahci: Box<dyn CreateAHCI>,
        create_ixgbe: Box<dyn CreateIxgbe>,
        create_xv6fs: Box<dyn CreateXv6FS>,
        create_xv6usr: Box<dyn CreateXv6Usr>,
        create_xv6: Box<dyn CreateXv6>) -> (Box<dyn Domain>, Arc<dyn Proxy>);
}

pub trait Proxy: CreatePCI + CreateAHCI + CreateIxgbe + CreateXv6FS + CreateXv6Usr + CreateXv6 {
    fn proxy_bdev(&self, bdev: Box<dyn bdev::BDev + Send + Sync>) -> Box<dyn bdev::BDev + Send + Sync>;
}
