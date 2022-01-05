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

use crate::syscalls::sys_sleep;
use crate::{eprintln, println};
use interface::rv6::Rv6;

pub fn main(args: &str) {
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
