use alloc::sync::Arc;
use spin::{Mutex, MutexGuard};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};
use core::ops::{Drop, Deref, DerefMut};
use core::convert::TryInto;
use crate::filesystem::params;
use crate::filesystem::bcache::{BCACHE, BufferBlock};

pub struct SuperBlock {
    pub size: u32,
    // Size of file system image (blocks)
    pub nblocks: u32,
    // Number of data blocks
    pub ninodes: u32,
    // Number of inodes.
    pub nlog: u32,
    // Number of log blocks
    pub logstart: u32,
    // Block number of first log block
    pub inodestart: u32,
    // Block number of first inode block
    pub bmapstart: u32,    // Block number of first free map block
}

pub struct INodeMeta {
    // Device number
    pub device: u32,
    // Inode number
    pub inum: u32,
    // inode has been read from disk?
    pub valid: AtomicBool,
}

#[repr(C)]
pub struct INodeData {
    // File type
    pub file_type: i16,
    // Major device number (T_DEVICE only)
    pub major: i16,
    // Minor device number (T_DEVICE only)
    pub minor: i16,
    // Number of links to inode in file system
    pub nlink: i16,
    // Size of file (bytes)
    pub size: u32,
    // Data block addresses
    pub addresses: [u32; params::NDIRECT+1],
}

pub type DINode = INodeData;

