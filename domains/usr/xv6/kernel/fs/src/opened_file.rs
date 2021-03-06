use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tls::ThreadLocal;
use interface::vfs::{ErrorKind, Result};

use crate::console_device::DEVICES;
use crate::icache::{ICache, INode};
use crate::log::LOG;
use crate::params;
use crate::pipe::Pipe;
use crate::sysfile::FileStat;

pub type FDTable = [Option<Arc<OpenedFile>>; params::NFILE];

lazy_static! {
    pub static ref FD_TABLE: ThreadLocal<FDTable> =
        ThreadLocal::new(|| array_init::array_init(|_| None));
}

// We want to avoid
#[derive(Debug)]
pub enum FileType {
    Pipe {
        pipe: Arc<Pipe>,
    },
    INode {
        inode: Arc<INode>,
        // Guarded by `ilock`
        offset: AtomicUsize,
    },
    Device {
        inode: Arc<INode>,
        // Set once then read-only
        major: AtomicUsize,
    },
    // Socket {
    //     socket: Option<Socket>,
    // },
}

#[derive(Debug)]
pub struct OpenedFile {
    pub file_type: FileType,
    // Set once
    pub readable: AtomicBool,
    // Set once
    pub writable: AtomicBool,
}

// xv6 equivalent: fileclose
impl Drop for OpenedFile {
    fn drop(&mut self) {
        match &self.file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => {
                let mut trans = LOG.r#try().unwrap().begin_transaction();
                ICache::put(&mut trans, inode.clone());
            }
            // TODO: pipe
            FileType::Pipe { pipe } => pipe.close(self.writable.load(Ordering::SeqCst)),
        }
    }
}

impl OpenedFile {
    pub fn new(file_type: FileType, readable: bool, writable: bool) -> Self {
        Self {
            file_type,
            readable: AtomicBool::new(readable),
            writable: AtomicBool::new(writable),
        }
    }

    pub fn seek(&self, new_offset: usize) -> Result<()> {
        match &self.file_type {
            FileType::INode { inode, offset } => {
                let _iguard = inode.lock();
                offset.store(new_offset, Ordering::SeqCst);
                Ok(())
            }
            _ => Err(ErrorKind::UnsupportedOperation),
        }
    }

    pub fn stat(&self) -> Result<FileStat> {
        match &self.file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => {
                Ok(inode.lock().stat())
            }
            _ => Err(ErrorKind::InvalidFileType),
        }
    }

    // Reads bytes from file into user_buffer.
    // Returns number of bytes read, or None if lacking read permissions or upon overflow.
    // xv6 equivalent: fileread
    pub fn read(&self, user_buffer: &mut [u8]) -> Result<usize> {
        if !self.readable.load(Ordering::SeqCst) {
            return Err(ErrorKind::PermissionDenied);
        }

        match &self.file_type {
            FileType::INode { inode, offset } => {
                let mut iguard = inode.lock();
                let mut trans = LOG.r#try().unwrap().begin_transaction();
                let bytes = iguard.read(&mut trans, user_buffer, offset.load(Ordering::SeqCst))?;
                offset.fetch_add(bytes, Ordering::SeqCst);
                Ok(bytes)
            }
            FileType::Device { inode: _, major } => {
                DEVICES
                    .get(major.load(Ordering::SeqCst))
                    .ok_or(ErrorKind::InvalidMajor)?
                    .as_ref()
                    .ok_or(ErrorKind::InvalidMajor)?
                    .read(user_buffer);
                Ok(user_buffer.len())
            }
            FileType::Pipe { pipe } => pipe.read(user_buffer),
        }
    }

    // Write bytes to file.
    // Returns number of bytes written, or None if lacking write permissions or upon overflow.
    // xv6 equivalent: filewrite
    pub fn write(&self, user_buffer: &[u8]) -> Result<usize> {
        if !self.writable.load(Ordering::SeqCst) {
            return Err(ErrorKind::PermissionDenied);
        }

        match &self.file_type {
            FileType::INode { inode, offset } => {
                let max = (params::MAXOPBLOCKS - 1 - 1 - 2 / 2) * params::BSIZE;
                let mut i = 0;
                while i < user_buffer.len() {
                    let bytes_to_write = core::cmp::min(user_buffer.len() - i, max);

                    {
                        let mut trans = LOG.r#try().unwrap().begin_transaction();
                        let mut iguard = inode.lock();
                        let bytes = iguard.write(
                            &mut trans,
                            &user_buffer[i..i + bytes_to_write],
                            offset.load(Ordering::SeqCst),
                        )?;
                        offset.fetch_add(bytes, Ordering::SeqCst);
                        i += bytes;
                    }
                }
                assert!(i == user_buffer.len());
                Ok(i)
            }
            FileType::Device { inode: _, major } => {
                DEVICES
                    .get(major.load(Ordering::SeqCst))
                    .ok_or(ErrorKind::InvalidMajor)?
                    .as_ref()
                    .ok_or(ErrorKind::InvalidMajor)?
                    .write(user_buffer);
                Ok(user_buffer.len())
            }
            FileType::Pipe { pipe } => pipe.write(user_buffer),
        }
    }
}
