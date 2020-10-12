#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use libsyscalls::syscalls::sys_println;
use syscalls::{Heap, Syscall};
use usr_interfaces::vfs::{DirectoryEntry, DirectoryEntryRef, FileMode, INodeFileType};
use usr_interfaces::xv6::Xv6;
use usrlib::syscalls::sys_link_slice_slow;
use usrlib::{eprintln, println};

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
    println!("Starting rv6 ln with args: {}", args);

    let mut args = args.split_whitespace();
    assert!(args.next().is_some());
    let old_path = args.next().unwrap();
    let new_path = args.next().unwrap();

    ln(old_path, new_path).unwrap();
}

fn ln(old_path: &str, new_path: &str) -> Result<(), String> {
    println!("ln <{}> <{}>", old_path, new_path);
    sys_link_slice_slow(old_path, new_path).map_err(|e| alloc::format!("ln: cannot ln {:?}", e))?;
    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("mkdir panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
