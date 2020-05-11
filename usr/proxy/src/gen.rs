use proxy;
use usr;
use create;
use rref::{RRef, RRefDeque};
use alloc::boxed::Box;
use alloc::sync::Arc;
use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id};
use syscalls::{Heap, Domain, Interrupt};
use usr::{bdev::{BDev, BSIZE}, vfs::VFS, xv6::Xv6, dom_a::DomA, dom_c::DomC, net::Net, pci::{PCI, PciBar, PciResource}};
use usr::rpc::{RpcResult, RpcError};
use console::{println, print};
use unwind::trampoline;

// TODO: remove once ixgbe on rrefdeque
use alloc::{vec::Vec, collections::VecDeque};

#[derive(Clone)]
pub struct Proxy {
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>,
    create_dom_c: Arc<dyn create::CreateDomC>,
    create_dom_d: Arc<dyn create::CreateDomD>,
    create_shadow: Arc<dyn create::CreateShadow>,
}

unsafe impl Send for Proxy {}
unsafe impl Sync for Proxy {}

impl Proxy {
    pub fn new(
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_membdev: Arc<dyn create::CreateMemBDev>,
        create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
        create_xv6: Arc<dyn create::CreateXv6>,
        create_dom_a: Arc<dyn create::CreateDomA>,
        create_dom_b: Arc<dyn create::CreateDomB>,
        create_dom_c: Arc<dyn create::CreateDomC>,
        create_dom_d: Arc<dyn create::CreateDomD>,
        create_shadow: Arc<dyn create::CreateShadow>,
    ) -> Proxy {
        Proxy {
            create_pci,
            create_ahci,
            create_membdev,
            create_bdev_shadow,
            create_ixgbe,
            create_xv6fs,
            create_xv6usr,
            create_xv6,
            create_dom_a,
            create_dom_b,
            create_dom_c,
            create_dom_d,
            create_shadow,
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
    fn as_create_membdev(&self) -> Arc<dyn create::CreateMemBDev> {
        Arc::new(self.clone())
    }
    fn as_create_bdev_shadow(&self) -> Arc<dyn create::CreateBDevShadow> {
        Arc::new(self.clone())
    }
    fn as_create_ixgbe(&self) -> Arc<dyn create::CreateIxgbe> {
        Arc::new(self.clone())
    }
    fn as_create_xv6fs(&self) -> Arc<dyn create::CreateXv6FS> {
        Arc::new(self.clone())
    }
    fn as_create_xv6usr(&self) -> Arc<dyn create::CreateXv6Usr + Send + Sync> {
        Arc::new(self.clone())
    }
    fn as_create_xv6(&self) -> Arc<dyn create::CreateXv6> {
        Arc::new(self.clone())
    }
    fn as_create_dom_a(&self) -> Arc<dyn create::CreateDomA> {
        Arc::new(self.clone())
    }
    fn as_create_dom_b(&self) -> Arc<dyn create::CreateDomB> {
        Arc::new(self.clone())
    }
    fn as_create_dom_c(&self) -> Arc<dyn create::CreateDomC> {
        Arc::new(self.clone())
    }
    fn as_create_dom_d(&self) -> Arc<dyn create::CreateDomD> {
        Arc::new(self.clone())
    }
    fn as_create_shadow(&self) -> Arc<dyn create::CreateShadow> {
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

impl create::CreateMemBDev for Proxy {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>) {
        let (domain, membdev) = self.create_membdev.create_domain_membdev(memdisk);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, membdev)));
    }

    fn recreate_domain_membdev(&self, dom: Box<dyn syscalls::Domain>, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>) {
        let (domain, membdev) = self.create_membdev.recreate_domain_membdev(dom, memdisk);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, membdev)));
    }
}

impl create::CreateBDevShadow for Proxy {
    fn create_domain_bdev_shadow(&self, create: Arc<dyn create::CreateMemBDev>) -> (Box<dyn Domain>, Box<dyn BDev + Send + Sync>) {
        let (domain, shadow) = self.create_bdev_shadow.create_domain_bdev_shadow(create);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, shadow)));
    }
}

impl create::CreateIxgbe for Proxy {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>) {
        let (domain, ixgbe) = self.create_ixgbe.create_domain_ixgbe(pci);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(IxgbeProxy::new(domain_id, ixgbe)))
    }
}

impl create::CreateXv6FS for Proxy {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) -> (Box<dyn Domain>, Box<dyn VFS + Send>) {
        // TODO: write Xv6FSProxy
        self.create_xv6fs.create_domain_xv6fs(bdev)
    }
}

impl create::CreateXv6Usr for Proxy {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn Domain>, &'static str> {
        // TODO: write Xv6UsrProxy
        self.create_xv6usr.create_domain_xv6usr(name, xv6, blob, args)
    }
}

impl create::CreateXv6 for Proxy {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn BDev + Send + Sync>) -> Box<dyn Domain> {
        // TODO: write Xv6KernelProxy
        self.create_xv6.create_domain_xv6kernel(ints, create_xv6fs, create_xv6usr, bdev)
    }
}

impl create::CreateDomA for Proxy {
    fn create_domain_dom_a(&self) ->(Box<dyn Domain>, Box<dyn DomA>) {
        let (domain, dom_a) = self.create_dom_a.create_domain_dom_a();
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(DomAProxy::new(domain_id, dom_a)));
    }
}

impl create::CreateDomB for Proxy {
    fn create_domain_dom_b(&self, dom_a: Box<dyn DomA>) ->(Box<dyn Domain>) {
        self.create_dom_b.create_domain_dom_b(dom_a)
    }
}

