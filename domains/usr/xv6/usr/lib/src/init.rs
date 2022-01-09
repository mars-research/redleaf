#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;
use crate::syscalls::{
    sys_dup, sys_mknod_slice_slow, sys_open_slice_slow, sys_spawn_domain_slice_slow,
};
use crate::{dbg, println};
use alloc::boxed::Box;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use interface::vfs::FileMode;
use syscalls::{Heap, Syscall};

pub fn main(_args: &str) {
    // stdout not initialized yet so we can't print it there yet

    // Create console device if it not there yet
    match sys_open_slice_slow("/console", FileMode::READWRITE) {
        Err(_) => {
            sys_mknod_slice_slow("/console", 1, 1).unwrap();
            assert_eq!(
                sys_open_slice_slow("/console", FileMode::READWRITE).unwrap(),
                0
            );
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
