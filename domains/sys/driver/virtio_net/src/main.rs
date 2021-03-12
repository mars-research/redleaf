#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra
)]

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};

use console::println;
use interface::rpc::RpcResult;
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;

pub use interface::error::{ErrorKind, Result};

use rref::RRefDeque;

pub use interface::net::NetworkStats;

mod mmio;
mod pci;

use mmio::Mmio;
use mmio::Register;
use mmio::VirtioDeviceStatus;
use pci::PciFactory;

/// The number of Descriptors (must be a multiple of 2), called "Queue Size" in documentation
pub const DESCRIPTOR_COUNT: usize = 256; // Maybe change this to 256, was 8 before

static mut memory_location: [u64; 10000] = [0u64; 10000];

#[derive(Debug)]
#[repr(C, packed(16))]
struct VirtualQueues {
    recieve_queue: VirtQueue,
    padding: u8,
    transmit_queue: VirtQueue,
}

static mut VIRTUAL_QUEUES: VirtualQueues = VirtualQueues {
    padding: 255,
    recieve_queue: VirtQueue {
        descriptors: [VirtqDescriptor {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }; DESCRIPTOR_COUNT],
        available: VirtqAvailable {
            flags: 0,
            idx: 0,
            ring: [0; DESCRIPTOR_COUNT],
        },
        used: VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElement { id: 0, len: 0 }; DESCRIPTOR_COUNT],
        },
    },
    transmit_queue: VirtQueue {
        descriptors: [VirtqDescriptor {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }; DESCRIPTOR_COUNT],
        available: VirtqAvailable {
            flags: 0,
            idx: 0,
            ring: [0; DESCRIPTOR_COUNT],
        },
        used: VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElement { id: 0, len: 0 }; DESCRIPTOR_COUNT],
        },
    },
};

// First page - first section is descriptors
// Second portion of 1st page is available
// Second page is used

// 2.6.12 Virtqueue Operation
// There are two parts to virtqueue operation: supplying new available buffers to the device, and processing used buffers from the device.
// Note: As an example, the simplest virtio network device has two virtqueues: the transmit virtqueue and the receive virtqueue.
// The driver adds outgoing (device-readable) packets to the transmit virtqueue, and then frees them after they are used.
// Similarly, incoming (device-writable) buffers are added to the receive virtqueue, and processed after they are used.

#[derive(Debug)]
#[repr(C, packed(16))]
struct VirtQueue {
    descriptors: [VirtqDescriptor; DESCRIPTOR_COUNT],
    available: VirtqAvailable,
    used: VirtqUsed,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed(16))]
struct VirtqDescriptor {
    /// Address (guest-physical) to Virtio Net Packet Header
    addr: u64,
    /// Length
    len: u32,

    flags: u16,

    /// Next field if flags contains NEXT
    next: u16,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqAvailable {
    flags: u16,

    /// Index into VirtqDescriptor Array (Count of Descriptor Chain Heads???)
    idx: u16,

    /// The number is the index of the head of the descriptor chain in the descriptor table
    ring: [u16; DESCRIPTOR_COUNT],
}

impl VirtqAvailable {
    fn default() -> VirtqAvailable {
        VirtqAvailable {
            flags: 0,
            idx: 0,
            ring: [0; DESCRIPTOR_COUNT],
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed)]
struct VirtqUsedElement {
    /// Index of start of used descriptor chain
    id: u32,
    /// Total length of the descriptor chain used
    len: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqUsed {
    flags: u16,

    /// Index into VirtqDescriptor Array
    idx: u16,

    ring: [VirtqUsedElement; DESCRIPTOR_COUNT],
}

impl VirtqUsed {
    fn default() -> VirtqUsed {
        VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElement { id: 0, len: 0 }; DESCRIPTOR_COUNT],
        }
    }
}

struct VirtioNetInner {
    mmio: Mmio,
    // add your stuff here
}

impl VirtioNetInner {
    /// Returns an initialized VirtioNet from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        let mut mmio = Mmio::new(mmio_base);

        // mmio.sanity_check_panic();

        println!("Initializing Virtio Network Device");

