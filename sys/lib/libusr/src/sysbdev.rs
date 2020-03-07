extern crate alloc;
use spin::Once;

use usr::bdev::BDevPtr;

pub static BDEV: Once<BDevPtr> = Once::new();

pub fn init(bdev: BDevPtr) {
    BDEV.call_once(|| bdev);
}

pub fn sys_read(block: u32, data: &mut [u8; 512]) {
    let bdev = BDEV.r#try().expect("BDev interface is not initialized.");
    bdev.read(block, data)
}

pub fn sys_write(block: u32, data: &[u8; 512]) {
    let bdev = BDEV.r#try().expect("BDev interface is not initialized.");
    bdev.write(block, data)
}
