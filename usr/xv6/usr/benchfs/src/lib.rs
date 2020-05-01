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
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::panic::PanicInfo;

use usrlib::println;
use usrlib::syscalls::{sys_open, sys_fstat, sys_read, sys_write, sys_close};
use syscalls::{Syscall, Heap};
use usr::xv6::Xv6;
use usr::vfs::{VFSPtr, DirectoryEntry, DirectoryEntryRef, INodeFileType, FileMode};

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6 + Send + Sync>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);
    usrlib::init(rv6.clone());
    println!("Starting rv6 benchfs with args: {}", args);

    let mut args = args.split_whitespace();
    args.next().unwrap();
    let options = args.next().unwrap_or("w");
    let file = args.next().unwrap_or("large");

    let sizes = [512, 1024, 4096, 8192, 16 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024, 16 * 1024 * 1024, 64 * 1024 * 1024];

    for bsize in sizes.iter() {
        let mut buffer = alloc::vec![123u8; *bsize];

        if options.contains('w') {
            let fd = sys_open(file, FileMode::Write | FileMode::Create).unwrap();

            // warm up
            sys_write(fd, buffer.as_slice()).unwrap();

            let start = rv6.sys_rdtsc();
            let mut total_size = 0;
            for _ in 0..1024 {
                if total_size > 64 * 1024 * 1024  { break; }
                let size = sys_write(fd, buffer.as_slice()).unwrap();
                total_size += size;
            }
            println!("Write: buffer size: {}, total bytes: {}, cycles: {}", bsize, total_size, rv6.sys_rdtsc() - start);
            
            sys_close(fd).unwrap();
        }

        if options.contains('r') {
            let fd = sys_open(file, FileMode::Read).unwrap();

            let start = rv6.sys_rdtsc();
            let mut total_size = 0;
            loop {
                let size = sys_read(fd, buffer.as_mut_slice()).unwrap();
                if size == 0 { break; }
                total_size += size;
            }
            println!("Read: buffer size: {}, total bytes: {}, cycles: {}", bsize, total_size, rv6.sys_rdtsc() - start);

            sys_close(fd).unwrap();
        }
    }
}


// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchfs panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
