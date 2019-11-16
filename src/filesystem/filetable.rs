use crate::filesystem::fs::{ICache, INode, Stat};
use crate::filesystem::params;
use alloc::sync::Arc;
use core::mem::{MaybeUninit, swap};
use spin::{Mutex, RwLock};

pub enum FileType {
    None,
    Pipe, // TODO: { pipe: Arc<Pipe> }
    INode { inode: Arc<INode>, offset: u32 },
    Device { inode: Arc<INode>, major: i16 },
}

pub struct File {
    pub file_type: FileType,
    pub readable: bool,
    pub writable: bool,
}

pub type FileHandle = Arc<RwLock<File>>;

pub struct FileTable {
    files: Mutex<[FileHandle; params::NFILE]>,
}

impl File {
    pub fn new() -> File {
        File {
            file_type: FileType::None,
            readable: false,
            writable: false
        }
    }
}

impl FileTable {
    pub fn new() -> FileTable {
        FileTable {
            files: Mutex::new(unsafe {
                let mut arr = MaybeUninit::<[Arc<RwLock<File>>; params::NFILE]>::uninit();
                for i in 0..params::NFILE {
                    (arr.as_mut_ptr() as *mut Arc<RwLock<File>>).add(i).write(Arc::new(RwLock::new(File::new())));
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

        let mut file_type = FileType::None;

        {
            // <=2 references, ie this pointer and the ftable's pointer, so actually close the file
            let mut fguard = file.write();

            // set file's type to None (since we are closing it), pull out file_type
            swap(&mut fguard.file_type, &mut file_type);
        }

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

    // xv6 equivalent: filestat
    pub fn stat(&self, file: FileHandle) -> Option<Stat> {
        match &file.read().file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => Some(inode.lock().stat()),
            _ => None
        }
    }
}

lazy_static! {
    pub static ref FTABLE: FileTable = FileTable::new();
}
