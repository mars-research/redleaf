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

use alloc::sync::Arc;
use console::println;
use hashbrown::HashMap;
use rref::{RRef, RRefDeque};
use spin::Mutex;
use virtio_device::defs::{
    VirtQueue, VirtqAvailable, VirtqDescriptor, VirtqUsed, VirtqUsedElement, VirtualQueues,
    DESCRIPTOR_COUNT,
};
use virtio_device::{Mmio, VirtioDeviceStatus};

#[derive(Debug)]
#[repr(C, packed)]
pub struct VirtioNetworkDeviceConfig {
    mac: [u8; 6],
    status: u16,
    // Not available without negotiating features VIRTIO_NET_F_MQ and VIRTIO_NET_F_MTU
    // max_virtqueue_pairs: u16,
    // mtu: u16,
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct VirtioNetworkHeader {
    pub flags: u8,
    pub gso_type: u8,
    pub header_length: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    // pub num_buffers: u16,
}

type NetworkPacketBuffer = [u8; 1514];

pub struct VirtioNet(Arc<Mutex<VirtioNetInner>>);

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
    tx_buffers: HashMap<u64, RRef<NetworkPacketBuffer>>,
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
            tx_buffers: HashMap::new(),
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

        println!("VIRTIO NET READY!");
    }

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
    }

    fn get_next_free_buffer(free_buffers: &mut [bool; DESCRIPTOR_COUNT]) -> Result<usize, ()> {
        for i in 0..DESCRIPTOR_COUNT {
            if free_buffers[i] {
                free_buffers[i] = false;
                return Ok(i);
            }
        }
        Err(())
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

        let buffer_addr = buffer.as_ptr() as u64;

        self.rx_buffers.insert(buffer_addr, buffer);

        // Header
        rx_q.descriptors[header_idx] = VirtqDescriptor {
            addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
            len: 10,
            // 1 is NEXT FLAG
            // 2 is WRITABLE FLAG
            flags: 1 | 2,
            next: buffer_idx as u16,
        };
        // Actual Buffer
        rx_q.descriptors[buffer_idx] = VirtqDescriptor {
            addr: buffer_addr,
            len: 1514,
            flags: 2,
            next: 0,
        };

        // Mark the buffer as usable
        rx_q.available.ring[(rx_q.available.idx as usize) % DESCRIPTOR_COUNT] = header_idx as u16;
        rx_q.available.idx += 1; // We only added one "chain head"
    }

    fn add_rx_buffers(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
        collect: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) {
        let mut added_buffers = false;

        while let Some(buffer) = packets.pop_front() {
            if (self.rx_free_descriptor_count >= 2) {
                added_buffers = true;
                self.add_rx_buffer(buffer);
            } else {
                // println!("RX QUEUE Full");
                packets.push_back(buffer);
                break;
            }
        }

        unsafe {
            if added_buffers {
                self.mmio.queue_notify(0, 0);
            }
        }
    }

    fn add_tx_packet(&mut self, buffer: RRef<NetworkPacketBuffer>) -> Result<(), ()> {
        let header_idx = Self::get_next_free_buffer(&mut self.tx_free_descriptors);
        let buffer_idx = Self::get_next_free_buffer(&mut self.tx_free_descriptors);

        if header_idx.is_err() || buffer_idx.is_err() {
            println!("TX: NO ROOM!");
            return Err(());
        }

        let header_idx = header_idx.unwrap();
        let buffer_idx = buffer_idx.unwrap();

        let buffer_addr = buffer.as_ptr() as u64;

        // Add the buffer to our HashMap
        self.tx_buffers.insert(buffer_addr, buffer);

        self.virtual_queues.transmit_queue.descriptors[header_idx] = VirtqDescriptor {
            addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
            len: 10,
            flags: 1, // 1 is next flag
            next: (buffer_idx as u16),
        };
        self.virtual_queues.transmit_queue.descriptors[buffer_idx] = VirtqDescriptor {
            addr: buffer_addr,
            len: 1514,
            flags: 0,
            next: 0,
        };

        self.virtual_queues.transmit_queue.available.ring
            [(self.virtual_queues.transmit_queue.available.idx as usize) % DESCRIPTOR_COUNT] =
            (header_idx as u16);
        self.virtual_queues.transmit_queue.available.idx += 1;

        // println!("ADDED TX BUFFER {:} {:}", header_idx, buffer_idx);

        Ok(())
    }

    fn add_tx_buffers(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        if packets.len() == 0 {
            return;
        }

        while let Some(packet) = packets.pop_front() {
            self.add_tx_packet(packet);
        }

        unsafe {
            self.mmio.queue_notify(1, 1);
        }
    }

    /// Adds new packets to `packets`. Returns the number of added packets
    fn get_received_packets(&mut self, collect: &mut RRefDeque<NetworkPacketBuffer, 32>) -> usize {
        /// We have to return the number of packets received
        let mut new_packets_count = 0;

        while self.rx_last_idx < self.virtual_queues.receive_queue.used.idx {
            println!("NEW PACKET: RX IDX: {:}", self.rx_last_idx);

            let used_element = self.virtual_queues.receive_queue.used.ring
                [(self.rx_last_idx as usize) % DESCRIPTOR_COUNT];
            let used_element_descriptor =
                self.virtual_queues.receive_queue.descriptors[used_element.id as usize];
            let buffer_descriptor = self.virtual_queues.receive_queue.descriptors
                [used_element_descriptor.next as usize];

            if let Some(buffer) = self.rx_buffers.remove(&buffer_descriptor.addr) {
                // Processed packets are "collected"
                collect.push_back(buffer);
                new_packets_count += 1;

                // Free the descriptor
                self.rx_free_descriptor_count += 2;
                self.rx_free_descriptors[used_element.id as usize] = true;
                self.rx_free_descriptors[used_element_descriptor.next as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: RX BUFFER MISSING OR BUFFER ADDRESS CHANGED!");
            }

            self.rx_last_idx += 1;
        }

        new_packets_count
    }

    fn free_processed_tx_packets(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) -> usize {
        let mut freed_count = 0;

        while self.tx_last_idx < self.virtual_queues.transmit_queue.used.idx {
            let used_element = self.virtual_queues.transmit_queue.used.ring
                [(self.tx_last_idx as usize) % DESCRIPTOR_COUNT];
            let used_element_descriptor =
                self.virtual_queues.transmit_queue.descriptors[used_element.id as usize];
            let buffer_descriptor = self.virtual_queues.transmit_queue.descriptors
                [used_element_descriptor.next as usize];

            if let Some(buffer) = self.tx_buffers.remove(&buffer_descriptor.addr) {
                packets.push_back(buffer);

                // println!("TX BUFFER SENT! {:}", self.tx_last_idx);
                // println!(
                //     "FREEING TX IDX {:} {:}",
                //     used_element.id, used_element_descriptor.next
                // );

                // Free the descriptor
                self.tx_free_descriptors[used_element.id as usize] = true;
                self.tx_free_descriptors[used_element_descriptor.next as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: TX BUFFER MISSING OR BUFFER ADDRESS CHANGED!");
            }

            freed_count += 1;
            self.tx_last_idx += 1;
        }

        freed_count
    }

    fn to_shared(self) -> VirtioNet {
        VirtioNet(Arc::new(Mutex::new(self)))
    }
}
