use crate::bdev;
use crate::domain_create::{
    CreateAHCI, CreateBDevShadow, CreateBenchnet, CreateBenchnvme, CreateDomC, CreateDomD,
    CreateIxgbe, CreateMemBDev, CreateNetShadow, CreateNvme, CreateNvmeShadow, CreatePCI,
    CreateRv6, CreateRv6FS, CreateRv6Net, CreateRv6NetShadow, CreateRv6Usr, CreateShadow,
    CreateVirtioBackend, CreateVirtioBlock, CreateVirtioNet, CreateVirtioNetMMIO,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use syscalls::Domain;

pub trait Proxy:
    CreatePCI
    + CreateMemBDev
    + CreateIxgbe
    + CreateNetShadow
    + CreateNvmeShadow
    + CreateBenchnvme
    + CreateRv6FS
    + CreateRv6Usr
    + CreateRv6
    + CreateDomC
    + CreateDomD
    + CreateShadow
    + CreateVirtioNet
    + CreateVirtioBlock
    + CreateVirtioBackend
    + CreateVirtioNetMMIO
{
    // necessary because rust doesn't support trait object upcasting
    fn as_domain_create_CreateVirtioNet(&self) -> Arc<dyn crate::domain_create::CreateVirtioNet>;
    fn as_domain_create_CreateVirtioBlock(
        &self,
    ) -> Arc<dyn crate::domain_create::CreateVirtioBlock>;
    fn as_domain_create_CreateVirtioBackend(
        &self,
    ) -> Arc<dyn crate::domain_create::CreateVirtioBackend>;
    fn as_domain_create_CreateVirtioNetMMIO(
        &self,
    ) -> Arc<dyn crate::domain_create::CreateVirtioNetMMIO>;
    fn as_domain_create_CreateIxgbe(&self) -> Arc<dyn crate::domain_create::CreateIxgbe>;
    fn as_domain_create_CreateDomD(&self) -> Arc<dyn crate::domain_create::CreateDomD>;
    fn as_domain_create_CreateMemBDev(&self) -> Arc<dyn crate::domain_create::CreateMemBDev>;
    fn as_domain_create_CreateRv6(&self) -> Arc<dyn crate::domain_create::CreateRv6>;
    fn as_domain_create_CreatePCI(&self) -> Arc<dyn crate::domain_create::CreatePCI>;
    fn as_domain_create_CreateRv6Net(&self) -> Arc<dyn crate::domain_create::CreateRv6Net>;
    fn as_domain_create_CreateDomC(&self) -> Arc<dyn crate::domain_create::CreateDomC>;
    fn as_domain_create_CreateTpm(&self) -> Arc<dyn crate::domain_create::CreateTpm>;
    fn as_domain_create_CreateBDevShadow(&self) -> Arc<dyn crate::domain_create::CreateBDevShadow>;
    fn as_domain_create_CreateBenchnvme(&self) -> Arc<dyn crate::domain_create::CreateBenchnvme>;
    fn as_domain_create_CreateRv6NetShadow(
        &self,
    ) -> Arc<dyn crate::domain_create::CreateRv6NetShadow>;
    fn as_domain_create_CreateNvmeShadow(&self) -> Arc<dyn crate::domain_create::CreateNvmeShadow>;
    fn as_domain_create_CreateShadow(&self) -> Arc<dyn crate::domain_create::CreateShadow>;
    fn as_domain_create_CreateNetShadow(&self) -> Arc<dyn crate::domain_create::CreateNetShadow>;
    fn as_domain_create_CreateRv6Usr(&self) -> Arc<dyn crate::domain_create::CreateRv6Usr>;
    fn as_domain_create_CreateNvme(&self) -> Arc<dyn crate::domain_create::CreateNvme>;
    fn as_domain_create_CreateRv6FS(&self) -> Arc<dyn crate::domain_create::CreateRv6FS>;
}
