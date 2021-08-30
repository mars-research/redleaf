use core::mem::size_of;
use virtio_device::VirtioPciCommonConfig;

pub const MAX_SUPPORTED_QUEUES: u16 = 3;

pub const MMIO_ADDRESS: *mut VirtioPciCommonConfig = 0x100000 as *mut VirtioPciCommonConfig;
pub const DEVICE_NOTIFY: *mut usize = (0x100000 - size_of::<usize>()) as *mut usize;

pub struct VirtioMMIOConfiguration {
    pub configuration_address: usize,
}

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
