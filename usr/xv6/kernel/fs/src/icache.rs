use alloc::sync::Arc;
use alloc::vec::Vec;
use byteorder::{ByteOrder, LittleEndian};
use core::convert::TryInto;
use core::mem;
use core::mem::MaybeUninit;
use core::ops::Drop;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::{Mutex, MutexGuard};

use crate::bcache::{BufferBlock, BCACHE};
use crate::block::Block;
use crate::directory::DirectoryEntry;
use crate::fs::{SUPER_BLOCK, block_num_for_node};
use crate::params;

#[derive(Debug)]
pub struct INodeMeta {
    // Device number
    pub device: u32,
    // Inode number
    pub inum: u32,
    // inode has been read from disk?
    pub valid: AtomicBool,
}

#[repr(C)]
#[derive(Debug)]
pub struct INodeData {
    // File type
    pub file_type: INodeFileType,
    // Major device number (T_DEVICE only)
    pub major: i16,
    // Minor device number (T_DEVICE only)
    pub minor: i16,
    // Number of links to inode in file system
    pub nlink: i16,
    // Size of file (bytes)
    pub size: u32,
    // Data block addresses
    pub addresses: [u32; params::NDIRECT + 1],
}

impl INodeData {
    pub fn to_bytes(&self, bytes: &mut [u8]) {
        let mut offset: usize = 0;
        LittleEndian::write_u16(&mut bytes[offset..], self.file_type as u16);
        offset += mem::size_of_val(&self.file_type);

        LittleEndian::write_i16(&mut bytes[offset..], self.major);
        offset += mem::size_of_val(&self.major);
        
        LittleEndian::write_i16(&mut bytes[offset..], self.minor);
        offset += mem::size_of_val(&self.minor);

        LittleEndian::write_i16(&mut bytes[offset..], self.nlink);
        offset += mem::size_of_val(&self.nlink);

        LittleEndian::write_u32(&mut bytes[offset..], self.size);
        offset += mem::size_of_val(&self.size);

        for a in &self.addresses {
            LittleEndian::write_u32(&mut bytes[offset..], *a);
            offset += mem::size_of_val(a);
        }
    }
}

pub type DINode = INodeData;

pub struct INodeDataGuard<'a> {
    pub node: &'a INode,
    pub data: MutexGuard<'a, INodeData>,
}

#[repr(u16)]
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum INodeFileType {
    // This is not a file type; it indicates that the inode is not initialized
    Unitialized,
    // Correspond to T_DIR in xv6
    Directory,
    // Correspond to T_FILE in xv6
    File,
    // Correspond to
    Device,
}

pub struct Stat {
    pub device: u32,
    pub inum: u32,
    pub file_type: INodeFileType,
    pub nlink: i16,
    pub size: u64,
}

impl<'a> Drop for INodeDataGuard<'a> {
    fn drop<'b>(&'b mut self) {
        // TODO: any cleanup needed?
    }
}

