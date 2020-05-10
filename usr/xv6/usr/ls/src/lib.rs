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

use usrlib::{eprintln, println};
use usrlib::syscalls::{sys_open, sys_fstat, sys_read, sys_write, sys_close};
use syscalls::{Syscall, Heap};
use libsyscalls::syscalls::sys_println;
use usr::xv6::Xv6;
use usr::vfs::{VFSPtr, DirectoryEntry, DirectoryEntryRef, INodeFileType, FileMode};

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6 + Send + Sync>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap);
    usrlib::init(rv6.clone());
    println!("Starting rv6 ls with args: {}", args);

    let mut args = args.split_whitespace();
    assert!(args.next().is_some());
    let path = args.next().or(Some("")).unwrap();

    ls(path).unwrap();
}


fn ls(path: &str) -> Result<(), String> {
    println!("ls <{}>", path);
    let fd = sys_open(path, FileMode::READ)
        .map_err(|e| alloc::format!("ls: cannot open {}. {:?}", path, e))?;
    let stat = sys_fstat(fd)
        .map_err(|e| alloc::format!("ls: cannot stat {}. {:?}", path, e))?;

    const DIRENT_SIZE: usize = core::mem::size_of::<DirectoryEntry>();
    let mut buffer = [0 as u8; DIRENT_SIZE];
    match &stat.file_type {
        INodeFileType::File => {
            println!("ls path:{} type:{:?} inum:{} size:{}", path, stat.file_type, stat.inum, stat.size);
        },
        INodeFileType::Directory => {
            // Assuming DIRENT_SIZE > 0
            while sys_read(fd, &mut buffer[..]).unwrap_or(0) == DIRENT_SIZE {
                let de = DirectoryEntryRef::from_bytes(&buffer[..]);
                if de.inum == 0 {
                    continue;
                }
                // null-terminated string to String
                let filename = utils::cstr::to_string(de.name)
                                .map_err(|_| String::from("ls: cannot convert filename to utf8 string"))?;
                let file_path = alloc::format!("{}/{}", path, filename);
                let file_fd = sys_open(&file_path, FileMode::READ)
                                .map_err(|e| alloc::format!("ls: cannot open {} {:?}", file_path, e))?;
                let file_stat = sys_fstat(file_fd)
                                .map_err(|e| alloc::format!("ls: cannot stat {} {:?}", file_path, e))?;
                sys_close(file_fd).map_err(|e| alloc::format!("ls: cannot close {} {}", file_path, fd))?;
                println!("ls: path:{} type:{:?} inum:{} size:{}", file_path, file_stat.file_type, file_stat.inum, file_stat.size);
            }
        }
        _ => unimplemented!(),
    }

    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("ls panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
