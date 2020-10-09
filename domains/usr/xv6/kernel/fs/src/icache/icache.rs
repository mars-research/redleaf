use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem;
use core::sync::atomic::Ordering;
use spin::{Mutex, MutexGuard};

use usr_interface::vfs::{ErrorKind, Result};

use super::inode::{DINode, INode, INodeFileType};
use crate::bcache::BCACHE;
use crate::cwd::CWD;
use crate::fs::{block_num_for_node, SUPER_BLOCK};
use crate::log::Transaction;
use crate::params;

pub struct ICache {
    pub inodes: [Arc<INode>; params::NINODE],
}

impl ICache {
    pub fn new() -> ICache {
        ICache {
            inodes: array_init::array_init(|_| core::default::Default::default()),
        }
    }

    // Allocate a node on device.
    // Looks for a free inode on disk, marks it as used
    pub fn alloc(
        &mut self,
        trans: &mut Transaction,
        device: u32,
        file_type: INodeFileType,
    ) -> Result<Arc<INode>> {
        let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");
        for inum in 1..super_block.ninodes as u16 {
            let bguard = BCACHE
                .r#try()
                .unwrap()
                .read(device, block_num_for_node(inum, super_block));
            let mut buffer = bguard.lock();

            // Okay, there're a lot of copying happening here but we don't have time to make it nice.
            const DINODE_SIZE: usize = mem::size_of::<DINode>();
            let dinode_offset = (inum as usize % params::IPB) * DINODE_SIZE;
            let dinode_slice = &mut buffer[dinode_offset..dinode_offset + DINODE_SIZE];
            let mut dinode = DINode::from_bytes(dinode_slice);

            if dinode.file_type == INodeFileType::Unitialized {
                // free inode
                // memset to 0
                dinode = DINode::new();
                // setting file_type marks it as used
                dinode.file_type = file_type;
                dinode.to_bytes(dinode_slice);
                trans.write(&bguard);

                drop(buffer);
                return self.get(device, inum);
            }
            drop(buffer);
        }
        Err(ErrorKind::OutOfINode)
    }

    // Get in-memory inode matching device and inum. Does not read from disk.
    pub fn get(&mut self, device: u32, inum: u16) -> Result<Arc<INode>> {
        let mut free_node: Option<&mut Arc<INode>> = None;
        for inode in self.inodes.iter_mut() {
            if Arc::strong_count(inode) == 1
                && inode.meta.device == device
                && inode.meta.inum == inum
            {
                return Ok(inode.clone());
            }
            if free_node.is_none() && Arc::strong_count(inode) == 1 {
                free_node = Some(inode);
            }
        }

        let node = free_node.ok_or(ErrorKind::ICacheExhausted)?;
        // we just checked that strong_count == 1, and self is locked, so this should never fail
        let node_mut = Arc::get_mut(node).unwrap();
        node_mut.meta.device = device;
        node_mut.meta.inum = inum;
        node_mut.meta.valid.store(false, Ordering::Relaxed);
        Ok(node.clone())
    }

    // Corresponds to iput
    pub fn put(trans: &mut Transaction, inode: Arc<INode>) {
        // TODO: race condition?
        if Arc::strong_count(&inode) == 2 && inode.meta.valid.load(Ordering::Relaxed) {
            // if this is the only reference (other than ICache), and it has no links,
            // then truncate and free

            // we already know inode is valid, so this is a cheap operation
            // TODO: ...right?
            let mut inode_guard = inode.lock();

            if inode_guard.data.nlink == 0 {
                inode_guard.truncate(trans);
                inode_guard.data.file_type = INodeFileType::Unitialized;
                inode_guard.update(trans);
                inode.meta.valid.store(false, Ordering::Relaxed);
            }
        }
        // make sure this reference is not used afterwards
        drop(inode);
    }

    // Look up and return the inode for a path.
    // If parent is true, return the inode for the parent and the final path element.
    // Must be called inside a transaction since it calls iput().
    fn namex<'a, 'b>(
        trans: &'a mut Transaction,
        path: &'b str,
        parent: bool,
    ) -> Result<(Arc<INode>, &'b str)> {
        let mut inode = if path.starts_with('/') {
            ICACHE.lock().get(params::ROOTDEV, params::ROOTINO)?
        } else {
            CWD.with(|cwd| cwd.clone())
        };

        let components: Vec<&str> = path.split('/').filter(|n| !n.is_empty()).collect();

        let mut components_iter = components.iter().peekable();
        while let Some(component) = components_iter.next() {
            let mut iguard = inode.lock();

            // only the last path component can be a file
            if iguard.data.file_type != INodeFileType::Directory {
                drop(iguard);
                Self::put(trans, inode);
                return Err(ErrorKind::InvalidFileType);
            }

            // return the parent of the last path component
            if parent && components_iter.peek().is_none() {
                drop(iguard);
                return Ok((inode, component));
            }

            let next = iguard.dirlookup(trans, component);
            drop(iguard);
            Self::put(trans, inode);

            match next {
                Ok((_, next)) => inode = next,
                Err(e) => return Err(e),
            }
        }

        if parent {
            Self::put(trans, inode);
            return Err(ErrorKind::FileNotFound);
        }

        // if we have a last component, return it along with the last inode
        Ok((inode, components.last().unwrap_or(&"")))
    }

    pub fn namei(trans: &mut Transaction, path: &str) -> Result<Arc<INode>> {
        Self::namex(trans, path, false).map(|(inode, _)| inode)
    }

    pub fn nameiparent<'a, 'b>(
        trans: &'a mut Transaction,
        path: &'b str,
    ) -> Result<(Arc<INode>, &'b str)> {
        Self::namex(trans, path, true)
    }

    pub fn create(
        trans: &mut Transaction,
        path: &str,
        file_type: INodeFileType,
        major: i16,
        minor: i16,
    ) -> Result<Arc<INode>> {
        let (dirnode, name) = ICache::nameiparent(trans, path)?;
        // found parent directory
        let mut dirguard = dirnode.lock();

        if let Ok((_, inode)) = dirguard.dirlookup(trans, name) {
            // full path already exists
            drop(&mut dirguard);

            let iguard = inode.lock();
            if file_type == INodeFileType::File && iguard.data.file_type == INodeFileType::File {
                return Ok(inode.clone());
            }
            return Err(ErrorKind::InvalidFileType);
        }

        // create child
        let inode = ICACHE
            .lock()
            .alloc(trans, dirnode.meta.device, file_type)
            .expect("ICache alloc failed");

        let mut iguard = inode.lock();
        iguard.data.major = major;
        iguard.data.minor = minor;
        iguard.data.nlink = 1;
        iguard.update(trans);

        if file_type == INodeFileType::Directory {
            // create . and ..
            dirguard.data.nlink += 1; // ..
            dirguard.update(trans);

            iguard.dirlink(trans, ".", inode.meta.inum)?;
            iguard.dirlink(trans, "..", dirnode.meta.inum)?;
        }

        dirguard.dirlink(trans, name, inode.meta.inum)?;
        drop(&mut dirguard);
        drop(iguard);

        Ok(inode)
    }
}

lazy_static! {
    pub static ref ICACHE: Mutex<ICache> = Mutex::new(ICache::new());
}
