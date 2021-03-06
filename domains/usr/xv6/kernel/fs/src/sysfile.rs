//
// File-system system calls.
// Arguments are checked
//

use alloc::sync::Arc;
use core::sync::atomic::AtomicUsize;

pub use interface::vfs::{DirectoryEntry, ErrorKind, FileMode, FileStat, Result, NFILE};

use crate::cross_thread_temp_store::CrossThreadTempStorage;
use crate::icache::{ICache, INode, INodeFileType, ICACHE};
use crate::log::LOG;
use crate::opened_file::{FDTable, FileType, OpenedFile, FD_TABLE};
use crate::params;
use crate::pipe::Pipe;

// TODO: We need to duplicate the file object. Fix this
pub fn sys_dup(fd: usize) -> Result<usize> {
    // console::println!("sys_dup {}", fd);
    FD_TABLE.with(|fdtable| {
        let f = fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .as_mut()
            .ok_or(ErrorKind::InvalidFileDescriptor)?;
        let f1 = f.clone();
        Ok(_fdalloc(fdtable, f1)?)
    })
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> Result<usize> {
    // console::println!("sys_read {} {}", fd, buffer.len());
    FD_TABLE.with(|fdtable| {
        fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .as_mut()
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .read(buffer)
    })
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> Result<usize> {
    // console::println!("sys_write {} {}", fd, buffer.len());
    FD_TABLE.with(|fdtable| {
        fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .as_mut()
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .write(buffer)
    })
}

pub fn sys_close(fd: usize) -> Result<()> {
    FD_TABLE.with(|fdtable| {
        let f = fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .take()
            .ok_or(ErrorKind::InvalidFileDescriptor)?;
        drop(f);
        Ok(())
    })
}

pub fn sys_seek(fd: usize, offset: usize) -> Result<()> {
    FD_TABLE.with(|fdtable| {
        fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .as_mut()
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .seek(offset)
    })
}

pub fn sys_fstat(fd: usize) -> Result<FileStat> {
    // console::println!("sys_fstat {}", fd);
    FD_TABLE.with(|fdtable| {
        fdtable
            .get_mut(fd)
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .as_mut()
            .ok_or(ErrorKind::InvalidFileDescriptor)?
            .stat()
    })
}

// TODO(tianjiao): this can be cleaned up a bit
pub fn sys_link(old_path: &str, new_path: &str) -> Result<()> {
    // console::println!("sys_link {} {}", old_path, new_path);
    let mut trans = LOG.r#try().unwrap().begin_transaction();
    let inode = ICache::namei(&mut trans, old_path)?;
    let mut iguard = inode.lock();
    if iguard.data.file_type == INodeFileType::Directory {
        drop(iguard);
        ICache::put(&mut trans, inode);
        return Err(ErrorKind::InvalidParameter);
    }
    iguard.data.nlink += 1;
    iguard.update(&mut trans);
    drop(iguard);

    let (parent_inode, name) = ICache::nameiparent(&mut trans, new_path)?;
    let mut parent_iguard = parent_inode.lock();

    let result = parent_iguard.dirlink(&mut trans, name, inode.meta.inum);
    if result.is_err() {
        drop(parent_iguard);
        ICache::put(&mut trans, parent_inode);
        let mut iguard = inode.lock();
        iguard.data.nlink -= 1;
        iguard.update(&mut trans);
        drop(iguard);
        ICache::put(&mut trans, inode);
        return result;
    }

    drop(parent_iguard);
    ICache::put(&mut trans, parent_inode);
    ICache::put(&mut trans, inode);
    Ok(())
}

pub fn sys_unlink(path: &str) -> Result<()> {
    // console::println!("sys_unlink {}", path);
    let mut trans = LOG.r#try().unwrap().begin_transaction();
    let (parent_inode, name) = ICache::nameiparent(&mut trans, path)?;
    let mut parent_iguard = parent_inode.lock();
    if name == "." || name == ".." {
        drop(parent_iguard);
        ICache::put(&mut trans, parent_inode);
        return Err(ErrorKind::InvalidParameter);
    }

    let inode = parent_iguard.dirlookup(&mut trans, name);
    if inode.is_err() {
        drop(parent_iguard);
        ICache::put(&mut trans, parent_inode);
        return Err(inode.err().unwrap());
    }

    let (offset, inode) = inode.unwrap();
    let mut iguard = inode.lock();
    if iguard.data.nlink < 1 {
        panic!("unlink: nlink < 1");
    }

    // If path is a dir, it must be empty;
    if iguard.data.file_type == INodeFileType::Directory && !iguard.is_dirempty(&mut trans)? {
        drop(iguard);
        ICache::put(&mut trans, inode);
        drop(parent_iguard);
        ICache::put(&mut trans, parent_inode);
        return Err(ErrorKind::InvalidParameter);
    }

    // Write an emptry entry
    let buffer = [0u8; core::mem::size_of::<DirectoryEntry>()];
    parent_iguard.write(&mut trans, &buffer, offset).unwrap();

    if iguard.data.file_type == INodeFileType::Directory {
        parent_iguard.data.nlink -= 1;
        parent_iguard.update(&mut trans);
    }
    drop(parent_iguard);

    iguard.data.nlink -= 1;
    iguard.update(&mut trans);
    drop(iguard);
    ICache::put(&mut trans, inode);

    Ok(())
}

pub fn sys_mknod(path: &str, major: i16, minor: i16) -> Result<()> {
    // console::println!("sys_mknod {} {}", major, minor);
    let mut trans = LOG.r#try().unwrap().begin_transaction();
    let inode = ICache::create(&mut trans, path, INodeFileType::Device, major, minor)?;
    ICache::put(&mut trans, inode);
    Ok(())
}

pub fn sys_open(path: &str, mode: FileMode) -> Result<usize> {
    // console::println!("sys_open {} {:?}", path, mode);
    let mut trans = LOG.r#try().unwrap().begin_transaction();
    let inode: Arc<INode> = match mode.contains(FileMode::CREATE) {
        true => ICache::create(&mut trans, path, INodeFileType::File, 0, 0),
        false => {
            let inode = ICache::namei(&mut trans, path)?;
            let is_directory = inode.lock().data.file_type == INodeFileType::Directory;
            if is_directory && (mode != FileMode::READ) {
                ICache::put(&mut trans, inode);
                Err(ErrorKind::PermissionDenied)
            } else {
                Ok(inode)
            }
        }
    }?;

    let iguard = inode.lock();

    if iguard.data.file_type == INodeFileType::Device
        && (iguard.data.major < 0 || iguard.data.major >= params::NDEV)
    {
        drop(iguard);
        ICache::put(&mut trans, inode);
        return Err(ErrorKind::InvalidMajor);
    }

    let file = match iguard.data.file_type {
        INodeFileType::Device => OpenedFile::new(
            FileType::Device {
                inode: inode.clone(),
                major: AtomicUsize::new(iguard.data.major as usize),
            },
            mode.contains(FileMode::READ),
            mode.contains(FileMode::WRITE),
        ),
        _ => OpenedFile::new(
            FileType::INode {
                inode: inode.clone(),
                offset: AtomicUsize::new(0),
            },
            mode.contains(FileMode::READ),
            mode.contains(FileMode::WRITE),
        ),
    };

    let fd = fdalloc(Arc::new(file));

    drop(iguard);

    fd
}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn _fdalloc(fd_table: &mut FDTable, f: Arc<OpenedFile>) -> Result<usize> {
    let fd = fd_table
        .iter()
        .position(|f| f.is_none())
        .ok_or(ErrorKind::TooManyOpenedFiles)?;
    fd_table[fd].replace(f);
    Ok(fd)
}

fn fdalloc(f: Arc<OpenedFile>) -> Result<usize> {
    FD_TABLE.with(|fd_table| _fdalloc(fd_table, f))
}

pub fn sys_pipe() -> Result<(usize, usize)> {
    let (rf, wf) = Pipe::pipealloc();

    // We need a custom fdalloc here because we want rf back if fdalloc failed
    let fd0 = FD_TABLE.with(|fdtable| {
        let fd = match fdtable.iter().position(|f| f.is_none()) {
            Some(fd) => fd,
            None => return Err(ErrorKind::TooManyOpenedFiles),
        };
        fdtable[fd].replace(rf);
        Ok(fd)
    });

    let fd0 = match fd0 {
        Ok(fd) => fd,
        Err(e) => return Err(e),
    };

    let fd1 = match fdalloc(wf) {
        Ok(fd) => fd,
        Err(e) => {
            FD_TABLE.with(|fdtable| {
                fdtable[fd0].take().unwrap();
            });
            return Err(e);
        }
    };

    Ok((fd0, fd1))
}

pub fn sys_mkdir(path: &str) -> Result<()> {
    // console::println!("sys_mkdir {}", path);
    let mut trans = LOG.r#try().unwrap().begin_transaction();
    let inode = ICache::create(&mut trans, path, INodeFileType::Directory, 0, 0)?;
    ICache::put(&mut trans, inode);
    Ok(())
}

pub fn sys_dump_inode() -> Result<()> {
    let inode = ICACHE.lock().get(params::ROOTDEV, params::ROOTINO).unwrap();
    inode
        .lock()
        .print(&mut LOG.r#try().unwrap().begin_transaction(), 0);
    Ok(())
}

//------------------------------------
// fork related stuff
//------------------------------------
lazy_static! {
    static ref CROSS_THREAD_TEMP_STORE: CrossThreadTempStorage<FDTable> =
        CrossThreadTempStorage::new();
}

// TODO: save CWD
pub fn sys_save_threadlocal(fds: [Option<usize>; NFILE]) -> Result<usize> {
    FD_TABLE.with(|fdtable| {
        let mut new_fdtable: FDTable = array_init::array_init(|_| None);
        for (fd, ofile) in fds.iter().zip(new_fdtable.iter_mut()) {
            match fd {
                None => continue,
                Some(fd) => core::mem::swap(
                    ofile,
                    &mut fdtable
                        .get(*fd)
                        .ok_or(ErrorKind::InvalidFileDescriptor)?
                        .clone(),
                ),
            }
        }
        Ok(CROSS_THREAD_TEMP_STORE.put(new_fdtable))
    })
}

// TODO: set CWD
pub fn sys_set_threadlocal(id: usize) -> Result<()> {
    let mut fdtable = CROSS_THREAD_TEMP_STORE
        .get(id)
        .ok_or(ErrorKind::InvalidCTTSId)?;
    FD_TABLE.with(|my_fdtable| {
        my_fdtable
            .iter_mut()
            .zip(fdtable.iter_mut())
            .for_each(|(my_fd, fd)| {
                assert!(core::mem::replace(my_fd, fd.take()).is_none());
            });
        Ok(())
    })
}

// TODO: take care of CWD
pub fn sys_thread_exit() {
    FD_TABLE.drop()
}
