#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::syscalls::sys_getpid;
use crate::{eprintln, println};
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(args: &str) {
    println!("Starting rv6 getpid with args: {}", args);

    getpid().unwrap();
}

fn getpid() -> Result<(), String> {
    println!(
        "pid: {}",
        sys_getpid().map_err(|e| alloc::format!("getpid: cannot getpid. {:?}", e))?
    );
    Ok(())
}
