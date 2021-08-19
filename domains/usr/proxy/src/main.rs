#![no_std]
#![no_main]
#![feature(global_asm, box_syntax, type_ascription)]

use interface::proxy;

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::panic::PanicInfo;
use interface::domain_create;
use interface::rref;
use libsyscalls;
use syscalls;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
    create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
    create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
    create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
    create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
    create_virtio_block: Arc<dyn interface::domain_create::CreateVirtioBlock>,
    create_virtio_backend: Arc<dyn interface::domain_create::CreateVirtioBackend>,
    create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
    create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
    create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
    create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
    create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
    create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
    create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
    create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
    create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
    create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
    create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
    create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
    create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
) -> Arc<dyn interface::proxy::Proxy> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    Arc::new(interface::proxy::ProxyObject::new(
        create_pci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_virtio_net,
        create_virtio_block,
        create_virtio_backend,
        create_net_shadow,
        create_nvme_shadow,
        create_nvme,
        create_xv6fs,
        create_xv6net,
        create_xv6net_shadow,
        create_xv6usr,
        create_xv6,
        create_dom_c,
        create_dom_d,
        create_shadow,
        create_benchnvme,
        create_tpm,
    ))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
