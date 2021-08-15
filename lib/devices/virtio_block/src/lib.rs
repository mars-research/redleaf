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

use core::{panic, u16, usize};

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use console::println;
use hashbrown::HashMap;
use interface::bdev::BlkReq;
use interface::rref::{RRef, RRefDeque};
use libtime;
use spin::Mutex;
use virtio_device::defs::{
    VirtQueue, VirtqAvailable, VirtqAvailablePacked, VirtqDescriptor, VirtqUsed, VirtqUsedElement,
    VirtqUsedPacked,
};
use virtio_device::{Mmio, VirtioDeviceStatus};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]

struct BlockBufferHeader {
    /// IN: 0, OUT: 1, FLUSH: 4, DISCARD: 11, WRITE_ZEROES: 13
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct BlockBufferStatus {
    /// OK: 0, IOERR: 1, UNSUPP: 2
    pub status: u8,
}

pub struct VirtioBlockInner {
    mmio: Mmio,

    /// The size of the virtio virtual queues (also known as descriptor count)
    queue_size: u16,

    /// The number of buffers that can be in the queues at any time.
    /// For Virtio Block, it takes 3 descriptors for every buffer (block request),
    /// so `buffer_count = queue_size / 3`
    buffer_count: usize,

    request_queue: Option<VirtQueue>,

    /// Tracks which descriptors on the queue are free
    free_descriptors: Vec<bool>,

    /// The last index (of the used ring) that was checked by the driver
    request_last_idx: u16,

    // rx_buffers: HashMap<u64, RRef<NetworkPacketBuffer>>,
    block_status: Vec<BlockBufferStatus>,
    block_headers: Vec<BlockBufferHeader>,

    /// Holds the buffers for requests. The key is the their address
    request_buffers: Vec<Option<RRef<BlkReq>>>,
}

impl VirtioBlockInner {
    /// Returns an initialized VirtioBlock from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        Self {
            mmio: Mmio::new(mmio_base),

            queue_size: 0,
            buffer_count: 0,

