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

pub mod pci;
extern crate alloc;

use core::usize;

use alloc::sync::Arc;
use console::println;
use hashbrown::HashMap;
use interface::rref::{RRef, RRefDeque};
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

/// There are always 2 descriptors for every buffer
const BUFFER_COUNT: usize = DESCRIPTOR_COUNT / 2;

pub struct VirtioNetInner {
    mmio: Mmio,
    virtual_queues: VirtualQueues,

    /// Dummy VirtioNetHeaders
    virtio_network_headers: [VirtioNetworkHeader; BUFFER_COUNT],

    /// Tracks the number of free descriptors spaces remaining
    /// Note that this really tracks the header + buffer pair count
    /// So if this is 1 then there are technically 2 free descriptors since you need a pair to represent a buffer.
    rx_free_descriptor_count: usize,

    /// Tracks which descriptors on the queue are free.
    /// Header Descriptor will be at i and Buffer Descriptor will be at i + BUFFER_COUNT
    rx_free_descriptors: [bool; BUFFER_COUNT],

    /// Tracks which descriptors on the queue are free.
    /// Header Descriptor will be at i and Buffer Descriptor will be at i + BUFFER_COUNT
    tx_free_descriptors: [bool; BUFFER_COUNT],

    /// These numbers are an alternative to rx/tx_free_descriptors
    /// Instead of doing a linear scan for a free descriptor we will simply
    /// increment this number and wrap it once it reaches BUFFER_COUNT
    rx_next_header_idx: u8,
    tx_next_header_idx: u8,

    // The last index (of the used ring) that was checked by the driver
    rx_last_idx: u16,
    tx_last_idx: u16,

    /// Holds the rx_packets (to prevent dropping) while they are in the rx_queue. Stored at the chain's buffer idx
    rx_buffers: [Option<RRef<NetworkPacketBuffer>>; BUFFER_COUNT],
    tx_buffers: [Option<RRef<NetworkPacketBuffer>>; BUFFER_COUNT],
}

impl VirtioNetInner {
    /// Returns an initialized VirtioNet from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        let mmio = Mmio::new(mmio_base);

        let virtual_queues = VirtualQueues {
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

        let virtio_network_headers = [VirtioNetworkHeader {
            flags: 0,
            gso_type: 0,
            header_length: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
        }; BUFFER_COUNT];

        let virtio_inner = Self {
            mmio,
            virtual_queues,
            virtio_network_headers,

            rx_free_descriptor_count: BUFFER_COUNT,

            rx_free_descriptors: [true; BUFFER_COUNT],
            tx_free_descriptors: [true; BUFFER_COUNT],

            rx_next_header_idx: 0,
            tx_next_header_idx: 0,

            rx_last_idx: 0,
            tx_last_idx: 0,

            rx_buffers: [None; BUFFER_COUNT],
            tx_buffers: [None; BUFFER_COUNT],
        };

        // virtio_inner.init();
        // virtio_inner.testing();

        virtio_inner
    }

    pub fn init(&mut self) {
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
        unsafe {
            self.mmio
                .update_device_status(VirtioDeviceStatus::Acknowledge);
            self.mmio.update_device_status(VirtioDeviceStatus::Driver); // But do we really know how to drive the device?
        }

        self.negotiate_features();

        // Tell the Device that feature Negotiation is complete
        unsafe {
            self.mmio
                .update_device_status(VirtioDeviceStatus::FeaturesOk);
        }

        // Check that Features OK Bit is still set!
        // self.print_device_status();
        if (self.mmio.accessor.read_device_status() & VirtioDeviceStatus::FeaturesOk.value()) == 0 {
            panic!("Failed to negotiate virtio net features!");
        }

        // Configure queue_size in common configuration
        // self.mmio.accessor.write_queue_size(DESCRIPTOR_COUNT as u16);
        if self.mmio.accessor.read_queue_size() != DESCRIPTOR_COUNT as u16 {
            println!("ERROR: VIRTIO NET: The queue size does not match the expected value. There will be errors with the driver!");
        }

        // Setup Virtual Queues
        self.initialize_virtual_queue(0, &(self.virtual_queues.receive_queue));
        self.initialize_virtual_queue(1, &(self.virtual_queues.transmit_queue));

        // Tell the Device we're all done, even though we aren't
        unsafe { self.mmio.update_device_status(VirtioDeviceStatus::DriverOk) };

        // self.print_device_status();

        // self.mmio.accessor.write_queue_select(0);
        // self.print_device_config();
        // self.mmio.accessor.write_queue_select(1);
        // self.print_device_config();

        println!("VIRTIO NET READY!");
    }

    /// Negotiates Virtio Driver Features
    fn negotiate_features(&mut self) {
        let mut driver_features: u32 = 0;
        driver_features |= 1 << 5; // Enable Device MAC Address
        driver_features |= 1 << 16; // Enable Device Status

        self.mmio.accessor.write_driver_feature(driver_features); // Should be &'d with device_features
    }

