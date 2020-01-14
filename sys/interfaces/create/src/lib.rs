#![no_std]
#![feature(associated_type_defaults)]
extern crate alloc;
use alloc::boxed::Box;
use syscalls::{Heap, Domain, PCI, PciBar, PciResource, Net, Interrupt};
use usr::{proxy::Proxy, bdev::BDev, vfs::VFS, xv6::Xv6};

pub trait CreateProxy {
    fn create_domain_proxy(&self, heap: Box<dyn Heap>) -> (Box<dyn Domain>, Box<dyn Proxy>);
}

pub trait CreatePCI {
    fn create_domain_pci(&self, pci_resource: Box<dyn PciResource>,
                         pci_bar: Box<dyn PciBar>) -> (Box<dyn Domain>, Box<dyn PCI>);
    fn get_pci_resource(&self) -> Box<dyn PciResource>;
    fn get_pci_bar(&self) -> Box<dyn PciBar>;
}

pub trait CreateAHCI {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateIxgbe {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

pub trait CreateXv6FS {
    fn create_domain_xv6fs(&self, proxy: Box<dyn Proxy>, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>);
}

pub trait CreateXv6Usr {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn Xv6>) -> Box<dyn Domain>;
}

pub trait CreateXv6 {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Box<dyn CreateXv6FS>,
                               create_xv6usr: Box<dyn CreateXv6Usr>,
                               proxy: Box<dyn Proxy>,
                               bdev: Box<dyn BDev>) -> Box<dyn Domain>;
}
