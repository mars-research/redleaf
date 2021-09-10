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

use alloc::alloc::{alloc, Layout};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use console::println;
use core::mem::size_of;
use core::ptr::{read_volatile, write_volatile};
use core::usize;
use hashbrown::HashMap;
use interface::rref::{RRef, RRefDeque};
use libsyscalls::syscalls::sys_yield;
use spin::Mutex;
use virtio_backend_trusted::defs::DeviceNotificationType;
use virtio_backend_trusted::device_notify;
use virtio_device::defs::{
    VirtQueue, VirtqAvailable, VirtqAvailablePacked, VirtqDescriptor, VirtqUsed, VirtqUsedElement,
    VirtqUsedPacked,
};
use virtio_device::{Mmio, VirtioDeviceStatus};

#[repr(C, packed)]
pub struct VirtioNetworkDeviceConfig {
    mac: [u8; 6],
    status: u16,
    // Not available without negotiating features VIRTIO_NET_F_MQ and VIRTIO_NET_F_MTU
    // max_virtqueue_pairs: u16,
    // mtu: u16,
}

#[derive(Debug, Clone, Copy)]
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

struct VirtualQueues {
    receive_queue: VirtQueue,
    transmit_queue: VirtQueue,
}

type NetworkPacketBuffer = [u8; 1514];

pub struct VirtioNetInner {
    mmio: Mmio,
    virtual_queues: Option<VirtualQueues>, // None until init() is called

    /// This is the size of the queues used by the device, it is read during init().
    /// It would be less annoying to use if it were usize but it truly is a u16 value.
    queue_size: u16,

    /// This tracks the maximum number of buffers or descriptor chains we can simultaneiously have.
    /// For the network driver, each network packet requires two descriptors so this will be
    /// queue_size / 2.
    buffer_count: usize,

    /// Dummy VirtioNetHeaders.
    /// The driver doesn't actually use these but they are required by the spec
    virtio_network_headers: Vec<VirtioNetworkHeader>,

    /// Tracks which descriptors on the queue are free
    rx_free_descriptors: Vec<bool>,
    /// Tracks which descriptors on the queue are free
    tx_free_descriptors: Vec<bool>,

    /// The last index (of the used ring) that was checked by the driver
    rx_last_idx: u16,
    /// The last index (of the used ring) that was checked by the driver
    tx_last_idx: u16,

    rx_buffers: Vec<Option<RRef<NetworkPacketBuffer>>>,
    tx_buffers: Vec<Option<RRef<NetworkPacketBuffer>>>,
}

impl VirtioNetInner {
    /// Returns an initialized VirtioNet from a base address.
    pub unsafe fn new(mmio_base: usize) -> Self {
        Self {
            mmio: Mmio::new(mmio_base),

            queue_size: 0, // We will update this (and the vecs) in init()
            buffer_count: 0,

            virtual_queues: None,

            virtio_network_headers: vec![],

            rx_free_descriptors: vec![],
            tx_free_descriptors: vec![],

            rx_buffers: vec![],
            tx_buffers: vec![],

            rx_last_idx: 0,
            tx_last_idx: 0,
        }
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

        // Reset the device
        // Failing to do this DOES cause errors, don't ask how I know *sigh*
        unsafe {
            self.mmio.accessor.write_device_status(0);
        }
        Mmio::memory_fence();

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
            panic!("Failed to negotiate Virtio Net features!");
        }

        // Read the queue size
        // This value is that largest possible queue size so we will use it to initialize all of our vectors
        let queue_size = self.mmio.accessor.read_queue_size();
        if queue_size == 0 {
            panic!("ERROR: VIRTIO NET: BAD QUEUE SIZE!");
        }

        self.queue_size = queue_size;
        self.buffer_count = (self.queue_size / 2) as usize; // Each buffer requires two descriptors

        unsafe {
            self.setup_virtual_queues();
        }

        self.initialize_vectors();

        // Setup Virtual Queues
        self.initialize_virtual_queue(0, &(self.virtual_queues.as_ref().unwrap().receive_queue));

        println!("Should call device_notify");
        device_notify(DeviceNotificationType::DeviceConfigurationUpdated);

        self.initialize_virtual_queue(1, &(self.virtual_queues.as_ref().unwrap().transmit_queue));

        device_notify(DeviceNotificationType::DeviceConfigurationUpdated);

