use std::path::Path;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    mem, slice,
};

use crate::inode::INodeFileType;
use crate::superblock::SuperBlock;
use crate::{
    inode::DINode,
    layer::{Layer, LayerType},
    params,
};

#[derive(Debug)]
pub struct FSHandler {
    super_block: SuperBlock,
    raw_disk_img: File,
    // sector_handler: SectorHandler,
    dinode_size: usize,
    freeblock: u32,
    freeinode: u32,
}

impl FSHandler {
    pub fn new(filename: &String) -> Self {
        let path = Path::new(&filename);

        FSHandler {
            super_block: SuperBlock::init(),
            raw_disk_img: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .unwrap(),
            dinode_size: mem::size_of::<DINode>(),
            freeinode: 1,
            freeblock: 0,
        }
    }

    fn iblock(&self, i: u32) -> u32 {
        i / params::IPB as u32 + self.super_block.inodestart
    }

    pub fn get_freeblock(&self) -> u32 {
        self.freeblock
    }

    pub fn set_freeblock(&mut self, rhs: u32) {
        self.freeblock = rhs;
    }

    pub fn alloc_disk_block(&mut self, blocks: i32) {
        let mut used = blocks;
        for block_offset in 0..params::NBITMAP {
            let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

            if used <= 0 {
                return;
            }

            let nbits: i32 = if used > params::BPB as i32 {
                params::BPB as i32
            } else {
                used.clone()
            };

            for bi in 0..nbits {
                let m = 1 << (bi % 8);
                let index: usize = bi as usize / 8;
                buf[index] |= m; // mark block as used
            }
            println!(
                "Block Alloc: write bitmap block at sector {}",
                self.super_block.bmapstart + block_offset as u32
            );
            self.write(self.super_block.bmapstart + block_offset as u32, &mut buf);
            used -= params::BPB as i32;
        }
        if used > 0 {
            panic!("Cannot allocate {} more blocks", used);
        }
    }

    pub fn write_inode(&mut self, inum: u32, inode: &DINode) {
        /// Writes inode onto the disk
        let mut buffer = [0u8; params::BSIZE];

        let block_num = self.iblock(inum);
        self.read(block_num, &mut buffer);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();

        let offset = (inum.clone() as usize % params::IPB) * DINODE_SIZE;
        let slice: &mut [u8] = &mut buffer[offset..offset + DINODE_SIZE];
        inode.to_bytes(slice);
        self.write(block_num, &mut buffer);
    }

    pub fn read_inode(&mut self, inum: u32, inode: &mut DINode) {
        /// Reads an inode from disk into inode
        let buf: &mut [u8; params::BSIZE] = &mut [0u8; params::BSIZE];
        self.read(self.iblock(inum), buf);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();
        let dinode_offset = (inum as usize % params::IPB) * DINODE_SIZE;

        unsafe {
            let dinode_slice = &buf[dinode_offset..dinode_offset + DINODE_SIZE];
            *inode = DINode::from_bytes(&dinode_slice);
        }
    }

    pub fn alloc_inode(&mut self, t: INodeFileType) -> u32 {
        /// Allocates a new inode and returns its inode number
        let inum = self.freeinode;
        self.freeinode += 1;

        let mut dinode: DINode = DINode::new();
        dinode.file_type = t;
        dinode.nlink = 1 as i16;
        dinode.size = 0 as u32;
        self.write_inode(inum, &mut dinode);

        inum
    }

