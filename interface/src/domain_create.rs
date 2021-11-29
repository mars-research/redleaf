use crate::error::Result;
use crate::tpm::UsrTpm;
use crate::{
    bdev::{BDev, NvmeBDev},
    dom_c::DomC,
    net::Net,
    pci::{PciBar, PciResource, PCI},
    rv6::Rv6,
    usrnet::UsrNet,
    vfs::VFS,
};
/// Domain create related interfaces
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Domain, Heap, Interrupt};

#[domain_create(path = "dom_proxy", relative_path = "usr/proxy")]
pub trait CreateProxy {
    fn create_domain_proxy(
        &self,
        create_pci: Arc<dyn CreatePCI>,
        create_membdev: Arc<dyn CreateMemBDev>,
        create_bdev_shadow: Arc<dyn CreateBDevShadow>,
        create_ixgbe: Arc<dyn CreateIxgbe>,
        create_virtio_net: Arc<dyn crate::domain_create::CreateVirtioNet>,
        create_virtio_block: Arc<dyn crate::domain_create::CreateVirtioBlock>,
        create_nvme: Arc<dyn CreateNvme>,
        create_net_shadow: Arc<dyn crate::domain_create::CreateNetShadow>,
        create_nvme_shadow: Arc<dyn crate::domain_create::CreateNvmeShadow>,
        create_benchnvme: Arc<dyn crate::domain_create::CreateBenchnvme>,
        create_xv6fs: Arc<dyn CreateRv6FS>,
        create_xv6net: Arc<dyn crate::domain_create::CreateRv6Net>,
        create_xv6net_shadow: Arc<dyn crate::domain_create::CreateRv6NetShadow>,
        create_xv6usr: Arc<dyn CreateRv6Usr>,
        create_xv6: Arc<dyn CreateRv6>,
        create_dom_c: Arc<dyn CreateDomC>,
        create_dom_d: Arc<dyn CreateDomD>,
        create_shadow: Arc<dyn CreateShadow>,
        create_tpm: Arc<dyn CreateTpm>,
    ) -> (Box<dyn Domain>, Arc<dyn crate::proxy::Proxy>);
}

/* AB: XXX: first thing: change all names to create_domain -- it's absurd */
#[domain_create(path = "pci", relative_path = "sys/driver/pci")]
pub trait CreatePCI: Send + Sync {
    fn create_domain_pci(&self) -> (Box<dyn Domain>, Box<dyn PCI>);
}

// #[domain_create(path = "ahci")]
pub trait CreateAHCI: Send + Sync {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

#[domain_create(path = "membdev", relative_path = "sys/driver/membdev")]
pub trait CreateMemBDev: Send + Sync {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8])
        -> (Box<dyn Domain>, Box<dyn BDev>);
    fn recreate_domain_membdev(
        &self,
        dom: Box<dyn syscalls::Domain>,
        memdisk: &'static mut [u8],
    ) -> (Box<dyn Domain>, Box<dyn BDev>);
}

#[domain_create(path = "bdev_shadow", relative_path = "usr/shadow/bdev")]
pub trait CreateBDevShadow: Send + Sync {
    fn create_domain_bdev_shadow(
        &self,
        create: Arc<dyn CreateMemBDev>,
    ) -> (Box<dyn Domain>, Box<dyn BDev>);
}

