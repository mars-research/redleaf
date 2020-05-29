use alloc::sync::Arc;
use byteorder::{ByteOrder, LittleEndian};
use core::convert::TryInto;
use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};
use num_traits::FromPrimitive;
use spin::{Mutex, MutexGuard};

pub use usr_interface::vfs::{DirectoryEntry, DirectoryEntryRef, INodeFileType};
use usr_interface::vfs::{ErrorKind, Result};

use crate::bcache::BCACHE;
use crate::block;
use crate::fs::{block_num_for_node, SUPER_BLOCK};
use crate::icache::{ICache, ICACHE};
use crate::log::Transaction;
use crate::params;
use crate::sysfile::FileStat;

#[derive(Debug)]
pub struct INodeMeta {
    // Device number
    pub device: u32,
    // Inode number
    pub inum: u16,
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
    pub fn new() -> Self {
        Self {
            file_type: INodeFileType::Unitialized,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addresses: [0; params::NDIRECT + 1],
        }
    }

    // TODO: A lot copying, fix it in the future
    pub fn copy_from_bytes(&mut self, bytes: &[u8]) {
        let mut offset: usize = 0;
        let file_type = LittleEndian::read_u16(&bytes[offset..]);
        self.file_type = FromPrimitive::from_u16(file_type).unwrap();
        offset += mem::size_of_val(&self.file_type);

        self.major = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.major);

        self.minor = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.minor);

        self.nlink = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.nlink);

        self.size = LittleEndian::read_u32(&bytes[offset..]);
        offset += mem::size_of_val(&self.size);

        for a in &mut self.addresses {
            *a = LittleEndian::read_u32(&bytes[offset..]);
            offset += mem::size_of_val(a);
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut dinode = Self::new();
        dinode.copy_from_bytes(bytes);
        dinode
    }

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

