#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(box_syntax, const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

mod rv6_syscalls;
mod thread;

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;

use console::println;
use libsyscalls::syscalls::{sys_current_thread, sys_recv_int, sys_yield};
use rref;
use syscalls::{Heap, Syscall};
use usr_interface::bdev::BDev;
use usr_interface::vfs::{FileMode, VFS};
use usr_interface::rv6::{Thread, Rv6};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    ints: Box<dyn syscalls::Interrupt + Send + Sync>,
    create_xv6fs: Arc<dyn create::CreateRv6FS>,
    create_xv6net: Arc<dyn create::CreateRv6Net>,
    create_xv6net_shadow: Arc<dyn create::CreateRv6NetShadow>,
    create_xv6usr: Arc<dyn create::CreateRv6Usr + Send + Sync>,
    bdev: Box<dyn BDev>,
    net: Box<dyn usr_interface::net::Net>,
    nvme: Box<dyn usr_interface::bdev::NvmeBDev>,
    usr_tpm: Box<dyn usr_interface::tpm::UsrTpm>,
) -> Box<dyn Rv6> {
    libsyscalls::syscalls::init(s);
    libsyscalls::syscalls::init_interrupts(ints);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("init xv6/core");

    // Init fs
    let (_dom_xv6fs, fs) = create_xv6fs.create_domain_xv6fs(bdev);
    // Init usrnet
    #[cfg(feature = "shadow")]
    let (_dom_xv6net, usrnet) = create_xv6net_shadow.create_domain_xv6net_shadow(create_xv6net, net.clone_net().unwrap());
    #[cfg(not(feature = "shadow"))]
    let (_dom_xv6net, usrnet) = create_xv6net.create_domain_xv6net(net.clone_net().unwrap());
    // Init kernel
    box rv6_syscalls::Rv6Syscalls::new(create_xv6usr, fs.clone(), usrnet, net, nvme, usr_tpm)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6kernel panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
