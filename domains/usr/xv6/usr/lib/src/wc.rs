#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;

use crate::println;
use crate::syscalls::{
    sys_close, sys_fstat, sys_open_slice_slow, sys_read_slice_slow, sys_write_slice_slow,
};
use alloc::boxed::Box;
use core::panic::PanicInfo;
use interface::vfs::FileMode;

pub fn main(args: &str) {
    println!("Starting rv6 wc with args: {}", args);

    let mut args = args.split_whitespace().peekable();
    assert!(args.next().is_some());

    if args.peek().is_none() {
        // Read from STDIN
        wc(0, "");
    }

    for arg in args {
        let fd = sys_open_slice_slow(arg, FileMode::READ).unwrap();
        wc(fd, arg).unwrap();
        sys_close(fd);
    }
}

fn wc(fd: usize, name: &str) -> Result<(), &'static str> {
    let mut line_cnt = 0;
    let mut word_cnt = 0;
    let mut char_cnt = 0;
    let mut in_word = false;

    let mut buff = [0u8; 512];
    loop {
        let bytes_read = sys_read_slice_slow(fd, &mut buff).unwrap();
        if bytes_read == 0 {
            break;
        }

        for c in &buff[..bytes_read] {
            let c = *c as char;
            char_cnt += 1;
            if c == '\n' {
                line_cnt += 1;
            }
            if c.is_ascii_whitespace() {
                in_word = false;
            } else if !in_word {
                word_cnt += 1;
                in_word = true;
            }
        }
    }

    println!(
        "wc: line:{} word:{} char:{} name:{}",
        line_cnt, word_cnt, char_cnt, name
    );
    Ok(())
}