            request_queue: None,
            free_descriptors: vec![],
            request_last_idx: 0,
            block_status: vec![],
            block_headers: vec![],
            request_buffers: vec![],
        }
    }

    pub fn init(&mut self) {
        println!("Initializing Virtio Block Device");

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
            panic!("Failed to negotiate Virtio Block features!");
        }

        // Figure out queue_size
        self.queue_size = self.mmio.accessor.read_queue_size();
        self.buffer_count = (self.queue_size / 3) as usize;

        unsafe {
            self.setup_virtual_queue();
        }
        self.initialize_vectors();

        // Setup Virtual Queues
        self.initialize_virtual_queue(0, &(self.request_queue.as_ref().unwrap()));

        // Tell the Device we're all done, even though we aren't (init must be called)
        unsafe { self.mmio.update_device_status(VirtioDeviceStatus::DriverOk) };

        // self.mmio.accessor.write_queue_select(0);
        // self.print_device_config();

        println!("VIRTIO BLOCK READY!");
    }

    fn negotiate_features(&mut self) {
        self.mmio.accessor.write_device_feature_select(0);
        self.mmio.accessor.write_driver_feature_select(0);

        let features = self.mmio.accessor.read_device_feature();
        // println!("DEVICE FEATURES: {:}", &features);

        if features & (1 << 5) != 0 {
            println!("VIRTIO DEVICE IS READ ONLY!");
        }

        // self.mmio.accessor.write_driver_feature(0);
    }

    fn print_device_config(&self) {
        let cfg = unsafe { self.mmio.read_common_config() };
        println!("{:#?}", cfg);
    }

    fn initialize_vectors(&mut self) {
        self.free_descriptors = vec![true; self.buffer_count];
        self.block_headers = vec![
            BlockBufferHeader {
                request_type: 0xFF,
                reserved: 0,
                sector: 0
            };
            self.buffer_count
        ];
        self.block_status = vec![BlockBufferStatus { status: 0xFF }; self.buffer_count];
        self.request_buffers = Vec::with_capacity(self.buffer_count);
        self.request_buffers.resize_with(self.buffer_count, || None); // Fill with None
    }

    unsafe fn setup_virtual_queue(&mut self) {
        self.request_queue = Some(VirtQueue {
            descriptors: vec![VirtqDescriptor::default(); self.queue_size as usize],
            available: VirtqAvailable::new(self.queue_size),
            used: VirtqUsed::new(self.queue_size),
        });
    }

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

        Mmio::memory_fence();
        self.mmio.accessor.write_queue_enable(1);
    }

    /// Returns a free descriptor chain index
    /// For Virtio Block, the header is placed at i, the buffer at i + self.buffer_count and the status at i + 2 * self.buffer_count
    fn get_free_idx(&mut self) -> Result<usize, ()> {
        for i in 0..self.free_descriptors.len() {
            if self.free_descriptors[i] {
                self.free_descriptors[i] = false;
                return Ok(i);
            }
        }

        return Err(());
    }

    #[inline]
    fn get_addr<T>(obj: &T) -> u64 {
        (obj as *const T) as u64
    }

    /// Frees processed requests, returns the number of processed requests
    pub fn free_request_buffers(&mut self, collect: &mut RRefDeque<BlkReq, 128>) -> usize {
        let mut freed_count = 0;

        let queue = &mut self.request_queue.as_mut().unwrap();

        while self.request_last_idx != queue.used.data.idx {
            let used_element = queue.used.ring(self.request_last_idx % self.queue_size);

            let header_idx = used_element.id as usize;
            let buffer_header_desc = &queue.descriptors[header_idx];
            // let buffer_data_desc = &queue.descriptors[buffer_header_desc.next as usize];

            if let Some(buffer) = self.request_buffers[header_idx].take() {
                if self.block_status[header_idx].status != 0 {
                    // Panic on failed requests. Not ideal, but such is life.
                    println!(
                        "IDX: {}, Used IDX: {}, Block Status: {:#X} (Default: 0xFF), Block Sector: {}, Block Data: {:?}",
                        self.request_last_idx, queue.used.data.idx,
                        &self.block_status[header_idx].status,
                        &self.block_headers[header_idx].sector,
                        &buffer.data[0..20]
                    );
                    panic!("ERROR: VIRTIO BLOCK: Block Request Failed with IO ERROR.");
                }

                collect.push_back(buffer);
                freed_count += 1;

                // Free the descriptor
                self.free_descriptors[header_idx] = true;
            } else {
                panic!("ERROR: VIRTIO BLOCK: REQUEST BUFFER MISSING BEFORE RELEASE!");
            }

            self.request_last_idx = self.request_last_idx.wrapping_add(1);
        }

        freed_count
    }

    pub fn submit_request(
        &mut self,
        block_request: RRef<BlkReq>,
        write: bool,
    ) -> Result<(), RRef<BlkReq>> {
        if let Ok(header_idx) = self.get_free_idx() {
            let queue = self.request_queue.as_mut().unwrap();

            self.block_headers[header_idx] = BlockBufferHeader {
                request_type: if write { 1 } else { 0 },
                reserved: 0,
                // sector: block_request.block * 8, // Data length is 4096, have to multiply by 8
                sector: block_request.block,
            };
            self.block_status[header_idx] = BlockBufferStatus { status: 0xFF };

            let buffer_idx = header_idx + self.buffer_count;
            let status_idx = buffer_idx + self.buffer_count;

            // Add the descriptors
            // Header
            queue.descriptors[header_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.block_headers[header_idx]),
                len: core::mem::size_of::<BlockBufferHeader>() as u32,
                flags: 1,
                next: buffer_idx as u16,
            };

            // Buffer
            queue.descriptors[buffer_idx] = VirtqDescriptor {
                addr: block_request.data.as_ptr() as u64,
                len: 4096,
                flags: if write { 1 } else { 1 | 2 },
                next: status_idx as u16,
            };

            // Status
            queue.descriptors[status_idx] = VirtqDescriptor {
                addr: Self::get_addr(&self.block_status[header_idx]),
                len: core::mem::size_of::<BlockBufferStatus>() as u32,
                flags: 2,
                next: 0,
            };

            self.request_buffers[header_idx] = Some(block_request);

            *queue
                .available
                .ring(queue.available.data.idx % self.queue_size) = header_idx as u16;
            queue.available.data.idx = queue.available.data.idx.wrapping_add(1);
            unsafe {
                self.mmio.queue_notify(0, 0);
            }

            Ok(())
        } else {
            // println!("Virtio Block: No free descriptors, request dropped");
            Err(block_request)
        }
    }
}
