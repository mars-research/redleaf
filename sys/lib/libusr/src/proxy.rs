extern crate alloc;
use spin::Once;
use usr::proxy::Proxy;
use alloc::boxed::Box;

pub static PROXY: Once<Box<dyn Proxy + Sync + Send>> = Once::new();

pub fn init(proxy: Box<dyn Proxy + Sync + Send>) {
    PROXY.call_once(|| proxy);
}