impl create::CreateDomC for Proxy {
    fn create_domain_dom_c(&self) -> (Box<dyn Domain>, Box<dyn DomC>) {
        let (domain, dom_c) = self.create_dom_c.create_domain_dom_c();
        let domain_id = domain.get_domain_id();
        (domain, Box::new(DomCProxy::new(domain_id, dom_c)))
    }

    fn recreate_domain_dom_c(&self, dom: Box<dyn Domain>) -> (Box<dyn Domain>, Box<dyn DomC>) {
        let (domain, dom_c) = self.create_dom_c.recreate_domain_dom_c(dom);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(DomCProxy::new(domain_id, dom_c)))
    }

}

impl create::CreateDomD for Proxy {
    fn create_domain_dom_d(&self, dom_c: Box<dyn DomC>) ->(Box<dyn Domain>) {
        self.create_dom_d.create_domain_dom_d(dom_c)
    }
}

impl create::CreateShadow for Proxy {
    fn create_domain_shadow(&self, create_dom_c: Arc<dyn create::CreateDomC>) ->(Box<dyn Domain>, Box<dyn DomC>) {
        let (domain, shadow) = self.create_shadow.create_domain_shadow(create_dom_c);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(DomCProxy::new(domain_id, shadow)))
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

/* 
 * Code to unwind bdev.read
 */

#[no_mangle]
pub extern fn read(s: &Box<usr::bdev::BDev>, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
    //println!("one_arg: x:{}", x);
    s.read(block, data)
}

#[no_mangle]
pub extern fn read_err(s: &Box<usr::bdev::BDev>, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
    println!("bdev.read was aborted, block:{}", block);
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn read_addr() -> u64 {
    read_err as u64
}

extern {
    fn read_tramp(s: &Box<usr::bdev::BDev>, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>>;
}

trampoline!(read);

impl BDev for BDevProxy {
    fn read(&self, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        data.move_to(self.domain_id);
        // let r = self.domain.read(block, data);
        let mut r = unsafe { read_tramp(&self.domain, block, data) };
        if r.is_ok() {
            r.as_mut().unwrap().move_to(caller_domain);
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        // data.move_to(callee_domain);
        let r = self.domain.write(block, data);
        // data.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }
}

struct IxgbeProxy {
    domain: Box<dyn Net>,
    domain_id: u64,
}

unsafe impl Sync for IxgbeProxy {}
unsafe impl Send for IxgbeProxy {}

impl IxgbeProxy {
    fn new(domain_id: u64, domain: Box<dyn Net>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

impl Net for IxgbeProxy {
    fn submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> usize {

        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        // packets.move_to(self.domain_id);
        // reap_queue.move_to(self.domain_id);
        let r = self.domain.submit_and_poll(packets, reap_queue, tx);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }
}

struct DomAProxy {
    domain: Box<dyn usr::dom_a::DomA>,
    domain_id: u64,
}

unsafe impl Sync for DomAProxy {}
unsafe impl Send for DomAProxy {}

impl DomAProxy {
    fn new(domain_id: u64, domain: Box<dyn usr::dom_a::DomA>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

impl usr::dom_a::DomA for DomAProxy {
    fn ping_pong(&self, buffer: RRef<[u8; 1024]>) -> RRef<[u8; 1024]> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        buffer.move_to(self.domain_id);
        let r = self.domain.ping_pong(buffer);
        r.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn tx_submit_and_poll(
        &mut self,
        packets: RRefDeque<[u8; 100], 32>,
        reap_queue: RRefDeque<[u8; 100], 32>)
    -> (
        usize,
        RRefDeque<[u8; 100], 32>,
        RRefDeque<[u8; 100], 32>
    ) {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        packets.move_to(self.domain_id);
        reap_queue.move_to(self.domain_id);
        let r = self.domain.tx_submit_and_poll(packets, reap_queue);
        r.1.move_to(caller_domain);
        r.2.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }
}

struct DomCProxy {
    domain: Box<dyn usr::dom_c::DomC>,
    domain_id: u64,
}

unsafe impl Sync for DomCProxy {}
unsafe impl Send for DomCProxy {}

impl DomCProxy {
    fn new(domain_id: u64, domain: Box<dyn usr::dom_c::DomC>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

/* 
 * Code to unwind one_arg
 */

#[no_mangle]
pub extern fn one_arg(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> Result<usize, i64> {
    //println!("one_arg: x:{}", x);
    let r = s.one_arg(x);

    match r {
        Ok(n) => {/*println!("one_arg:{}", n)*/},
        Err(e) => println!("one_arg: error:{}", e),
    }

    r
}

#[no_mangle]
pub extern fn one_arg_err(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> Result<usize, i64> {
    println!("one_arg was aborted, x:{}", x);
    Err(-1)
}

#[no_mangle]
pub extern "C" fn one_arg_addr() -> u64 {
    one_arg_err as u64
}

extern {
    fn one_arg_tramp(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> Result<usize, i64>;
}

trampoline!(one_arg);

impl usr::dom_c::DomC for DomCProxy {
    fn no_arg(&self) {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        let r = self.domain.no_arg();

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn one_arg(&self, x: usize) -> Result<usize, i64> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        #[cfg(not(feature = "unwind_dom_c"))]
        let r = self.domain.one_arg(x);

        #[cfg(feature = "unwind_dom_c")]
        let r = unsafe { one_arg_tramp(&self.domain, x) };

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn one_rref(&self, x: RRef<usize>) -> RRef<usize> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        x.move_to(self.domain_id);
        let r = self.domain.one_rref(x);
        r.move_to(caller_domain);

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }
}
