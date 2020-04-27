#![no_std]
#![feature(associated_type_defaults)]
extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Heap, Domain, PCI, PciBar, PciResource, Net, Interrupt};
use usr::{bdev::BDev, vfs::VFS, xv6::Xv6, dom_a::DomA};

pub trait CreatePCI {
    fn create_domain_pci(&self, pci_resource: Box<dyn PciResource>,
                         pci_bar: Box<dyn PciBar>) -> (Box<dyn Domain>, Box<dyn PCI>);
    fn get_pci_resource(&self) -> Box<dyn PciResource>;
    fn get_pci_bar(&self) -> Box<dyn PciBar>;
}

pub trait CreateAHCI {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>);
}

pub trait CreateIxgbe {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

pub trait CreateXv6FS {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>);
}

pub trait CreateXv6Usr {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn Xv6>) -> Box<dyn Domain>;
}

pub trait CreateXv6 {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn CreateXv6FS>,
                               create_xv6usr: Arc<dyn CreateXv6Usr>,
                               bdev: Box<dyn BDev + Send + Sync>) -> Box<dyn Domain>;
}

pub trait CreateDomA {
    fn create_domain_dom_a(&self) -> (Box<dyn Domain>, Box<dyn DomA>);
}

pub trait CreateDomB {
    fn create_domain_dom_b(&self, dom_a: Box<dyn DomA>) -> Box<dyn Domain>;
}