impl INodeDataGuard<'_> {
    // Copy a modified in-memory inode to disk (ie flush)
    // Call after every modification to Inode.data
    // xv6 equivalent: iupdate()
    pub fn update(&self) {
        // TODO: global superblock
        let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");

        let mut bguard = BCACHE.read(
            self.node.meta.device,
            block_num_for_node(self.node.meta.inum, &super_block),
        );
        let mut buffer = bguard.lock();

        const DINODE_SIZE: usize = mem::size_of::<DINode>();
        let dinode_offset = (self.node.meta.inum as usize % params::IPB) * DINODE_SIZE;
        self.data.to_bytes(&mut buffer.data[dinode_offset..dinode_offset + DINODE_SIZE]);

        // TODO: log_write

        drop(buffer);
        BCACHE.release(&mut bguard);
    }

    // Discard contents of node
    // Only called when node has no links and no other in-memory references to it
    // xv6 equivalent: itrunc
    pub fn truncate(&mut self) {
        for i in 0..params::NDIRECT {
            if self.data.addresses[i] != 0 {
                Block::free(self.node.meta.device, self.data.addresses[i]);
                self.data.addresses[i] = 0;
            }
        }

        if self.data.addresses[params::NDIRECT] != 0 {
            let mut bguard =
                BCACHE.read(self.node.meta.device, self.data.addresses[params::NDIRECT]);
            let buffer = bguard.lock();

            let mut chunks_iter = buffer.data.chunks_exact(core::mem::size_of::<u32>());
            for _ in 0..params::NINDIRECT {
                if let chunk = chunks_iter.next().unwrap() {
                    let block = u32::from_ne_bytes(chunk.try_into().unwrap());
                    if block != 0 {
                        Block::free(self.node.meta.device, block);
                    }
                }
            }
            drop(buffer);
            BCACHE.release(&mut bguard);

            self.data.addresses[params::NDIRECT] = 0;
        }

        self.data.size = 0;
        self.update();
    }

    // xv6 equivalent: stati
    pub fn stat(&self) -> Stat {
        Stat {
            device: self.node.meta.device,
            inum: self.node.meta.inum,
            file_type: self.data.file_type,
            nlink: self.data.nlink,
            size: self.data.size as u64,
        }
    }

    // The content (data) associated with each inode is stored
    // in blocks on the disk. The first NDIRECT block numbers
    // are listed in self.data.addresses. The next NINDIRECT blocks are
    // listed in block self.data.addresses[NDIRECT].
    // Return the disk block address of the nth block in self,
    // if there is no such block, block_map allocates one.
    // xv6 equivalent: bmap
    pub fn block_map(&mut self, block_number: u32) -> u32 {
        let block_number = block_number as usize;

        if block_number < params::NDIRECT {
            let mut address = self.data.addresses[block_number];
            if address == 0 {
                address = Block::alloc(self.node.meta.device).expect("Block::alloc out of blocks");
                self.data.addresses[block_number] = address;
            }
            return address;
        }

        let block_number = block_number - params::NDIRECT;

        if block_number < params::NINDIRECT {
            // Load indirect block, allocating if necessary.
            let mut address = self.data.addresses[params::NDIRECT];
            if address == 0 {
                address = Block::alloc(self.node.meta.device).expect("Block::alloc out of blocks");
                self.data.addresses[params::NDIRECT] = address;
            }

            let mut bguard = BCACHE.read(self.node.meta.device, address);
            let buffer = bguard.lock();

            // get 4-byte slice from offset block_number * 4
            let mut address = {
                let start_index = block_number * core::mem::size_of::<u32>();
                let end_index = (block_number + 1) * core::mem::size_of::<u32>();
                let chunk = &buffer.data[start_index..end_index];
                u32::from_ne_bytes(chunk.try_into().unwrap())
            };

            if address == 0 {
                address = Block::alloc(self.node.meta.device).expect("Block::alloc out of blocks");
                // TODO: log_write here
            }

            drop(buffer);
            BCACHE.release(&mut bguard);

            return address;
        }

        panic!("bmap: out of range");
    }

    // Look for a directory entry in a directory.
    // If found, set *poff to byte offset of entry(currently not supported).
    pub fn dirlookup(&mut self, name: &str) -> Option<Arc<INode>> {
        if self.data.file_type != INodeFileType::Directory {
            panic!("dirlookup not DIR");
        }

        const SIZE_OF_DIRENT: usize = core::mem::size_of::<DirectoryEntry>();
        for offset in (0usize..self.data.size as usize).step_by(SIZE_OF_DIRENT) {
            let mut buffer = [0; SIZE_OF_DIRENT];
            if self.read(&mut buffer[..], offset).is_none() {
                panic!("dirlookup read");
            }
            let dirent = DirectoryEntry::from_byte_array(&buffer[..]);
            if dirent.inum == 0 {
                continue;
            }
            if dirent.name == name.as_bytes() {
                return ICACHE.lock().get(self.node.meta.device, dirent.inum);
            }
        }

        None
    }

    // Write a new directory entry (name, inum) into the directory.
    pub fn dirlink(&mut self, name: &str, inum: u32) -> Result<(), &'static str> {
        // check that the name is not present
        if let Some(inode) = self.dirlookup(name) {
            ICache::put(inode);
            return Err("directory name already present");
        }

        // look for empty dirent
        const SIZE_OF_DIRENT: usize = core::mem::size_of::<DirectoryEntry>();
        let mut buffer = [0; SIZE_OF_DIRENT];

        for offset in (0usize..self.data.size as usize).step_by(SIZE_OF_DIRENT) {
            if self.read(&mut buffer[..], offset).is_none() {
                return Err("dirlink read");
            }
            let mut dirent = DirectoryEntry::from_byte_array(&buffer[..]);
            if dirent.inum == 0 {
                dirent.name = name.as_bytes().clone();
                dirent.inum = inum;

                buffer = dirent.as_bytes();
                if self.write(&mut buffer[..], offset).is_none() {
                    return Err("dirlink write");
                }
                return Ok(());
            }
        }

        Err("no empty directory entries")
    }

    // Read data from inode
    // Returns number of bytes read, or None upon overflow
    // xv6 equivalent: readi
    pub fn read(&mut self, user_buffer: &mut [u8], mut offset: usize) -> Option<usize> {
        let mut bytes_to_read = user_buffer.len();

        if offset as u32 > self.data.size || offset.checked_add(bytes_to_read).is_none() {
            return None;
        }

        if offset + bytes_to_read > self.data.size as usize {
            bytes_to_read = self.data.size as usize - offset;
        }

        let mut total = 0usize;
        let mut user_offset = 0usize;

        while total < bytes_to_read {
            let mut bguard = BCACHE.read(
                self.node.meta.device,
                self.block_map((offset / params::BSIZE) as u32),
            );
            let buffer = bguard.lock();

            let start = offset % params::BSIZE;
            let bytes_read = core::cmp::min(bytes_to_read - total, params::BSIZE - start);

            user_buffer[user_offset..].copy_from_slice(&buffer.data[start..(start + bytes_read)]);

            drop(buffer);
            BCACHE.release(&mut bguard);

            total += bytes_read;
            offset += bytes_read;
            user_offset += bytes_read;
        }

        Some(bytes_to_read)
    }

    // Write data to inode
    // Returns number of bytes written, or None upon overflow
    // xv6 equivalent: writei
    pub fn write(&mut self, user_buffer: &mut [u8], mut offset: usize) -> Option<usize> {
        let bytes_to_write = user_buffer.len();

        if offset as u32 > self.data.size || offset.checked_add(bytes_to_write).is_none() {
            return None;
        }

        if offset + bytes_to_write > params::MAXFILE * params::BSIZE {
            return None;
        }

        let mut total = 0usize;
        let mut user_offset = 0usize;

        while total < bytes_to_write {
            let mut bguard = BCACHE.read(
                self.node.meta.device,
                self.block_map((offset / params::BSIZE) as u32),
            );
            let mut buffer = bguard.lock();

            let start = offset % params::BSIZE;
            let bytes_written = core::cmp::min(bytes_to_write - total, params::BSIZE - start);

            buffer.data[start..]
                .copy_from_slice(&user_buffer[user_offset..(user_offset + bytes_written)]);

            // TODO: log_write here
            drop(buffer);
            BCACHE.release(&mut bguard);

            total += bytes_written;
            offset += bytes_written;
            user_offset += bytes_written;
        }

        if bytes_to_write > 0 {
            self.data.size = core::cmp::max(offset as u32, self.data.size);
            // write the node back to disk even if size didn't change, because block_map
            // could have added a new block to self.addresses
            self.update()
        }

        Some(bytes_to_write)
    }
}

