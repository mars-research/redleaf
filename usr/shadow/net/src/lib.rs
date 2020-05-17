#![no_std]
#![feature(
    box_syntax,
)]
// #![forbid(unsafe_code)]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::collections::VecDeque;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use rref::{RRef, RRefDeque};
use alloc::vec::Vec;
use usr::net::Net;
use usr::pci::PCI;
use usr::rpc::RpcResult;
use create::CreateIxgbe;
use spin::Mutex;

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
    shadow: Mutex<ShadowInternal>,
}

impl Shadow {
    fn new(create: Arc<dyn CreateIxgbe>, pci: Box<dyn PCI>) -> Self {
        Self {
            shadow: Mutex::new(ShadowInternal::new(create, pci)),
        }
    }
}

impl Net for Shadow {
    fn submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> usize {
        self.shadow.lock().net.submit_and_poll(packets, reap_queue, tx)
    }

    fn submit_and_poll_rref(
        &mut self,
        packets: RRefDeque<[u8; 1512], 32>,
        collect: RRefDeque<[u8; 1512], 32>,
        tx: bool,
        pkt_len: usize) -> (
            usize,
            RRefDeque<[u8; 1512], 32>,
            RRefDeque<[u8; 1512], 32>
        )
    {
        self.shadow.lock().net.submit_and_poll_rref(packets, collect, tx, pkt_len)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, create: Arc<dyn CreateIxgbe>, pci: Box<dyn PCI>) -> Box<dyn Net + Send> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init bdev shadow domain");

    box Shadow::new(create, pci)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain bdev shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