        // VIRTIO DEVICE INIT
        // https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-920001
        //
        // Reset the device.
        // Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
        // Set the DRIVER status bit: the guest OS knows how to drive the device.
        // Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device. During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check that it can support the device before accepting it.
        // Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
        // Re-read device status to ensure the FEATURES_OK bit is still set: otherwise, the device does not support our subset of features and the device is unusable.
        // Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup, reading and possibly writing the device’s virtio configuration space, and population of virtqueues.
        // Set the DRIVER_OK status bit. At this point the device is “live”.

        // Acknowlege Device
        mmio.write_device_status(VirtioDeviceStatus::Acknowledge);
        mmio.write_device_status(VirtioDeviceStatus::Driver); // But do we really know how to drive the device?

        // Negotiate Features
        let mut cfg = mmio.read_common_config();
        println!("{:#?}", cfg);

        // let mut feature_bits: u32 = cfg.device_feature;
        let mut feature_bits: u32 = 0;
        feature_bits |= 5; // Enable Device MAC Address
        feature_bits |= 16; // Enable Device Status
        cfg.driver_feature = feature_bits;

        // Write back the Config
        println!("{:#?}", cfg);
        mmio.write_common_config(cfg);

        // Negotiate Features Part 2
        // Select the other feature bits
        // let mut cfg = mmio.read_common_config();
        // cfg.device_feature_select = 1;
        // cfg.driver_feature_select = 1;
        // mmio.write_common_config(cfg);

        // let mut cfg = mmio.read_common_config();
        // println!("{:#?}", cfg);
        // let feature_bits: u32 = 1;
        // cfg.driver_feature = feature_bits;
        // println!("{:#?}", cfg);
        // mmio.write_common_config(cfg);

        // Tell the Device that feature Negotiation is complete
        mmio.write_device_status(VirtioDeviceStatus::FeaturesOk);

        if mmio.read_device_status() != VirtioDeviceStatus::FeaturesOk {
            panic!("Requested features *NOT* supported by VirtIO Device!");
        }

        println!("{:#?}", mmio.read_common_config());

        // Setup VirtQueues

        // Setup RECEIEVE_QUEUE
        let mut cfg = mmio.read_common_config();
        cfg.queue_select = 0;
        cfg.queue_desc = (&VIRTUAL_QUEUES.recieve_queue.descriptors
            as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64;
        cfg.queue_driver =
            (&VIRTUAL_QUEUES.recieve_queue.available as *const VirtqAvailable) as u64;
        cfg.queue_device = (&VIRTUAL_QUEUES.recieve_queue.used as *const VirtqUsed) as u64;
        cfg.queue_enable = 1;
        println!("WRITING RECEIVE_QUEUE: {:#?}", cfg);
        mmio.write_common_config(cfg);

        // Setup TRANSMIT_QUEUE
        let mut cfg = mmio.read_common_config();
        cfg.queue_select = 1;
        cfg.queue_desc = (&VIRTUAL_QUEUES.transmit_queue.descriptors
            as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64;
        cfg.queue_driver =
            (&VIRTUAL_QUEUES.transmit_queue.available as *const VirtqAvailable) as u64;
        cfg.queue_device = (&VIRTUAL_QUEUES.transmit_queue.used as *const VirtqUsed) as u64;
        cfg.queue_enable = 1;
        println!("WRITING TRANSMIT_QUEUE: {:#?}", cfg);
        mmio.write_common_config(cfg);

        VIRTUAL_QUEUES.recieve_queue.descriptors[0] = VirtqDescriptor {
            addr: (&memory_location as *const u64) as u64,
            len: 8 * 1000,
            flags: 2, // For VIRTQ_DESC_F_WRITE
            next: 0,
        };

        println!(
            "LOCATION OF BUFFER: {:x?}",
            VIRTUAL_QUEUES.recieve_queue.descriptors[0].addr
        );

        VIRTUAL_QUEUES.recieve_queue.available.ring[0] = 0;
        VIRTUAL_QUEUES.recieve_queue.available.idx = 1;

        // Tell the Device we're all done
        mmio.write_device_status(VirtioDeviceStatus::DriverOk);

        println!("{:#?}", mmio.read_device_status());

        println!("VirtIO Device Initialized!");

        Self { mmio }
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
