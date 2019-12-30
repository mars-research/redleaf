//
// File-system system calls.
// Arguments are checked
//
use alloc::boxed::Box;
use core::cell::RefCell;

use tls::{ThreadLocal};
use libsyscalls::syscalls::{sys_get_thread_id};
use crate::fcntl::{O_RDONLY, O_WRONLY, O_RDWR, O_CREATE};
use crate::file::File;
use crate::params;


// Opened files of the current thread. Use file descriptors to index into this array.
// I put it in a thread_local variable in fs instead of a member variable of `Thread`
// because this helps isolating the two modules.
//#[thread_local]
//static FD_TABLE: RefCell<[Option<Box<File>>; params::NOFILE]> = RefCell::new(
//    [None, None, None, None, None, None, None, None,
//    None, None, None, None, None, None, None, None]
//);

lazy_static! {
    static ref FD_TABLE: ThreadLocal<u32, [Option<Box<File>>; params::NOFILE]> = ThreadLocal::new({ ||
        [None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None]
    });
}

// pub fn sys_open(path: &str, mode: FileMode) -> Option<usize> {
//     // TODO: log begin_op here
//     let inode: Option<Arc<INode>> = match mode {
//         FileMode::Create => {
//             if let Some(inode) = ICache::create(path, INodeFileType::File, 0, 0) {

//                 Some(inode)
//             } else {
//                 // TODO: log end_op here
//                 None
//             }
//         },
//         _ => {
//             if let Some(inode) = ICache::namei(path) {
//                 let is_directory = inode.lock().data.file_type == INodeFileType::Directory;
//                 if is_directory && mode != FileMode::Read {
//                     ICache::put(inode);
//                     // TODO: log end_op here
//                     None
//                 } else {
//                     Some(inode)
//                 }
//             } else {
//                 // TODO: log end_op here
//                 None
//             }
//         }
//     };

//     if inode.is_none() {
//         return None;
//     }

//     let inode = inode.unwrap();
//     let iguard = inode.lock();

//     if iguard.data.file_type == INodeFileType::Device && (iguard.data.major < 0 || iguard.data.major >= params::NDEV) {
//         drop(iguard);
//         ICache::put(inode);
//         // TODO: log end_op here
//         return None;
//     }

//     let file: Arc<File> = match iguard.data.file_type {
//         Device => {
//             Arc::new(File::new(FileType::Device { inode: inode.clone(), major: iguard.data.major }, mode.readable(), mode.writeable()))
//         },
//         _ => {
//             Arc::new(File::new(FileType::INode { inode: inode.clone(), offset: 0 }, mode.readable(), mode.writeable()))
//         }
//     };

//     let fd = unsafe { FDTABLE.alloc_fd(file) };

//     drop(iguard);
//     // TODO: log end_op here

//     Some(fd)
// }

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fdalloc(f: Box<File>) -> Option<usize> {
    let key = sys_get_thread_id();
    FD_TABLE.with(key, { |fd_table|
        fd_table
            .iter()
            .position(|f| f.is_none())
            .map(|fd| {
                fd_table[fd].replace(f);
                fd
            })
    })
}
