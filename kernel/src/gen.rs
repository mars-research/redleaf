use syscalls;
use create;
use proxy;
use usr;

use spin::Mutex;
use alloc::sync::Arc;
use alloc::boxed::Box;

use usr::error::Result;

use crate::domain::load_domain;
use crate::syscalls::{PDomain, Interrupt, Mmap};
use crate::heap::PHeap;
use crate::interrupt::{disable_irq, enable_irq};
use crate::thread;

impl create::CreatePCI for PDomain {
    fn create_domain_pci(&self)
                         -> (Box<dyn syscalls::Domain>, Box<dyn usr::pci::PCI>) {
        disable_irq();
        let r = create_domain_pci();
        enable_irq();
        r
    }
}

impl create::CreateAHCI for PDomain {
    fn create_domain_ahci(&self,
                          pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
        disable_irq();
        let r = create_domain_ahci(pci);
        enable_irq();
        r
    }
}

impl create::CreateMemBDev for PDomain {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8]) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
        disable_irq();
        let r = create_domain_membdev(memdisk);
        enable_irq();
        r
    }

    fn recreate_domain_membdev(&self, _dom: Box<dyn syscalls::Domain>, memdisk: &'static mut [u8]) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
        disable_irq();
        let r = create_domain_membdev(memdisk);
        enable_irq();
        r
    }
}

impl create::CreateBDevShadow for PDomain {
    fn create_domain_bdev_shadow(&self, create: Arc<dyn create::CreateMemBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
        disable_irq();
        let r = create_domain_bdev_shadow(create);
        enable_irq();
        r
    }
}

impl create::CreateIxgbe for PDomain {
    fn create_domain_ixgbe(&self, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {
        disable_irq();
        let r = create_domain_ixgbe(pci);
        enable_irq();
        r
    }
}

impl create::CreateNetShadow for PDomain {
    fn create_domain_net_shadow(&self, create: Arc<dyn create::CreateIxgbe>, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {
        disable_irq();
        let r = create_domain_net_shadow(create, pci);
        enable_irq();
        r
    }
}

impl create::CreateNvmeShadow for PDomain {
    fn create_domain_nvme_shadow(&self, create: Arc<dyn create::CreateNvme>, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {
        disable_irq();
        let r = create_domain_nvme_shadow(create, pci);
        enable_irq();
        r
    }
}

impl create::CreateNvme for PDomain {
    fn create_domain_nvme(&self, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {
        disable_irq();
        let r = create_domain_nvme(pci);
        enable_irq();
        r
    }
}

impl create::CreateXv6 for PDomain {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn syscalls::Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6net: Arc<dyn create::CreateXv6Net>,
                               create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn usr::bdev::BDev>,
                               net: Box<dyn usr::net::Net>,
                               nvme: Box<dyn usr::bdev::NvmeBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::xv6::Xv6>) {
        disable_irq();
        let r = create_domain_xv6kernel(ints,
                                        create_xv6fs,
                                        create_xv6net,
                                        create_xv6net_shadow,
                                        create_xv6usr,
                                        bdev,
                                        net,
                                        nvme);
        enable_irq();
        r
    }
}

impl create::CreateXv6FS for PDomain {
    fn create_domain_xv6fs(&self, bdev: Box<dyn usr::bdev::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS>) {
        disable_irq();
        let r = create_domain_xv6fs(bdev);
        enable_irq();
        r
    }
}

impl create::CreateXv6Net for PDomain {
    fn create_domain_xv6net(&self, net: Box<dyn usr::net::Net>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>) {
        disable_irq();
        let r = create_domain_xv6net(net);
        enable_irq();
        r
    }
}

impl create::CreateXv6NetShadow for PDomain {
    fn create_domain_xv6net_shadow(&self, create: Arc<dyn create::CreateXv6Net>, net: Box<dyn usr::net::Net>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>) {
        disable_irq();
        let r = create_domain_xv6net_shadow(create, net);
        enable_irq();
        r
    }
}

impl create::CreateXv6Usr for PDomain {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>> {
        disable_irq();
        let r = create_domain_xv6usr(name, xv6, blob, args);
        enable_irq();
        r
    }
}

impl create::CreateDomA for PDomain {
    fn create_domain_dom_a(&self) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>) {
        disable_irq();
        let r = create_domain_dom_a();
        enable_irq();
        r
    }
}

impl create::CreateDomB for PDomain {
    fn create_domain_dom_b(&self, dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_dom_b(dom_a);
        enable_irq();
        r
    }
}

impl create::CreateDomC for PDomain {
    fn create_domain_dom_c(&self) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
        disable_irq();
        let r = create_domain_dom_c();
        enable_irq();
        r
    }

