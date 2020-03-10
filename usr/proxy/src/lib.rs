#![no_std]

extern crate malloc;
extern crate alloc;
use rref::RRef;
use create;
use syscalls;
use libsyscalls;
use syscalls::Syscall;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[inline(always)]
fn get_caller_domain() -> u64 { libsyscalls::syscalls::sys_get_current_domain_id() }

#[inline(always)]
fn update_caller_domain_id(new_domain_id: u64) -> u64 {
    unsafe { libsyscalls::syscalls::sys_update_current_domain_id(new_domain_id) }
}

struct Proxy {
}

unsafe impl Send for Proxy {}
unsafe impl Sync for Proxy {}

impl Proxy {
    fn new() -> Proxy {
        Proxy {
        }
    }
}

impl usr::proxy::Proxy for Proxy {
    fn proxy_clone(&self) -> Box<dyn usr::proxy::Proxy + Send + Sync> {
        Box::new(Proxy::new())
    }

    fn proxy_bdev(&self, bdev: Box<dyn usr::bdev::BDev + Send + Sync>) -> Box<dyn usr::bdev::BDev + Send + Sync> {
        Box::new(BDevProxy::new(get_caller_domain(), bdev))
    }
}

struct BDevProxy {
    domain: Box<dyn usr::bdev::BDev>,
    domain_id: u64,
}

impl BDevProxy {
    fn new(domain_id: u64, domain: Box<dyn usr::bdev::BDev>) -> Self {
        Self {
            domain,
            domain_id,
        }
    }
}

unsafe impl Sync for BDevProxy {}
unsafe impl Send for BDevProxy {}

impl usr::bdev::BDev for BDevProxy {
    fn read(&self, block: u32, data: &mut RRef<[u8; 512]>) {
        // move thread to next domain
        let caller_domain = update_caller_domain_id(self.domain_id);

        println!("[proxy::bdev_read] caller: {}, callee: {}", caller_domain, self.domain_id);

        data.move_to(self.domain_id);
        let r = self.domain.read(block, data);
        data.move_to(caller_domain);

        // move thread back
        update_caller_domain_id(caller_domain);

        r
    }

    fn write(&self, block: u32, data: &[u8; 512]) {
        // move thread to next domain
        let caller_domain = update_caller_domain_id(self.domain_id);

//        data.move_to(callee_domain);
        let r = self.domain.write(block, data);
//        data.move_to(caller_domain);

        // move thread back
        update_caller_domain_id(caller_domain);

        r
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>) -> Box<dyn usr::proxy::Proxy> {
    libsyscalls::syscalls::init(s);

    Box::new(Proxy::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
