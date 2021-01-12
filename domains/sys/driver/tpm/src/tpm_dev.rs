use alloc::boxed::Box;
use tpm_device::TpmDevice;
use interface::tpm::TpmRegs;

pub struct Tpm {
    device: TpmDevice,
    device_initialized: bool,
    timeout_a: usize,
}

impl Tpm {
    pub fn new() -> Self {
        Self {
            device: TpmDevice::new(),
            device_initialized: true,
            timeout_a: 750,
        }
    }

    #[inline(always)]
    fn active(&self) -> bool {
        self.device_initialized
    }

    #[inline(always)]
    fn read_u8(&self, locality: u32, reg: TpmRegs) -> u8 {
        self.device.read_u8(locality, reg)
    }

    #[inline(always)]
    fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8) {
        self.device.write_u8(locality, reg, val);
    }

    #[inline(always)]
    fn read_u32(&self, locality: u32, reg: TpmRegs) -> u32 {
        self.device.read_u32(locality, reg)
    }

    #[inline(always)]
    fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32) {
        self.device.write_u32(locality, reg, val);
    }

    #[inline(always)]
    fn read_data(&self, locality: u32, reg: TpmRegs, buf: &mut [u8]) {
        for byte in buf.iter_mut() {
            *byte = self.read_u8(locality, reg);
        }
    }

    #[inline(always)]
    fn write_data(&self, locality: u32, reg: TpmRegs, buf: &[u8]) {
        for byte in buf.iter() {
            self.write_u8(locality, reg, *byte);
        }
    }
}

impl interface::tpm::TpmDev for Tpm {
    fn clone_tpmdev(&self) -> Box<dyn interface::tpm::TpmDev> {
        box Self::new()
    }

    fn read_u8(&self, locality: u32, reg: TpmRegs) -> u8 {
        self.device.read_u8(locality, reg)
    }

    fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8) {
        self.device.write_u8(locality, reg, val);
    }

    fn read_u32(&self, locality: u32, reg: TpmRegs) -> u32 {
        self.device.read_u32(locality, reg)
    }

    fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32) {
        self.device.write_u32(locality, reg, val);
    }
}