    fn recreate_domain_dom_c(&self, dom: Box<dyn syscalls::Domain>) ->
                            (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) 
    {
        disable_irq();
        let r = recreate_domain_dom_c(dom);
        enable_irq();
        r
    }

}

impl create::CreateDomD for PDomain {
    fn create_domain_dom_d(&self, dom_c: Box<dyn usr::dom_c::DomC>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_dom_d(dom_c);
        enable_irq();
        r
    }
}

impl create::CreateShadow for PDomain {
    fn create_domain_shadow(&self, create_dom_c: Arc<dyn create::CreateDomC>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
        disable_irq();
        let r = create_domain_shadow(create_dom_c);
        enable_irq();
        r
    }
}

impl create::CreateBenchnet for PDomain {
    fn create_domain_benchnet(&self, net: Box<dyn usr::net::Net>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_benchnet(net);
        enable_irq();
        r
    }
}

impl create::CreateBenchnvme for PDomain {
    fn create_domain_benchnvme(&self, nvme: Box<dyn usr::bdev::NvmeBDev>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_benchnvme(nvme);
        enable_irq();
        r
    }
}

impl create::CreateHashStore for PDomain {
    fn create_domain_hashstore(&self) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_hashstore();
        enable_irq();
        r
    }
}

impl create::CreateTpm for PDomain {
    fn create_domain_tpm(&self) -> (Box<dyn syscalls::Domain>, Box<dyn usr::tpm::TpmDev>) {
        disable_irq();
        let r = create_domain_tpm();
        enable_irq();
        r
    }
}

impl proxy::CreateProxy for PDomain {
    fn create_domain_proxy(
        &self,
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_membdev: Arc<dyn create::CreateMemBDev>,
        create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_nvme: Arc<dyn create::CreateNvme>,
        create_net_shadow: Arc<dyn create::CreateNetShadow>,
        create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
        create_benchnet: Arc<dyn create::CreateBenchnet>,
        create_benchnvme: Arc<dyn create::CreateBenchnvme>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6net: Arc<dyn create::CreateXv6Net>,
        create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>,
        create_dom_a: Arc<dyn create::CreateDomA>,
        create_dom_b: Arc<dyn create::CreateDomB>,
        create_dom_c: Arc<dyn create::CreateDomC>,
        create_dom_d: Arc<dyn create::CreateDomD>,
        create_shadow: Arc<dyn create::CreateShadow>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
        disable_irq();
        let r = create_domain_proxy(
            create_pci,
            create_ahci,
            create_membdev,
            create_bdev_shadow,
            create_ixgbe,
            create_nvme,
            create_net_shadow,
            create_nvme_shadow,
            create_benchnet,
            create_benchnvme,
            create_xv6fs,
            create_xv6net,
            create_xv6net_shadow,
            create_xv6usr,
            create_xv6,
            create_dom_a,
            create_dom_b,
            create_dom_c,
            create_dom_d,
            create_shadow);
        enable_irq();
        r
    }
}

pub fn create_domain_init() -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_redleaf_init_start();
        fn _binary_domains_build_redleaf_init_end();
    }

    let binary_range = (
        _binary_domains_build_redleaf_init_start as *const u8,
        _binary_domains_build_redleaf_init_end as *const u8
    );

    return build_domain_init("sys_init", binary_range);
}

