#![no_std]
mod gen;

extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls;
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[no_mangle]
pub fn init(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>) -> Arc<dyn proxy::Proxy> {

    libsyscalls::syscalls::init(s);

    Arc::new(gen::Proxy::new(
        create_pci,
        create_ahci,
        create_ixgbe,
        create_xv6fs,
        create_xv6usr,
        create_xv6
    ))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
