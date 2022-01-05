#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::syscalls::sys_link_slice_slow;
use crate::{eprintln, println};
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(args: &str) {
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