pub fn create_domain_pci() -> (Box<dyn syscalls::Domain>,
                               Box<dyn usr::pci::PCI>) {

    extern "C" {
        fn _binary_domains_build_pci_start();
        fn _binary_domains_build_pci_end();
    }

    let binary_range = (
        _binary_domains_build_pci_start as *const u8,
        _binary_domains_build_pci_end as *const u8
    );

    create_domain_pci_bus("pci", binary_range)
}

pub fn create_domain_ahci(pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {

    // extern "C" {
    //     fn _binary_domains_build_ahci_driver_start();
    //     fn _binary_domains_build_ahci_driver_end();
    // }

    // let binary_range = (
    //     _binary_domains_build_ahci_driver_start as *const u8,
    //     _binary_domains_build_ahci_driver_end as *const u8
    // );

    // create_domain_bdev("ahci", binary_range, pci)
    unimplemented!()
}

pub fn create_domain_ixgbe(pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {

    extern "C" {
        fn _binary_domains_build_ixgbe_start();
        fn _binary_domains_build_ixgbe_end();
    }

    let binary_range = (
        _binary_domains_build_ixgbe_start as *const u8,
        _binary_domains_build_ixgbe_end as *const u8
    );

    create_domain_net("ixgbe_driver", binary_range, pci)
}

pub fn create_domain_net_shadow(create: Arc<dyn create::CreateIxgbe>, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {

    extern "C" {
        fn _binary_domains_build_net_shadow_start();
        fn _binary_domains_build_net_shadow_end();
    }

    let binary_range = (
        _binary_domains_build_net_shadow_start as *const u8,
        _binary_domains_build_net_shadow_end as *const u8
    );

    build_domain_net_shadow("net_shadow", binary_range, create, pci)
}

pub fn create_domain_nvme_shadow(create: Arc<dyn create::CreateNvme>, pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {

    extern "C" {
        fn _binary_domains_build_nvme_shadow_start();
        fn _binary_domains_build_nvme_shadow_end();
    }

    let binary_range = (
        _binary_domains_build_nvme_shadow_start as *const u8,
        _binary_domains_build_nvme_shadow_end as *const u8
    );

    build_domain_nvme_shadow("nvme_shadow", binary_range, create, pci)
}

pub fn create_domain_nvme(pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {

    extern "C" {
        fn _binary_domains_build_nvme_start();
        fn _binary_domains_build_nvme_end();
    }

    let binary_range = (
        _binary_domains_build_nvme_start as *const u8,
        _binary_domains_build_nvme_end as *const u8
    );

    create_domain_nvmedev("nvme_driver", binary_range, pci)
}

pub fn create_domain_membdev(memdisk: &'static mut [u8]) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
    #[cfg(debug_assertions)]
    let binary_range  = {
        extern "C" {
            fn _binary_domains_build_membdev_start();
            fn _binary_domains_build_membdev_end();
        }

        (
            _binary_domains_build_membdev_start as *const u8,
            _binary_domains_build_membdev_end as *const u8
        )
    };
    #[cfg(not(debug_assertions))]
    let binary_range  = {
        extern "C" {
            fn _binary_domains_build_membdev_start();
            fn _binary_domains_build_membdev_end();
        }

        (
            _binary_domains_build_membdev_start as *const u8,
            _binary_domains_build_membdev_end as *const u8
        )
    };

    create_domain_bdev_mem("membdev", binary_range, memdisk)
}

pub fn create_domain_bdev_shadow(create: Arc<dyn create::CreateMemBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
    #[cfg(debug_assertions)]
    let binary_range  = {
        extern "C" {
            fn _binary_domains_build_bdev_shadow_start();
            fn _binary_domains_build_bdev_shadow_end();
        }

        (
            _binary_domains_build_bdev_shadow_start as *const u8,
            _binary_domains_build_bdev_shadow_end as *const u8
        )
    };
    #[cfg(not(debug_assertions))]
    let binary_range  = {
        extern "C" {
            fn _binary_domains_build_bdev_shadow_start();
            fn _binary_domains_build_bdev_shadow_end();
        }

        (
            _binary_domains_build_bdev_shadow_start as *const u8,
            _binary_domains_build_bdev_shadow_end as *const u8
        )
    };

    create_domain_bdev_shadow_helper("bdev_shadow", binary_range, create)
}

pub fn create_domain_xv6kernel(ints: Box<dyn syscalls::Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6net: Arc<dyn create::CreateXv6Net>,
                               create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn usr::bdev::BDev>,
                               net: Box<dyn usr::net::Net>,
                               nvme: Box<dyn usr::bdev::NvmeBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::xv6::Xv6>) {
    extern "C" {
        fn _binary_domains_build_xv6kernel_start();
        fn _binary_domains_build_xv6kernel_end();
    }

    let binary_range = (
        _binary_domains_build_xv6kernel_start as *const u8,
        _binary_domains_build_xv6kernel_end as *const u8
    );

    build_domain_xv6kernel("xv6kernel", binary_range, ints, create_xv6fs, create_xv6net, create_xv6net_shadow, create_xv6usr, bdev, net, nvme)
}


pub fn create_domain_xv6fs(bdev: Box<dyn usr::bdev::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS>) {

    extern "C" {
        fn _binary_domains_build_xv6fs_start();
        fn _binary_domains_build_xv6fs_end();
    }

    let binary_range = (
        _binary_domains_build_xv6fs_start as *const u8,
        _binary_domains_build_xv6fs_end as *const u8
    );

    build_domain_fs("xv6fs", binary_range, bdev)
}

// AB: We have to split ukern syscalls into some that are
// accessible to xv6 user, e.g., memory management, and the rest
// which is hidden, e.g., create_thread, etc.
pub fn create_domain_xv6usr(name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>> {
    // TODO: verify that the blob is signed
    // if !signed(blob) return Err("Not signed")

    Ok(build_domain_xv6usr(name, xv6, blob, args))
}

pub fn create_domain_xv6net(net: Box<dyn usr::net::Net>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>) {

    extern "C" {
        fn _binary_domains_build_xv6net_start();
        fn _binary_domains_build_xv6net_end();
    }

    let binary_range = (
        _binary_domains_build_xv6net_start as *const u8,
        _binary_domains_build_xv6net_end as *const u8
    );


    build_domain_xv6net("xv6net", binary_range, net)
}

pub fn create_domain_xv6net_shadow(create: Arc<dyn create::CreateXv6Net>, net: Box<dyn usr::net::Net>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>) {

    extern "C" {
        fn _binary_domains_build_xv6net_shadow_start();
        fn _binary_domains_build_xv6net_shadow_end();
    }

    let binary_range = (
        _binary_domains_build_xv6net_shadow_start as *const u8,
        _binary_domains_build_xv6net_shadow_end as *const u8
    );


    build_domain_xv6net_shadow("xv6net_shadow", binary_range, create, net)
}

pub fn create_domain_dom_a() -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>) {
    extern "C" {
        fn _binary_domains_build_dom_a_start();
        fn _binary_domains_build_dom_a_end();
    }

    let binary_range = (
        _binary_domains_build_dom_a_start as *const u8,
        _binary_domains_build_dom_a_end as *const u8
    );

    build_domain_dom_a("dom_a", binary_range)
}

pub fn create_domain_dom_b(dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_dom_b_start();
        fn _binary_domains_build_dom_b_end();
    }

    let binary_range = (
        _binary_domains_build_dom_b_start as *const u8,
        _binary_domains_build_dom_b_end as *const u8
    );

    build_domain_dom_b("dom_b", binary_range, dom_a)
}

pub fn create_domain_dom_c() -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
    extern "C" {
        fn _binary_domains_build_dom_c_start();
        fn _binary_domains_build_dom_c_end();
    }

    let binary_range = (
        _binary_domains_build_dom_c_start as *const u8,
        _binary_domains_build_dom_c_end as *const u8
    );

    build_domain_dom_c("dom_c", binary_range)
}

pub fn recreate_domain_dom_c(dom: Box<dyn syscalls::Domain>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
    extern "C" {
        fn _binary_domains_build_dom_c_start();
        fn _binary_domains_build_dom_c_end();
    }

    let binary_range = (
        _binary_domains_build_dom_c_start as *const u8,
        _binary_domains_build_dom_c_end as *const u8
    );

    build_domain_dom_c("dom_c", binary_range)
}


pub fn create_domain_dom_d(dom_c: Box<dyn usr::dom_c::DomC>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_dom_d_start();
        fn _binary_domains_build_dom_d_end();
    }

    let binary_range = (
        _binary_domains_build_dom_d_start as *const u8,
        _binary_domains_build_dom_d_end as *const u8
    );

    build_domain_dom_d("dom_d", binary_range, dom_c)
}

pub fn create_domain_shadow(create_dom_c: Arc<dyn create::CreateDomC>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
    extern "C" {
        fn _binary_domains_build_shadow_start();
        fn _binary_domains_build_shadow_end();
    }

    let binary_range = (
        _binary_domains_build_shadow_start as *const u8,
        _binary_domains_build_shadow_end as *const u8
    );

    build_domain_shadow("shadow", binary_range, create_dom_c)
}

pub fn create_domain_benchnet(net: Box<dyn usr::net::Net>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_benchnet_inside_start();
        fn _binary_domains_build_benchnet_inside_end();
    }

    let binary_range = (
        _binary_domains_build_benchnet_inside_start as *const u8,
        _binary_domains_build_benchnet_inside_end as *const u8
    );

    build_domain_benchnet_helper("benchnet", binary_range, net)
}

pub fn create_domain_benchnvme(nvme: Box<dyn usr::bdev::NvmeBDev>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_benchnvme_start();
        fn _binary_domains_build_benchnvme_end();
    }

    let binary_range = (
        _binary_domains_build_benchnvme_start as *const u8,
        _binary_domains_build_benchnvme_end as *const u8
    );

    build_domain_benchnvme("benchnvme", binary_range, nvme)
}

