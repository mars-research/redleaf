use crate::filesystem::fs::{ICache, INode, Stat};
use crate::filesystem::params;
use alloc::sync::Arc;
use core::cell::RefCell;
use core::mem::{MaybeUninit, swap};
use spin::Mutex;

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

pub type FileHandle = Arc<RefCell<File>>;

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
                let mut arr = MaybeUninit::<[Arc<RefCell<File>>; params::NFILE]>::uninit();
                for i in 0..params::NFILE {
                    (arr.as_mut_ptr() as *mut Arc<RefCell<File>>).add(i).write(Arc::new(RefCell::new(File::new())));
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
        // files lock should be acquired so we can safely manipulate RefCell<File>
        let lock = self.files.lock();

        if Arc::strong_count(&file) > 2 {
            drop(file);
            return
        }

        // <=2 references, ie this pointer and the ftable's pointer, so actually close the file
        // this borrow will never panic because we hold the self.files lock, so borrow is exclusive
        let mut file_ref = file.borrow_mut();
        let mut file_type = FileType::None;

        // set file's type to None (since we are closing it), pull out file_type
        swap(&mut file_ref.file_type, &mut file_type);

        drop(file_ref);
        drop(lock);

        match file_type {
            FileType::INode { inode, .. } => {
                // TODO: log begin_op here
                ICache::put(inode);
                // TODO: log end_op here
            },
            FileType::Device { inode, .. } => {
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
        let lock = self.files.lock();

        let file_ref = file.borrow();
        match &file_ref.file_type {
            FileType::INode { inode, .. } | FileType::Device { inode, .. } => Some(inode.lock().stat()),
            _ => None
        }
    }
}
