#![no_std]
#![no_main]
#![feature(
    global_asm,
    box_syntax,
    type_ascription,
)]
mod gen;

use interface::proxy;

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::panic::PanicInfo;
use interface::domain_creation;
use libsyscalls;
use rref;
use syscalls;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    create_pci: Arc<dyn interface::domain_creation::CreatePCI>,
    create_ahci: Arc<dyn interface::domain_creation::CreateAHCI>,
    create_membdev: Arc<dyn interface::domain_creation::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn interface::domain_creation::CreateBDevShadow>,
    create_ixgbe: Arc<dyn interface::domain_creation::CreateIxgbe>,
    create_virtio_net: Arc<dyn interface::domain_creation::CreateVirtioNet>,
    create_nvme: Arc<dyn interface::domain_creation::CreateNvme>,
    create_net_shadow: Arc<dyn interface::domain_creation::CreateNetShadow>,
    create_nvme_shadow: Arc<dyn interface::domain_creation::CreateNvmeShadow>,
    create_benchnet: Arc<dyn interface::domain_creation::CreateBenchnet>,
    create_benchnvme: Arc<dyn interface::domain_creation::CreateBenchnvme>,
    create_xv6fs: Arc<dyn interface::domain_creation::CreateRv6FS>,
    create_xv6net: Arc<dyn interface::domain_creation::CreateRv6Net>,
    create_xv6net_shadow: Arc<dyn interface::domain_creation::CreateRv6NetShadow>,
    create_xv6usr: Arc<dyn interface::domain_creation::CreateRv6Usr + Send + Sync>,
    create_xv6: Arc<dyn interface::domain_creation::CreateRv6>,
    create_dom_a: Arc<dyn interface::domain_creation::CreateDomA>,
    create_dom_b: Arc<dyn interface::domain_creation::CreateDomB>,
    create_dom_c: Arc<dyn interface::domain_creation::CreateDomC>,
    create_dom_d: Arc<dyn interface::domain_creation::CreateDomD>,
    create_shadow: Arc<dyn interface::domain_creation::CreateShadow>,
) -> Arc<dyn interface::proxy::Proxy> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    Arc::new(gen::Proxy::new(
        create_pci,
        create_ahci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_virtio_net,
        create_nvme,
        create_net_shadow,
        create_nvme_shadow,
        create_benchnet,
        create_benchnvme,
        create_xv6fs,
        create_xv6net,
        create_xv6net_shadow,
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