pub fn create_domain_hashstore() -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_domains_build_benchhash_start();
        fn _binary_domains_build_benchhash_end();
    }

    let binary_range = (
        _binary_domains_build_benchhash_start as *const u8,
        _binary_domains_build_benchhash_end as *const u8
    );

    build_domain_hashstore("benchhash", binary_range)
}

pub fn create_domain_tpm() -> (Box<dyn syscalls::Domain>, Box<dyn usr::tpm::TpmDev>) {

    extern "C" {
        fn _binary_domains_build_tpm_start();
        fn _binary_domains_build_tpm_end();
    }

    let binary_range = (
        _binary_domains_build_tpm_start as *const u8,
        _binary_domains_build_tpm_end as *const u8
    );

    build_domain_tpm("tpm_driver", binary_range)
}

pub fn create_domain_proxy(
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_nvme: Arc<dyn create::CreateNvme>,
    create_net_shadow: Arc<dyn create::CreateNetShadow>,
    create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
    create_benchnet: Arc<dyn create::CreateBenchnet>,
    create_benchnvme: Arc<dyn create::CreateBenchnvme>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6net: Arc<dyn create::CreateXv6Net>,
    create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>,
    create_dom_c: Arc<dyn create::CreateDomC>,
    create_dom_d: Arc<dyn create::CreateDomD>,
    create_shadow: Arc<dyn create::CreateShadow>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
    extern "C" {
        fn _binary_domains_build_dom_proxy_start();
        fn _binary_domains_build_dom_proxy_end();
    }

    let binary_range = (
        _binary_domains_build_dom_proxy_start as *const u8,
        _binary_domains_build_dom_proxy_end as *const u8
    );

    build_domain_proxy(
        "dom_proxy",
        binary_range,
        create_pci,
        create_ahci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_nvme,
        create_net_shadow,
        create_nvme_shadow,
        create_benchnet,
        create_benchnvme,
        create_xv6fs,
        create_xv6net,
        create_xv6net_shadow,
        create_xv6usr,
        create_xv6,
        create_dom_a,
        create_dom_b,
        create_dom_c,
        create_dom_d,
        create_shadow)
}

