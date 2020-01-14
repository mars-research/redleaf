extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use usr::proxy::Proxy;
use rref::RRef;

pub static PROXY: Once<Box<dyn Proxy + Sync + Send>> = Once::new();

pub fn init(proxy: Box<dyn Proxy + Sync + Send>) {
    PROXY.call_once(|| proxy);
}

pub fn proxy_foo() -> usize {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.foo()
}

pub fn proxy_new_value(value: [u8; 512]) -> RRef<[u8; 512]> {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.new_value(value)
}

pub fn proxy_drop_value(value: RRef<[u8; 512]>) {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.drop_value(value)
}
