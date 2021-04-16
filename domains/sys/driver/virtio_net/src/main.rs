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
// use alloc::collections::btree_map::BTreeMap; // Faulty implementation!
use hashbrown::HashMap;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::{borrow::BorrowMut, panic::PanicInfo, pin::Pin};
use syscalls::{Heap, Syscall};

use console::{print, println};
use interface::{net::Net, rpc::RpcResult};
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;

pub use interface::error::{ErrorKind, Result};

use rref::{RRef, RRefDeque};

use smolnet::{self, SmolPhy};

pub use interface::net::NetworkStats;

mod mmio;
mod pci;

use mmio::VirtioDeviceStatus;
use mmio::{Mmio, VirtioNetCompletePacket, VirtioNetworkHeader};
use mmio::{Register, VirtioPciCommonConfig};
use pci::PciFactory;

/// The number of Descriptors (must be a multiple of 2), called "Queue Size" in documentation
pub const DESCRIPTOR_COUNT: usize = 256; // Maybe change this to 256, was 8 before

type NetworkPacketBuffer = [u8; 1514];

#[derive(Debug)]
#[repr(C, align(16))]
struct VirtualQueues {
    receive_queue: VirtQueue,
    transmit_queue: VirtQueue,
}

// First page - first section is descriptors
// Second portion of 1st page is available
// Second page is used

// 2.6.12 Virtqueue Operation
// There are two parts to virtqueue operation: supplying new available buffers to the device, and processing used buffers from the device.
// Note: As an example, the simplest virtio network device has two virtqueues: the transmit virtqueue and the receive virtqueue.
// The driver adds outgoing (device-readable) packets to the transmit virtqueue, and then frees them after they are used.
// Similarly, incoming (device-writable) buffers are added to the receive virtqueue, and processed after they are used.

#[derive(Debug)]
#[repr(C, align(16))]
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
#[repr(C, packed(2))]
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
#[repr(C, packed(4))]
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
    virtual_queues: VirtualQueues,

    /// Dummy VirtioNetHeaders
    virtio_network_headers: [VirtioNetworkHeader; DESCRIPTOR_COUNT],

    // Tracks the number of free descriptors spaces remaining
    rx_free_descriptor_count: usize,

    /// Tracks which descriptors on the queue are free
    rx_free_descriptors: [bool; DESCRIPTOR_COUNT],

    /// Tracks which descriptors on the queue are free
    tx_free_descriptors: [bool; DESCRIPTOR_COUNT],

    // The last index (of the used ring) that was checked by the driver
    rx_last_idx: u16,
    tx_last_idx: u16,

    /// Holds the rx_packets (to prevent dropping) while they are in the rx_queue. The key is their address.
    rx_buffers: HashMap<u64, RRef<NetworkPacketBuffer>>,
}

impl VirtioNetInner {
    /// Returns an initialized VirtioNet from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        let mut mmio = Mmio::new(mmio_base);