pub fn create_domain_pci_bus(name: &str,
                             binary_range: (*const u8, *const u8))
                             -> (Box<dyn syscalls::Domain>, Box<dyn usr::pci::PCI>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>,
                       Box<dyn syscalls::Mmap>,
                       Box<dyn syscalls::Heap>,
    ) -> Box<dyn usr::pci::PCI>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let mmap = Box::new(Mmap::new());
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let pci = user_ep(pdom, mmap, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), pci)
}


pub fn create_domain_bdev(name: &str,
                          binary_range: (*const u8, *const u8),
                          pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::pci::PCI>) -> Box<dyn usr::bdev::BDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pheap, pci);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)
}

pub fn create_domain_bdev_mem(name: &str,
                              binary_range: (*const u8, *const u8),
                              memdisk: &'static mut [u8]) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, &'static mut [u8]) -> Box<dyn usr::bdev::BDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pheap, memdisk);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)
}

pub fn create_domain_bdev_shadow_helper(name: &str,
                              binary_range: (*const u8, *const u8),
                              create: Arc<dyn create::CreateMemBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Arc<dyn create::CreateMemBDev>) -> Box<dyn usr::bdev::BDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pheap, create);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)
}

pub fn create_domain_net(name: &str,
                         binary_range: (*const u8, *const u8),
                         pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::pci::PCI>) -> Box<dyn usr::net::Net>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let net = user_ep(pdom, pheap, pci);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), net)
}