        // Tell the Device we're all done, even though we aren't
        unsafe { self.mmio.update_device_status(VirtioDeviceStatus::DriverOk) };
        device_notify(DeviceNotificationType::DeviceConfigurationUpdated);

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

    /// Initializes all the vectors using the set buffer_count
    fn initialize_vectors(&mut self) {
        self.virtio_network_headers = vec![
            VirtioNetworkHeader {
                csum_offset: 0,
                csum_start: 0,
                flags: 0,
                gso_size: 0,
                gso_type: 0,
                header_length: 0,
            };
            self.buffer_count
        ];

        self.rx_free_descriptors = vec![true; self.buffer_count];
        self.tx_free_descriptors = vec![true; self.buffer_count];

        self.rx_buffers = Vec::with_capacity(self.buffer_count);
        self.rx_buffers.resize_with(self.buffer_count, || None);
        self.tx_buffers = Vec::with_capacity(self.buffer_count);
        self.tx_buffers.resize_with(self.buffer_count, || None);
    }

    unsafe fn setup_virtual_queues(&mut self) {
        self.virtual_queues = Some(VirtualQueues {
            receive_queue: VirtQueue {
                descriptors: vec![VirtqDescriptor::default(); self.queue_size as usize],
                available: VirtqAvailable::new(self.queue_size),
                used: VirtqUsed::new(self.queue_size),
            },
            transmit_queue: VirtQueue {
                descriptors: vec![VirtqDescriptor::default(); self.queue_size as usize],
                available: VirtqAvailable::new(self.queue_size),
                used: VirtqUsed::new(self.queue_size),
            },
        });
    }

    /// Receive Queues must be 2*N and Transmit Queues must be 2*N + 1
    /// For example, Receive Queue must be 0 and Transmit Queue must be 1
    fn initialize_virtual_queue(&self, queue_index: u16, virt_queue: &VirtQueue) {
        self.mmio.accessor.write_queue_select(queue_index);

        self.mmio
            .accessor
            .write_queue_desc(virt_queue.descriptors.as_ptr() as u64);
        self.mmio.accessor.write_queue_driver(
            (virt_queue.available.data.as_ref() as *const VirtqAvailablePacked) as u64,
        );
        self.mmio
            .accessor
            .write_queue_device((virt_queue.used.data.as_ref() as *const VirtqUsedPacked) as u64);
        self.mmio.accessor.write_queue_enable(1);
    }

    /// Returns a free descriptor chain index
    /// For Virtio Net, the VirtioNetworkHeader is placed at i and the Packet Buffer will be placed at i + self.buffer_count
    fn get_free_idx(free_buffers: &mut Vec<bool>) -> Result<usize, ()> {
        for i in 0..free_buffers.len() {
            if free_buffers[i] {
                free_buffers[i] = false;
                return Ok(i);
            }
        }

        return Err(());
    }

    #[inline]
    fn get_addr<T>(obj: &T) -> u64 {
        (obj as *const T) as u64
    }

