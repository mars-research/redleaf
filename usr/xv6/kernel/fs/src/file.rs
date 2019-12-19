use crate::filesystem::fs::{ICache, INode, Stat};
use crate::filesystem::params;
use alloc::sync::Arc;
use core::mem::{MaybeUninit, swap};
use alloc::vec::Vec;

#[derive(Debug)]
pub enum FileType {
    Pipe, // TODO: { pipe: Arc<Pipe> }
    INode { inode: Arc<INode>, offset: usize },
    Device { inode: Arc<INode>, major: i16 },
}

pub struct File {
    pub file_type: FileType,
    pub readable: bool,
    pub writable: bool,
}

pub struct FileDescriptorTable {
    pub files: Vec<Option<Arc<File>>>,
}

impl File {
    pub fn new(file_type: FileType, readable: bool, writable: bool) -> File {
        File {
            file_type,
            readable,
            writable
        }
    }

    pub fn close(self) {
        match self.file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => {
                // TODO: log begin_op here
                ICache::put(inode);
                // TODO: log end_op here
            },
            // TODO: pipe
            _ => ()
        }
    }

    pub fn stat(&self) -> Option<Stat> {
        match &self.file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => Some(inode.lock().stat()),
            _ => None
        }
    }

    // Reads bytes from file into user_buffer.
    // Returns number of bytes read, or None if lacking read permissions or upon overflow.
    // xv6 equivalent: fileread
    pub fn read(&mut self, user_buffer: &mut [u8]) -> Option<usize> {
        if !self.readable {
            return None;
        }

        match &mut self.file_type {
            FileType::INode { inode, offset } => {
                let mut iguard = inode.lock();
                if let Some(bytes) = iguard.read(user_buffer, *offset) {
                    *offset += bytes;
                    return Some(bytes);
                } else {
                    return None;
                }
            },
            // TODO: device, pipe
            _ => unimplemented!()
        }
    }

    // Write bytes to file.
    // Returns number of bytes written, or None if lacking write permissions or upon overflow.
    // xv6 equivalent: filewrite
    pub fn write(&mut self, user_buffer: &mut [u8]) -> Option<usize> {
        if !self.writable {
            return None;
        }

        match &mut self.file_type {
            FileType::INode { inode, offset } => {
                let max = (params::MAXOPBLOCKS-1-1-2 / 2) * params::BSIZE;
                let mut i = 0;
                while i < user_buffer.len() {
                    let bytes_to_write = core::cmp::min(user_buffer.len() - i, max);

                    {
                        // TODO: log begin_op
                        let mut iguard = inode.lock();
                        if let Some(bytes) = iguard.write(&mut user_buffer[i..i+bytes_to_write], *offset) {
                            *offset += bytes;
                            i += bytes;
                        }
                        // TODO: log end_op
                    }
                }
                if i == user_buffer.len() {
                    Some(i)
                } else {
                    None
                }
            },
            // TODO: device, pipe
            _ => unimplemented!()
        }
    }
}

impl FileDescriptorTable {
    // xv6 equivalent: fdalloc
    pub fn alloc_fd(&mut self, file: Arc<File>) -> usize {
        for (fd, f) in self.files.iter_mut().enumerate() {
            if f.is_none() {
                *f = Some(file.clone());
                return fd;
            }
        }
        self.files.push(Some(file.clone()));
        return self.files.len() - 1;
    }

    pub fn free_fd(&mut self, fd: usize) {
        self.files[fd] = None;
    }
}

// TODO: remove mutex here... can't figure out 'static mut' even with #[thread_local]
#[thread_local]
pub static mut FDTABLE: FileDescriptorTable = FileDescriptorTable { files: Vec::new() };
