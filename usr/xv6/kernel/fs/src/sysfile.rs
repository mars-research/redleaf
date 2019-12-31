//
// File-system system calls.
// Arguments are checked
//

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use tls::ThreadLocal;
use crate::params;
use crate::file::{File, FileType};
use crate::fs::{ICache, INode, INodeFileType};

lazy_static! {
    static ref FD_TABLE: ThreadLocal<Vec<Option<Box<File>>>> = ThreadLocal::new(Vec::new);
}

#[derive(Copy, Clone, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    ReadWrite,
    Create
}

impl FileMode {
    fn readable(self) -> bool {
        match self {
            Self::Read | Self::ReadWrite => true,
            _ => false
        }
    }

    fn writable(self) -> bool {
        match self {
            Self::Write | Self::ReadWrite => true,
            _ => false
        }
    }
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

    let file: Box<File> = match iguard.data.file_type {
        Device => {
            Box::new(File::new(FileType::Device { inode: inode.clone(), major: iguard.data.major }, mode.readable(), mode.writable()))
        },
        _ => {
            Box::new(File::new(FileType::INode { inode: inode.clone(), offset: 0 }, mode.readable(), mode.writable()))
        }
    };

    let fd = fdalloc(file);

    drop(iguard);
    // TODO: log end_op here

    Some(fd)
}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fdalloc(f: Box<File>) -> usize {
    FD_TABLE.with({ |fd_table: &mut Vec<Option<Box<File>>>|
        match fd_table.iter().position(|f| f.is_none()) {
            Some(fd) => {
                fd_table[fd].replace(f);
                fd
            },
            None => {
                fd_table.push(Some(f));
                fd_table.len() - 1
            }
        }
    })
}
