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

    pub fn read_reg(&self, locality: u32, reg: TpmRegs, buf: &mut Vec<u8>) {
        assert!(locality <= 4);
        unsafe {
            for i in 0..buf.len() {
                    buf[i] = ptr::read_volatile((self.mmio.get_base() + locality * 4096 + reg as u32 + i as u32) as *const u8);
            }
        }
    }

    pub fn write_reg(&self, locality: u32, reg: TpmRegs, buf: &Vec<u8>) {
        assert!(locality <= 4);
        unsafe {
            for (i, byte) in buf.iter().enumerate() {
                    ptr::write_volatile((self.mmio.get_base() + locality * 4096 + reg as u32 + i as u32) as *mut u8, *byte);
            }
        }
    }
}
