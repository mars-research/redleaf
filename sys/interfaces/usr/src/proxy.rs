use rref::RRef;
use alloc::boxed::Box;

pub trait Proxy {
    fn proxy_clone(&self) -> Box<dyn Proxy>;

    fn foo(&self) -> usize;
    fn new_value(&self, value: [u8; 512]) -> RRef<[u8; 512]>;
    fn drop_value(&self, value: RRef<[u8; 512]>);

    fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>);
    fn bdev_write(&self, block: u32, data: &[u8; 512]);
}
