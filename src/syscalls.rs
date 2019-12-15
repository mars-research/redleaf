use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use crate::filesystem::fs::{ICACHE, ICache, INode, INodeFileType, fsinit};
use crate::filesystem::file::{File, FileType, FDTABLE};
use crate::filesystem::params;
use usr::capabilities::Capability;
use usr::syscalls::{Syscall, FileMode};
use alloc::sync::Arc;

// A temperory function that we can use to init the fs in user land.
// TODO: delete this after we split fs into a seperate crate
pub fn init_fs_temp() {
    fsinit(0);
}

// Print a string 
pub fn sys_print(s: &str) {

    disable_irq();
    println!("{}", s);
    enable_irq(); 
}


// Yield to any thread
pub fn sys_yield() {

    disable_irq();
    println!("sys_yield"); 
    do_yield();
    enable_irq(); 
}

// Create a new thread
pub fn sys_create_thread(name: &str, func: extern fn()) -> Capability  {

    disable_irq();
    println!("sys_create_thread"); 
    let cap = create_thread(name, func);
    enable_irq();
    return cap;
}

pub fn sys_open(path: &str, mode: FileMode) -> Option<usize> {
    // TODO: log begin_op here
    let inode: Option<Arc<INode>> = match mode {
        FileMode::Create => {
            if let Some(inode) = ICache::create(path, INodeFileType::File, 0, 0) {

                Some(inode)
            } else {
                // TODO: log end_op here
                None
            }
        },
        _ => {
            if let Some(inode) = ICache::namei(path) {
                let is_directory = inode.lock().data.file_type == INodeFileType::Directory;
                if is_directory && mode != FileMode::Read {
                    ICache::put(inode);
                    // TODO: log end_op here
                    None
                } else {
                    Some(inode)
                }
            } else {
                // TODO: log end_op here
                None
            }
        }
    };

    if inode.is_none() {
        return None;
    }

    let inode = inode.unwrap();
    let iguard = inode.lock();

    if iguard.data.file_type == INodeFileType::Device && (iguard.data.major < 0 || iguard.data.major >= params::NDEV) {
        drop(iguard);
        ICache::put(inode);
        // TODO: log end_op here
        return None;
    }

    let file: Arc<File> = match iguard.data.file_type {
        Device => {
            Arc::new(File::new(FileType::Device { inode: inode.clone(), major: iguard.data.major }, mode.readable(), mode.writeable()))
        },
        _ => {
            Arc::new(File::new(FileType::INode { inode: inode.clone(), offset: 0 }, mode.readable(), mode.writeable()))
        }
    };

    let fd = unsafe { FDTABLE.alloc_fd(file) };

    drop(iguard);
    // TODO: log end_op here

    Some(fd)
}

pub static UKERN: Syscall = Syscall{
    sys_print,
    sys_yield,
    sys_create_thread,
    sys_open,
    init_fs_temp,
};
