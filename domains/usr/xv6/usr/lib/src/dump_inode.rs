#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::syscalls::sys_dump_inode;
use crate::{eprintln, println};
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};

pub fn main(args: &str) {
    println!("Starting rv6 dump_inode with args: {}", args);

    dump_inode().unwrap();
}

fn dump_inode() -> Result<(), String> {
    sys_dump_inode().map_err(|e| alloc::format!("dump_inode failed {:?}", e))?;
    Ok(())
}
