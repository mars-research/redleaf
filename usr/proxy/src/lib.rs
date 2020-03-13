#![no_std]

extern crate malloc;
extern crate alloc;
use rref::RRef;
use create;
use syscalls;
use libsyscalls;
use syscalls::Syscall;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[inline(always)]
fn get_caller_domain() -> u64 { libsyscalls::syscalls::sys_get_current_domain_id() }

#[inline(always)]
fn update_caller_domain_id(new_domain_id: u64) -> u64 {
    unsafe { libsyscalls::syscalls::sys_update_current_domain_id(new_domain_id) }
}

struct Proxy {
//    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
//    create_ixgbe: Arc<dyn create::CreateIxgbe>,
//    create_xv6fs: Arc<dyn create::CreateXv6FS>,
}

unsafe impl Send for Proxy {}
unsafe impl Sync for Proxy {}

impl Proxy {
    fn new(
//        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
//        create_ixgbe: Arc<dyn create::CreateIxgbe>,
//        create_xv6fs: Arc<dyn create::CreateXv6FS>
    ) -> Proxy {
        Proxy {
//            create_pci,
            create_ahci,
//            create_ixgbe,
//            create_xv6fs,
        }
    }
}

impl usr::proxy::Proxy for Proxy {
    fn proxy_clone(&self) -> Box<dyn usr::proxy::Proxy + Send + Sync> {
        Box::new(Proxy::new(self.create_ahci.clone()))
    }

    fn proxy_bdev(&self, bdev: Box<dyn usr::bdev::BDev + Send + Sync>) -> Box<dyn usr::bdev::BDev + Send + Sync> {
        Box::new(BDevProxy::new(get_caller_domain(), bdev))
    }
}

impl create::CreateAHCI for Proxy {
    fn create_domain_ahci(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev>) {
        let (domain, ahci) = self.create_ahci.create_domain_ahci(pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, ahci)));
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
        let caller_domain = update_caller_domain_id(self.domain_id);

        println!("[proxy::bdev_read] caller: {}, callee: {}", caller_domain, self.domain_id);

        data.move_to(self.domain_id);
        let r = self.domain.read(block, data);
        data.move_to(caller_domain);

        // move thread back
        update_caller_domain_id(caller_domain);

        r
    }

    fn write(&self, block: u32, data: &[u8; 512]) {
        // move thread to next domain
        let caller_domain = update_caller_domain_id(self.domain_id);

//        data.move_to(callee_domain);
        let r = self.domain.write(block, data);
//        data.move_to(caller_domain);

        // move thread back
        update_caller_domain_id(caller_domain);

        r
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, create_ahci: Arc<dyn create::CreateAHCI>) -> Box<dyn usr::proxy::Proxy> {
    libsyscalls::syscalls::init(s);

    Box::new(Proxy::new(
        create_ahci
    ))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
