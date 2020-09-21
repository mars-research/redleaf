#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    str_strip,
    untagged_unions
)]

extern crate alloc;
extern crate core;
extern crate malloc;

use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use syscalls::{Heap, Syscall};
use usr_interfaces::xv6::Xv6;
use usrlib::syscalls::{sys_read, sys_spawn_domain};
use usrlib::{print, println};

mod parse;

use crate::parse::{Command, Redir};

#[no_mangle]
pub fn init(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Xv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    usrlib::init(rv6.clone().unwrap());
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    println!("Starting rv6 shell with args: {}", args);

    // sys_spawn_domain("benchfs", "throughput", &[Some(0), Some(1), Some(2)]).unwrap().join();
    // sys_spawn_domain("benchfs", &alloc::format!("benchfs r large {}", 30usize * 1024 * 1024 * 1024), &[Some(0), Some(1), Some(2)]).unwrap().join();
    // sys_spawn_domain("benchfs", &alloc::format!("benchfs w large {}", 4usize * 1024 * 1024 * 1024), &[Some(0), Some(1), Some(2)]).unwrap().join();

    //sys_spawn_domain("benchnvme", &alloc::format!("benchfs r large {}", 30usize * 1024 * 1024 * 1024), &[Some(0), Some(1), Some(2)]).unwrap().join();
    //sys_spawn_domain("benchnet", "benchnet", &[Some(0), Some(1), Some(2)]).unwrap();

    const prompt: &'static str = "rv6> ";
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
        sys_read(1, &mut buff[i..i + 1]).unwrap();
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
