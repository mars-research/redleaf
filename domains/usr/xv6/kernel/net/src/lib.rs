#![no_std]
#![no_main]
// #![forbid(unsafe_code)]
#![feature(box_syntax, untagged_unions)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate byteorder;

use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;

use interface::error::{ErrorKind, Result};
use interface::net::Net;
use interface::rpc::RpcResult;
use interface::rref::RRefVec;
use interface::usrnet::UsrNet;
use spin::Mutex;
use syscalls::{Heap, Syscall};

use smolnet::SmolPhy;
use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache};
use smoltcp::socket::{Socket, SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};

use arrayvec::ArrayVec;

// c'mon...
struct Rv6Net {
    state: Arc<Mutex<Rv6NetInner>>,
}

impl Rv6Net {
    fn new(net: Box<dyn Net>) -> Self {
        Self {
            state: Arc::new(Mutex::new(Rv6NetInner::new(net))),
        }
    }
}

struct Rv6NetInner {
    // 'b neighbor_cache
    // 'c ip_addrs
    // 'e routes
    iface: EthernetInterface<'static, 'static, 'static, SmolPhy>,

    ip_addresses: [IpCidr; 1],
    num_ip_addresses: usize,

    mac_address: [u8; 6],
    // neighbor_cache_entries: [Option<(IpAddress, Neighbor)>; 8],
    // neighbor_cache_entries: Vec<Option<(IpAddress, Neighbor)>>,
    socketset: SocketSet<'static, 'static, 'static>,
    handles: ArrayVec<[SocketHandle; 1024]>,
}

impl Rv6NetInner {
    fn new(net: Box<dyn Net>) -> Self {
        // FIXME: Provide ways to setup IP and MAC
        let smol = SmolPhy::new(net);

        // let mut neighbor_cache_entries = [None; 8];
        let neighbor_cache = NeighborCache::new(BTreeMap::new());

        let ip_addresses = [IpCidr::new(IpAddress::v4(10, 10, 1, 1), 24)];
        let mac_address = [0x90, 0xe2, 0xba, 0xb3, 0xb9, 0x10];
        let iface = EthernetInterfaceBuilder::new(smol)
            .ethernet_addr(EthernetAddress::from_bytes(&mac_address))
            .neighbor_cache(neighbor_cache)
            .ip_addrs(ip_addresses)
            .finalize();

        let socketset = SocketSet::new(Vec::with_capacity(512));

        Self {
            iface,

            ip_addresses,
            num_ip_addresses: 1,

            mac_address,
            // neighbor_cache_entries,
            socketset,
            handles: ArrayVec::new(),
        }
    }

    fn create_socket(&mut self) -> Option<usize> {
        // Create new socket
        let socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 2048]),
            TcpSocketBuffer::new(vec![0; 2048]),
        );
        let socket: Socket = socket.into();
        let handle = self.socketset.add(socket);
        self.handles.push(handle);

        Some(self.handles.len() - 1)
    }

    fn poll(&mut self, tx: bool) {
        let current = libtime::get_ns_time() / 1000000;

        if tx {
            self.iface.device_mut().do_tx();
        } else {
            self.iface.device_mut().do_rx();
            self.iface
                .poll(&mut self.socketset, Instant::from_millis(current as i64));
        }
    }
}

impl UsrNet for Rv6Net {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        /*
        let steal = &self.state as *const Arc<Mutex<Rv6NetInner>>;
        let steal = unsafe { &'static *steal };
        */
        let cloned = self.state.clone();
        Ok(box Self {
            state: cloned,
            // state: self.state.clone(),
        })
    }

    fn create(&self) -> RpcResult<Result<usize>> {
        let mut state = self.state.lock();

        match state.create_socket() {
            Some(socket) => Ok(Ok(socket)),
            _ => Ok(Err(ErrorKind::AddrNotAvailable)),
        }
    }

    fn listen(&self, socket: usize, port: u16) -> RpcResult<Result<()>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        if socket.listen(port).is_ok() {
            Ok(Ok(()))
        } else {
            Ok(Err(ErrorKind::InvalidFileDescriptor))
        }
    }

    fn poll(&self, tx: bool) -> RpcResult<Result<()>> {
        let mut state = self.state.lock();
        state.poll(tx);
        Ok(Ok(()))
    }

    fn can_recv(&self, socket: usize) -> RpcResult<Result<bool>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let socket = state.socketset.get::<TcpSocket>(handle);

        Ok(Ok(socket.can_recv()))
    }

    fn is_listening(&self, socket: usize) -> RpcResult<Result<bool>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let socket = state.socketset.get::<TcpSocket>(handle);

        Ok(Ok(socket.is_listening()))
    }

    fn is_active(&self, socket: usize) -> RpcResult<Result<bool>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let socket = state.socketset.get::<TcpSocket>(handle);

        Ok(Ok(socket.is_active()))
    }

    fn close(&self, socket: usize) -> RpcResult<Result<()>> {
        // FIXME: very bad - there should be a way to take app's
        // access to the socket away
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        socket.close();
        // we should consume the socket in this function

        Ok(Ok(()))
    }

    fn read_socket(
        &self,
        socket: usize,
        mut buffer: RRefVec<u8>,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        let dstbuf = buffer.as_mut_slice();

        let r = socket.recv(|buf| {
            let size = if buf.len() > dstbuf.len() {
                dstbuf.len()
            } else {
                buf.len()
            };

            dstbuf[..size].copy_from_slice(&buf[..size]);

            (size, size)
        });

        match r {
            Ok(size) => Ok(Ok((size, buffer))),
            Err(_) => Ok(Err(ErrorKind::Other)),
        }
    }

    fn write_socket(
        &self,
        socket: usize,
        mut buffer: RRefVec<u8>,
        size: usize,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        let mut state = self.state.lock();

        let handle = state.handles[socket];

        let mut socket = state.socketset.get::<TcpSocket>(handle);

        let buf = buffer.as_mut_slice();

        // buf.len() is not the actual size...
        let r = socket.send(|dstbuf| {
            let to_send = if size > dstbuf.len() {
                dstbuf.len()
            } else {
                size
            };

            dstbuf[..to_send].copy_from_slice(&buf[..to_send]);

            (to_send, to_send)
        });

        match r {
            Ok(sent) => Ok(Ok((sent, buffer))),
            Err(_) => Ok(Err(ErrorKind::Other)),
        }
    }
}

pub fn main(net: Box<dyn Net>) -> Box<dyn UsrNet> {
    println!("init xv6 network driver");
    Box::new(Rv6Net::new(net))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6net panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
