use core::{mem, ptr::read_volatile};

use alloc::{slice, vec::Vec};
use virtio_device::defs::{
    VirtQueue, VirtqAvailablePacked, VirtqDescriptor, VirtqUsedElement, VirtqUsedPacked,
};

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

pub struct DeviceQueue {
    address: *mut VirtqUsedPacked,
    queue_size: u16,

    pub previous_idx: u16,
}

impl DeviceQueue {
    pub fn new(address: *mut VirtqUsedPacked, queue_size: u16) -> Self {
        Self {
            address,
            queue_size,

            previous_idx: 0,
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

    pub fn idx(&mut self) -> &u16 {
        unsafe { &mut (*self.address).idx }
    }

    pub fn ring(&mut self, idx: u16) -> &u16 {
        assert!(idx < self.queue_size);

        unsafe { (*self.address).ring(idx) }
    }
}

pub struct VirtualQueues {
    pub descriptor_queue: DescriptorQueue,
    pub driver_queue: DriverQueue,
    pub device_queue: DeviceQueue,
}

impl VirtualQueues {
    pub fn new(
        descriptor_queue_address: u64,
        device_queue_address: u64,
        driver_queue_address: u64,
        queue_size: u16,
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
        }
    }
}
