use crate::filesystem::fs::{ICache, INode, Stat};
use crate::filesystem::params;
use alloc::sync::Arc;
use core::mem::{MaybeUninit, swap};
use spin::Mutex;

pub enum FileType {
    Pipe, // TODO: { pipe: Arc<Pipe> }
    INode { inode: Arc<INode>, offset: usize },
    Device { inode: Arc<INode>, major: i16 },
}

pub struct File {
    pub file_type: Option<FileType>,
    pub readable: bool,
    pub writable: bool,
}

pub type FileHandle = Arc<Mutex<File>>;

pub struct FileTable {
    files: Mutex<[FileHandle; params::NFILE]>,
}

impl File {
    pub fn new() -> File {
        File {
            file_type: None,
            readable: false,
            writable: false
        }
    }
}

impl FileTable {
    pub fn new() -> FileTable {
        FileTable {
            files: Mutex::new(unsafe {
                let mut arr = MaybeUninit::<[FileHandle; params::NFILE]>::uninit();
                for i in 0..params::NFILE {
                    (arr.as_mut_ptr() as *mut FileHandle).add(i).write(Arc::new(Mutex::new(File::new())));
                }
                arr.assume_init()
            })
        }
    }

    // xv6 equivalent: filealloc
    pub fn allocate(&self) -> Option<FileHandle> {
        for file in self.files.lock().iter() {
            if Arc::strong_count(file) == 1 {
                return Some(file.clone());
            }
        }
        return None
    }

    // xv6 equivalent: fileclose
    pub fn close(&self, file: FileHandle) {
        if Arc::strong_count(&file) > 2 {
            drop(file);
            return
        }

        // <=2 references, ie this pointer and the ftable's pointer, so actually close the file

        let file_type = { file.lock().file_type.take() };

        if let Some(file_type) = file_type {
            match file_type {
                FileType::INode { inode, .. } | FileType::Device { inode, .. } => {
                    // TODO: log begin_op here
                    ICache::put(inode);
                    // TODO: log end_op here
                },
                // TODO: pipe
                _ => ()
            }
        }
    }

    // xv6 equivalent: filestat
    pub fn stat(&self, file: FileHandle) -> Option<Stat> {
        match &file.lock().file_type {
            Some(FileType::INode { inode, .. }) | Some(FileType::Device { inode, .. }) => Some(inode.lock().stat()),
            _ => None
        }
    }

    // Reads bytes from file into user_buffer.
    // Returns number of bytes read, or None if lacking read permissions or upon overflow.
    // xv6 equivalent: fileread
    pub fn read(&self, file: FileHandle, user_buffer: &mut [u8]) -> Option<usize> {
        let mut fguard = file.lock();

        if !fguard.readable {
            return None;
        }

        match &mut fguard.file_type {
            Some(FileType::INode { inode, offset }) => {
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
}

lazy_static! {
    pub static ref FTABLE: FileTable = FileTable::new();
}
