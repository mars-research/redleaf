use alloc::boxed::Box;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

use libsyscalls::errors::Result;
use rref::RRef;
use usr::bdev::{BDev, BSIZE};
use usr::rpc::RpcResult;

pub struct MemBDev {
    memdisk: Mutex<&'static mut [u8]>,
    seen: AtomicBool,
}

impl MemBDev {
    const SECTOR_SIZE: usize = 512;

    pub fn new(memdisk: &'static mut [u8]) -> Self {
        Self {
            memdisk: Mutex::new(memdisk),
            seen: AtomicBool::new(false),
        }
    }
}

impl BDev for MemBDev {
    fn read(&self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        // console::println!("bdev.read {}", block);
        // if block == 304 {
        //     // Will panic the second time we see this block
        //     assert!(!self.seen.swap(true, Ordering::SeqCst));
        // }
        let start = block as usize * Self::SECTOR_SIZE;
        let size = data.len();

        data.copy_from_slice(&self.memdisk.lock()[start..start+size]);

        Ok(data)
    }
    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        let start = block as usize * Self::SECTOR_SIZE;
        let size = data.len();

        self.memdisk.lock()[start..start+size].copy_from_slice(&**data);
        
        Ok(())
    }
}
