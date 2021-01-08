use proxy;
use usr;
use create;
use rref::{RRef, RRefDeque, RRefVec, traits::CustomCleanup};
use alloc::boxed::Box;
use alloc::sync::Arc;
use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id, sys_discard_cont};
use syscalls::{Heap, Domain, Interrupt};
use usr::{bdev::{BDev, BSIZE, NvmeBDev, BlkReq}, vfs::{UsrVFS, VFS}, rv6::Rv6, dom_a::DomA, dom_c::DomC, net::{Net, NetworkStats}, usrnet::UsrNet, pci::{PCI, PciBar, PciResource}, tpm::UsrTpm};
use usr::rpc::{RpcResult, RpcError};
use usr::error::Result;
use core::mem::transmute;
use console::{println, print};
use unwind::trampoline;

use codegen::bdev::BDevProxy;

// TODO: remove once ixgbe on rrefdeque
use alloc::{vec::Vec, collections::VecDeque};

#[derive(Clone)]
pub struct Proxy {
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
    create_xv6fs: Arc<dyn create::CreateRv6FS>,
    create_xv6net: Arc<dyn create::CreateRv6Net>,
create_xv6net_shadow: Arc<dyn create::CreateRv6NetShadow>,
    create_xv6usr: Arc<dyn create::CreateRv6Usr + Send + Sync>,
    create_xv6: Arc<dyn create::CreateRv6>,
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
        create_nvme: Arc<dyn create::CreateNvme>,
        create_net_shadow: Arc<dyn create::CreateNetShadow>,
        create_nvme_shadow: Arc<dyn create::CreateNvmeShadow>,
        create_benchnet: Arc<dyn create::CreateBenchnet>,
        create_benchnvme: Arc<dyn create::CreateBenchnvme>,
        create_xv6fs: Arc<dyn create::CreateRv6FS>,
        create_xv6net: Arc<dyn create::CreateRv6Net>,
        create_xv6net_shadow: Arc<dyn create::CreateRv6NetShadow>,
        create_xv6usr: Arc<dyn create::CreateRv6Usr + Send + Sync>,
        create_xv6: Arc<dyn create::CreateRv6>,
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
    fn as_create_net_shadow(&self) -> Arc<dyn create::CreateNetShadow> {
        Arc::new(self.clone())
    }
    fn as_create_nvme_shadow(&self) -> Arc<dyn create::CreateNvmeShadow> {
        Arc::new(self.clone())
    }
    fn as_create_benchnet(&self) -> Arc<dyn create::CreateBenchnet> {
        Arc::new(self.clone())
    }
    fn as_create_benchnvme(&self) -> Arc<dyn create::CreateBenchnvme> {
        Arc::new(self.clone())
    }
    fn as_create_nvme(&self) -> Arc<dyn create::CreateNvme> {
        Arc::new(self.clone())
    }
    fn as_create_xv6fs(&self) -> Arc<dyn create::CreateRv6FS> {
        Arc::new(self.clone())
    }
    fn as_create_xv6net(&self) -> Arc<dyn create::CreateRv6Net> {
        Arc::new(self.clone())
    }
    fn as_create_xv6net_shadow(&self) -> Arc<dyn create::CreateRv6NetShadow> {
        Arc::new(self.clone())
    }
    fn as_create_xv6usr(&self) -> Arc<dyn create::CreateRv6Usr + Send + Sync> {
        Arc::new(self.clone())
    }
    fn as_create_xv6(&self) -> Arc<dyn create::CreateRv6> {
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
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>) {
        let (domain, ahci) = self.create_ahci.create_domain_ahci(pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, ahci)));
    }
}

impl create::CreateMemBDev for Proxy {
    fn create_domain_membdev(&self, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev>) {
        let (domain, membdev) = self.create_membdev.create_domain_membdev(memdisk);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, membdev)));
    }

    fn recreate_domain_membdev(&self, dom: Box<dyn syscalls::Domain>, memdisk: &'static mut [u8]) -> (Box<dyn Domain>, Box<dyn BDev>) {
        let (domain, membdev) = self.create_membdev.recreate_domain_membdev(dom, memdisk);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(BDevProxy::new(domain_id, membdev)));
    }
}

