use rref::RRef;
use proxy;
use usr;
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id};
use syscalls::{Heap, Domain, PCI, PciBar, PciResource, Net, Interrupt};
use usr::{bdev::BDev, vfs::VFS, xv6::Xv6};

#[derive(Clone)]
pub struct Proxy {
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>,
}

unsafe impl Send for Proxy {}
unsafe impl Sync for Proxy {}

impl Proxy {
    pub fn new(
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>
    ) -> Proxy {
        Proxy {
            create_pci,
            create_ahci,
            create_ixgbe,
            create_xv6fs,
            create_xv6usr,
            create_xv6,
        }
    }
}

impl proxy::Proxy for Proxy {
    // TODO: figure out how to do this without Arc::new every time
    fn as_create_pci(&self) -> Arc<dyn create::CreatePCI> {
        Arc::new(self.clone())
    }
    fn as_create_ahci(&self) -> Arc<dyn create::CreateAHCI> {
        Arc::new(self.clone())
    }
    fn as_create_ixgbe(&self) -> Arc<dyn create::CreateIxgbe> {
        Arc::new(self.clone())
    }
    fn as_create_xv6fs(&self) -> Arc<dyn create::CreateXv6FS> {
        Arc::new(self.clone())
    }
    fn as_create_xv6usr(&self) -> Arc<dyn create::CreateXv6Usr> {
        Arc::new(self.clone())
    }
    fn as_create_xv6(&self) -> Arc<dyn create::CreateXv6> {
        Arc::new(self.clone())
    }
}

impl create::CreatePCI for Proxy {
    fn create_domain_pci(&self,
                         ) -> (Box<dyn Domain>, Box<dyn PCI>) {
        self.create_pci.create_domain_pci()
    }
}

impl create::CreateAHCI for Proxy {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>) {
        let (domain, ahci) = self.create_ahci.create_domain_ahci(pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, ahci)));
    }
}

impl create::CreateIxgbe for Proxy {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>) {
        // TODO: write IxgbeProxy
        self.create_ixgbe.create_domain_ixgbe(pci)
    }
}

impl create::CreateXv6FS for Proxy {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>) {
        // TODO: write Xv6FSProxy
        self.create_xv6fs.create_domain_xv6fs(bdev)
    }
}

impl create::CreateXv6Usr for Proxy {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn Xv6>) -> Box<dyn Domain> {
        // TODO: write Xv6UsrProxy
        self.create_xv6usr.create_domain_xv6usr(name, xv6)
    }
}

impl create::CreateXv6 for Proxy {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr>,
                               bdev: Box<dyn BDev + Send + Sync>) -> Box<dyn Domain> {
        // TODO: write Xv6KernelProxy
        self.create_xv6.create_domain_xv6kernel(ints, create_xv6fs, create_xv6usr, bdev)
    }
}

struct BDevProxy {
    domain: Box<dyn usr::bdev::BDev>,
    domain_id: u64,
}

unsafe impl Sync for BDevProxy {}
unsafe impl Send for BDevProxy {}

impl BDevProxy {
    fn new(domain_id: u64, domain: Box<dyn usr::bdev::BDev>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

impl usr::bdev::BDev for BDevProxy {
    fn read(&self, block: u32, data: &mut RRef<[u8; 512]>) {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        // data.move_to(self.domain_id);
        let r = self.domain.read(block, data);
        // data.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn write(&self, block: u32, data: &[u8; 512]) {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        // data.move_to(callee_domain);
        let r = self.domain.write(block, data);
        // data.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn read_contig(&self, block: u32, data: &mut RRef<[u8; 512]>) {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        data.move_to(self.domain_id);
        let r = self.domain.read(block, data);
        data.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }
}