#[derive(Debug)]
pub struct INodeDataGuard<'a> {
    pub node: &'a INode,
    pub data: MutexGuard<'a, INodeData>,
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
    pub fn update(&self, trans: &mut Transaction) {
        // TODO: global superblock
        let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");

        let mut bguard = BCACHE.r#try().unwrap().read(
            self.node.meta.device,
            block_num_for_node(self.node.meta.inum, &super_block),
        );
        let mut buffer = bguard.lock();

        const DINODE_SIZE: usize = mem::size_of::<DINode>();
        let dinode_offset = (self.node.meta.inum as usize % params::IPB) * DINODE_SIZE;
        self.data
            .to_bytes(&mut buffer[dinode_offset..dinode_offset + DINODE_SIZE]);

        trans.write(&bguard);

        drop(buffer);
    }

    // Discard contents of node
    // Only called when node has no links and no other in-memory references to it
    // xv6 equivalent: itrunc
    pub fn truncate(&mut self, trans: &mut Transaction) {
        for i in 0..params::NDIRECT {
            if self.data.addresses[i] != 0 {
                block::free(trans, self.node.meta.device, self.data.addresses[i]);
                self.data.addresses[i] = 0;
            }
        }

        if self.data.addresses[params::NDIRECT] != 0 {
            let mut bguard = BCACHE
                .r#try()
                .unwrap()
                .read(self.node.meta.device, self.data.addresses[params::NDIRECT]);
            let buffer = bguard.lock();

            let mut chunks_iter = buffer.chunks_exact(core::mem::size_of::<u32>());
            for _ in 0..params::NINDIRECT {
                let chunk = chunks_iter.next().unwrap();
                let block = u32::from_ne_bytes(chunk.try_into().unwrap());
                if block != 0 {
                    block::free(trans, self.node.meta.device, block);
                }
            }
            drop(buffer);

            self.data.addresses[params::NDIRECT] = 0;
        }

        self.data.size = 0;
        self.update(trans);
    }

    // xv6 equivalent: stati
    pub fn stat(&self) -> FileStat {
        FileStat {
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
    pub fn block_map(&mut self, trans: &mut Transaction, block_number: u32) -> u32 {
        let block_number = block_number as usize;

        // From direct
        if block_number < params::NDIRECT {
            let mut address = self.data.addresses[block_number];
            if address == 0 {
                address =
                    block::alloc(trans, self.node.meta.device).expect("block::alloc out of blocks");
                self.data.addresses[block_number] = address;
            }
            return address;
        }

        // From a 2-layer indirect table
        let block_number = block_number - params::NDIRECT;
        assert!(
            block_number < params::NINDIRECT * params::NINDIRECT,
            "bmap: out of range"
        );
        // Load level 1 indirect block, allocating if necessary.
        let mut address = self.data.addresses[params::NDIRECT];
        if address == 0 {
            address =
                block::alloc(trans, self.node.meta.device).expect("block::alloc out of blocks");
            self.data.addresses[params::NDIRECT] = address;
        }

        let mut bguard = BCACHE.r#try().unwrap().read(self.node.meta.device, address);
        let buffer = bguard.lock();

        // The index of the level 1 table entry that this block belongs to
        let table_index = block_number / params::NINDIRECT;
        // get 4-byte slice from offset block_number * 4
        let mut address = {
            let start_index = table_index * core::mem::size_of::<u32>();
            let end_index = (table_index + 1) * core::mem::size_of::<u32>();
            let chunk = &buffer[start_index..end_index];
            u32::from_ne_bytes(chunk.try_into().unwrap())
        };

        if address == 0 {
            address =
                block::alloc(trans, self.node.meta.device).expect("block::alloc out of blocks");
            trans.write(&bguard);
        }

        drop(buffer);

        // Load level 2 indirect block, allocating if necessary.
        let mut bguard = BCACHE.r#try().unwrap().read(self.node.meta.device, address);
        let buffer = bguard.lock();

        // The index of the level 1 table entry that this block belongs to
        let table_index = block_number - params::NINDIRECT * table_index;
        // get 4-byte slice from offset block_number * 4
        let mut address = {
            let start_index = table_index * core::mem::size_of::<u32>();
            let end_index = (table_index + 1) * core::mem::size_of::<u32>();
            let chunk = &buffer[start_index..end_index];
            u32::from_ne_bytes(chunk.try_into().unwrap())
        };

        if address == 0 {
            address =
                block::alloc(trans, self.node.meta.device).expect("block::alloc out of blocks");
            trans.write(&bguard);
        }

        drop(buffer);

        address
    }

    // Look for a directory entry in a directory.
    // If found, set *poff to byte offset of entry(currently not supported).
    pub fn dirlookup(&mut self, trans: &mut Transaction, name: &str) -> Result<Arc<INode>> {
        if self.data.file_type != INodeFileType::Directory {
            panic!("dirlookup not DIR");
        }

        const SIZE_OF_DIRENT: usize = core::mem::size_of::<DirectoryEntry>();
        for offset in (0usize..self.data.size as usize).step_by(SIZE_OF_DIRENT) {
            let mut buffer = [0; SIZE_OF_DIRENT];
            self.read(trans, &mut buffer[..], offset).unwrap();
            let dirent = DirectoryEntryRef::from_bytes(&buffer[..]);
            if dirent.inum == 0 {
                continue;
            }
            let dirent_name = utils::cstr::to_string(dirent.name).unwrap();
            if dirent_name == name {
                return ICACHE.lock().get(self.node.meta.device, dirent.inum);
            }
        }

        Err(ErrorKind::FileNotFound)
    }

    // Write a new directory entry (name, inum) into the directory.
    pub fn dirlink(&mut self, trans: &mut Transaction, name: &str, inum: u16) -> Result<()> {
        // check that the name is not present
        if let Ok(inode) = self.dirlookup(trans, name) {
            ICache::put(trans, inode);
            return Err(ErrorKind::FileAlreadyExists);
        }

        // look for empty dirent
        const SIZE_OF_DIRENT: usize = core::mem::size_of::<DirectoryEntry>();
        let mut buffer = [0; SIZE_OF_DIRENT];

        for offset in (0usize..self.data.size as usize).step_by(SIZE_OF_DIRENT) {
            self.read(trans, &mut buffer[..], offset)?;
            let mut dirent = DirectoryEntryRef::from_bytes(&buffer[..]);
            if dirent.inum == 0 {
                let mut cloned_name = name.as_bytes().clone().to_vec();
                for _ in cloned_name.len()..params::DIRSIZ {
                    cloned_name.push(0);
                }
                dirent.name = cloned_name.as_slice();
                dirent.inum = inum;

                buffer = dirent.as_bytes();
                self.write(trans, &mut buffer[..], offset)?;
                return Ok(());
            }
        }

        Err(ErrorKind::DirectoryExhausted)
    }

    // Read data from inode
    // Returns number of bytes read, or None upon overflow
    // xv6 equivalent: readi
    pub fn read(
        &mut self,
        trans: &mut Transaction,
        user_buffer: &mut [u8],
        mut offset: usize,
    ) -> Result<usize> {
        let mut bytes_to_read = user_buffer.len();

        // We ask Rust to always check overflow so we don't need to check it manually
        if offset + bytes_to_read > self.data.size as usize {
            bytes_to_read = self.data.size as usize - offset;
        }

        let mut total = 0usize;
        let mut user_offset = 0usize;

        while total < bytes_to_read {
            let mut bguard = BCACHE.r#try().unwrap().read(
                self.node.meta.device,
                self.block_map(trans, (offset / params::BSIZE) as u32),
            );
            let buffer = bguard.lock();

            let start = offset % params::BSIZE;
            let bytes_read = core::cmp::min(bytes_to_read - total, params::BSIZE - start);

            user_buffer[user_offset..(user_offset + bytes_read)]
                .copy_from_slice(&buffer[start..(start + bytes_read)]);

            drop(buffer);

            total += bytes_read;
            offset += bytes_read;
            user_offset += bytes_read;
        }

        Ok(bytes_to_read)
    }

    // Write data to inode
    // Returns number of bytes written, or None upon overflow
    // xv6 equivalent: writei
    pub fn write(
        &mut self,
        trans: &mut Transaction,
        user_buffer: &[u8],
        mut offset: usize,
    ) -> Result<usize> {
        let bytes_to_write = user_buffer.len();
        let mut total = 0usize;
        let mut user_offset = 0usize;

        while total < bytes_to_write {
            let mut bguard = BCACHE.r#try().unwrap().read(
                self.node.meta.device,
                self.block_map(trans, (offset / params::BSIZE) as u32),
            );
            let mut buffer = bguard.lock();

            let start = offset % params::BSIZE;
            let bytes_written = core::cmp::min(bytes_to_write - total, params::BSIZE - start);

            buffer[start..start + bytes_written]
                .copy_from_slice(&user_buffer[user_offset..(user_offset + bytes_written)]);

            trans.write(&bguard);
            drop(buffer);

            total += bytes_written;
            offset += bytes_written;
            user_offset += bytes_written;
        }

        if bytes_to_write > 0 {
            self.data.size = core::cmp::max(offset as u32, self.data.size);
            // write the node back to disk even if size didn't change, because block_map
            // could have added a new block to self.addresses
            self.update(trans)
        }

        Ok(bytes_to_write)
    }

    pub fn print(&mut self, trans: &mut Transaction, ident: usize) {
        use alloc::string::String;
        use console::println;

        let block_number = self.data.addresses[params::NDIRECT];
        println!(
            "{}inum:{} indirect: {}",
            core::iter::repeat(" ").take(ident).collect::<String>(),
            self.node.meta.inum,
            block_number
        );

        // From layer 1 indirect
        let mut bguard = BCACHE
            .r#try()
            .unwrap()
            .read(self.node.meta.device, block_number);
        let buffer = bguard.lock();
        for block_number in buffer.chunks(mem::size_of::<u32>()) {
            let block_number = u32::from_ne_bytes(block_number.try_into().unwrap());
            let ident = ident + 2;
            println!(
                "{}indirect: {}",
                core::iter::repeat(" ").take(ident).collect::<String>(),
                block_number
            );
            if block_number == 0 {
                break;
            }

            // From layer 2 indirect
            let mut bguard = BCACHE
                .r#try()
                .unwrap()
                .read(self.node.meta.device, block_number);
            let buffer = bguard.lock();
            for block_number in buffer.chunks(mem::size_of::<u32>()) {
                let block_number = u32::from_ne_bytes(block_number.try_into().unwrap());
                let ident = ident + 2;
                println!(
                    "{}direct: {}",
                    core::iter::repeat(" ").take(ident).collect::<String>(),
                    block_number
                );
                if block_number == 0 {
                    break;
                }
            }
            drop(buffer);
        }
        drop(buffer);

        if self.data.file_type != INodeFileType::Directory {
            return;
        }

        println!(
            "{}showing directory:",
            core::iter::repeat(" ").take(ident).collect::<String>()
        );
        const SIZE_OF_DIRENT: usize = core::mem::size_of::<DirectoryEntry>();
        for offset in (0usize..self.data.size as usize).step_by(SIZE_OF_DIRENT) {
            let ident = ident + 2;
            let mut buffer = [0; SIZE_OF_DIRENT];
            self.read(trans, &mut buffer[..], offset).unwrap();
            let dirent = DirectoryEntryRef::from_bytes(&buffer[..]);
            if dirent.inum == 0 {
                continue;
            }
            let dirent_name = utils::cstr::to_string(dirent.name);
            if dirent_name.is_err() {
                console::println!("dirlookup: warning. invalid filename");
                continue;
            }
            let dirent_name = dirent_name.unwrap();
            println!(
                "{}link: {}",
                core::iter::repeat(" ").take(ident).collect::<String>(),
                dirent_name
            );
            if dirent_name == "." || dirent_name == ".." {
                continue;
            }
            let inode = ICACHE
                .lock()
                .get(self.node.meta.device, dirent.inum)
                .unwrap();
            inode.lock().print(trans, ident + 2);
        }
    }
}

#[derive(Debug)]
pub struct INode {
    pub meta: INodeMeta,
    pub data: Mutex<INodeData>,
}

impl core::default::Default for INode {
    fn default() -> Self {
        Self::new()
    }
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
            let mut bguard = BCACHE.r#try().unwrap().read(
                self.meta.device,
                block_num_for_node(self.meta.inum, super_block),
            );
            let buffer = bguard.lock();

            const DINODE_SIZE: usize = mem::size_of::<DINode>();
            let dinode_offset = (self.meta.inum as usize % params::IPB) * DINODE_SIZE;
            data.copy_from_bytes(&buffer[dinode_offset..dinode_offset + DINODE_SIZE]);

            drop(buffer);

            self.meta.valid.store(true, Ordering::Relaxed);

            if data.file_type == INodeFileType::Unitialized {
                // TODO: better error handling here
                panic!("ilock: no type. {:?}, {:?}", self, data);
            }
        }

        // console::println!("ilock inode#{}: {:?}", self.meta.inum, data);
        INodeDataGuard { node: &self, data }
    }
}
