use rref::RRef;
use alloc::boxed::Box;
use crate::bdev;

pub trait Proxy {
    fn proxy_clone(&self) -> Box<dyn Proxy + Send + Sync>;
    fn proxy_bdev(&self, bdev: Box<dyn bdev::BDev + Send + Sync>) -> Box<dyn bdev::BDev + Send + Sync>;
}
