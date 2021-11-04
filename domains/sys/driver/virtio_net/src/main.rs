#![no_std]
#![no_main]
#![feature(
    box_syntax,
    untagged_unions,
    maybe_uninit_extra
)]

extern crate alloc;
extern crate malloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{boxed::Box, collections::BTreeMap};
use core::{borrow::BorrowMut, panic::PanicInfo, pin::Pin, usize};
use hashbrown::HashMap;
use syscalls::{Heap, Syscall};

use console::{print, println};
use interface::{net::Net, rpc::RpcResult};
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;

pub use interface::error::{ErrorKind, Result};
use virtio_network_device::pci::PciFactory;
use virtio_network_device::VirtioNetInner;

mod nullnet;

use interface::rref::{RRef, RRefDeque};

use smolnet::{self, SmolPhy};

pub use interface::net::NetworkStats;

pub struct VirtioNet(Arc<Mutex<VirtioNetInner>>);

impl interface::net::Net for VirtioNet {
    fn clone_net(&self) -> RpcResult<Box<dyn interface::net::Net>> {
        Ok(box Self(self.0.clone()))
    }

    fn submit_and_poll(
        &self,
        mut packets: &mut VecDeque<Vec<u8>>,
        mut collect: &mut VecDeque<Vec<u8>>,
        tx: bool,
    ) -> RpcResult<Result<usize>> {
        unimplemented!()
    }

    /// If `tx` is true, packets in packets are for transmitting, else they are receive buffers
    fn submit_and_poll_rref(
        &self,
        mut packets: RRefDeque<[u8; 1514], 32>,
        mut collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        let mut device = self.0.lock();

        let mut new_packet_count = 0;

        if tx {
            new_packet_count = device.free_processed_tx_packets(&mut collect);
            device.add_tx_buffers(&mut packets);
        } else {
            new_packet_count = device.get_received_packets(&mut collect);
            device.add_rx_buffers(&mut packets, &mut collect);
        }

        Ok(Ok((new_packet_count, packets, collect)))
    }

    fn poll(&self, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        unimplemented!()
    }

    fn poll_rref(
        &self,
        mut collect: RRefDeque<[u8; 1514], 512>,
        tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        // let mut new_packet_count = 0;
        // let device = self.0.lock();

        // if tx {
        //     new_packet_count = device.free_processed_tx_packets(&mut collect);
        // } else {
        //     new_packet_count = device.get_received_packets(&mut collect);
        // }

        // Ok(Ok((new_packet_count, collect)))
        Ok(Ok((0, collect)))
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        // unimplemented!()
        Ok(Ok(NetworkStats {
            tx_count: 0,
            rx_count: 0,
            tx_dma_ok: 0,
            rx_dma_ok: 0,
            rx_missed: 0,
            rx_crc_err: 0,
        }))
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        unimplemented!()
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn interface::net::Net> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    #[cfg(feature = "virtio_net")]
    let net = {
        let net = {
            let mut pci_factory = PciFactory::new();
            if pci.pci_register_driver(&mut pci_factory, 4, None).is_err() {
                panic!("Failed to probe VirtioNet PCI");
            }
            let dev = pci_factory.to_device().unwrap();
            VirtioNet(Arc::new(Mutex::new(dev)))
        };
        net.0.lock().init();
        net
    };

    #[cfg(not(feature = "virtio_net"))]
    let net = { nullnet::NullNet::new() };

    // libbenchnet::run_fwd_udptest_rref(&net, 1514);

    // VIRTIO DEMO LOOP
    // Run SmolNet

    // let mut smol = SmolPhy::new(Box::new(net));

    // use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache};
    // use smoltcp::socket::SocketSet;
    // use smoltcp::time::Instant;
    // use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};

    // let neighbor_cache = NeighborCache::new(BTreeMap::new());

    // let ip_addresses = [IpCidr::new(IpAddress::v4(10, 10, 10, 10), 24)];
    // let mac_address = [0x90, 0xe2, 0xba, 0xb3, 0xb9, 0x10];
    // let mut iface = EthernetInterfaceBuilder::new(smol)
    //     .ethernet_addr(EthernetAddress::from_bytes(&mac_address))
    //     .neighbor_cache(neighbor_cache)
    //     .ip_addrs(ip_addresses)
    //     .finalize();

    // let mut sockets = SocketSet::new(Vec::with_capacity(512));

    // let mut httpd = redhttpd::Httpd::new();

    // loop {
    //     iface.device_mut().do_rx();

    //     let current = libtime::get_ns_time() / 1000000;
    //     let timestamp = Instant::from_millis(current as i64);

    //     iface.poll(&mut sockets, timestamp);
    //     httpd.handle(&mut sockets);
    //     iface.device_mut().do_tx();
    // }

    Box::new(net)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}

#[link_name = "xyz"]
pub fn foo() {
    println!("foo");
}
