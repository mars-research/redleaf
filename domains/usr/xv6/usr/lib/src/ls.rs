#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;

use crate::syscalls::{sys_close, sys_fstat, sys_open_slice_slow, sys_read_slice_slow};
use crate::{eprintln, println};
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;
use interface::rv6::Rv6;
use interface::vfs::{DirectoryEntry, DirectoryEntryRef, FileMode, INodeFileType};
use syscalls::{Heap, Syscall};

pub fn main(args: &str) {
    println!("Starting rv6 ls with args: {}", args);

    let mut args = args.split_whitespace();
    assert!(args.next().is_some());
    let path = args.next().or(Some("")).unwrap();

    ls(path).unwrap();
}

fn ls(path: &str) -> Result<(), String> {
    println!("ls <{}>", path);
    let fd = sys_open_slice_slow(path, FileMode::READ)
        .map_err(|e| alloc::format!("ls: cannot open {}. {:?}", path, e))?;
    let stat = sys_fstat(fd).map_err(|e| alloc::format!("ls: cannot stat {}. {:?}", path, e))?;

    const DIRENT_SIZE: usize = core::mem::size_of::<DirectoryEntry>();
    let mut buffer = [0_u8; DIRENT_SIZE];
    match &stat.file_type {
        INodeFileType::File => {
            println!(
                "ls path:{} type:{:?} inum:{} size:{}",
                path, stat.file_type, stat.inum, stat.size
            );
        }
        INodeFileType::Directory => {
            // Assuming DIRENT_SIZE > 0
            while sys_read_slice_slow(fd, &mut buffer[..]).unwrap_or(0) == DIRENT_SIZE {
                let de = DirectoryEntryRef::from_bytes(&buffer[..]);
                if de.inum == 0 {
                    continue;
                }
                // null-terminated string to String
                let filename = utils::cstr::to_string(de.name)
                    .map_err(|_| String::from("ls: cannot convert filename to utf8 string"))?;
                let file_path = alloc::format!("{}/{}", path, filename);
                let file_fd = sys_open_slice_slow(&file_path, FileMode::READ)
                    .map_err(|e| alloc::format!("ls: cannot open {} {:?}", file_path, e))?;
                let file_stat = sys_fstat(file_fd)
                    .map_err(|e| alloc::format!("ls: cannot stat {} {:?}", file_path, e))?;
                sys_close(file_fd)
                    .map_err(|e| alloc::format!("ls: cannot close {} {} {:?}", file_path, fd, e))?;
                println!(
                    "ls: path:{} type:{:?} inum:{} size:{}",
                    file_path, file_stat.file_type, file_stat.inum, file_stat.size
                );
            }
        }
        _ => unimplemented!(),
    }

    Ok(())
}