pub fn build_domain_net_shadow(name: &str,
                         binary_range: (*const u8, *const u8),
                         create: Arc<dyn create::CreateIxgbe>,
                         pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::net::Net>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Arc<dyn create::CreateIxgbe>, Box<dyn usr::pci::PCI>) -> Box<dyn usr::net::Net>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let net = user_ep(pdom, pheap, create, pci);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), net)
}

pub fn build_domain_nvme_shadow(name: &str,
                         binary_range: (*const u8, *const u8),
                         create: Arc<dyn create::CreateNvme>,
                         pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Arc<dyn create::CreateNvme>, Box<dyn usr::pci::PCI>) -> Box<dyn usr::bdev::NvmeBDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let net = user_ep(pdom, pheap, create, pci);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), net)
}

pub fn create_domain_nvmedev(name: &str,
                         binary_range: (*const u8, *const u8),
                         pci: Box<dyn usr::pci::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::NvmeBDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::pci::PCI>) -> Box<dyn usr::bdev::NvmeBDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let nvme = user_ep(pdom, pheap, pci);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), nvme)
}

// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall
//                    + create::CreateXv6 + create::CreateXv6FS /* + CreateXv6User */
//                    + create::CreatePCI + create::CreateAHCI + Send + Sync>)
// See
//   rustc --explain E0225
//
// We have to re-write in an ugly way