    pub fn print_device_config(&mut self) {
        let mut cfg = unsafe { self.mmio.read_common_config() };
        println!("{:#?}", cfg);
    }

    pub fn print_device_status(&mut self) {
        let device_status = self.mmio.accessor.read_device_status();
        println!("Device Status Bits: {:b}", device_status);
    }

    /// Receive Queues must be 2*N and Transmit Queues must be 2*N + 1
    /// For example, Receive Queue must be 0 and Transmit Queue must be 1
    pub fn initialize_virtual_queue(&self, queue_index: u16, virt_queue: &VirtQueue) {
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

    pub fn infinite_rx(&mut self) {
        let BUFFERS = [[0x0u8; 1514]; 128];

        // Init Descriptors
        let rx_q = &mut self.virtual_queues.receive_queue;

        for i in 0..128 {
            let buffer_addr = BUFFERS[i].as_ptr() as u64;

            // One descriptor points at the network header, chain this with a descriptor to the buffer
            // Header
            rx_q.descriptors[i] = VirtqDescriptor {
                addr: Self::get_addr(&self.virtio_network_headers[i]),
                len: 10,
                // 1 is NEXT FLAG
                // 2 is WRITABLE FLAG
                flags: 1 | 2,
                next: (i + 128) as u16,
            };
            // Actual Buffer
            rx_q.descriptors[i + 128] = VirtqDescriptor {
                addr: buffer_addr,
                len: 1514,
                flags: 2,
                next: 0,
            };

            // Mark the buffer as usable
            rx_q.available.ring[i] = i as u16;
        }

        loop {
            rx_q.available.idx = rx_q.available.idx.wrapping_add(128); // We only added one "chain head"

            unsafe {
                self.mmio.queue_notify(0, 0);
            }
        }
    }

    pub fn infinite_tx(&mut self) {
        const BUFFER: [u8; 16] = [0xAA; 16];

        // Create a buffer
        let buffer_addr = BUFFER.as_ptr() as u64;

        self.virtual_queues.transmit_queue.descriptors[0] = VirtqDescriptor {
            addr: Self::get_addr(&self.virtio_network_headers[0]),
            len: 10,
            flags: 1, // 1 is next flag
            next: 1 as u16,
        };
        self.virtual_queues.transmit_queue.descriptors[1] = VirtqDescriptor {
            addr: buffer_addr,
            len: 16,
            flags: 0,
            next: 0,
        };

        for i in 0..DESCRIPTOR_COUNT {
            self.virtual_queues.transmit_queue.available.ring[i] = 0 as u16;
        }

        // Continually add packet
        loop {
            self.virtual_queues.transmit_queue.available.idx = self
                .virtual_queues
                .transmit_queue
                .available
                .idx
                .wrapping_add(128);
            unsafe {
                self.mmio.queue_notify(1, 1);
            }
        }
    }

    /// Will return an index between 0 and BUFFER_COUNT
    /// Place the header at i and the buffer at i + BUFFER_COUNT
    fn get_free_buffer_descriptor(free_buffers: &mut [bool; BUFFER_COUNT]) -> Result<usize, ()> {
        for i in 0..BUFFER_COUNT {
            if free_buffers[i] {
                free_buffers[i] = false;
                return Ok(i);
            }
        }
        Err(())
    }

    fn get_next_header_idx(next_idx: &mut u8) {
        let val = next_idx;
        next_idx += 1;

        if next_idx >= BUFFER_COUNT {
            next_idx = 0;
        }
        return val;
    }

    #[inline]
    fn get_addr<T>(obj: &T) -> u64 {
        (obj as *const T) as u64
    }

    pub fn add_rx_buffer(&mut self, buffer: RRef<NetworkPacketBuffer>) {
        if self.rx_free_descriptor_count < 1 {
            return;
        }

        let rx_q = &mut self.virtual_queues.receive_queue;

        if let Ok(header_idx) = Self::get_free_buffer_descriptor(&mut self.rx_free_descriptors) {
            self.rx_free_descriptor_count -= 1;

            let buffer_addr = buffer.as_ptr() as u64;
            self.rx_buffers[header_idx] = Some(buffer);

            // One descriptor points at the network header, chain this with a descriptor to the buffer
            // Header
            rx_q.descriptors[header_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
                len: core::mem::size_of::<VirtioNetworkHeader>() as u32,
                // 1 is NEXT FLAG
                // 2 is WRITABLE FLAG
                flags: 1 | 2,
                next: (header_idx + BUFFER_COUNT) as u16,
            };
            // Actual Buffer
            rx_q.descriptors[header_idx + BUFFER_COUNT] = VirtqDescriptor {
                addr: buffer_addr,
                len: 1514,
                flags: 2,
                next: 0,
            };

            // Mark the buffer as usable
            rx_q.available.ring[(rx_q.available.idx as usize) % DESCRIPTOR_COUNT] =
                header_idx as u16;
            rx_q.available.idx = rx_q.available.idx.wrapping_add(1); // We only added one "chain head"
        } else {
            println!("ERR: Virtio Net RX: Invariant failed, free descriptor count does not match number of free descriptors!");
        }
    }

    pub fn add_rx_buffers(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        let mut added_buffers = false;

        while self.rx_free_descriptor_count >= 1 && packets.len() > 0 {
            added_buffers = true;
            if let Some(buffer) = packets.pop_front() {
                self.add_rx_buffer(buffer);
            }
        }

        unsafe {
            if added_buffers {
                self.mmio.queue_notify(0, 0);
            }
        }
    }

    /// Returns an error if there's no free space in the TX queue, Ok otherwise
    pub fn add_tx_packet(&mut self, buffer: RRef<NetworkPacketBuffer>) -> Result<(), ()> {
        if let Ok(header_idx) = Self::get_free_buffer_descriptor(&mut self.tx_free_descriptors) {
            let buffer_addr = buffer.as_ptr() as u64;
            self.tx_buffers[header_idx] = Some(buffer);

            self.virtual_queues.transmit_queue.descriptors[header_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
                len: core::mem::size_of::<VirtioNetworkHeader>() as u32,
                flags: 1, // 1 is next flag
                next: (header_idx + BUFFER_COUNT) as u16,
            };
            self.virtual_queues.transmit_queue.descriptors[header_idx + BUFFER_COUNT] =
                VirtqDescriptor {
                    addr: buffer_addr,
                    len: 53,
                    flags: 0,
                    next: 0,
                };

            self.virtual_queues.transmit_queue.available.ring
                [(self.virtual_queues.transmit_queue.available.idx as usize) % DESCRIPTOR_COUNT] =
                header_idx as u16;
            self.virtual_queues.transmit_queue.available.idx = self
                .virtual_queues
                .transmit_queue
                .available
                .idx
                .wrapping_add(1);
        } else {
            println!("ERR: Virtio Net TX: No Free Buffers!");
            return Err(());
        }

        Ok(())
    }

    pub fn add_tx_buffers(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        if packets.len() == 0 {
            return;
        }

        while let Some(packet) = packets.pop_front() {
            let res = self.add_tx_packet(packet);

            if res.is_err() {
                println!("VIRTIO NET: FAILED TO ADD TX PACKET!");
                break;
            }
        }

        unsafe {
            self.mmio.queue_notify(1, 1);
        }
    }

    /// Adds new packets to `packets`. Returns the number of received packets
    pub fn get_received_packets(
        &mut self,
        collect: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) -> usize {
        /// We have to return the number of packets received
        let mut new_packets_count = 0;

        while self.rx_last_idx != self.virtual_queues.receive_queue.used.idx {
            let used_element = self.virtual_queues.receive_queue.used.ring
                [(self.rx_last_idx as usize) % DESCRIPTOR_COUNT];
            let header_descriptor =
                self.virtual_queues.receive_queue.descriptors[used_element.id as usize];
            let buffer_descriptor =
                self.virtual_queues.receive_queue.descriptors[header_descriptor.next as usize];

            // println!(
            //     "{}",
            //     used_element.len as usize - core::mem::size_of::<VirtioNetworkHeader>()
            // );
            // println!("{}", core::mem::size_of::<VirtioNetworkHeader>());

            if let Some(buffer) = self.rx_buffers[used_element.id as usize].take() {
                // Processed packets are "collected"
                collect.push_back(buffer);
                new_packets_count += 1;

                // Free the descriptor
                self.rx_free_descriptor_count += 1;
                self.rx_free_descriptors[used_element.id as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: RX BUFFER MISSING OR BUFFER ADDRESS CHANGED!");
            }

            self.rx_last_idx = self.rx_last_idx.wrapping_add(1);
        }

        new_packets_count
    }

    pub fn free_processed_tx_packets(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) -> usize {
        let mut freed_count = 0;

        while self.tx_last_idx != self.virtual_queues.transmit_queue.used.idx {
            let used_element = self.virtual_queues.transmit_queue.used.ring
                [(self.tx_last_idx as usize) % DESCRIPTOR_COUNT];
            let header_descriptor =
                self.virtual_queues.transmit_queue.descriptors[used_element.id as usize];
            let buffer_descriptor =
                self.virtual_queues.transmit_queue.descriptors[header_descriptor.next as usize];

            if let Some(buffer) = self.tx_buffers[used_element.id as usize].take() {
                packets.push_back(buffer);
                freed_count += 1;

                // Free the descriptor
                self.tx_free_descriptors[used_element.id as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: TX BUFFER MISSING OR BUFFER ADDRESS CHANGED!");
            }

            self.tx_last_idx = self.tx_last_idx.wrapping_add(1);
        }

        freed_count
    }
}