impl create::CreateBDevShadow for Proxy {
    fn create_domain_bdev_shadow(&self, create: Arc<dyn create::CreateMemBDev>) -> (Box<dyn Domain>, Box<dyn BDev>) {
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

impl create::CreateNetShadow for Proxy {
    fn create_domain_net_shadow(&self, create: Arc<dyn create::CreateIxgbe>, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>) {
        let (domain, shadow) = self.create_net_shadow.create_domain_net_shadow(create, pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(IxgbeProxy::new(domain_id, shadow)));
    }
}

impl create::CreateNvmeShadow for Proxy {
    fn create_domain_nvme_shadow(&self, create: Arc<dyn create::CreateNvme>, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn NvmeBDev>) {
        let (domain, shadow) = self.create_nvme_shadow.create_domain_nvme_shadow(create, pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(NvmeBDevProxy::new(domain_id, shadow)));
    }
}

impl create::CreateNvme for Proxy {
    fn create_domain_nvme(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn usr::bdev::NvmeBDev>) {
        // TODO: write NvmeBDevProxy
        let (domain, nvme) = self.create_nvme.create_domain_nvme(pci);
        let domain_id = domain.get_domain_id();
        return (domain, Box::new(NvmeBDevProxy::new(domain_id, nvme)));
    }
}

impl create::CreateRv6FS for Proxy {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) -> (Box<dyn Domain>, Box<dyn VFS>) {
        // TODO: write Rv6FSProxy
        self.create_xv6fs.create_domain_xv6fs(bdev)
    }
}

impl create::CreateRv6Net for Proxy {
    fn create_domain_xv6net(&self, net: Box<dyn Net>) -> (Box<dyn Domain>, Box<dyn UsrNet>) {
        let (domain, xv6net) = self.create_xv6net.create_domain_xv6net(net);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(UsrNetProxy::new(domain_id, xv6net)))
    }
}

impl create::CreateRv6NetShadow for Proxy {
    fn create_domain_xv6net_shadow(&self, create: Arc<dyn create::CreateRv6Net>, net: Box<dyn Net>) -> (Box<dyn Domain>, Box<dyn UsrNet>) {
        let (domain, xv6net) = self.create_xv6net_shadow.create_domain_xv6net_shadow(create, net);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(UsrNetProxy::new(domain_id, xv6net)))
    }
}

impl create::CreateRv6Usr for Proxy {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn usr::rv6::Rv6>, blob: &[u8], args: &str) -> Result<Box<dyn Domain>> {
        // TODO: write Rv6UsrProxy
        self.create_xv6usr.create_domain_xv6usr(name, xv6, blob, args)
    }
}

