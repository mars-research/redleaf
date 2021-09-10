use virtio_device::defs::{VirtQueue, VirtqAvailablePacked, VirtqUsedPacked};

#[derive(Debug)]
pub struct VirtioQueueConfig {
    pub queue_index: u16,
    pub queue_size: u16,
    pub queue_enable: bool,
    pub queue_descriptor: u64,
    pub queue_driver: u64,
    pub queue_device: u64,

    /// The next idx to process for the driver queue
    pub driver_idx: u16,

    /// The next idx to process for the device queue
    pub device_idx: u16,
}

impl VirtioQueueConfig {
    pub fn get_driver_queue(&mut self) -> &mut VirtqAvailablePacked {
        unsafe {
            return &mut *(self.queue_driver as *mut VirtqAvailablePacked);
        }
    }

    pub fn get_device_queue(&mut self) -> &mut VirtqUsedPacked {
        unsafe {
            return &mut *(self.queue_driver as *mut VirtqUsedPacked);
        }
    }
}
