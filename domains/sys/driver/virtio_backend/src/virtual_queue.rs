use core::{mem, ptr::read_volatile};

use alloc::{collections::VecDeque, slice, vec::Vec};
use hashbrown::HashMap;
use virtio_backend_trusted::defs::{BufferPtr, BATCH_SIZE, SHARED_MEMORY_REGION_PTR};
use virtio_device::defs::{
    VirtQueue, VirtqAvailablePacked, VirtqDescriptor, VirtqUsedElement, VirtqUsedPacked,
};

#[derive(Debug)]
pub struct DescriptorQueue {
    address: *mut VirtqDescriptor,
    queue_size: u16,
}

impl DescriptorQueue {
    pub fn new(address: *mut VirtqDescriptor, queue_size: u16) -> Self {
        Self {
            address,
            queue_size,
        }
    }

    pub fn get_descriptors(&self) -> &mut [VirtqDescriptor] {
        unsafe { slice::from_raw_parts_mut(self.address, self.queue_size as usize) }
    }

    pub fn queue_size(&self) -> u16 {
        self.queue_size
    }
}
#[derive(Debug)]
pub struct DeviceQueue {
    address: *mut VirtqUsedPacked,
    queue_size: u16,
}

impl DeviceQueue {
    pub fn new(address: *mut VirtqUsedPacked, queue_size: u16) -> Self {
        Self {
            address,
            queue_size,
        }
    }

    pub fn idx(&mut self) -> &mut u16 {
        unsafe { &mut (*self.address).idx }
    }

    pub fn ring(&mut self, idx: u16) -> &mut VirtqUsedElement {
        assert!(idx < self.queue_size);

        unsafe { (*self.address).ring(idx) }
    }
}
#[derive(Debug)]
pub struct DriverQueue {
    address: *mut VirtqAvailablePacked,
    queue_size: u16,

    pub previous_idx: u16,
}

impl DriverQueue {
    pub fn new(address: *mut VirtqAvailablePacked, queue_size: u16) -> Self {
        Self {
            address,
            queue_size,

            previous_idx: 0,
        }
    }

    pub fn idx(&self) -> &u16 {
        unsafe { &(*self.address).idx }
    }

    pub fn ring(&mut self, idx: u16) -> &u16 {
        assert!(idx < self.queue_size);

        unsafe { (*self.address).ring(idx) }
    }
}
#[derive(Debug)]
pub struct VirtualQueue {
    pub descriptor_queue: DescriptorQueue,
    pub driver_queue: DriverQueue,
    pub device_queue: DeviceQueue,

    queue_size: u16,
    rx_queue: bool,

    // Variables used for efficiency
    /// Holds the pointers to buffers that either need to be given to the device (rx queue) or given to the frontend (tx queue)
    buffer_deque: VecDeque<BufferPtr>,

    /// Maps the buffer's ptr to the chain_header_idx
    buffer_map: HashMap<u64, u16>,
}

impl VirtualQueue {
    /// rx_queue should be true if the queue has an even index, false otherwise
    pub fn new(
        descriptor_queue_address: u64,
        device_queue_address: u64,
        driver_queue_address: u64,
        queue_size: u16,
        rx_queue: bool,
    ) -> Self {
        Self {
            descriptor_queue: DescriptorQueue::new(
                descriptor_queue_address as *mut VirtqDescriptor,
                queue_size,
            ),
            device_queue: DeviceQueue::new(
                device_queue_address as *mut VirtqUsedPacked,
                queue_size,
            ),
            driver_queue: DriverQueue::new(
                driver_queue_address as *mut VirtqAvailablePacked,
                queue_size,
            ),

            queue_size,
            rx_queue,

            buffer_deque: VecDeque::with_capacity(BATCH_SIZE),
            buffer_map: HashMap::new(),
        }
    }

    pub fn is_rx_queue(&self) -> bool {
        self.rx_queue
    }

    fn assert_is_rx(&self) {
        assert!(
            self.rx_queue,
            "This function should only be called if the queue is an rx_queue (even index)!"
        );
    }

    fn assert_is_tx(&self) {
        assert!(
            !self.rx_queue,
            "This function should only be called if the queue is an tx_queue (odd index)!"
        );
    }

    pub fn fetch_new_buffers(&mut self) -> &mut VecDeque<BufferPtr> {
        while self.driver_queue.previous_idx != *self.driver_queue.idx()
            && self.buffer_deque.len() < BATCH_SIZE
        {
            let idx = (self.driver_queue.previous_idx % self.queue_size);
            let chain_header_idx = *self.driver_queue.ring(idx);

            let descriptors = self.descriptor_queue.get_descriptors();
            let mut current_idx: usize = chain_header_idx.into();

            // Find the descriptor with the correct length
            loop {
                let descriptor = descriptors[current_idx];

                if descriptor.len == 1514 {
                    // Add it to the device and break

                    // descriptor.addr is an offset, convert it to an address
                    let addr = (unsafe { *SHARED_MEMORY_REGION_PTR } as usize
                        + descriptor.addr as usize) as u64;

                    self.buffer_deque.push_back(addr as BufferPtr);
                    self.buffer_map.insert(addr, chain_header_idx);
                    break;
                } else {
                    // Try the next descriptor
                    if (descriptor.flags & 0b1) == 0b1 {
                        current_idx = descriptor.next.into();
                    } else {
                        break;
                    }
                }
            }

            // Move to the next chain
            self.driver_queue.previous_idx = self.driver_queue.previous_idx.wrapping_add(1);
        }

        return &mut self.buffer_deque;
    }

    pub fn mark_buffers_as_complete(&mut self, buffers: &[BufferPtr]) {
        for buffer in buffers {
            // Look up the chain header idx
            let buffer_key = (*buffer) as u64;
            if let Some(chain_header_idx) = self.buffer_map.remove(&buffer_key) {
                // Mark that chain as complete
                let idx = *self.device_queue.idx();
                *self.device_queue.ring(idx % self.queue_size) = VirtqUsedElement {
                    id: chain_header_idx.into(),
                    len: 1514,
                };

                // Update the idx
                *self.device_queue.idx() = idx.wrapping_add(1);
            } else {
                panic!(
                    "Buffer Address Changed during processing! FAILED ADDR: {:#?}, MAP: {:#?}",
                    buffer, self.buffer_map
                );
            }
        }
    }
}
