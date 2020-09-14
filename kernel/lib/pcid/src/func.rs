use byteorder::{LittleEndian, ByteOrder};

extern crate alloc;
use crate::alloc::vec::Vec;

use crate::pci::PciDev;

pub trait ConfigReader {
    unsafe fn read_range(&self, offset: u8, len: u8) -> Vec<u8> {
        assert!(len > 3 && len % 4 == 0);
        let mut ret = Vec::with_capacity(len as usize);
        let results = (offset..offset + len).step_by(4).fold(Vec::new(), |mut acc, offset| {
            let val = self.read_u32(offset);
            acc.push(val);
            acc
        });
        ret.set_len(len as usize);
        LittleEndian::write_u32_into(&*results, &mut ret);
        ret
    }

    unsafe fn read_u32(&self, offset: u8) -> u32;
}

pub struct PciFunc<'pci> {
    pub dev: &'pci PciDev<'pci>,
    pub num: u8
}

impl<'pci> ConfigReader for PciFunc<'pci> {
    unsafe fn read_u32(&self, offset: u8) -> u32 {
        self.dev.read(self.num, offset)
    }
}