#[derive(Debug)]
pub struct INode {
    pub meta: INodeMeta,
    pub data: Mutex<INodeData>,
}

impl INode {
    fn new() -> INode {
        INode {
            meta: INodeMeta {
                device: 0,
                inum: 0,
                valid: AtomicBool::new(false),
            },
            data: Mutex::new(INodeData {
                file_type: INodeFileType::Unitialized,
                major: 0,
                minor: 0,
                nlink: 0,
                size: 0,
                addresses: [0; params::NDIRECT + 1],
            }),
        }
    }

    // Locks node, reads from disk if necessary
    // xv6 equivalent: ilock(...)
    pub fn lock(&self) -> INodeDataGuard {
        let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");

        let mut data = self.data.lock();

        if !self.meta.valid.load(Ordering::Relaxed) {
            // if not valid, load from disk
            let mut bguard = BCACHE.read(
                self.meta.device,
                block_num_for_node(self.meta.inum, super_block),
            );
            let buffer = bguard.lock();

            // TODO: work around unsafe
            let dinode = unsafe {
                &*(&buffer.data as *const BufferBlock as *mut BufferBlock as *mut DINode)
                    .offset((self.meta.inum % params::IPB as u32) as isize)
            };

            data.file_type = dinode.file_type;
            data.major = dinode.major;
            data.minor = dinode.minor;
            data.nlink = dinode.nlink;
            data.size = dinode.size;
            data.addresses.copy_from_slice(&dinode.addresses);

            drop(buffer);
            BCACHE.release(&mut bguard);

            self.meta.valid.store(true, Ordering::Relaxed);

            if dinode.file_type == INodeFileType::Unitialized {
                // TODO: better error handling here
                panic!("ilock: no type");
            }
        }

        INodeDataGuard {
            node: &self,
            data: data,
        }
    }
}

