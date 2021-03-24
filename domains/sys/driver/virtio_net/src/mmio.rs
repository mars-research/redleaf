use core::ptr;

use console::println;

#[volatile_accessor::volatile_accessor]
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
pub struct VirtioNetCompletePacket {
    pub header: VirtioNetworkHeader,
    pub data: [u8; 1514],
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
    pub num_buffers: u16,
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
}

impl Register {
    /// Returns the byte offset of the register.
    fn offset(&self) -> usize {
        match self {
            Register::CommonCfg => 0x0,
            Register::ISR => 0x1000,
            Register::DeviceCfg => 0x2000,
            Register::Notify => 0x3000,
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
    pub accessor: VirtioPciCommonConfigVolatileAccessor,
}

impl Mmio {
    pub fn new(mmio_base: usize) -> Self {
        unsafe {
            Self {
                mmio_base,
                accessor: VirtioPciCommonConfigVolatileAccessor::new(mmio_base),
            }
        }
    }

    pub unsafe fn memory_fence() {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    // pub unsafe fn read_device_config(&mut self) -> VirtioNetworkDeviceConfig {
    //     let cfg_ptr =
    //         (self.mmio_base + Register::DeviceCfg.offset()) as *const VirtioNetworkDeviceConfig;
    //     ptr::read_volatile(cfg_ptr)
    // }

    pub unsafe fn read_common_config(&mut self) -> VirtioPciCommonConfig {
        let cfg_ptr =
            (self.mmio_base + Register::CommonCfg.offset()) as *const VirtioPciCommonConfig;
        ptr::read_volatile(cfg_ptr)
    }

    // pub unsafe fn write_common_config(&mut self, common_config: VirtioPciCommonConfig) {
    //     let cfg_ptr = (self.mmio_base + Register::CommonCfg.offset()) as *mut VirtioPciCommonConfig;
    //     ptr::write_volatile(cfg_ptr, common_config);
    // }

    // pub unsafe fn common_config_as_raw_ptr(&mut self) -> *mut VirtioPciCommonConfig {
    //     (self.mmio_base + Register::CommonCfg.offset()) as *mut VirtioPciCommonConfig
    // }

    // pub unsafe fn read_device_status(&mut self) -> u8 {
    //     ptr::read_volatile((self.mmio_base + Register::DeviceStatus.offset()) as *const u8)
    // }

    pub unsafe fn update_device_status(&mut self, status: VirtioDeviceStatus) {
        let mut device_status = self.accessor.read_device_status();

        device_status |= status.value();

        self.accessor.write_device_status(device_status);

        // let cfg = self.common_config_as_raw_ptr();
        // (*cfg).device_status |= status.value();

        // ptr::write_volatile(
        //     (self.mmio_base + Register::DeviceStatus.offset()) as *mut u8,
        //     self.read_device_status() | status.value(),
        // );
    }

    // pub unsafe fn write(&mut self, register: Register, value: u32) {
    //     ptr::write_volatile(register.as_mut_ptr(self.mmio_base), value)
    // }

    // pub unsafe fn read(&mut self, register: Register) -> u32 {
    //     ptr::read_volatile(register.as_ptr(self.mmio_base))
    // }
}

impl Mmio {
    // pub unsafe fn read_queue_config(&mut self) -> VirtioPciQueueConfig {
    //     ptr::read_volatile(
    //         (self.mmio_base + Register::QueueConfig.offset()) as *mut VirtioPciQueueConfig,
    //     )
    // }

    pub unsafe fn write<T>(&mut self, register: Register, value: T) {
        ptr::write_volatile((self.mmio_base + register.offset()) as *mut T, value)
    }

    // pub unsafe fn read_queue_select(&mut self) -> u16 {
    //     ptr::read_volatile((self.mmio_base + Register::QueueSelect.offset()) as *const u16)
    // }
}
