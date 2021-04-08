use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::{Domain};
use crate::{bdev};
use crate::domain_creation::{CreatePCI,
             CreateAHCI,
             CreateMemBDev, 
             CreateBDevShadow,
             CreateIxgbe, 
             CreateNvme,
             CreateNetShadow,
             CreateNvmeShadow,
             CreateBenchnet,
             CreateBenchnvme,
             CreateRv6FS,
             CreateRv6Net,
             CreateRv6NetShadow, 
             CreateRv6Usr, 
             CreateRv6, 
             CreateDomA, 
             CreateDomB, 
             CreateDomC, 
             CreateDomD, 
             CreateShadow,
             CreateKeyboard,
};

pub trait CreateProxy {
    fn create_domain_proxy(
        &self,
        create_pci: Arc<dyn CreatePCI>,
        create_ahci: Arc<dyn CreateAHCI>,
        create_membdev: Arc<dyn CreateMemBDev>,
        create_bdev_shadow: Arc<dyn CreateBDevShadow>,
        create_ixgbe: Arc<dyn CreateIxgbe>,
        create_nvme: Arc<dyn CreateNvme>,
        create_net_shadow: Arc<dyn crate::domain_creation::CreateNetShadow>,
        create_nvme_shadow: Arc<dyn crate::domain_creation::CreateNvmeShadow>,
        create_benchnet: Arc<dyn CreateBenchnet>,
        create_benchnvme: Arc<dyn crate::domain_creation::CreateBenchnvme>,
        create_xv6fs: Arc<dyn CreateRv6FS>,
        create_xv6net: Arc<dyn crate::domain_creation::CreateRv6Net>,
        create_xv6net_shadow: Arc<dyn crate::domain_creation::CreateRv6NetShadow>,
        create_xv6usr: Arc<dyn CreateRv6Usr>,
        create_xv6: Arc<dyn CreateRv6>,
        create_dom_a: Arc<dyn CreateDomA>,
        create_dom_b: Arc<dyn CreateDomB>,
        create_dom_c: Arc<dyn CreateDomC>,
        create_dom_d: Arc<dyn CreateDomD>,
        create_shadow: Arc<dyn CreateShadow>,
        create_keyboard: Arc<dyn CreateKeyboard>,
        ) -> (Box<dyn Domain>, Arc<dyn Proxy>);
}

pub trait Proxy: CreatePCI +
                 CreateAHCI +
                 CreateMemBDev +
                 CreateBDevShadow +
                 CreateIxgbe +
                 CreateNetShadow +
                 CreateNvmeShadow +
                 CreateBenchnet +
                 CreateBenchnvme +
                 CreateRv6FS + 
                 CreateRv6Usr + 
                 CreateRv6 + 
                 CreateDomA + 
                 CreateDomB + 
                 CreateDomC + 
                 CreateDomD + 
                 CreateShadow +
                 CreateKeyboard {
    // necessary because rust doesn't support trait object upcasting
    fn as_create_pci(&self) -> Arc<dyn CreatePCI>;
    fn as_create_ahci(&self) -> Arc<dyn CreateAHCI>;
    fn as_create_membdev(&self) -> Arc<dyn CreateMemBDev>;
    fn as_create_bdev_shadow(&self) -> Arc<dyn CreateBDevShadow>;
    fn as_create_ixgbe(&self) -> Arc<dyn CreateIxgbe>;
    fn as_create_nvme(&self) -> Arc<dyn CreateNvme>;
    fn as_create_net_shadow(&self) -> Arc<dyn CreateNetShadow>;
    fn as_create_nvme_shadow(&self) -> Arc<dyn CreateNvmeShadow>;
    fn as_create_benchnet(&self) -> Arc<dyn CreateBenchnet>;
    fn as_create_benchnvme(&self) -> Arc<dyn CreateBenchnvme>;
    fn as_create_xv6fs(&self) -> Arc<dyn CreateRv6FS>;
    fn as_create_xv6net(&self) -> Arc<dyn CreateRv6Net>;
    fn as_create_xv6net_shadow(&self) -> Arc<dyn CreateRv6NetShadow>;
    fn as_create_xv6usr(&self) -> Arc<dyn CreateRv6Usr + Send + Sync>;
    fn as_create_xv6(&self) -> Arc<dyn CreateRv6>;
    fn as_create_dom_a(&self) -> Arc<dyn CreateDomA>;
    fn as_create_dom_b(&self) -> Arc<dyn CreateDomB>;
    fn as_create_dom_c(&self) -> Arc<dyn CreateDomC>;
    fn as_create_dom_d(&self) -> Arc<dyn CreateDomD>;
    fn as_create_shadow(&self) -> Arc<dyn CreateShadow>;
    fn as_create_keyboard(&self) -> Arc<dyn CreateKeyboard>;
}
