#![no_std]
#![no_main]
// #![forbid(unsafe_code)]
#![feature(box_syntax, str_strip, untagged_unions)]

extern crate alloc;
extern crate core;
extern crate malloc;

use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use interface::rv6::Rv6;
use syscalls::{Heap, Syscall};
use usrlib::syscalls::sys_read_slice_slow;
use usrlib::{print, println};

mod parse;

use crate::parse::{Command, Redir};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    usrlib::init(rv6.clone_rv6().unwrap());
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    println!("Starting rv6 shell with args: {}", args);

    // Shell commands that get run automatically after shell is launched
    let predefined_commands = [
        // String::from("dump_inode"),
        String::from("ls"),
        String::from("rv6_testtpm"),
        // String::from("ls > foo"),
        // String::from("mkdir bar"),
        // String::from("ls"),
        // String::from("rm foo"),
        // String::from("ls"),
        // String::from("httpd"),
        // alloc::format!("benchfs r large {}", 30usize * 1024 * 1024 * 1024),
        // alloc::format!("benchfs w large {}", 4usize * 1024 * 1024 * 1024),
        // alloc::format!("benchfs r large {}", 30usize * 1024 * 1024 * 1024),
    ];

    const prompt: &str = "rv6> ";
    for command in &predefined_commands {
        print!("{}", prompt);
        print!("{}", command);
        let (cmd, leftover) = Command::parse(command);
        assert!(
            leftover.is_empty(),
            "Leftover after parsing: <{}>",
            leftover
        );
        println!("Parsed command: {:?}", cmd);
        cmd.run(Redir::new()).iter().for_each(|t| t.join().unwrap());
    }

    loop {
        print!("{}", prompt);
        let line = read_until('\n');
        let trimmed_line = line.trim();
        if !trimmed_line.is_empty() {
            let (cmd, leftover) = Command::parse(trimmed_line);
            assert!(
                leftover.is_empty(),
                "Leftover after parsing: <{}>",
                leftover
            );
            println!("Parsed command: {:?}", cmd);
            cmd.run(Redir::new()).iter().for_each(|t| t.join().unwrap());
        }
    }
    println!("Finish shell");
}

fn read_until(c: char) -> String {
    let mut buff = [0u8; 1024];
    for i in 0..buff.len() {
        sys_read_slice_slow(1, &mut buff[i..i + 1]).unwrap();
        if buff[i] == c as u8 {
            return String::from_utf8(buff[..i + 1].to_vec()).unwrap();
        }
    }
    panic!("read_until");
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("shell panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
