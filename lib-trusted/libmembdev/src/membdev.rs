use spin::Mutex;

use rref::RRef;
use usr::bdev::{BDev, BSIZE};
use usr::rpc::RpcResult;

pub struct MemBDev {
    memdisk: Mutex<&'static mut [u8]>,
    end_time: u64,
}

impl MemBDev {
    const SECTOR_SIZE: usize = 512;

    pub fn new(memdisk: &'static mut [u8]) -> Self {
        Self {
            memdisk: Mutex::new(memdisk),
            end_time: libtime::get_rdtsc() + ONE_HOUR,
        }
    }
}

const ONE_HOUR: u64 = 2_400_000_000;

impl BDev for MemBDev {
    fn read(&self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        // console::println!("bdev.read {}", block);
        // assert!(libtime::get_rdtsc() < self.end_time);
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
