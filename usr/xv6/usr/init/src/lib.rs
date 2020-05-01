#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use core::panic::PanicInfo;
use alloc::boxed::Box;

use usrlib::{dbg, println};
use usrlib::syscalls::{sys_load_domain};
use syscalls::{Syscall, Heap};
use usr_interface::xv6::Xv6;
use usr_interface::vfs::FileMode;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6 + Send + Sync>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);
    usrlib::init(rv6.clone());

    // stdout not initialized yet so we can't print it there yet 
    console::println!("Rv6 init");

    // Create console device if it not there yet
    match rv6.sys_open("/console", FileMode::ReadWrite) {
        Err(_) => {
            console::println!("/console doesnt exist; creating a new one.");
            rv6.sys_mknod("/console", 1, 1).unwrap();
            assert_eq!(rv6.sys_open("/console", FileMode::ReadWrite).unwrap(), 0);
        },
        Ok(fd) => {
            console::println!("/console already exists; reusing the old one.");
            assert_eq!(fd, 0);
            console::println!("{:?}", rv6.sys_fstat(fd).unwrap());
        },
    }
    // Dup stdin to stdout and stderr
    assert_eq!(rv6.sys_dup(0).unwrap(), 1);
    assert_eq!(rv6.sys_dup(0).unwrap(), 2);

    dbg!("Init finished");
    sys_load_domain("/sh", "", &[Some(0), Some(1), Some(2)]).unwrap();
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    console::println!("init panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
