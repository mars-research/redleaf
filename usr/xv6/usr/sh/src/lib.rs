#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    str_strip,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::string::String;

use syscalls::{Syscall, Heap};
use usrlib::{print, println};
use usrlib::syscalls::{sys_read, sys_spawn_domain};
use usr_interfaces::xv6::Xv6Ptr;
use usr_interfaces::vfs::VFSPtr;

mod parse;

use crate::parse::{Command, Redir};

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Xv6Ptr, args: &str) {
    libsyscalls::syscalls::init(s);
    usrlib::init(rv6.clone());
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    println!("Starting rv6 shell with args: {}", args);

    // sys_spawn_domain("benchfs", "benchfs", &[Some(0), Some(1), Some(2)]).unwrap();

    const prompt: &'static str = "rv6> ";
    loop {
        print!("{}", prompt);
        let line = read_until('\n');
        let trimmed_line = line.trim();
        if !trimmed_line.is_empty() {
            let (cmd, leftover) = Command::parse(trimmed_line);
            assert!(leftover.is_empty(), "Leftover after parsing: <{}>", leftover);
            println!("Parsed command: {:?}", cmd);
            cmd.run(Redir::new()).iter().for_each(|t| t.join());
        }
    }
    println!("Finish shell");
}

fn read_until(c: char) -> String {
    let mut buff = [0u8; 1024];
    for i in 0..buff.len() {
        sys_read(1, &mut buff[i..i+1]).unwrap();
        if buff[i] == c as u8 {
            return String::from_utf8(buff[..i+1].to_vec()).unwrap();
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
