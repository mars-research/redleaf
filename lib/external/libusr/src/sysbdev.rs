extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use interface::rref::RRef;
use interface::bdev::{BDev, BSIZE};
use interface::rpc::RpcResult;

pub static BDEV: Once<Box<dyn BDev + Sync + Send>> = Once::new();

pub fn init(bdev: Box<dyn BDev + Sync + Send>) {
    BDEV.call_once(|| bdev);
}

pub fn sys_read(block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
    let bdev = BDEV.r#try().expect("BDev interface is not initialized.");
    bdev.read(block, data)
}

pub fn sys_write(block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
    let bdev = BDEV.r#try().expect("BDev interface is not initialized.");
    bdev.write(block, data)
}