        let mut virtual_queues = VirtualQueues {
            receive_queue: VirtQueue {
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

        let mut virtio_network_headers = [VirtioNetworkHeader {
            flags: 0,
            gso_type: 0,
            header_length: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
        }; DESCRIPTOR_COUNT];

        let mut rx_free_descriptors = [true; DESCRIPTOR_COUNT];
        let mut tx_free_descriptors = [true; DESCRIPTOR_COUNT];

        let mut virtio_inner = Self {
            mmio,
            virtual_queues,
            virtio_network_headers,

            rx_free_descriptor_count: DESCRIPTOR_COUNT,

            rx_free_descriptors,
            tx_free_descriptors,

            rx_last_idx: 0,
            tx_last_idx: 0,

            rx_buffers: HashMap::new(),
        };

        // virtio_inner.init();
        // virtio_inner.testing();

        virtio_inner
    }

    unsafe fn init(&mut self) {
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

        // Acknowledge Device
        self.mmio
            .update_device_status(VirtioDeviceStatus::Acknowledge);
        self.mmio.update_device_status(VirtioDeviceStatus::Driver); // But do we really know how to drive the device?

        self.negotiate_features();

        // Tell the Device that feature Negotiation is complete
        self.mmio
            .update_device_status(VirtioDeviceStatus::FeaturesOk);

        // Check that Features OK Bit is still set!
        // TODO: Actually check the Feature Bit!
        self.print_device_status();
        if (self.mmio.accessor.read_device_status() & VirtioDeviceStatus::FeaturesOk.value()) == 0 {
            panic!("Failed to negotiate virtio net features!");
        }

        // Setup Virtual Queues
        self.initialize_virtual_queue(0, &(self.virtual_queues.receive_queue));
        self.initialize_virtual_queue(1, &(self.virtual_queues.transmit_queue));

        // Tell the Device we're all done, even though we aren't
        self.mmio.update_device_status(VirtioDeviceStatus::DriverOk);

        self.print_device_status();

        self.mmio.accessor.write_queue_select(0);
        self.print_device_config();
        self.mmio.accessor.write_queue_select(1);
        self.print_device_config();
    }

    // unsafe fn testing(&mut self) {
    //     println!(
    //         "VIRTQ STRUCT ADDR: {:x}",
    //         (&self.virtual_queues as *const VirtualQueues) as usize
    //     );

    //     // *** Stuff For Testing ***

    //     // Populate the testing packet with Tian's packet
    //     for i in 0..TIAN_PACKET.len() {
    //         PACKET_FOR_SENDING.data[i] = TIAN_PACKET[i];
    //     }

    //     // Populate Receive Queue
    //     for i in 0..DESCRIPTOR_COUNT {
    //         self.virtual_queues.receive_queue.descriptors[i] = VirtqDescriptor {
    //             addr: (&BUFFERS[i] as *const VirtioNetCompletePacket) as u64,
    //             len: 1526,
    //             flags: 0 | 2, // Writeable
    //             next: 0,
    //         };
    //         self.virtual_queues.receive_queue.available.ring[i] = i as u16;
    //         self.virtual_queues.receive_queue.available.idx += 1;
    //     }

    //     for i in 30..32 {
    //         println!(
    //             "DESCRIPTOR {:}: {:#x?}",
    //             i, self.virtual_queues.receive_queue.descriptors[i]
    //         );
    //     }

    //     // Notify the device that we've changed things in Queue 0, the receive queue
    //     self.mmio.queue_notify(0, 0);

    //     println!("VirtIO Device Initialized!");

    //     // SENDING PACKET BELOW

    //     self.virtual_queues.transmit_queue.descriptors[0] = VirtqDescriptor {
    //         addr: (&PACKET_FOR_SENDING as *const VirtioNetCompletePacket) as u64,
    //         len: 1526,
    //         flags: 0,
    //         next: 0,
    //     };

    //     println!("{:#?}", self.virtual_queues.transmit_queue.descriptors[0]);

    //     self.virtual_queues.transmit_queue.available.ring[0] = 0;
    //     self.virtual_queues.transmit_queue.available.idx += 1;

    //     println!("Notification Sending");
    //     self.mmio.queue_notify(1, 1);
    //     println!("Notification Sent");

    //     println!("---------------------");
    //     self.mmio.accessor.write_queue_select(0);
    //     self.print_device_config();
    //     self.mmio.accessor.write_queue_select(1);
    //     self.print_device_config();
    // }

    /// Negotiates Virtio Driver Features
    unsafe fn negotiate_features(&mut self) {
        let mut driver_features: u32 = 0;
        driver_features |= 1 << 5; // Enable Device MAC Address
        driver_features |= 1 << 16; // Enable Device Status

        self.mmio.accessor.write_driver_feature(driver_features); // Should be &'d with device_features
    }

    unsafe fn print_device_config(&mut self) {
        let mut cfg = self.mmio.read_common_config();
        println!("{:#?}", cfg);
    }

    unsafe fn print_device_status(&mut self) {
        let device_status = self.mmio.accessor.read_device_status();
        println!("Device Status Bits: {:b}", device_status);
    }

    /// Receive Queues must be 2*N and Transmit Queues must be 2*N + 1
    /// For example, Receive Queue must be 0 and Transmit Queue must be 1
    unsafe fn initialize_virtual_queue(&self, queue_index: u16, virt_queue: &VirtQueue) {
        self.mmio.accessor.write_queue_select(queue_index);

        self.mmio.accessor.write_queue_desc(
            (&virt_queue.descriptors as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64,
        );
        self.mmio
            .accessor
            .write_queue_driver((&virt_queue.available as *const VirtqAvailable) as u64);
        self.mmio
            .accessor
            .write_queue_device((&virt_queue.used as *const VirtqUsed) as u64);
        self.mmio.accessor.write_queue_enable(1);

        println!(
            "Config for QUEUE {:} should be: {:#?}",
            queue_index,
            (&virt_queue.descriptors as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64
        );
    }

    fn get_next_free_buffer(free_buffers: &mut [bool; DESCRIPTOR_COUNT]) -> Result<usize> {
        for i in 0..DESCRIPTOR_COUNT {
            if free_buffers[i] {
                free_buffers[i] = false;
                return Ok(i);
            }
        }
        Err(ErrorKind::Other)
    }

    fn get_addr<T>(obj: &T) -> u64 {
        (obj as *const T) as u64
    }

    fn add_rx_buffer(&mut self, buffer: RRef<NetworkPacketBuffer>) {
        // One descriptor points at the network header, chain this with a descriptor to the buffer

        if self.rx_free_descriptor_count < 2 {
            // Send the buffer back
            // Err(buffer)
            return;
        }

        let rx_q = &mut self.virtual_queues.receive_queue;

        let header_idx = Self::get_next_free_buffer(&mut self.rx_free_descriptors).unwrap();
        let buffer_idx = Self::get_next_free_buffer(&mut self.rx_free_descriptors).unwrap();

        self.rx_free_descriptor_count -= 2;

        // Pin our buffer
        let buffer_addr = buffer.as_ptr() as u64;

        if self.rx_buffers.contains_key(&buffer_addr) {
            println!("BUFFER ADDR KEY CONFLICT!!! {:}", buffer_addr);
        }

        self.rx_buffers.insert(buffer_addr, buffer);

        println!(
            "ADDED KEY: {:?}, is in map: {:?}",
            buffer_addr,
            self.rx_buffers.contains_key(&buffer_addr)
        );

        // Add the buffer for the header
        rx_q.descriptors[header_idx] = VirtqDescriptor {
            addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
            len: 14,
            // 1 is NEXT FLAG
            // 2 is WRITABLE FLAG
            flags: 1 | 2,
            next: buffer_idx as u16,
        };

        // Add the buffer
        rx_q.descriptors[buffer_idx] = VirtqDescriptor {
            addr: buffer_addr,
            len: 1514,
            flags: 2,
            next: 0,
        };

        println!("ADDED BUFFER: ID: {:}, ADDR: {:}", buffer_idx, buffer_addr);

        // println!(
        //     "ADDED BUFFERS: {:#?} {:#?}",
        //     rx_q.descriptors[header_idx], rx_q.descriptors[buffer_idx]
        // );

        println!("AVAIL IDX: {:}", rx_q.available.idx);

        // Mark the buffer as usable
        rx_q.available.ring[(rx_q.available.idx as usize) % DESCRIPTOR_COUNT] = header_idx as u16;
        rx_q.available.idx += 1; // We only added one "chain head"

        // Ok(())
    }

    fn add_rx_buffers(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
        collect: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) {
        while let Some(buffer) = packets.pop_front() {
            if (self.rx_free_descriptor_count >= 2) {
                let res = self.add_rx_buffer(buffer);
            } else {
                collect.push_back(buffer);
            }
        }
        // Notify the device (could be moved to later, when there're more buffers)
        unsafe {
            self.mmio.queue_notify(0, 0);
        }
    }

    fn add_tx_packet(&mut self, buffer: &NetworkPacketBuffer) -> Result<()> {
        let header_idx = Self::get_next_free_buffer(&mut self.tx_free_descriptors);
        let buffer_idx = Self::get_next_free_buffer(&mut self.tx_free_descriptors);

        if header_idx.is_err() || buffer_idx.is_err() {
            // println!(
            //     "No space in RX Queue available for new buffers. Need to receive packets first!"
            // );
            return Err(ErrorKind::Other);
        }

        let header_idx = header_idx.unwrap();
        let buffer_idx = buffer_idx.unwrap();

        self.virtual_queues.transmit_queue.descriptors[header_idx] = VirtqDescriptor {
            addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
            len: 14,
            flags: 1, // 1 is next flag
            next: (buffer_idx as u16),
        };
        self.virtual_queues.transmit_queue.descriptors[buffer_idx] = VirtqDescriptor {
            addr: Self::get_addr(buffer),
            len: 1514,
            flags: 0,
            next: 0,
        };

        self.virtual_queues.transmit_queue.available.ring
            [(self.virtual_queues.transmit_queue.available.idx as usize) % DESCRIPTOR_COUNT] =
            (header_idx as u16);
        self.virtual_queues.transmit_queue.available.idx += 1;

        Ok(())
    }

    fn add_tx_packets(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        while let Some(packet) = packets.pop_front() {
            self.add_tx_packet(&packet);
        }

        unsafe {
            self.mmio.queue_notify(1, 1);
        }
    }

    fn get_received_packets(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        println!(
            "Looking for new packets. RX_LAST: {:}, USED_IDX: {:}",
            self.rx_last_idx, self.virtual_queues.receive_queue.used.idx
        );

        while self.rx_last_idx < self.virtual_queues.receive_queue.used.idx {
            let used_element = self.virtual_queues.receive_queue.used.ring
                [(self.rx_last_idx as usize) % DESCRIPTOR_COUNT];
            println!("Received buffer: {:#?}", used_element);

            let used_element_descriptor =
                self.virtual_queues.receive_queue.descriptors[used_element.id as usize];
            let buffer_descriptor = self.virtual_queues.receive_queue.descriptors
                [used_element_descriptor.next as usize];

            println!(
                "RETRIEVTING BUFFER: ID: {:}, ADDR: {:}",
                used_element_descriptor.next, buffer_descriptor.addr
            );

            // for (k, v) in self.rx_buffers.iter() {
            //     println!("KEY: {:}, ADDR: {:?}", k, v.as_ptr() as u64);
            // }
            println!("{:?}", self.rx_buffers.len());

            println!("USED ELEMENT DESCRIPTOR {:#?}", used_element_descriptor);
            println!("BUFFER DESCRIPTOR {:#?}", buffer_descriptor);

            println!(
                "KEY IN RX_BUFFERS? {:#?}",
                self.rx_buffers.contains_key(&buffer_descriptor.addr)
            );

            if let Some(buffer) = self.rx_buffers.remove(&buffer_descriptor.addr) {
                for i in 1..used_element.len as usize {
                    print!("{:x} ", buffer[i]);
                }
                println!("");
                packets.push_back(buffer);
            } else {
                println!("ERROR: VIRTIO NET: RX BUFFER MISSING OR BUFFER ADDRESS CHANGED!");
            }

            self.rx_last_idx += 1;
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

    /// If `tx` is true, packets in packets are for transmitting, else they are receive buffers
    fn submit_and_poll_rref(
        &self,
        mut packets: RRefDeque<[u8; 1514], 32>,
        mut collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        //println!("SUBMIT AND POLL RREF CALLED!");

        // println!("{:#?}", packets.len());
        // println!("{:#?}", collect.len());
        // println!("{:#?}", tx);

        let mut device = self.0.lock();

        if tx {
            //println!("HANDLE TX");
            device.add_tx_packets(&mut packets);
        } else {
            //println!("HANDLE RX");
            device.add_rx_buffers(&mut packets, &mut collect);
        }

        device.get_received_packets(&mut packets);

        // This 0 here is the number of packets received
        Ok(Ok((0usize, packets, collect)))
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

    // let new_net = net.clone_net();

    unsafe {
        net.0.lock().init();
    }

    // Run SmolNet
    let mut smol = SmolPhy::new(Box::new(net));

    loop {
        smol.do_rx();
        smol.do_tx();

        libtime::sys_ns_sleep(10_000_000_000);
    }

    // // let mut neighbor_cache_entries = [None; 8];
    // let neighbor_cache = NeighborCache::new(BTreeMap::new());

    // let ip_addresses = [IpCidr::new(IpAddress::v4(10, 10, 1, 1), 24)];
    // let mac_address = [0x90, 0xe2, 0xba, 0xb3, 0xb9, 0x10];
    // let iface = EthernetInterfaceBuilder::new(smol)
    //     .ethernet_addr(EthernetAddress::from_bytes(&mac_address))
    //     .neighbor_cache(neighbor_cache)
    //     .ip_addrs(ip_addresses)
    //     .finalize();

    // let socketset = SocketSet::new(Vec::with_capacity(512));

    // new_net.unwrap()
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
