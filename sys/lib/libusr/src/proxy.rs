extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use usr::proxy::Proxy;
use rref::RRef;

pub static PROXY: Once<Box<dyn Proxy + Sync + Send>> = Once::new();

pub fn init(proxy: Box<dyn Proxy + Sync + Send>) {
    PROXY.call_once(|| proxy);
}

pub fn sys_proxy_bench(iterations: u64) {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.proxy_bench(iterations)
}

pub fn sys_proxy_foo() {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.proxy_foo()
}

pub fn sys_proxy_bar() {
    let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
    proxy.proxy_bar()
}