pub struct ICache {
    pub inodes: [Arc<INode>; params::NINODE],
}

impl ICache {
    pub fn new() -> ICache {
        ICache {
            inodes: unsafe {
                let mut arr = MaybeUninit::<[Arc<INode>; params::NINODE]>::uninit();
                for i in 0..params::NINODE {
                    (arr.as_mut_ptr() as *mut Arc<INode>)
                        .add(i)
                        .write(Arc::new(INode::new()));
                }
                arr.assume_init()
            },
        }
    }

    // Allocate a node on device.
    // Looks for a free inode on disk, marks it as used
    pub fn alloc(&mut self, device: u32, file_type: INodeFileType) -> Option<Arc<INode>> {
        let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");
        for inum in 1..super_block.ninodes {
            let mut bguard = BCACHE.read(device, block_num_for_node(inum, super_block));
            let buffer = bguard.lock();

            // TODO: work around unsafe
            let mut dinode = unsafe {
                &mut *(&buffer.data as *const BufferBlock as *mut BufferBlock as *mut DINode)
                    .offset((inum % params::IPB as u32) as isize)
            };
            if dinode.file_type == INodeFileType::Unitialized {
                // free inode
                // memset to 0
                unsafe {
                    core::ptr::write_bytes(dinode as *const DINode as *mut DINode, 0, 1);
                }
                // setting file_type marks it as used
                dinode.file_type = file_type;
                // TODO: log_write here
                drop(buffer);
                BCACHE.release(&mut bguard);
                return self.get(device, inum);
            }
            drop(buffer);
            BCACHE.release(&mut bguard);
        }
        None
    }

    // Get in-memory inode matching device and inum. Does not read from disk.
    pub fn get(&mut self, device: u32, inum: u32) -> Option<Arc<INode>> {
        let mut empty: Option<&mut Arc<INode>> = None;
        for inode in self.inodes.iter_mut() {
            if Arc::strong_count(inode) == 1
                && inode.meta.device == device
                && inode.meta.inum == inum
            {
                return Some(inode.clone());
            }
            if empty.is_none() && Arc::strong_count(inode) == 1 {
                empty = Some(inode);
            }
        }

        match empty {
            None => return None,
            Some(node) => {
                // we just checked that strong_count == 1, and self is locked, so this should never fail
                let node_mut = Arc::get_mut(node).unwrap();
                node_mut.meta.device = device;
                node_mut.meta.inum = inum;
                node_mut.meta.valid.store(false, Ordering::Relaxed);
                return Some(node.clone());
            }
        }
    }

