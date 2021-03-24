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

use mmio::VirtioDeviceStatus;
use mmio::{Mmio, VirtioNetCompletePacket, VirtioNetworkHeader};
use mmio::{Register, VirtioPciCommonConfig};
use pci::PciFactory;

/// The number of Descriptors (must be a multiple of 2), called "Queue Size" in documentation
pub const DESCRIPTOR_COUNT: usize = 256; // Maybe change this to 256, was 8 before

static BUFFERS: [VirtioNetCompletePacket; DESCRIPTOR_COUNT] = [VirtioNetCompletePacket {
    header: VirtioNetworkHeader {
        flags: 0,
        gso_type: 0,
        header_length: 0, // This Header: 12, Ethernet: 22
        gso_size: 0,
        csum_start: 0,
        csum_offset: 0,
        num_buffers: 0,
    },
    data: [0; 1514],
}; DESCRIPTOR_COUNT];

static mut PACKET_FOR_SENDING: VirtioNetCompletePacket = VirtioNetCompletePacket {
    header: VirtioNetworkHeader {
        flags: 0,
        gso_type: 0,
        header_length: 12 + 22,
        gso_size: 1514,
        csum_start: 0,
        csum_offset: 1514,
        num_buffers: 0,
    },
    data: [0; 1514],
};

static tian_packet: [u8; 66] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x61, 0x5F, 0x08, 0x37, 0x23, 0x08, 0x00, 0x45, 0x00,
    0x00, 0x34, 0x20, 0xF3, 0x40, 0x00, 0x34, 0x06, 0x1C, 0xBF, 0xB8, 0x69, 0x94, 0x72, 0xAC, 0x14,
    0x10, 0x22, 0x01, 0xBB, 0xCA, 0xFC, 0xDA, 0xD3, 0x61, 0x34, 0xD8, 0xBF, 0xCC, 0x09, 0x80, 0x10,
    0x00, 0x13, 0xF5, 0xAD, 0x00, 0x00, 0x01, 0x01, 0x08, 0x0A, 0x9E, 0xC6, 0xA7, 0xE1, 0x1D, 0x2A,
    0x66, 0x8E,
];

#[derive(Debug)]
#[repr(C, packed(16))]
struct VirtualQueues {
    recieve_queue: VirtQueue,
    padding: u8,
    transmit_queue: VirtQueue,
}