    /// If the buffer can't be added, it is returned in the Err()
    fn add_rx_buffer(
        &mut self,
        buffer: RRef<NetworkPacketBuffer>,
    ) -> Result<(), RRef<NetworkPacketBuffer>> {
        let rx_q = &mut self.virtual_queues.as_mut().unwrap().receive_queue;

        if let Ok(header_idx) = Self::get_free_idx(&mut self.rx_free_descriptors) {
            let buffer_idx = header_idx + self.buffer_count;
            let buffer_addr = buffer.as_ptr() as u64;

            // Store it so it isn't dropped
            self.rx_buffers[header_idx] = Some(buffer);

            // One descriptor points at the network header, chain this with a descriptor to the buffer
            // Header
            rx_q.descriptors[header_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
                len: core::mem::size_of::<VirtioNetworkHeader>() as u32, // 10 bytes
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
            *rx_q
                .available
                .ring(rx_q.available.data.idx % self.queue_size) = header_idx as u16;
            rx_q.available.data.idx = rx_q.available.data.idx.wrapping_add(1); // We only added one "chain head"

            // unsafe {
            //     self.mmio.queue_notify(0, 0);
            // }

            device_notify(DeviceNotificationType::QueueUpdated);

            return Ok(());
        } else {
            return Err(buffer);
        }
    }

    pub fn add_rx_buffers(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
        collect: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) {
        if packets.len() == 0 {
            return;
        }

        while let Some(buffer) = packets.pop_front() {
            let res = self.add_rx_buffer(buffer);

            if res.is_err() {
                packets.push_back(res.unwrap_err());
                break;
            }
        }
    }

    /// Returns an error if there's no free space in the TX queue, Ok otherwise
    fn add_tx_packet(
        &mut self,
        buffer: RRef<NetworkPacketBuffer>,
    ) -> Result<(), RRef<NetworkPacketBuffer>> {
        let tx_q = &mut self.virtual_queues.as_mut().unwrap().transmit_queue;

        if let Ok(header_idx) = Self::get_free_idx(&mut self.tx_free_descriptors) {
            let buffer_idx = header_idx + self.buffer_count;
            let buffer_addr = buffer.as_ptr() as u64;

            // Store it so it isn't dropped
            self.tx_buffers[header_idx] = Some(buffer);

            tx_q.descriptors[header_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.virtio_network_headers[header_idx]),
                len: core::mem::size_of::<VirtioNetworkHeader>() as u32, // 10 bytes
                flags: 1,                                                // 1 is next flag
                next: buffer_idx as u16,
            };
            tx_q.descriptors[buffer_idx] = VirtqDescriptor {
                addr: buffer_addr,
                len: 1514,
                flags: 0,
                next: 0,
            };

            *tx_q
                .available
                .ring(tx_q.available.data.idx % self.queue_size) = header_idx as u16;
            tx_q.available.data.idx = tx_q.available.data.idx.wrapping_add(1);

            unsafe {
                self.mmio.queue_notify(1, 1);
            }
            return Ok(());
        } else {
            return Err(buffer);
        }
    }

    pub fn add_tx_buffers(&mut self, packets: &mut RRefDeque<NetworkPacketBuffer, 32>) {
        if packets.len() == 0 {
            return;
        }

        while let Some(packet) = packets.pop_front() {
            let res = self.add_tx_packet(packet);

            if res.is_err() {
                println!("ERROR: VIRTIO NET: COULD NOT ADD TX PACKET. NO FREE SPACE!");
                packets.push_back(res.unwrap_err());
                break;
            }
        }
    }

    /// Adds new packets to `packets`. Returns the number of received packets
    /// Returns the number of new packets found
    pub fn get_received_packets(
        &mut self,
        collect: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) -> usize {
        /// We have to return the number of packets received
        let mut new_packets_count = 0;
        let rx_q = &mut self.virtual_queues.as_mut().unwrap().receive_queue;

        while self.rx_last_idx != rx_q.used.data.idx {
            let used_element = rx_q.used.ring(self.rx_last_idx % self.queue_size);
            let header_descriptor = &rx_q.descriptors[used_element.id as usize];
            let buffer_descriptor = &rx_q.descriptors[header_descriptor.next as usize];

            if let Some(buffer) = self.rx_buffers[used_element.id as usize].take() {
                // Processed packets are "collected"
                collect.push_back(buffer);
                new_packets_count += 1;

                // Free the descriptor
                self.rx_free_descriptors[used_element.id as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: RX BUFFER MISSING");
            }

            self.rx_last_idx = self.rx_last_idx.wrapping_add(1);
        }

        new_packets_count
    }

    /// Returns the number of tx packets that have been sent
    pub fn free_processed_tx_packets(
        &mut self,
        packets: &mut RRefDeque<NetworkPacketBuffer, 32>,
    ) -> usize {
        let mut freed_count = 0;
        let tx_q = &mut self.virtual_queues.as_mut().unwrap().transmit_queue;

        while self.tx_last_idx != tx_q.used.data.idx {
            let used_element = tx_q.used.ring(self.tx_last_idx % self.queue_size);
            let header_descriptor = &tx_q.descriptors[used_element.id as usize];
            let buffer_descriptor = &tx_q.descriptors[header_descriptor.next as usize];

            if let Some(buffer) = self.tx_buffers[used_element.id as usize].take() {
                packets.push_back(buffer);
                freed_count += 1;

                // Free the descriptor
                self.tx_free_descriptors[used_element.id as usize] = true;
            } else {
                println!("ERROR: VIRTIO NET: TX BUFFER MISSING");
            }

            self.tx_last_idx = self.tx_last_idx.wrapping_add(1);
        }

        freed_count
    }
}
