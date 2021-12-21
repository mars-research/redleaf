#![no_std]
#![no_main]
#![feature(box_syntax)]
#![forbid(unsafe_code)]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;

use core::panic::PanicInfo;

use interface::rref::RRefVec;

use interface::domain_create::CreateRv6Net;
use interface::error::Result;
use interface::net::Net;
use interface::rpc::RpcResult;
use interface::usrnet::UsrNet;
use spin::Mutex;

struct ShadowInternal {
    create: Arc<dyn CreateRv6Net>,
    usrnet: Box<dyn UsrNet>,
    dom: Option<Box<dyn syscalls::Domain>>,
}

impl ShadowInternal {
    fn new(create: Arc<dyn CreateRv6Net>, net: Box<dyn Net>) -> Self {
        let (dom, usrnet) = create.create_domain_xv6net(net);
        Self {
            create,
            usrnet,
            dom: Some(dom),
        }
    }
}

struct Shadow {
    shadow: Arc<Mutex<ShadowInternal>>,
}

impl Shadow {
    fn new(create: Arc<dyn CreateRv6Net>, net: Box<dyn Net>) -> Self {
        Self {
            shadow: Arc::new(Mutex::new(ShadowInternal::new(create, net))),
        }
    }
}

impl UsrNet for Shadow {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        self.shadow.lock().usrnet.clone_usrnet()
    }
    fn create(&self) -> RpcResult<Result<usize>> {
        self.shadow.lock().usrnet.create()
    }
    fn listen(&self, socket: usize, port: u16) -> RpcResult<Result<()>> {
        self.shadow.lock().usrnet.listen(socket, port)
    }
    fn poll(&self, tx: bool) -> RpcResult<Result<()>> {
        self.shadow.lock().usrnet.poll(tx)
    }
    fn can_recv(&self, server: usize) -> RpcResult<Result<bool>> {
        self.shadow.lock().usrnet.can_recv(server)
    }
    fn is_listening(&self, server: usize) -> RpcResult<Result<bool>> {
        self.shadow.lock().usrnet.is_listening(server)
    }
    fn is_active(&self, socket: usize) -> RpcResult<Result<bool>> {
        self.shadow.lock().usrnet.is_active(socket)
    }
    fn close(&self, server: usize) -> RpcResult<Result<()>> {
        self.shadow.lock().usrnet.close(server)
    }
    fn read_socket(
        &self,
        socket: usize,
        buffer: RRefVec<u8>,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.shadow.lock().usrnet.read_socket(socket, buffer)
    }
    fn write_socket(
        &self,
        socket: usize,
        buffer: RRefVec<u8>,
        size: usize,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.shadow.lock().usrnet.write_socket(socket, buffer, size)
    }
}

pub fn main(create: Arc<dyn CreateRv6Net>, net: Box<dyn Net>) -> Box<dyn UsrNet> {
    println!("Init usrnet shadow domain");

    box Shadow::new(create, net)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain usrnet shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
