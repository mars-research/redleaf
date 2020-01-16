extern crate alloc;
use crate::proxy::PROXY;
use spin::Once;
use alloc::boxed::Box;
use rref::RRef;

pub fn sys_new_data(data: [u8; 512]) -> RRef<[u8; 512]> {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_new_data(data)
}

pub fn sys_drop_data(data: RRef<[u8; 512]>) {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_drop_data(data)
}

pub fn sys_read(block: u32, data: &mut RRef<[u8; 512]>) {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_read(block, data)
}

pub fn sys_write(block: u32, data: &[u8; 512]) {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_write(block, data)
}

pub fn sys_foo() {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_foo()
}

pub fn sys_bar(data: &mut RRef<[u8; 512]>) {
    let proxy = PROXY.force_get();//.r#try().expect("Proxy interface is not initialized.");
    proxy.bdev_bar(data)
}