impl create::CreateRv6 for Proxy {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateRv6FS>,
                               create_xv6net: Arc<dyn create::CreateRv6Net>,
                               create_xv6net_shadow: Arc<dyn create::CreateRv6NetShadow>,
                               create_xv6usr: Arc<dyn create::CreateRv6Usr + Send + Sync>,
                               bdev: Box<dyn BDev>,
                               net: Box<dyn usr::net::Net>,
                               nvme: Box<dyn usr::bdev::NvmeBDev>,
                               usr_tpm: Box<dyn usr::tpm::UsrTpm>,
                            ) -> (Box<dyn Domain>, Box<dyn Rv6>) {
        let (domain, rv6) = self.create_xv6.create_domain_xv6kernel(ints, create_xv6fs, create_xv6net, create_xv6net_shadow, create_xv6usr, bdev, net, nvme, usr_tpm);
        let domain_id = domain.get_domain_id();
        (domain, Box::new(Rv6Proxy::new(domain_id, rv6)))
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

impl create::CreateBenchnet for Proxy {
    fn create_domain_benchnet(&self, net: Box<dyn Net>) ->(Box<dyn Domain>) {
        self.create_benchnet.create_domain_benchnet(net)
    }
}

impl create::CreateBenchnvme for Proxy {
    fn create_domain_benchnvme(&self, nvme: Box<dyn usr::bdev::NvmeBDev>) ->(Box<dyn Domain>) {
        self.create_benchnvme.create_domain_benchnvme(nvme)
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

//Code to unwind net_submit_and_poll
#[no_mangle]
pub extern fn net_submit_and_poll(s: &Box<usr::net::Net>, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
    //println!("one_arg: x:{}", x);
    s.submit_and_poll(packets, reap_queue, tx)
}

#[no_mangle]
pub extern fn net_submit_and_poll_err(s: &Box<usr::net::Net>, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
    println!("net_submit_and_poll was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn net_submit_and_poll_addr() -> u64 {
    net_submit_and_poll_err as u64
}

extern {
    fn net_submit_and_poll_tramp(s: &Box<usr::net::Net>, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>>;
}

trampoline!(net_submit_and_poll);

//Code to unwind net_poll
#[no_mangle]
pub extern fn net_poll(s: &Box<usr::net::Net>, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
    s.poll(collect, tx)
}

#[no_mangle]
pub extern fn net_poll_err(s: &Box<usr::net::Net>, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
    println!("net_poll was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn net_poll_addr() -> u64 {
    net_poll_err as u64
}

extern {
    fn net_poll_tramp(s: &Box<usr::net::Net>, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>>;
}

trampoline!(net_poll);

//Code to unwind net_submit_and_poll_rref
#[no_mangle]
pub extern fn net_submit_and_poll_rref(s: &Box<usr::net::Net>,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>> {
    s.submit_and_poll_rref(packets, collect, tx, pkt_len)
}

#[no_mangle]
pub extern fn net_submit_and_poll_rref_err(s: &Box<usr::net::Net>,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>> {
    println!("net_submit_and_poll_rref was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn net_submit_and_poll_rref_addr() -> u64 {
    net_submit_and_poll_rref_err as u64
}

extern {
    fn net_submit_and_poll_rref_tramp(s: &Box<usr::net::Net>,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>>;
}

trampoline!(net_submit_and_poll_rref);

//Code to unwind poll_rref
#[no_mangle]
pub extern fn net_poll_rref(s: &Box<usr::net::Net>, collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
    s.poll_rref(collect, tx)
}

#[no_mangle]
pub extern fn net_poll_rref_err(s: &Box<usr::net::Net>, collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
    println!("net_poll_rref was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn net_poll_rref_addr() -> u64 {
    net_poll_rref_err as u64
}

extern {
    fn net_poll_rref_tramp(s: &Box<usr::net::Net>, collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>>;
}

trampoline!(net_poll_rref);


//Code to unwind get_stats
#[no_mangle]
pub extern fn get_stats(s: &Box<usr::net::Net>) -> RpcResult<Result<NetworkStats>> {
    //println!("one_arg: x:{}", x);
    s.get_stats()
}

#[no_mangle]
pub extern fn get_stats_err(s: &Box<usr::net::Net>) -> RpcResult<Result<NetworkStats>> {
    println!("get_stats was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn get_stats_addr() -> u64 {
    get_stats_err as u64
}

extern {
    fn get_stats_tramp(s: &Box<usr::net::Net>) -> RpcResult<Result<NetworkStats>>;
}

trampoline!(get_stats);

impl Net for IxgbeProxy {
    fn clone_net(&self) -> RpcResult<Box<dyn Net>> {
        Ok(box Self::new(self.domain_id, self.domain.clone_net()?))
    }

    fn submit_and_poll(&self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {

        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        #[cfg(not(feature = "tramp"))]
        let r = self.domain.submit_and_poll(packets, reap_queue, tx);
        #[cfg(feature = "tramp")]
        let r = unsafe { net_submit_and_poll_tramp(&self.domain, packets, reap_queue, tx) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        #[cfg(not(feature = "tramp"))]
        let r = self.domain.poll(collect, tx);
        #[cfg(feature = "tramp")]
        let r = unsafe { net_poll_tramp(&self.domain, collect, tx) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>>
    {
        //println!("ixgbe proxy");
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        packets.move_to(self.domain_id);
        collect.move_to(self.domain_id);

        #[cfg(not(feature = "tramp"))]
        let r = self.domain.submit_and_poll_rref(packets, collect, tx, pkt_len);
        #[cfg(feature = "tramp")]
        let r = unsafe{ net_submit_and_poll_rref_tramp(&self.domain, packets, collect, tx, pkt_len) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        if let Ok(Ok(r)) = r.as_ref() {
            r.1.move_to(caller_domain);
            r.2.move_to(caller_domain);
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn poll_rref(&self, collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        collect.move_to(self.domain_id);

        #[cfg(not(feature = "tramp"))]
        let r = self.domain.poll_rref(collect, tx);
        #[cfg(feature = "tramp")]
        let r = unsafe { net_poll_rref_tramp(&self.domain, collect, tx) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        if let Ok(Ok(r)) = r.as_ref() {
            r.1.move_to(caller_domain);
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
         // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        let r = unsafe{ get_stats_tramp(&self.domain) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        self.domain.test_domain_crossing()
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

use codegen::dom_c::DomCProxy;

use codegen::rv6::Rv6Proxy;

use codegen::bdev::NvmeBDevProxy;




/* 
 * Code to unwind usrnet_read_socket
 */

#[no_mangle]
pub extern fn usrnet_read_socket(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
    //println!("usrnet_read_socket: x:{}", x);
    s.read_socket(socket, buffer)
}

#[no_mangle]
pub extern fn usrnet_read_socket_err(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
    println!("usrnet_read_socket was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn usrnet_read_socket_addr() -> u64 {
    usrnet_read_socket_err as u64
}

extern {
    fn usrnet_read_socket_tramp(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
}

trampoline!(usrnet_read_socket);

/* 
 * Code to unwind usrnet_write_socket
 */

#[no_mangle]
pub extern fn usrnet_write_socket(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>, size: usize) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
    //println!("usrnet_write_socket: x:{}", x);
    s.write_socket(socket, buffer, size)
}

#[no_mangle]
pub extern fn usrnet_write_socket_err(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>, size: usize) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
    println!("usrnet_write_socket was aborted");
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
pub extern "C" fn usrnet_write_socket_addr() -> u64 {
    usrnet_write_socket_err as u64
}

extern {
    fn usrnet_write_socket_tramp(s: &Box<dyn UsrNet>, socket: usize, buffer: RRefVec<u8>, size: usize) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
}

trampoline!(usrnet_write_socket);

// Rv6 proxy
struct UsrNetProxy {
    domain: Box<dyn UsrNet>,
    domain_id: u64,
}

unsafe impl Sync for UsrNetProxy {}
unsafe impl Send for UsrNetProxy {}

impl UsrNetProxy {
    fn new(domain_id: u64, domain: Box<dyn UsrNet>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

impl UsrNet for UsrNetProxy {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        Ok(box Self::new(self.domain_id, self.domain.clone_usrnet()?))
    }

    fn create(&self) -> RpcResult<Result<usize>> {
        self.domain.create()
    }

    fn listen(&self, socket: usize, port: u16) -> RpcResult<Result<()>> {
        self.domain.listen(socket, port)
    }

    fn read_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        buffer.move_to(self.domain_id);
        #[cfg(not(feature = "tramp"))]
        let r = self.domain.read_socket(socket, buffer);
        #[cfg(feature = "tramp")]
        let r = unsafe { usrnet_read_socket_tramp(&self.domain, socket, buffer) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        if let Ok(Ok(r)) = r.as_ref() {
            r.1.move_to(caller_domain);
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn write_socket(&self, socket: usize, buffer: RRefVec<u8>, size: usize) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        // move thread to next domain
        let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };

        buffer.move_to(self.domain_id);
        #[cfg(not(feature = "tramp"))]
        let r = self.domain.write_socket(socket, buffer, size);
        #[cfg(feature = "tramp")]
        let r = unsafe { usrnet_write_socket_tramp(&self.domain, socket, buffer, size) };

        #[cfg(feature = "tramp")]
        unsafe {
            sys_discard_cont();
        }

        if let Ok(Ok(r)) = r.as_ref() {
            r.1.move_to(caller_domain);
        }

        // move thread back
        unsafe { sys_update_current_domain_id(caller_domain) };

        r
    }

    fn poll(&self, tx: bool) -> RpcResult<Result<()>> {
        UsrNet::poll(&*self.domain, tx)
    }

    fn can_recv(&self, server: usize) -> RpcResult<Result<bool>> {
        self.domain.can_recv(server)
    }

    fn is_listening(&self, server: usize) -> RpcResult<Result<bool>> {
        self.domain.is_listening(server)
    }

    fn is_active(&self, socket: usize) -> RpcResult<Result<bool>> {
        self.domain.is_active(socket)
    }

    fn close(&self, server: usize) -> RpcResult<Result<()>> {
        self.domain.close(server)
    }
}