static mut VIRTUAL_QUEUES: VirtualQueues = VirtualQueues {
    padding: 0,
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
        mmio.update_device_status(VirtioDeviceStatus::Acknowledge);
        mmio.update_device_status(VirtioDeviceStatus::Driver); // But do we really know how to drive the device?

        Self::negotiate_features(&mut mmio);

        // Tell the Device that feature Negotiation is complete
        mmio.update_device_status(VirtioDeviceStatus::FeaturesOk);

        // Check that Features OK Bit is still set!
        // TODO: Actually check the Feature Bit!
        Self::print_device_status(&mut mmio);

        // Setup VirtQueues
        println!(
            "Queue select offset {}, address {}, value {}",
            mmio.accessor.queue_select_offset(),
            mmio.accessor.queue_select_address(),
            mmio.accessor.read_queue_select()
        );
        Self::initialize_virtual_queue(&mut mmio, 0, &VIRTUAL_QUEUES.recieve_queue);
        Self::initialize_virtual_queue(&mut mmio, 1, &VIRTUAL_QUEUES.transmit_queue);

        // Tell the Device we're all done, even though we aren't
        mmio.update_device_status(VirtioDeviceStatus::DriverOk);

        Self::print_device_status(&mut mmio);

        // *** Stuff For Testing ***

        // Populate the testing packet with Tian's packet
        for i in 0..tian_packet.len() {
            PACKET_FOR_SENDING.data[i] = tian_packet[i];
        }

        // Populate Recieve Queue
        for i in 0..DESCRIPTOR_COUNT {
            VIRTUAL_QUEUES.recieve_queue.descriptors[i] = VirtqDescriptor {
                addr: (&BUFFERS[i] as *const VirtioNetCompletePacket) as u64,
                len: 1526,
                flags: 0 | 2, // Writeable
                next: 0,
            };
            VIRTUAL_QUEUES.recieve_queue.available.ring[i] = i as u16;
            Mmio::memory_fence();
            VIRTUAL_QUEUES.recieve_queue.available.idx += 1;
        }

        println!(
            "RX AVAIL IDX: {:} aka {:x?}",
            VIRTUAL_QUEUES.recieve_queue.available.idx, VIRTUAL_QUEUES.recieve_queue.available.idx
        );

        for i in 0..5 {
            println!(
                "DESCRIPTOR {:}: {:#x?}",
                i, VIRTUAL_QUEUES.recieve_queue.descriptors[i]
            );
        }

        Mmio::memory_fence();
        // Notify the device that we've changed things in Queue 0, the recieve queue
        mmio.write(Register::Notify, 0u16);

        // SENDING PACKET BELOW

        VIRTUAL_QUEUES.transmit_queue.descriptors[0] = VirtqDescriptor {
            addr: (&PACKET_FOR_SENDING as *const VirtioNetCompletePacket) as u64,
            len: 1526,
            flags: 0,
            next: 0,
        };

        println!("{:#?}", VIRTUAL_QUEUES.transmit_queue.descriptors[0]);
        // println!("{:#?}", PACKET_FOR_SENDING);

        VIRTUAL_QUEUES.transmit_queue.available.ring[0] = 0;
        Mmio::memory_fence();
        VIRTUAL_QUEUES.transmit_queue.available.idx += 1;
        Mmio::memory_fence();

        // 4.1.5.2
        // When VIRTIO_F_NOTIFICATION_DATA has not been negotiated,
        // the driver sends an available buffer notification to the device
        // by writing the 16-bit virtqueue index of this virtqueue to the Queue Notify address.

        println!("Notification Sending");
        mmio.write(Register::Notify, 1u16);
        println!("Notification Sent");

        // Print out some final info
        // println!("{:#x?}", mmio.read_device_config());

        println!("VirtIO Device Initialized!");

        println!("{:#?}", VIRTUAL_QUEUES.transmit_queue.used.ring[0]);
        println!("{:#?}", VIRTUAL_QUEUES.transmit_queue.used.idx);

        // Read the config back
        Self::print_device_config(&mut mmio);

        Self { mmio }
    }

    unsafe fn negotiate_features(mmio: &mut Mmio) {
        // Negotiate Features

        let mut driver_features: u32 = 0;
        driver_features |= 1 << 5; // Enable Device MAC Address
        driver_features |= 1 << 16; // Enable Device Status
                                    // feature_bits |= 15; // VIRTIO_NET_F_MRG_RXBUF - Driver can merge recieved buffers
        mmio.accessor.write_driver_feature(driver_features); // Should be &'d with device_features
    }

    unsafe fn print_device_config(mmio: &mut Mmio) {
        let mut cfg = mmio.read_common_config();
        println!("{:#?}", cfg);
    }

    unsafe fn print_device_status(mmio: &mut Mmio) {
        let device_status = mmio.accessor.read_device_status();
        println!("Device Status Bits: {:b}", device_status);
    }

    /// Recieve Queues must be 2*N and Transmit Queues must be 2*N + 1
    /// For example, Revieve Queue must be 0 and Transmit Queue must be 1
    unsafe fn initialize_virtual_queue(mmio: &mut Mmio, queue_index: u16, virt_queue: &VirtQueue) {
        println!("###CONFIG BEFORE###");
        Self::print_device_config(mmio);
        println!("###################");

        mmio.accessor.write_queue_select(queue_index);

        mmio.accessor.write_queue_desc(
            (&virt_queue.descriptors as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64,
        );
        mmio.accessor
            .write_queue_driver((&virt_queue.available as *const VirtqAvailable) as u64);
        mmio.accessor
            .write_queue_device((&virt_queue.used as *const VirtqUsed) as u64);
        mmio.accessor.write_queue_enable(1);

        println!(
            "Config for QUEUE {:} should be: {:#?}",
            queue_index,
            (&virt_queue.descriptors as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64
        );

        println!("###CONFIG AFTER###");
        Self::print_device_config(mmio);
        println!("##################");
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
