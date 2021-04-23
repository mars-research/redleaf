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

extern crate alloc;

pub mod defs;

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
    pub fn value(&self) -> u8 {
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

    pub unsafe fn read_common_config(&mut self) -> VirtioPciCommonConfig {
        let cfg_ptr =
            (self.mmio_base + Register::CommonCfg.offset()) as *const VirtioPciCommonConfig;
        ptr::read_volatile(cfg_ptr)
    }

    pub unsafe fn update_device_status(&mut self, status: VirtioDeviceStatus) {
        let mut device_status = self.accessor.read_device_status();

        device_status |= status.value();

        self.accessor.write_device_status(device_status);
    }

    pub unsafe fn write<T>(&mut self, register: Register, value: T) {
        ptr::write_volatile((self.mmio_base + register.offset()) as *mut T, value)
    }

    pub unsafe fn queue_notify(&mut self, queue_notify_offset: u16, queue_index: u16) {
        ptr::write_volatile(
            (self.mmio_base + Register::Notify.offset() + ((queue_notify_offset * 4) as usize))
                as *mut u16,
            queue_index,
        )
    }
}
