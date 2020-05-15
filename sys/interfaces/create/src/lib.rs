#![no_std]
#![feature(associated_type_defaults)]
extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Heap, Domain, Interrupt};
use usr::{bdev::BDev, vfs::VFS, xv6::Xv6, dom_a::DomA, dom_c::DomC, net::Net, pci::{PCI, PciBar, PciResource}};
use usr::error::Result;

/* AB: XXX: first thing: change all names to create_domain -- it's absurd */
pub trait CreatePCI {
    fn create_domain_pci(&self) -> (Box<dyn Domain>, Box<dyn PCI>);
}

pub trait CreateAHCI {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>);
}

pub trait CreateMemBDev {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>);
    fn recreate_domain_membdev(&self, dom: Box<dyn syscalls::Domain>, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>);
}

pub trait CreateBDevShadow {
    fn create_domain_bdev_shadow(&self, create: Arc<dyn CreateMemBDev>) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>);
}

pub trait CreateIxgbe {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net + Send>);
}

pub trait CreateXv6FS {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS + Send>);
}

pub trait CreateXv6Usr {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>>;
}
pub type CreateXv6UsrPtr = Box<dyn CreateXv6Usr + Send + Sync>;

pub trait CreateXv6 {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn CreateXv6FS>,
                               create_xv6usr: Arc<dyn CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn BDev + Send + Sync>,
                               net: Box<dyn usr::net::Net>) -> (Box<dyn Domain>, Box<dyn Xv6 + Send + Sync>);
}

pub trait CreateDomA {
    fn create_domain_dom_a(&self) -> (Box<dyn Domain>, Box<dyn DomA>);
}

pub trait CreateDomB {
    fn create_domain_dom_b(&self, dom_a: Box<dyn DomA>) -> Box<dyn Domain>;
}

pub trait CreateDomC {
    fn create_domain_dom_c(&self) -> (Box<dyn Domain>, Box<dyn DomC>);
    fn recreate_domain_dom_c(&self, dom: Box<dyn Domain>) -> (Box<dyn Domain>, Box<dyn DomC>);
}

pub trait CreateDomD {
    fn create_domain_dom_d(&self, dom_c: Box<dyn DomC>) -> Box<dyn Domain>;
}

pub trait CreateShadow {
    fn create_domain_shadow(&self, create_dom_c: Arc<dyn CreateDomC>) -> (Box<dyn Domain>, Box<dyn DomC>);
}

pub trait CreateBenchnet {
    fn create_domain_benchnet(&self, net: Box<dyn Net>) -> Box<dyn Domain>;
}
