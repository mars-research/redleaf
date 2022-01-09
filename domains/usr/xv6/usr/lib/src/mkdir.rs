#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::syscalls::sys_mkdir_slice_slow;
use crate::{eprintln, println};
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(args: &str) {
    println!("Starting rv6 mkdir with args: {}", args);

    let mut args = args.split_whitespace();
    assert!(args.next().is_some());
    let path = args.next().unwrap();

    mkdir(path).unwrap();
}

fn mkdir(path: &str) -> Result<(), String> {
    println!("mkdir <{}>", path);
    sys_mkdir_slice_slow(path)
        .map_err(|e| alloc::format!("mkdir: cannot mkdir {}. {:?}", path, e))?;
    Ok(())
}