    pub fn append_data_to_inode(&mut self, inum: u32, data: &mut [u8]) {
        /// Read the inode with inode number inum and append the contents of the buffer into it
        ///
        /// # Arguments
        /// * `inum` - The inode number of the inode we want to append data to
        /// * `data` - A buffer containing the data we wish to append onto the inode
        let mut dinode: DINode = DINode::new();

        // read inode number inum into the dinode
        self.read_inode(inum, &mut dinode);
        let mut offset: usize = dinode.size.clone() as usize;
        let mut ptr_offset = 0;
        let mut sector_num;

        let data_ptr: *mut u8 = data.as_mut_ptr();

        let mut indirect = Layer::new(LayerType::Indirect);

        let mut bytes_left = data.len();
        while bytes_left > 0 {
            let block_num: usize = offset / params::BSIZE;

            // if block number is still inside direct block
            if block_num < params::NDIRECT as usize {
                // If empty, allocate teh block by incrementing freeblock ptr
                if dinode.addresses[block_num] == 0 {
                    dinode.addresses[block_num] = self.freeblock;
                    self.freeblock += 1;
                }
                sector_num = dinode.addresses[block_num];
            } else {
                // Layer 1 indirect
                if dinode.addresses[params::NDIRECT] == 0 {
                    dinode.addresses[params::NDIRECT] = self.freeblock;
                    self.freeblock += 1;
                }
                // read the disk sector that contains the level1 indirect table
                self.read(dinode.addresses[params::NDIRECT], indirect.as_mut_slice());

                let indirect_block_num = block_num - params::NDIRECT;
                let layer1_index = indirect_block_num / params::NINDIRECT;

                // check if the entry is already allocated in the level1 indirect table
                if indirect.is_block_empty(layer1_index) {
                    // Not allocated; allocated a new block, update the level1 indirect table,
                    // and write the level1 indirect table back to disk
                    indirect.update(self.freeblock, layer1_index);
                    self.freeblock += 1;
                    self.write(dinode.addresses[params::NDIRECT], indirect.as_mut_slice());
                }

                let level2_block_num = indirect.get(layer1_index);

                // read the disk sector that contains the level2 indirect table into a buffer
                let mut level2_indirect = Layer::new(LayerType::Indirect);
                self.read(level2_block_num as u32, level2_indirect.as_mut_slice());

                let layer2_index = indirect_block_num - layer1_index * params::NINDIRECT;

                // check if the entry is already allocated in the level2 indirect table
                if level2_indirect.is_block_empty(layer2_index) {
                    // Not allocated; allocated a new block, update the level2 indirect table,
                    // and write the level2 indirect table back to disk
                    level2_indirect.update(self.freeblock, layer2_index);
                    self.freeblock += 1;
                    self.write(level2_block_num as u32, level2_indirect.as_mut_slice());
                }

                let actual_block_num: u32 = level2_indirect.get(layer2_index);
                sector_num = actual_block_num;
            }

            let sector: i32 = ((block_num + 1) * params::BSIZE - offset) as i32;
            let n1 = std::cmp::min(bytes_left, sector as usize);
            let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];
            self.read(sector_num, &mut buf);

            // copy_from_slice
            // slice is length n1 and make sure we take correct slice
            // copy data back into the buffer
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data_ptr.offset(ptr_offset),
                    buf.as_mut_ptr()
                        .offset((offset - (block_num * params::BSIZE)) as isize),
                    n1 as usize,
                );
            }

            // write the buffer back into the sector
            self.write(sector_num, &mut buf);

            bytes_left -= n1;
            offset += (n1 as usize);
            ptr_offset += n1 as isize;
        }
        // once all the bytes have been written,
        // update the inode's size and write back the inode
        dinode.size = offset as u32;
        self.write_inode(inum, &dinode);
    }

    pub fn superblock_bytes(&self) -> &[u8] {
        /// Returns the bytes of the superblock as a slice
        self.super_block.bytes()
    }

    pub fn read(&mut self, sec: u32, buf: &mut [u8]) {
        /// Reads a section of a  file on disk into a buffer
        ///
        /// # Arguments
        /// * `sec` - The location within the file to read from
        /// * `buf` - The buffer which the contents of the raw image will be read into
        let offset: u64 = sec as u64 * params::BSIZE as u64;

        if self.raw_disk_img.seek(SeekFrom::Start(offset)).unwrap() != offset {
            panic!("seek");
        }

        let bytes_read = self.raw_disk_img.read(buf).unwrap();
        if bytes_read != params::BSIZE {
            eprint!(
                "error: read {} bytes. usually caused by not having enough space.
                    increase FSZIE in params.rs to fix this. \n",
                bytes_read
            );
            panic!("read");
        }
    }

    pub fn write(&mut self, sec: u32, buf: &mut [u8]) {
        /// Writes the contents of the buffer into a section of a file on disk
        ///
        /// # Arguments
        /// * `sec` - The location within the file to write into
        /// * `buf` - The buffer whose contents will be written into the raw disk image
        assert_eq!(buf.len(), params::BSIZE);

        let location: u64 = (sec as usize * params::BSIZE) as u64;
        if self.raw_disk_img.seek(SeekFrom::Start(location)).unwrap() != location {
            panic!("seek");
        }

        let count = self.raw_disk_img.write(buf).unwrap();
        if count != params::BSIZE {
            panic!("write");
        }
    }
}

#[repr(C)]
pub struct DirEntry {
    inum: u16,
    pub name: [u8; params::DIRSIZE],
}

impl DirEntry {
    pub fn default() -> Self {
        DirEntry {
            inum: 0,
            name: [0; params::DIRSIZE],
        }
    }

    pub fn new(n: u16, string: &str) -> Self {
        let mut dir = DirEntry {
            inum: n,
            name: [0; params::DIRSIZE],
        };

        let str_bytes = string.as_bytes();
        let mut i = 0;
        for byte in str_bytes {
            dir.name[i] = *byte;
            i += 1;
        }
        dir
    }

    pub fn bytes(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut DirEntry as *mut u8, mem::size_of::<DirEntry>())
                as &mut [u8]
        }
    }
}
