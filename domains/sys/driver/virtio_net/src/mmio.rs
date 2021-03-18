use core::ptr;

use console::println;

#[derive(Debug)]
#[repr(C, packed)]
pub struct VirtioPciCommonConfig {
    /* About the whole device. */
    pub device_feature_select: u32, /* read-write */
    pub device_feature: u32,        /* read-only for driver */
    pub driver_feature_select: u32, /* read-write */
    pub driver_feature: u32,        /* read-write */
    pub msix_config: u16,           /* read-write */
    pub num_queues: u16,            /* read-only for driver */
    pub device_status: u8,          /* read-write */
    pub config_generation: u8,      /* read-only for driver */

    /* About a specific virtqueue. */
    pub queue_select: u16, /* read-write */

    /// Maximum number of items in Descriptor Queue
    pub queue_size: u16, /* read-write */
    pub queue_msix_vector: u16, /* read-write */
    pub queue_enable: u16,      /* read-write */
    pub queue_notify_off: u16,  /* read-only for driver */
    pub queue_desc: u64,        /* read-write */

    /// Available Ring
    pub queue_driver: u64, /* read-write */

    /// Used Ring
    pub queue_device: u64, /* read-write */
}
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
    flags: u8,
    gso_type: u8,
    header_length: u16,
    gso_side: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

#[derive(PartialEq, Debug)]
pub enum VirtioDeviceStatus {
    Reset,
    Acknowledge,
    Driver,
    Failed,
    FeaturesOk,
    DriverOk,
    DeviceNeedsReset,
}

impl VirtioDeviceStatus {
    fn value(&self) -> u8 {
        match self {
            VirtioDeviceStatus::Reset => 0,
            VirtioDeviceStatus::Acknowledge => 1,
            VirtioDeviceStatus::Driver => 2,
            VirtioDeviceStatus::Failed => 128,
            VirtioDeviceStatus::FeaturesOk => 8,
            VirtioDeviceStatus::DriverOk => 4,
            VirtioDeviceStatus::DeviceNeedsReset => 64,
        }
    }

    fn value_to_status(value: u8) -> VirtioDeviceStatus {
        match value {
            0 => Self::Reset,
            1 => Self::Acknowledge,
            2 => Self::Driver,
            128 => Self::Failed,
            8 => Self::FeaturesOk,
            4 => Self::DriverOk,
            64 => Self::DeviceNeedsReset,
            _ => Self::DeviceNeedsReset,
        }
    }
}

/// VirtIO Network Device registers.
///
/// Specs: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-1460002
#[derive(Debug, Copy, Clone)]
pub enum Register {
    // TODO: Implement here

    // Capabilities: [70] Vendor Specific Information: VirtIO: Notify
    //     BAR=4 offset=00003000 size=00001000 multiplier=00000004
    // Capabilities: [60] Vendor Specific Information: VirtIO: DeviceCfg
    //     BAR=4 offset=00002000 size=00001000
    // Capabilities: [50] Vendor Specific Information: VirtIO: ISR
    //     BAR=4 offset=00001000 size=00001000
    // Capabilities: [40] Vendor Specific Information: VirtIO: CommonCfg
    //     BAR=4 offset=00000000 size=00001000
    CommonCfg,
    ISR,
    DeviceCfg,
    Notify,

    // If reading and writting CommonCfg is too much work
    DeviceStatus,
}

impl Register {
    /// Returns the byte offset of the register.
    fn offset(&self) -> usize {
        match self {
            Register::CommonCfg => 0x0,
            Register::ISR => 0x1000,
            Register::DeviceCfg => 0x2000,
            Register::Notify => 0x3000,

            Register::DeviceStatus => 0x14,
        }
    }

    /// Returns a raw pointer to the register.
    fn as_ptr(&self, mmio_base: usize) -> *const u32 {
        (mmio_base + self.offset()) as *const u32
    }

    /// Returns a raw pointer to the register.
    fn as_mut_ptr(&self, mmio_base: usize) -> *mut u32 {
        (mmio_base + self.offset()) as *mut u32
    }
}

/// A VirtIO Network Device MMIO region.
pub struct Mmio {
    mmio_base: usize,
}

impl Mmio {
    pub fn new(mmio_base: usize) -> Self {
        Self { mmio_base }
    }

    pub unsafe fn memory_fence() {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    pub unsafe fn read_device_config(&mut self) -> VirtioNetworkDeviceConfig {
        let cfg_ptr =
            (self.mmio_base + Register::DeviceCfg.offset()) as *const VirtioNetworkDeviceConfig;
        ptr::read_unaligned(cfg_ptr)
    }

    pub unsafe fn read_common_config(&mut self) -> VirtioPciCommonConfig {
        let cfg_ptr =
            (self.mmio_base + Register::CommonCfg.offset()) as *const VirtioPciCommonConfig;
        ptr::read_unaligned(cfg_ptr)
    }

    pub unsafe fn write_common_config(&mut self, common_config: VirtioPciCommonConfig) {
        let cfg_ptr = (self.mmio_base + Register::CommonCfg.offset()) as *mut VirtioPciCommonConfig;
        ptr::write_unaligned(cfg_ptr, common_config);
    }

    pub unsafe fn read_device_status(&mut self) -> VirtioDeviceStatus {
        let value =
            ptr::read_volatile((self.mmio_base + Register::DeviceStatus.offset()) as *const u8);
        VirtioDeviceStatus::value_to_status(value)
    }

    pub unsafe fn write_device_status(&mut self, status: VirtioDeviceStatus) {
        ptr::write_volatile(
            (self.mmio_base + Register::DeviceStatus.offset()) as *mut u8,
            status.value(),
        );
    }

    // pub unsafe fn write(&mut self, register: Register, value: u32) {
    //     ptr::write_volatile(register.as_mut_ptr(self.mmio_base), value)
    // }

    pub unsafe fn read(&mut self, register: Register) -> u32 {
        ptr::read_volatile(register.as_ptr(self.mmio_base))
    }
}

impl Mmio {
    pub unsafe fn write<T>(&mut self, register: Register, value: T) {
        ptr::write_volatile((self.mmio_base + register.offset()) as *mut T, value)
    }
}
