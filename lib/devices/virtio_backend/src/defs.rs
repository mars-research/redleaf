use core::mem::size_of;
use virtio_device::{
    defs::{VirtqAvailablePacked, VirtqUsedPacked},
    VirtioPciCommonConfig,
};

pub const MAX_SUPPORTED_QUEUES: u16 = 2;
pub const BATCH_SIZE: usize = 32;

pub const MMIO_ADDRESS: *mut VirtioPciCommonConfig = 0x100000 as *mut VirtioPciCommonConfig;
pub const DEVICE_NOTIFY: *mut usize = (0x100000 - size_of::<usize>()) as *mut usize;
pub const SHARED_MEMORY_REGION_PTR: *mut *mut Buffer =
    (0x100000 + size_of::<VirtioPciCommonConfig>() + 0x1000) as *mut *mut Buffer;

pub type Buffer = [u8; 1514];
pub type BufferPtr = *const Buffer;

#[derive(Debug, PartialEq)]
pub enum DeviceNotificationType {
    None,
    DeviceConfigurationUpdated,
    QueueUpdated,
}

impl DeviceNotificationType {
    pub const fn value(&self) -> usize {
        match self {
            Self::None => 0,
            Self::DeviceConfigurationUpdated => 1,
            Self::QueueUpdated => 2,
        }
    }

    pub const fn from_value(value: usize) -> Self {
        match value {
            0 => Self::None,
            1 => Self::DeviceConfigurationUpdated,
            2 => Self::QueueUpdated,
            _ => Self::None,
        }
    }
}

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
