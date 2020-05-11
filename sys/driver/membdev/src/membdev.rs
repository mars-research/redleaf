use alloc::boxed::Box;
use spin::Mutex;

use libsyscalls::errors::Result;
use rref::RRef;
use usr::bdev::{BDev, BSIZE};
use usr::rpc::RpcResult;

pub struct MemBDev {
    memdisk: Mutex<&'static mut [u8]>,
}

impl MemBDev {
    const SECTOR_SIZE: usize = 512;

    pub fn new() -> Self {
        extern "C" {
            fn _binary___________usr_mkfs_build_fs_img_start();
            fn _binary___________usr_mkfs_build_fs_img_end();
        }

        let (start, end) = (
            _binary___________usr_mkfs_build_fs_img_start as *const u8,
            _binary___________usr_mkfs_build_fs_img_end as *const u8,
        );

        let size = end as usize - start as usize;

        Self {
            memdisk: unsafe{ Mutex::new(core::slice::from_raw_parts_mut(start as *mut u8, size))},
        }
    }
}

impl BDev for MemBDev {
    fn read(&self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        let start = block as usize * Self::SECTOR_SIZE;
        let size = data.len();

        data.copy_from_slice(&self.memdisk.lock()[start..start+size]);

        Ok(data)
    }
    fn write(&self, block: u32, data: RRef<[u8; BSIZE]>) -> RRef<[u8; BSIZE]> {
        let start = block as usize * Self::SECTOR_SIZE;
        let size = data.len();

        self.memdisk.lock()[start..start+size].copy_from_slice(&*data);
        
        data
    }
}
