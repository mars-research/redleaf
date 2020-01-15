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
    thread_local,
    untagged_unions,
    panic_info_message
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
extern crate spin;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate num_derive;
extern crate byteorder;
extern crate syscalls;
extern crate tls;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;
use syscalls::Syscall;
use libsyscalls::time::get_rdtsc;

mod bcache;
mod block;
mod directory;
mod file;
mod fs;
mod icache;
mod log;
mod params;
mod sysfile;

struct VFS {}

impl VFS {
    fn new() -> VFS {
        VFS{}
    }
}

impl usr::vfs::VFS for VFS {}


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            proxy: Box<dyn usr::proxy::Proxy + Send + Sync>) -> Box<dyn usr::vfs::VFS> {
    libsyscalls::syscalls::init(s);
    libusr::proxy::init(proxy);

    println!("init xv6 filesystem");
    fs::fsinit(0);
    println!("finish init xv6 filesystem");
    ls("/").unwrap();
    fs_benchmark(512, "/big_file");
    Box::new(VFS::new()) 
}

fn fs_benchmark(buf_size: usize, path: &str) {
    let start = get_rdtsc();
    let fd = sysfile::sys_open(path, sysfile::FileMode::Read).unwrap();
    let mut buff = Vec::new();
    buff.resize(buf_size, 0 as u8);
    let mut bytes_read = 0;
    while let Some(sz) = sysfile::sys_read(fd, buff.as_mut_slice()) {
        bytes_read += sz;
        if sz < 512 {
            break;
        }
    }
    sysfile::sys_close(fd);
    let end = get_rdtsc();
    println!("we read {} bytes at a time, in total {} bytes from {} using {} cycles", buf_size, bytes_read, path, end - start);
}

fn ls(path: &str) -> Result<(), String> {
    let fd = sysfile::sys_open("/", sysfile::FileMode::Read)
        .ok_or(alloc::format!("ls: cannot open {}", path))?;
    let stat = sysfile::sys_fstat(fd)
        .ok_or(alloc::format!("ls: cannot stat {}", path))?;

    const DIRENT_SIZE: usize = core::mem::size_of::<directory::DirectoryEntryDisk>();
    let mut buffer = [0 as u8; DIRENT_SIZE];
    match &stat.file_type {
        icache::INodeFileType::File => {
            println!("ls path:{} type:{:?} inum:{} size:{}", path, stat.file_type, stat.inum, stat.size);
        },
        icache::INodeFileType::Directory => {
            // Assuming DIRENT_SIZE > 0
            while sysfile::sys_read(fd, &mut buffer[..]).unwrap_or(0) == DIRENT_SIZE {
                let de = directory::DirectoryEntry::from_byte_array(&buffer[..]);
                println!("ls de.inum: {:?} de.name {:X?}", de.inum, de.name);
                if de.inum == 0 {
                    continue;
                }
                // null-terminated string to String
                let filename = utils::cstr::to_string(de.name)
                                .map_err(|_| String::from("ls: cannot convert filename to utf8 string"))?;
                let file_path = alloc::format!("{}/{}", path, filename);
                let file_fd = sysfile::sys_open(&file_path, sysfile::FileMode::Read)
                                .ok_or(alloc::format!("ls: cannot open {}", file_path))?;
                let file_stat = sysfile::sys_fstat(file_fd)
                                .ok_or(alloc::format!("ls: cannot stat {}", file_path))?;
                println!("ls path:{} type:{:?} inum:{} size:{}", file_path, file_stat.file_type, file_stat.inum, file_stat.size);
            }
        }
        _ => unimplemented!(),
    }

    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6fs panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