    // Corresponds to iput
    pub fn put(inode: Arc<INode>) {
        // TODO: race condition?
        if Arc::strong_count(&inode) == 2 && inode.meta.valid.load(Ordering::Relaxed) {
            // if this is the only reference (other than ICache), and it has no links,
            // then truncate and free

            // we already know inode is valid, so this is a cheap operation
            // TODO: ...right?
            let mut inode_guard = inode.lock();

            if inode_guard.data.nlink == 0 {
                inode_guard.truncate();
                inode_guard.data.file_type = INodeFileType::Unitialized;
                inode_guard.update();
                inode.meta.valid.store(false, Ordering::Relaxed);
            }
        }
        // make sure this reference is not used afterwards
        drop(inode);
    }

    // Look up and return the inode for a path.
    // If parent is true, return the inode for the parent and the final path element.
    // Must be called inside a transaction since it calls iput().
    fn namex(path: &str, parent: bool) -> Option<(Arc<INode>, &str)> {
        let mut inode: Arc<INode>;
        if path.starts_with("/") {
            if let Some(root) = ICACHE.lock().get(params::ROOTDEV, params::ROOTINO) {
                inode = root;
            } else {
                return None;
            }
        } else {
            unimplemented!("idup(myproc()->cwd)")
        }

        let components: Vec<&str> = path.split('/').filter(|n| !n.is_empty()).collect();

        let mut components_iter = components.iter().peekable();
        while let Some(component) = components_iter.next() {
            let mut iguard = inode.lock();

            // only the last path component can be a file
            if iguard.data.file_type != INodeFileType::Directory {
                drop(iguard);
                Self::put(inode);
                return None;
            }

            // return the parent of the last path component
            if parent && components_iter.peek().is_none() {
                drop(iguard);
                return Some((inode, component));
            }

            let next = iguard.dirlookup(component);
            drop(iguard);
            Self::put(inode);

            match next {
                Some(next) => inode = next,
                None => return None,
            }
        }

        if parent {
            Self::put(inode);
            return None;
        }

        // if we have a last component, return it along with the last inode
        Some((inode, components.last().unwrap_or(&"")))
    }

    pub fn namei(path: &str) -> Option<Arc<INode>> {
        Self::namex(path, false).map(|(inode, _)| inode)
    }

    pub fn nameiparent(path: &str) -> Option<(Arc<INode>, &str)> {
        Self::namex(path, true)
    }

    pub fn create(path: &str, file_type: INodeFileType, major: i16, minor: i16) -> Option<Arc<INode>> {
        return match ICache::nameiparent(path) {
            None => None,
            Some((dirnode, name)) => {
                // found parent directory
                let mut dirguard = dirnode.lock();

                match dirguard.dirlookup(name) {
                    Some(inode) => {
                        // full path already exists
                        drop(&mut dirguard);

                        let iguard = inode.lock();
                        if file_type == INodeFileType::File && iguard.data.file_type == INodeFileType::File {
                            return Some(inode.clone());
                        }
                        None
                    },
                    None => {
                        // create child
                        let inode = ICACHE.lock().alloc(dirnode.meta.device, file_type).expect("ICache alloc failed");

                        let mut iguard = inode.lock();
                        iguard.data.major = major;
                        iguard.data.minor = minor;
                        iguard.data.nlink = 1;
                        iguard.update();

                        if file_type == INodeFileType::Directory {
                            // create . and ..
                            dirguard.data.nlink += 1; // ..
                            dirguard.update();

                            if iguard.dirlink(".", inode.meta.inum).is_err() ||
                                iguard.dirlink("..", dirnode.meta.inum).is_err() {
                                // failed to add dot entries
                                return None;
                            }
                        }

                        dirguard.dirlink(name, inode.meta.inum);
                        drop(&mut dirguard);

                        Some(inode.clone())
                    }
                }
            }
        }
    }
}

lazy_static! {
    pub static ref ICACHE: Mutex<ICache> = { Mutex::new(ICache::new()) };
}



fn test() {
    let mut icache = ICACHE.lock();
    let inode = icache.alloc(0, INodeFileType::Directory).unwrap();
    let mut inode_guard = inode.lock();
    inode_guard.data.addresses[0] = 10;
    drop(inode_guard);
    ICache::put(inode);
}
