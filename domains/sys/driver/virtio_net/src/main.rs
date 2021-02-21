#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    maybe_uninit_extra
)]

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};

use console::println;
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;
use interface::rpc::RpcResult;

pub use interface::error::{ErrorKind, Result};

use rref::RRefDeque;

pub use interface::net::NetworkStats;

mod mmio;
mod pci;

use mmio::Mmio;
use mmio::Register;
use pci::PciFactory;

// Virtio Constants
const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
const VIRTIO_CONFIG_S_DRIVER: u32 = 2;
const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 4;
const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 8;

const VIRTIO_NET_F_MAC: u32 = 5;
const VIRTIO_NET_F_STATUS: u32 = 16;

struct VirtioNetInner {
    mmio: Mmio,

    // add your stuff here
}

impl VirtioNetInner {
    /// Returns an initialized VirtioNet from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        let mut mmio = Mmio::new(mmio_base);

        // mmio.sanity_check_panic();

        // Negotiate Features
        let mut status: u32 = 0;

        // Reset the device

        mmio.write(Register::MmioStatus, 0);

        for i in 0..100 {
            println!("VIRTIO DEVICE STATUS BEFORE CHANGE {}", mmio.read(Register::MmioStatus));
        }

        //

        status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
        mmio.write(Register::MmioStatus, status);

        status |= VIRTIO_CONFIG_S_DRIVER;
        mmio.write(Register::MmioStatus, status);

        mmio.write(Register::MmioStatus, 0xDEADBEEF);

        for i in 0..100 {
            println!("VIRTIO DEVICE STATUS AFTER CHANGE {}", mmio.read(Register::MmioStatus));
        }

        let mut features = mmio.read(Register::MmioDeviceFeatures);
        println!("VIRTIO FEATURES BEFORE: {:x}", features);

        features &= !(1 << VIRTIO_NET_F_MAC);
        features &= !(1 << VIRTIO_NET_F_STATUS);

        mmio.write(Register::MmioDeviceFeatures, features);

        // status |= VIRTIO_CONFIG_S_FEATURES_OK;
        // mmio.write(Register::MmioStatus, status);

        // status |= VIRTIO_CONFIG_S_DRIVER_OK;
        // mmio.write(Register::MmioStatus, status);

        println!("VIRTIO STATUS: {:x}", mmio.read(Register::MmioStatus));
        
        Self {
            mmio,
        }
    }

    fn to_shared(self) -> VirtioNet {
        VirtioNet(Arc::new(Mutex::new(self)))
    }
}

struct VirtioNet(Arc<Mutex<VirtioNetInner>>);

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

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        unimplemented!()
    }

    fn poll(&self, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        unimplemented!()
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<[u8; 1514], 512>,
        tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        unimplemented!()
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        unimplemented!()
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
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    let net = {
        let mut pci_factory = PciFactory::new();
        if pci.pci_register_driver(&mut pci_factory, 4, None).is_err() {
            panic!("Failed to probe VirtioNet PCI");
        }
        pci_factory.to_device().unwrap()
    };

    Box::new(net)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
