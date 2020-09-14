#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::boxed::Box;
use console::{println, print};
use core::{mem, ptr};
use platform::MmioAddr;
use libtime::sys_ns_loopsleep;
use alloc::format;
use usr::tpm::TpmRegs;

const TPM_BASE_ADDR: u32 = 0xFED4_0000;
const TPM_REGION_SIZE: usize = 5 * 4096;

pub struct TpmDevice {
    mmio: MmioAddr,
}

impl TpmDevice {
    pub fn new() -> TpmDevice {
        TpmDevice {
            mmio: unsafe { MmioAddr::new(TPM_BASE_ADDR, TPM_REGION_SIZE) },
        }
    }

    #[inline(always)]
    pub fn read_u8(&self, locality: u32, reg: TpmRegs) -> u8 {
        assert!(locality <= 4);
        unsafe {    
            ptr::read_volatile((self.mmio.get_base() + locality * 4096 + reg as u32) as *const u8)
        }
    }

    #[inline(always)]
    pub fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8) {
        assert!(locality <= 4);
        unsafe {
            ptr::write_volatile((self.mmio.get_base() + locality * 4096 + reg as u32) as *mut u8, val);
        }
    }

    #[inline(always)]
    pub fn read_u32(&self, locality: u32, reg: TpmRegs) -> u32 {
        assert!(locality <= 4);
        unsafe {    
            ptr::read_volatile((self.mmio.get_base() + locality * 4096 + reg as u32) as *const u32)
        }
    }

    #[inline(always)]
    pub fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32) {
        assert!(locality <= 4);
        unsafe {
            ptr::write_volatile((self.mmio.get_base() + locality * 4096 + reg as u32) as *mut u32, val);
        }
    }
}
