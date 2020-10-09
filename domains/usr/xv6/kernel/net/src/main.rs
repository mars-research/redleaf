#![no_std]
#![no_main]
// #![forbid(unsafe_code)]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate byteorder;

mod smoltcp_device;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::btree_map::BTreeMap;
use console::println;
use core::panic::PanicInfo;
use core::cell::RefCell;
use syscalls::{Heap, Syscall};
use rref::RRefVec;
use usr_interface::error::{Result, ErrorKind};
use usr_interface::net::Net;
use usr_interface::usrnet::UsrNet;
use usr_interface::rpc::RpcResult;
use spin::Mutex;

use smoltcp_device::SmolPhy;
use smoltcp::iface::{EthernetInterfaceBuilder, EthernetInterface, NeighborCache, Neighbor};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use smoltcp::socket::{
    Socket,
    SocketHandle,
    SocketRef,
    SocketSet,
    TcpSocket,
    TcpSocketBuffer
};

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

    ip_addresses: [IpCidr; 3],
    num_ip_addresses: usize,

    mac_address: [u8; 6],
    // neighbor_cache_entries: [Option<(IpAddress, Neighbor)>; 8],
    // neighbor_cache_entries: Vec<Option<(IpAddress, Neighbor)>>,

    last_polled: u64,

    socketset: SocketSet<'static, 'static, 'static>,
    handles: ArrayVec<[SocketHandle; 512]>,
}

impl Rv6NetInner {
    fn new(net: Box<dyn Net>) -> Self {
        // FIXME: Provide ways to setup IP and MAC
        let smol = SmolPhy::new(net);

        // let mut neighbor_cache_entries = [None; 8];
        let neighbor_cache = NeighborCache::new(BTreeMap::new());

        let ip_addresses = [
            IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24),
            IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24),
            IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24),
        ];
        let mac_address = [0x90, 0xe2, 0xba, 0xac, 0x16, 0x59];
        let mut iface = EthernetInterfaceBuilder::new(smol)
            .ethernet_addr(EthernetAddress::from_bytes(&mac_address))
            .neighbor_cache(neighbor_cache)
            .ip_addrs(ip_addresses)
            .finalize();

        let mut socketset = SocketSet::new(vec![]);

        Self {
            iface,

            ip_addresses,
            num_ip_addresses: 1,

            mac_address,
            // neighbor_cache_entries,
            last_polled: 0,

            socketset,
            handles: ArrayVec::new(),
        }
    }

    fn find_vacant_socket(&mut self) -> Option<usize> {
        for (i, &handle) in self.handles.iter().enumerate() {
            let socket = self.socketset.get::<TcpSocket>(handle);
            if !socket.is_active() && !socket.is_listening() {
                return Some(i);
            }
        }

        // Create new socket
        let socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 1024]),
            TcpSocketBuffer::new(vec![0; 1024]),
        );
        let socket: Socket = socket.into();
        let handle = self.socketset.add(socket);
        self.handles.push(handle);

        Some(self.handles.len() - 1)
    }

    fn poll(&mut self) {
        let current = libtime::get_ns_time();

        if (current - self.last_polled) > 100_000_000 || self.last_polled == 0 {
            // try to tx first
            self.iface.device_mut().do_tx();

            self.iface.device_mut().do_rx();
            self.iface.poll(&mut self.socketset, current);

            self.last_polled = current;
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

    fn listen(&self, port: u16) -> RpcResult<Result<usize>> {
        let mut state = self.state.lock();

        if let Some(i) = state.find_vacant_socket() {
            let handle = state.handles[i];
            let mut socket = state.socketset.get::<TcpSocket>(handle);

            if let Ok(_) = socket.listen(port) {
                Ok(Ok(i))
            } else {
                Ok(Err(ErrorKind::InvalidFileDescriptor))
            }
        } else {
            Ok(Err(ErrorKind::AddrNotAvailable))
        }
    }

    fn is_active(&self, server: usize) -> RpcResult<Result<bool>> {
        let mut state = self.state.lock();

        let handle = state.handles[server];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        Ok(Ok(socket.is_active()))
    }

    fn close(&self, server: usize) -> RpcResult<Result<()>> {
        let mut state = self.state.lock();

        let handle = state.handles[server];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        socket.close();

        Ok(Ok(()))
    }

    fn read_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        let mut state = self.state.lock();
        state.poll();

        let handle = state.handles[server];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        let mut dstbuf = buffer.as_mut_slice();

        let size = socket.recv(|buf| {
            let size = if buf.len() > dstbuf.len() {
                dstbuf.len()
            } else {
                buf.len()
            };

            dstbuf[..size].copy_from_slice(&buf[..size]);

            (size, size)
        }).expect("Failed to receive");

        Ok(Ok((size, buffer)))
    }

    fn write_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        let mut state = self.state.lock();
        state.poll();

        let handle = state.handles[server];
        let mut socket = state.socketset.get::<TcpSocket>(handle);

        let mut buf = buffer.as_mut_slice();

        let size = socket.recv(|dstbuf| {
            let size = if buf.len() > dstbuf.len() {
                dstbuf.len()
            } else {
                buf.len()
            };

            dstbuf[..size].copy_from_slice(&buf[..size]);

            (size, size)
        }).expect("Failed to send");

        Ok(Ok((size, buffer)))
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    net: Box<dyn Net>,
) -> Box<dyn UsrNet> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

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
