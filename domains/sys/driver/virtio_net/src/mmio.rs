use core::ptr;

use console::println;

pub const VIRTIO_MAGIC: u32 = 0x74726976;
pub const VIRTIO_DEVID_NET: u32 = 0x1;

/// VirtIO Network Device registers.
///
/// Specs: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-1460002
#[derive(Debug, Copy, Clone)]
pub enum Register {
    Magic,
    Version,
    DeviceId,
    VendorId,

    // TODO: Implement here
    MmioStatus,
    MmioDeviceFeatures,
    MimoDriverFeatures,
}

impl Register {
    /// Returns the byte offset of the register.
    fn offset(&self) -> usize {
        match self {
            Register::Magic => 0x0,
            Register::Version => 0x4,
            Register::DeviceId => 0x8,
            Register::VendorId => 0xc,

            Register::MmioStatus => 0x1000,
            Register::MmioDeviceFeatures => 0x10,
            Register::MimoDriverFeatures => 0x20,
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

    /// Performs a sanity check.
    pub unsafe fn sanity_check_panic(&mut self) {
        if self.read(Register::Magic) != VIRTIO_MAGIC {
            panic!("Invalid MMIO base: Not a VirtIO device");
        }

        if self.read(Register::Version) != 0x2 {
            panic!("Invalid MMIO base: Unsupported VirtIO version");
        }

        let device_id = self.read(Register::DeviceId);
        if device_id != VIRTIO_DEVID_NET {
            panic!(
                "Invalid MMIO base: Not a VirtIO Network device but {}",
                device_id
            );
        }

        // TODO: Remove this
        println!("Found valid VirtIO Network MMIO @ {:X?}", self.mmio_base);
    }

    pub unsafe fn write(&mut self, register: Register, value: u32) {
        ptr::write_volatile(register.as_mut_ptr(self.mmio_base), value)
    }

    pub unsafe fn read(&mut self, register: Register) -> u32 {
        ptr::read_volatile(register.as_ptr(self.mmio_base))
    }
}
