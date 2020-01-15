use rref::RRef;
use alloc::boxed::Box;

pub trait Proxy {
    fn proxy_clone(&self) -> Box<dyn Proxy>;

    fn proxy_bench(&self, iterations: u64);

    fn proxy_foo(&self);
    fn proxy_bar(&self);

    fn bdev_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]>;
    fn bdev_drop_data(&self, data: RRef<[u8; 512]>);

    fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>);
    fn bdev_write(&self, block: u32, data: &[u8; 512]);

    fn bdev_foo(&self);
    fn bdev_bar(&self, data: &mut RRef<[u8; 512]>);
}
