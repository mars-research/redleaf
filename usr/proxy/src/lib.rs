#![no_std]
#![feature(global_asm)]
mod gen;

extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls;
use create;
use rref;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[no_mangle]
pub fn init(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn create::CreateBDevShadow>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_benchnet: Arc<dyn create::CreateBenchnet>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>,
    create_dom_c: Arc<dyn create::CreateDomC>,
    create_dom_d: Arc<dyn create::CreateDomD>,
    create_shadow: Arc<dyn create::CreateShadow>,
) -> Arc<dyn proxy::Proxy> {

    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    Arc::new(gen::Proxy::new(
        create_pci,
        create_ahci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_benchnet,
        create_xv6fs,
        create_xv6usr,
        create_xv6,
        create_dom_a,
        create_dom_b,
        create_dom_c,
        create_dom_d,
        create_shadow,
    ))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
