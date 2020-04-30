use alloc::boxed::Box;
use spin::Mutex;
use libsyscalls::errors::Result;

pub struct MemAhci {
    memdisk: Mutex<&'static mut [u8]>,
}

impl MemAhci {
    const BLOCK_SIZE: usize = 512;

    pub fn new() -> Self {
        extern "C" {
            fn _binary______________usr_mkfs_build_fs_img_start();
            fn _binary______________usr_mkfs_build_fs_img_end();
        }

        let (start, end) = (
            _binary______________usr_mkfs_build_fs_img_start as *const u8,
            _binary______________usr_mkfs_build_fs_img_end as *const u8,
        );

        let size = end as usize - start as usize;

        Self {
            memdisk: unsafe{ Mutex::new(core::slice::from_raw_parts_mut(start as *mut u8, size))},
        }
    }
}

impl usr_interface::bdev::SyncBDev for MemAhci {
    fn read(&self, block: u32, data: &mut [u8]) {
        let start = block as usize * Self::BLOCK_SIZE;
        let size = data.len();

        data.copy_from_slice(&self.memdisk.lock()[start..start+size]);
    }
    fn write(&self, block: u32, data: &[u8]) {
        let start = block as usize * Self::BLOCK_SIZE;
        let size = data.len();

        self.memdisk.lock()[start..start+size].copy_from_slice(data);
    }
}

impl usr_interface::bdev::AsyncBDev for MemAhci {
    fn submit(&self, block: u64, write: bool, buf: Box<[u8]>) -> Result<u32> {
        unimplemented!()
    }
    fn poll(&self, slot: u32) -> Result<Option<Box<[u8]>>> {
        unimplemented!()
    }
}

impl usr_interface::bdev::BDev for MemAhci {}