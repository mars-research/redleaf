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

use crate::syscalls::sys_uptime;
use crate::{eprintln, println};
use interface::rv6::Rv6;

pub fn main(args: &str) {
    println!("Starting rv6 uptime with args: {}", args);

    uptime().unwrap();
}

fn uptime() -> Result<(), String> {
    println!(
        "uptime: {}",
        sys_uptime().map_err(|e| alloc::format!("uptime: cannot uptime. {:?}", e))?
    );
    Ok(())
}