#[domain_create(path = "ixgbe", relative_path = "sys/driver/ixgbe")]
pub trait CreateIxgbe: Send + Sync {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

#[domain_create(path = "virtio_net", relative_path = "sys/driver/virtio_net")]
pub trait CreateVirtioNet: Send + Sync {
    fn create_domain_virtio_net(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

#[domain_create(path = "virtio_block", relative_path = "sys/driver/virtio_block")]
pub trait CreateVirtioBlock: Send + Sync {
    fn create_domain_virtio_block(&self, pci: Box<dyn PCI>)
        -> (Box<dyn Domain>, Box<dyn NvmeBDev>);
}

#[domain_create(path = "net_shadow", relative_path = "usr/shadow/net")]
pub trait CreateNetShadow: Send + Sync {
    fn create_domain_net_shadow(
        &self,
        create: Arc<dyn CreateIxgbe>,
        pci: Box<dyn PCI>,
    ) -> (Box<dyn Domain>, Box<dyn Net>);
}

#[domain_create(path = "nvme_shadow", relative_path = "usr/shadow/nvme")]
pub trait CreateNvmeShadow: Send + Sync {
    fn create_domain_nvme_shadow(
        &self,
        create: Arc<dyn CreateNvme>,
        pci: Box<dyn PCI>,
    ) -> (Box<dyn Domain>, Box<dyn NvmeBDev>);
}

#[domain_create(path = "nvme", relative_path = "sys/driver/nvme")]
pub trait CreateNvme: Send + Sync {
    fn create_domain_nvme(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn NvmeBDev>);
}

#[domain_create(path = "xv6fs", relative_path = "usr/xv6/kernel/fs")]
pub trait CreateRv6FS: Send + Sync {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) -> (Box<dyn Domain>, Box<dyn VFS>);
}

#[domain_create(path = "xv6net", relative_path = "usr/xv6/kernel/net")]
pub trait CreateRv6Net: Send + Sync {
    fn create_domain_xv6net(&self, net: Box<dyn Net>) -> (Box<dyn Domain>, Box<dyn UsrNet>);
}

#[domain_create(path = "xv6net_shadow", relative_path = "usr/shadow/xv6net")]
pub trait CreateRv6NetShadow: Send + Sync {
    fn create_domain_xv6net_shadow(
        &self,
        create: Arc<dyn CreateRv6Net>,
        net: Box<dyn Net>,
    ) -> (Box<dyn Domain>, Box<dyn UsrNet>);
}

#[domain_create_blob(path = "xv6_user")]
pub trait CreateRv6Usr: Send + Sync {
    fn create_domain_xv6usr(
        &self,
        name: &str,
        blob: &[u8],
        xv6: Box<dyn crate::rv6::Rv6>,
        args: &str,
    ) -> (Box<dyn syscalls::Domain>, ());
}
pub type CreateRv6UsrPtr = Box<dyn CreateRv6Usr + Send + Sync>;

#[domain_create(path = "xv6kernel", relative_path = "usr/xv6/kernel/core")]
pub trait CreateRv6: Send + Sync {
    fn create_domain_xv6kernel(
        &self,
        ints: Box<dyn Interrupt>,
        create_xv6fs: Arc<dyn CreateRv6FS>,
        create_xv6net: Arc<dyn CreateRv6Net>,
        create_xv6net_shadow: Arc<dyn CreateRv6NetShadow>,
        create_xv6usr: Arc<dyn CreateRv6Usr>,
        bdev: Box<dyn BDev>,
        net: Box<dyn Net>,
        nvme: Box<dyn NvmeBDev>,
        usr_tpm: Box<dyn UsrTpm>,
    ) -> (Box<dyn Domain>, Box<dyn Rv6>);
}

#[domain_create(path = "dom_c", relative_path = "usr/test/dom_c")]
pub trait CreateDomC: Send + Sync {
    fn create_domain_dom_c(&self) -> (Box<dyn Domain>, Box<dyn DomC>);
    fn recreate_domain_dom_c(&self, dom: Box<dyn Domain>) -> (Box<dyn Domain>, Box<dyn DomC>);
}

#[domain_create(path = "dom_d", relative_path = "usr/test/dom_d")]
pub trait CreateDomD: Send + Sync {
    fn create_domain_dom_d(&self, dom_c: Box<dyn DomC>) -> (Box<dyn Domain>, ());
}

#[domain_create(path = "shadow", relative_path = "usr/test/shadow")]
pub trait CreateShadow: Send + Sync {
    fn create_domain_shadow(
        &self,
        create_dom_c: Arc<dyn CreateDomC>,
    ) -> (Box<dyn Domain>, Box<dyn DomC>);
}

// #[domain_create(path = "benchnet")]
pub trait CreateBenchnet: Send + Sync {
    fn create_domain_benchnet(&self, net: Box<dyn Net>) -> (Box<dyn Domain>, ());
}

#[domain_create(path = "benchnvme", relative_path = "usr/test/benchnvme")]
pub trait CreateBenchnvme: Send + Sync {
    fn create_domain_benchnvme(&self, nvme: Box<dyn NvmeBDev>) -> (Box<dyn Domain>, ());
}

// #[domain_create(path = "sashstore")]
pub trait CreateHashStore: Send + Sync {
    fn create_domain_hashstore(&self) -> (Box<dyn Domain>, ());
}

#[domain_create(path = "tpm", relative_path = "sys/driver/tpm")]
pub trait CreateTpm: Send + Sync {
    fn create_domain_tpm(&self) -> (Box<dyn Domain>, Box<dyn UsrTpm>);
}
