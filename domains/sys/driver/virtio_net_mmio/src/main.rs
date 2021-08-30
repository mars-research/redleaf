#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra,
    core_intrinsics
)]

extern crate alloc;
extern crate malloc;

mod nullnet;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{boxed::Box, collections::BTreeMap};
use core::intrinsics::size_of;
use core::ptr::{read_volatile, write_volatile};
use core::{borrow::BorrowMut, panic::PanicInfo, pin::Pin, usize};
use syscalls::{Heap, Syscall};

use console::{print, println};
use interface::{net::Net, rpc::RpcResult};
use libsyscalls::syscalls::{sys_backtrace, sys_yield};
pub use platform::PciBarAddr;
use spin::Mutex;

pub use interface::error::{ErrorKind, Result};
use virtio_net_mmio_device::VirtioNetInner;

use interface::rref::{RRef, RRefDeque};

use smolnet::{self, SmolPhy};

pub use interface::net::NetworkStats;

const MMIO_CONFIG_ADDRESS: usize = 0x100000;

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

    unsafe {
        let mut inner = VirtioNetInner::new(MMIO_CONFIG_ADDRESS);
        inner.init();
    }

    loop {}

    Box::new(nullnet::NullNet::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
