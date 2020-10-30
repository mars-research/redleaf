#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use core::panic::PanicInfo;

use syscalls::{Heap, Syscall};
use usr_interfaces::vfs::FileMode;
use usr_interfaces::rv6::Xv6;
use usrlib::syscalls::{sys_spawn_domain_slice_slow, sys_open_slice_slow, sys_mknod_slice_slow, sys_dup};
use usrlib::{dbg, println};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Xv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone().unwrap());

    // stdout not initialized yet so we can't print it there yet

    // Create console device if it not there yet
    match sys_open_slice_slow("/console", FileMode::READWRITE) {
        Err(_) => {
            sys_mknod_slice_slow("/console", 1, 1).unwrap();
            assert_eq!(sys_open_slice_slow("/console", FileMode::READWRITE).unwrap(), 0);
        }
        Ok(fd) => {
            assert_eq!(fd, 0);
        }
    }
    // Dup stdin to stdout and stderr
    assert_eq!(sys_dup(0).unwrap(), 1);
    assert_eq!(sys_dup(0).unwrap(), 2);

    dbg!("Init finished");
    sys_spawn_domain_slice_slow("/sh", "", &[Some(0), Some(1), Some(2)]).unwrap();
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Could be a recursive panic if fs is failed to init
    println!("init panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