pub fn build_domain_init(name: &str,
                         binary_range: (*const u8, *const u8))
                         -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn syscalls::Syscall + Send + Sync>,
                       Box<dyn syscalls::Heap + Send + Sync>,
                       Box<dyn syscalls::Interrupt>,
                       Box<dyn proxy::CreateProxy>,
                       Arc<dyn create::CreateXv6>,
                       Arc<dyn create::CreateXv6FS>,
                       Arc<dyn create::CreateXv6Net>,
                       Arc<dyn create::CreateXv6NetShadow>,
                       Arc<dyn create::CreateXv6Usr>,
                       Arc<dyn create::CreatePCI>,
                       Arc<dyn create::CreateIxgbe>,
                       Arc<dyn create::CreateNvme>,
                       Arc<dyn create::CreateNetShadow>,
                       create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
                       Arc<dyn create::CreateBenchnet>,
                       Arc<dyn create::CreateBenchnvme>,
                       Arc<dyn create::CreateAHCI>,
                       Arc<dyn create::CreateMemBDev>,
                       Arc<dyn create::CreateBDevShadow>,
                       Arc<dyn create::CreateDomA>,
                       Arc<dyn create::CreateDomB>,
                       Arc<dyn create::CreateDomC>,
                       Arc<dyn create::CreateDomD>,
                       Arc<dyn create::CreateHashStore>,
                       Arc<dyn create::CreateTpm>,
                       Arc<dyn create::CreateShadow>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PHeap::new()),
            Box::new(Interrupt::new()),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))));
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_fs(
    name: &str,
    binary_range: (*const u8, *const u8),
    bdev: Box<dyn usr::bdev::BDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::bdev::BDev>) -> Box<dyn usr::vfs::VFS>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let vfs = user_ep(pdom, pheap, bdev);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), vfs)
}

pub fn build_domain_xv6net(
    name: &str,
    binary_range: (*const u8, *const u8),
    net: Box<dyn usr::net::Net>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::net::Net>) -> Box<dyn usr::usrnet::UsrNet>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let usrnet = user_ep(pdom, pheap, net);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), usrnet)
}

pub fn build_domain_xv6net_shadow(
    name: &str,
    binary_range: (*const u8, *const u8),
    create: Arc<dyn create::CreateXv6Net>,
    net: Box<dyn usr::net::Net>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::usrnet::UsrNet>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Arc<dyn create::CreateXv6Net>, Box<dyn usr::net::Net>) -> Box<dyn usr::usrnet::UsrNet>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let usrnet = user_ep(pdom, pheap, create, net);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), usrnet)
}

pub fn build_domain_proxy(
    name: &str,
    binary_range: (*const u8, *const u8),
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_nvme: Arc<dyn create::CreateNvme>,
    create_net_shadow: Arc<dyn create::CreateNetShadow>,
    create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
    create_benchnet: Arc<dyn create::CreateBenchnet>,
    create_benchnvme: Arc<dyn create::CreateBenchnvme>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6net: Arc<dyn create::CreateXv6Net>,
    create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>,
    create_dom_c: Arc<dyn create::CreateDomC>,
    create_dom_d: Arc<dyn create::CreateDomD>,
    create_shadow: Arc<dyn create::CreateShadow>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
    type UserInit = fn(
        Box<dyn syscalls::Syscall>,
        Box<dyn syscalls::Heap>,
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_membdev: Arc<dyn create::CreateMemBDev>,
        create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_nvme: Arc<dyn create::CreateNvme>,
        create_net_shadow: Arc<dyn create::CreateNetShadow>,
        create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
        create_benchnet: Arc<dyn create::CreateBenchnet>,
        create_benchnvme: Arc<dyn create::CreateBenchnvme>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6net: Arc<dyn create::CreateXv6Net>,
        create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>,
        create_dom_a: Arc<dyn create::CreateDomA>,
        create_dom_b: Arc<dyn create::CreateDomB>,
        create_dom_c: Arc<dyn create::CreateDomC>,
        create_dom_d: Arc<dyn create::CreateDomD>,
        create_shadow: Arc<dyn create::CreateShadow>) -> Arc<dyn proxy::Proxy>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let proxy = user_ep(
        pdom,
        pheap,
        create_pci,
        create_ahci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_nvme,
        create_net_shadow,
        create_nvme_shadow,
        create_benchnet,
        create_benchnvme,
        create_xv6fs,
        create_xv6net,
        create_xv6net_shadow,
        create_xv6usr,
        create_xv6,
        create_dom_a,
        create_dom_b,
        create_dom_c,
        create_dom_d,
        create_shadow);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), proxy)
}

