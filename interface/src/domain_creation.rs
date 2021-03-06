use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Heap, Domain, Interrupt};
use crate::{bdev::{BDev, NvmeBDev}, vfs::VFS, usrnet::UsrNet, rv6::Rv6, dom_a::DomA, dom_c::DomC, net::Net, pci::{PCI, PciBar, PciResource}};
use crate::error::Result;

/* AB: XXX: first thing: change all names to create_domain -- it's absurd */
pub trait CreatePCI: Send + Sync {
    fn create_domain_pci(&self) -> (Box<dyn Domain>, Box<dyn PCI>);
}

pub trait CreateAHCI: Send + Sync {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateMemBDev: Send + Sync {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev>);
    fn recreate_domain_membdev(&self, dom: Box<dyn syscalls::Domain>, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateBDevShadow: Send + Sync {
    fn create_domain_bdev_shadow(&self, create: Arc<dyn CreateMemBDev>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateIxgbe: Send + Sync {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

pub trait CreateNetShadow: Send + Sync {
    fn create_domain_net_shadow(&self, create: Arc<dyn CreateIxgbe>, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

pub trait CreateNvmeShadow: Send + Sync {
    fn create_domain_nvme_shadow(&self, create: Arc<dyn CreateNvme>, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn NvmeBDev>);
}

pub trait CreateNvme: Send + Sync {
    fn create_domain_nvme(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn crate::bdev::NvmeBDev>);
}

pub trait CreateRv6FS: Send + Sync {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>);
}

pub trait CreateRv6Net: Send + Sync {
    fn create_domain_xv6net(&self, net: Box<dyn Net>) ->(Box<dyn Domain>, Box<dyn UsrNet>);
}

pub trait CreateRv6NetShadow: Send + Sync {
    fn create_domain_xv6net_shadow(&self, create: Arc<dyn CreateRv6Net>, net: Box<dyn Net>) ->(Box<dyn Domain>, Box<dyn UsrNet>);
}

pub trait CreateRv6Usr: Send + Sync {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn crate::rv6::Rv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>>;
}
pub type CreateRv6UsrPtr = Box<dyn CreateRv6Usr + Send + Sync>;

pub trait CreateRv6: Send + Sync {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn CreateRv6FS>,
                               create_xv6net: Arc<dyn CreateRv6Net>,
                               create_xv6net_shadow: Arc<dyn CreateRv6NetShadow>,
                               create_xv6usr: Arc<dyn CreateRv6Usr + Send + Sync>,
                               bdev: Box<dyn BDev>,
                               net: Box<dyn crate::net::Net>,
                               nvme: Box<dyn crate::bdev::NvmeBDev>,
                               usr_tpm: Box<dyn crate::tpm::UsrTpm>,
                            ) -> (Box<dyn Domain>, Box<dyn Rv6>);
}

pub trait CreateDomA: Send + Sync {
    fn create_domain_dom_a(&self) -> (Box<dyn Domain>, Box<dyn DomA>);
}

pub trait CreateDomB: Send + Sync {
    fn create_domain_dom_b(&self, dom_a: Box<dyn DomA>) -> Box<dyn Domain>;
}

pub trait CreateDomC: Send + Sync {
    fn create_domain_dom_c(&self) -> (Box<dyn Domain>, Box<dyn DomC>);
    fn recreate_domain_dom_c(&self, dom: Box<dyn Domain>) -> (Box<dyn Domain>, Box<dyn DomC>);
}

pub trait CreateDomD: Send + Sync {
    fn create_domain_dom_d(&self, dom_c: Box<dyn DomC>) -> Box<dyn Domain>;
}

pub trait CreateShadow: Send + Sync {
    fn create_domain_shadow(&self, create_dom_c: Arc<dyn CreateDomC>) -> (Box<dyn Domain>, Box<dyn DomC>);
}

pub trait CreateBenchnet: Send + Sync {
    fn create_domain_benchnet(&self, net: Box<dyn Net>) -> Box<dyn Domain>;
}

pub trait CreateBenchnvme: Send + Sync {
    fn create_domain_benchnvme(&self, nvme: Box<dyn NvmeBDev>) -> Box<dyn Domain>;
}

pub trait CreateHashStore: Send + Sync {
    fn create_domain_hashstore(&self) -> Box<dyn Domain>;
}

pub trait CreateTpm: Send + Sync {
    fn create_domain_tpm(&self) -> (Box<dyn Domain>, Box<dyn crate::tpm::UsrTpm>);
}
