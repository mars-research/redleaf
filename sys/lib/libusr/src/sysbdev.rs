extern crate alloc;
use crate::proxy::PROXY;
use spin::Once;
use alloc::boxed::Box;
use rref::RRef;

pub fn sys_read(block: u32, data: &mut RRef<[u8; 512]>) {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_read(block, data)
}

pub fn sys_write(block: u32, data: &[u8; 512]) {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_write(block, data)
}
