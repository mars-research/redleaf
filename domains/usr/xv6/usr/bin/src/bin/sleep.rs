#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use syscalls::{Heap, Syscall};

use interface::rv6::Rv6;
use usrlib::syscalls::sys_sleep;
use usrlib::{eprintln, println};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone_rv6().unwrap());
    println!("Starting rv6 sleep with args: {}", args);

    let mut args = args.split_whitespace();
    assert!(args.next().is_some());
    let ns = args.next().or(Some("")).unwrap();

    sleep(ns.parse::<u64>().unwrap()).unwrap();
}

fn sleep(ns: u64) -> Result<(), String> {
    sys_sleep(ns).map_err(|e| alloc::format!("sleep: cannot sleep. {:?}", e))?;
    println!("sleep finished");
    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("sleep panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
