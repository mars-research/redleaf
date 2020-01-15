use rref::RRef;
use alloc::boxed::Box;

pub trait Proxy {
    fn proxy_clone(&self) -> Box<dyn Proxy>;

    fn bdev_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]>;
    fn bdev_drop_data(&self, data: RRef<[u8; 512]>);

    fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>);
    fn bdev_write(&self, block: u32, data: &[u8; 512]);
}
