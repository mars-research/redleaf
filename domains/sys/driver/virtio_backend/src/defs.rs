use virtio_device::defs::VirtQueue;

pub const MAX_SUPPORTED_QUEUES: u16 = 3;

#[derive(Debug)]
pub struct VirtioBackendQueue {
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

impl VirtioBackendQueue {
    pub fn get_driver_queue(&mut self) -> &mut VirtQueue {
        unsafe {
            return &mut *(self.queue_driver as *mut VirtQueue);
        }
    }
}
