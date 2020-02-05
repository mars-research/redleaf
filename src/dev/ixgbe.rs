use core::ptr;
use ixgbe::BarRegion;
use syscalls::PciBar;
use alloc::boxed::Box;
use crate::interrupt::{disable_irq, enable_irq};

pub struct Bar {
    base: usize,
    size: usize,
}

impl Bar {
    pub fn new(base: usize, size: usize) -> Bar {
        Bar {
            base,
            size,
        }
    }
}

impl BarRegion for Bar {

    #[inline(always)]
    fn read_reg32(&self, offset: usize) -> u32 {
        let mut ret: u32 = 0;
        disable_irq();
        // Check bounds
        if (self.base + offset) >= self.size {
            println!("Write failed! out of bounds");
        } else {
            let ret = unsafe {
                ptr::read_volatile((self.base + offset) as *const u32)
            };
        }
        enable_irq();
        ret
    }

    #[inline(always)]
    fn write_reg32(&self, offset: usize, val: u32) {
        disable_irq();
        // Check bounds
        if (self.base + offset) >= self.size {
            println!("Write failed! out of bounds");
        } else {
            unsafe {
                ptr::write_volatile((self.base + offset) as *mut u32, val as u32);
            }
        }
        enable_irq();
    }

    #[inline(always)]
    fn read_reg64(&self, offset: usize) -> u64 {
        let mut ret: u64 = 0;
        disable_irq();
        // Check bounds
        if (self.base + offset) >= self.size {
            println!("Write failed! out of bounds");
        } else {
            let ret = unsafe {
                ptr::read_volatile((self.base + offset) as *const u64)
            };
        }
        enable_irq();
        ret
    }

    #[inline(always)]
    fn write_reg64(&self, offset: usize, val: u64) {
        disable_irq();
        // Check bounds
        if (self.base + offset) >= self.size {
            println!("Write failed! out of bounds");
        } else {
            unsafe {
                ptr::write_volatile((self.base + offset) as *mut u64, val as u64);
            }
        }
        enable_irq();
    }
}