pub struct INodeDataGuard<'a> {
    node: &'a INode,
    data: MutexGuard<'a, INodeData>,
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
    fn update(&self) {
        // TODO: global superblock
        let super_block = get_super_block();

        let mut bguard = BCACHE.read(self.node.meta.device, block_num_for_node(self.node.meta.inum, &super_block));
        let buffer = bguard.lock();

        // TODO: work around unsafe
        let mut dinode = unsafe {
            &mut *(&buffer.data as *const BufferBlock as *mut BufferBlock as *mut DINode)
                .offset((self.node.meta.inum % params::IPB as u32) as isize)
        };

        dinode.file_type = self.data.file_type;
        dinode.major = self.data.major;
        dinode.minor = self.data.minor;
        dinode.nlink = self.data.nlink;
        dinode.size = self.data.size;
        dinode.addresses.copy_from_slice(&self.data.addresses);

        // TODO: log_write

        drop(buffer);
        BCACHE.release(&mut bguard);
    }

    // Discard contents of node
    // Only called when node has no links and no other in-memory references to it
    // xv6 equivalent: itrunc
    fn truncate(&mut self) {
        for i in 0..params::NDIRECT {
            if self.data.addresses[i] != 0 {
                ICache::free_block(self.node.meta.device, self.data.addresses[i]);
                self.data.addresses[i] = 0;
            }
        }

        if self.data.addresses[params::NDIRECT] != 0 {
            let mut bguard = BCACHE.read(self.node.meta.device, self.data.addresses[params::NDIRECT]);
            let buffer = bguard.lock();

            let mut chunks_iter = buffer.data.chunks_exact(core::mem::size_of::<u32>());
            for j in 0..params::NINDIRECT {
                if let chunk = chunks_iter.next().unwrap() {
                    let block = u32::from_ne_bytes(chunk.try_into().unwrap());
                    if block != 0 {
                        ICache::free_block(self.node.meta.device, block);
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
}

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
                valid: AtomicBool::new(false)
            },
            data: Mutex::new(INodeData {
                file_type: 0,
                major: 0,
                minor: 0,
                nlink: 0,
                size: 0,
                addresses: [0; params::NDIRECT+1]
            })
        }
    }

    // Locks node, reads from disk if necessary
    // xv6 equivalent: ilock(...)
    fn lock(&self) -> INodeDataGuard {
        let super_block = get_super_block();

        let mut data = self.data.lock();

        if !self.meta.valid.load(Ordering::Relaxed) {
            // if not valid, load from disk
            let mut bguard = BCACHE.read(self.meta.device, block_num_for_node(self.meta.inum, &super_block));
            let buffer = bguard.lock();

            // TODO: work around unsafe
            let dinode = unsafe {
                & *(&buffer.data as *const BufferBlock as *mut BufferBlock as *mut DINode)
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

            if dinode.file_type == 0 {
                // TODO: better error handling here
                panic!("ilock: no type");
            }
        }

        INodeDataGuard {
            node: &self,
            data: data
        }
    }
}

pub struct ICache {
    pub inodes: [Arc<INode>; params::NINODE],
}

// TODO: better name
fn block_num_for_node(inum: u32, super_block: &Arc<SuperBlock>) -> u32 {
    return inum / params::IPB as u32 + super_block.inodestart;
}

impl ICache {
    fn new() -> ICache {
        ICache {
            inodes: unsafe {
                let mut arr = MaybeUninit::<[Arc<INode>; params::NINODE]>::uninit();
                for i in 0..params::NINODE {
                    (arr.as_mut_ptr() as *mut Arc<INode>).add(i).write(Arc::new(INode::new()));
                }
                arr.assume_init()
            }
        }
    }

    // Allocate a node on device.
    // Looks for a free inode on disk, marks it as used
    pub fn alloc(&mut self, device: u32, file_type: i16) -> Option<Arc<INode>> {
        let super_block = get_super_block();
        for inum in 1..super_block.ninodes {
            let mut bguard = BCACHE.read(device, block_num_for_node(inum, &super_block));
            let buffer = bguard.lock();

            // TODO: work around unsafe
            let mut dinode = unsafe {
                &mut *(&buffer.data as *const BufferBlock as *mut BufferBlock as *mut DINode).offset((inum % params::IPB as u32) as isize)
            };
            if dinode.file_type == 0 { // free inode
                // memset to 0
                unsafe { core::ptr::write_bytes(dinode as *const DINode as *mut DINode, 0, 1); }
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
            if Arc::strong_count(inode) == 1 && inode.meta.device == device && inode.meta.inum == inum {
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

    // Frees a disk block
    // xv6 equivalent: bfree
    pub fn free_block(device: u32, block: u32) {
        let super_block = get_super_block();

        let mut bguard = BCACHE.read(device, block_num_for_node(block, &super_block));
        let mut buffer = bguard.lock();
        let bi = (block as usize) % params::BPB;
        let m = 1 << (bi % 8);
        if buffer.data[bi / 8] & m == 0 {
            panic!("freeing freed block");
        }
        buffer.data[bi / 8] &= !m;
        // TODO: log_write here
        drop(buffer);
        BCACHE.release(&mut bguard);
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
                inode_guard.data.file_type = 0;
                inode_guard.update();
                inode.meta.valid.store(false, Ordering::Relaxed);
            }
        }
        // make sure this reference is not used afterwards
        drop(inode);
    }
}

lazy_static! {
    pub static ref ICACHE: Mutex<ICache> = { Mutex::new(ICache::new()) };
}

// Hardcoded superblock
pub fn get_super_block() -> Arc<SuperBlock> {

    const NINODES: usize = 200;

    let nbitmap = params::FSSIZE / (params::BSIZE*8) + 1;
    let ninodeblocks = NINODES / params::IPB + 1;
    let nlog = params::LOGSIZE;

    // 1 fs block = 1 disk sector
    let nmeta = 2 + nlog + ninodeblocks + nbitmap;
    let nblocks = params::FSSIZE - nmeta;
    // TODO: ensure the encoding is intel's encoding
    Arc::new(SuperBlock {
        size: params::FSSIZE as u32,
        nblocks: nlog as u32,
        ninodes: NINODES as u32,
        nlog: nlog as u32,
        logstart: 2,
        inodestart: 2 + nlog as u32,
        bmapstart: (2 + nlog + ninodeblocks) as u32,
    })
}

fn test() {
    let mut icache = ICACHE.lock();
    let inode = icache.alloc(0, 1).unwrap();
    let mut inode_guard = inode.lock();
    inode_guard.data.addresses[0] = 10;
}