pub fn build_domain_xv6kernel(name: &str,
                              binary_range: (*const u8, *const u8),
                              ints: Box<dyn syscalls::Interrupt>,
                              create_xv6fs: Arc<dyn create::CreateXv6FS>,
                              create_xv6net: Arc<dyn create::CreateXv6Net>,
                              create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
                              create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                              bdev: Box<dyn usr::bdev::BDev>,
                              net: Box<dyn usr::net::Net>,
                              nvme: Box<dyn usr::bdev::NvmeBDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::xv6::Xv6>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>,
                       Box<dyn syscalls::Heap>,
                       Box<dyn syscalls::Interrupt>,
                       create_xv6fs: Arc<dyn create::CreateXv6FS>,
                       create_xv6net: Arc<dyn create::CreateXv6Net>,
                       create_xv6net_shadow: Arc<dyn create::CreateXv6NetShadow>,
                       create_xv6kernel: Arc<dyn create::CreateXv6Usr>,
                       bdev: Box<dyn usr::bdev::BDev>,
                       net: Box<dyn usr::net::Net>,
                       nvme: Box<dyn usr::bdev::NvmeBDev>) -> Box<dyn usr::xv6::Xv6>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe{
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let rv6 = user_ep(pdom, pheap, ints, create_xv6fs, create_xv6net, create_xv6net_shadow, create_xv6usr, bdev, net, nvme);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), rv6)
}

pub fn build_domain_xv6usr(name: &str,
                           xv6: Box<dyn usr::xv6::Xv6>,
                           blob: &[u8],
                           args: &str) -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::xv6::Xv6>, &str);

    let begin = blob.as_ptr();
    let end = unsafe { begin.offset(blob.len() as isize) };
    let (dom, entry) = unsafe {
        load_domain(name, (begin, end))
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, xv6, args);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point with return code", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_dom_a(name: &str,
                          binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn usr::dom_a::DomA>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let dom_a = user_ep(pdom, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), dom_a)
}

pub fn build_domain_dom_b(name: &str,
                          binary_range: (*const u8, *const u8),
                          dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::dom_a::DomA>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, dom_a);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_dom_c(name: &str,
                          binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn usr::dom_c::DomC>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let dom_c = user_ep(pdom, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), dom_c)
}

pub fn build_domain_dom_d(name: &str,
                          binary_range: (*const u8, *const u8),
                          dom_c: Box<dyn usr::dom_c::DomC>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::dom_c::DomC>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, dom_c);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}


pub fn build_domain_shadow(name: &str,
                          binary_range: (*const u8, *const u8),
                          create_dom_c: Arc<dyn create::CreateDomC>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_c::DomC>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, create_dom_c: Arc<dyn create::CreateDomC>) -> Box<dyn usr::dom_c::DomC>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let shadow = user_ep(pdom, pheap, create_dom_c);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), shadow)
}

pub fn build_domain_benchnet_helper(name: &str,
                          binary_range: (*const u8, *const u8),
                          net: Box<dyn usr::net::Net>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, net: Box<dyn usr::net::Net>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let shadow = user_ep(pdom, pheap, net);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_benchnvme(name: &str,
                          binary_range: (*const u8, *const u8),
                          nvme: Box<dyn usr::bdev::NvmeBDev>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, nvme: Box<dyn usr::bdev::NvmeBDev>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let shadow = user_ep(pdom, pheap, nvme);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_hashstore(name: &str,
                          binary_range: (*const u8, *const u8)) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let shadow = user_ep(pdom, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_tpm(name: &str,
                        binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn usr::tpm::TpmDev>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn usr::tpm::TpmDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let tpmdev = user_ep(pdom, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), tpmdev)
}
