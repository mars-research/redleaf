use core::ptr;

use console::println;

#[derive(Debug)]
#[repr(packed, C)]
pub struct VirtioPciCommonConfig {
    /* About the whole device. */
    device_feature_select: u32, /* read-write */
    device_feature: u32,        /* read-only for driver */
    driver_feature_select: u32, /* read-write */
    driver_feature: u32,        /* read-write */
    msix_config: u16,           /* read-write */
    num_queues: u16,            /* read-only for driver */
    device_status: u8,          /* read-write */
    config_generation: u8,      /* read-only for driver */

    /* About a specific virtqueue. */
    queue_select: u16,      /* read-write */
    queue_size: u16,        /* read-write */
    queue_msix_vector: u16, /* read-write */
    queue_enable: u16,      /* read-write */
    queue_notify_off: u16,  /* read-only for driver */
    queue_desc: u64,        /* read-write */
    queue_driver: u64,      /* read-write */
    queue_device: u64,      /* read-write */
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
}

impl Mmio {
    pub fn new(mmio_base: usize) -> Self {
        Self { mmio_base }
    }

    pub unsafe fn readCommonConfig(&mut self) -> VirtioPciCommonConfig {
        let cfg_ptr = (self.mmio_base + Register::CommonCfg.offset()) as *mut VirtioPciCommonConfig;
        ptr::read_unaligned(cfg_ptr)
    }

    pub unsafe fn write(&mut self, register: Register, value: u32) {
        ptr::write_volatile(register.as_mut_ptr(self.mmio_base), value)
    }

    pub unsafe fn read(&mut self, register: Register) -> u32 {
        ptr::read_volatile(register.as_ptr(self.mmio_base))
    }
}
