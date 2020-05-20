#![no_std]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::string::String;

use usrlib::println;
use usrlib::syscalls::{sys_open, sys_fstat, sys_read, sys_write, sys_close};
use syscalls::{Syscall, Heap};
use libsyscalls::syscalls::sys_println;
use usr::xv6::Xv6;
use usr::vfs::{VFSPtr, DirectoryEntry, DirectoryEntryRef, INodeFileType, FileMode};

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone());
    println!("Starting rv6 wc with args: {}", args);

    let mut args = args.split_whitespace().peekable();
    assert!(args.next().is_some());

    if args.peek().is_none() {
        // Read from STDIN
        wc(0, "");
    }

    for arg in args {
        let fd = sys_open(arg, FileMode::READ).unwrap();
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
        let bytes_read = sys_read(fd, &mut buff).unwrap();
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
            } else if (!in_word) {
                word_cnt += 1;
                in_word = true;
            }
        }
    }

    println!("wc: line:{} word:{} char:{} name:{}", line_cnt, word_cnt, char_cnt, name);
    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("wc panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
