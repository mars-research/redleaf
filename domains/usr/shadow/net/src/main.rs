#![no_std]
#![no_main]
#![feature(box_syntax)]
#![forbid(unsafe_code)]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use console::println;

use core::panic::PanicInfo;

use alloc::vec::Vec;
use create::CreateIxgbe;
use rref::RRefDeque;
use spin::Mutex;
use usr::error::Result;
use usr::net::{Net, NetworkStats};
use usr::pci::PCI;
use usr::rpc::RpcResult;

struct ShadowInternal {
    create: Arc<dyn CreateIxgbe>,
    net: Box<dyn Net>,
    dom: Option<Box<dyn syscalls::Domain>>,
}

impl ShadowInternal {
    fn new(create: Arc<dyn CreateIxgbe>, pci: Box<dyn PCI>) -> Self {
        let (dom, net) = create.create_domain_ixgbe(pci);
        Self {
            create,
            net,
            dom: Some(dom),
        }
    }
}

struct Shadow {
    shadow: Arc<Mutex<ShadowInternal>>,
}

impl Shadow {
    fn new(create: Arc<dyn CreateIxgbe>, pci: Box<dyn PCI>) -> Self {
        Self {
            shadow: Arc::new(Mutex::new(ShadowInternal::new(create, pci))),
        }
    }
}

impl Net for Shadow {
    fn clone_net(&self) -> RpcResult<Box<dyn Net>> {
        self.shadow.lock().net.clone_net()
    }

    fn submit_and_poll(
        &self,
        packets: &mut VecDeque<Vec<u8>>,
        reap_queue: &mut VecDeque<Vec<u8>>,
        tx: bool,
    ) -> RpcResult<Result<usize>> {
        self.shadow
            .lock()
            .net
            .submit_and_poll(packets, reap_queue, tx)
    }

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        //println!("in shadow");
        self.shadow
            .lock()
            .net
            .submit_and_poll_rref(packets, collect, tx, pkt_len)
    }

    fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        self.shadow.lock().net.poll(collect, tx)
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<[u8; 1514], 512>,
        tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        self.shadow.lock().net.poll_rref(collect, tx)
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        self.shadow.lock().net.get_stats()
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        self.shadow.lock().net.test_domain_crossing()
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    create: Arc<dyn CreateIxgbe>,
    pci: Box<dyn PCI>,
) -> Box<dyn Net> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init net shadow domain");

    box Shadow::new(create, pci)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain net shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
