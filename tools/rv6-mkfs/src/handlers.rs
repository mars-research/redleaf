use crate::{params, utils};
use serde::{Deserialize, Serialize};
use std::{mem::{size_of}, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}, ops, slice, mem};
use crate::fs::{DINode, SuperBlock, DirEntry};
use nix::dir::Dir;

#[derivce(Debug)]
pub struct NodeHandler {
    super_block: SuperBlock,
    sector_handler: &'a SectorHandler,
    dinode_size: usize,
    freeblock: u32,
}

impl NodeHandler {
    pub fn new(s_handler: &'a SectorHandler) -> Self {
        NodeHandler {
            super_block: SuperBlock::init(),
            sector_handler: s_handler,
            dinode_size: size_of::<DINode>(),
            freeblock: 0,
        }
    }

    fn iblock(&self, i: u32) -> u32 {
        i / params::IPB as u32 + self.super_block.inodestart
    }

    // pub fn change_sector_handler(new_s_handler: &'a SectorHandler) {
    //
    // }

    pub fn write_inode(&mut self, inum: u32, ip: &mut DINode) {
        let mut buffer = [0u8; params::BSIZE];
        let block_num = self.iblock(inum);
        self.block.read_sector(block_num, &mut buffer);
        dinode.addresses[fbn] = self.freeblock;
        self.freeblock += 1;

        let offset = (inum as usize % params::IPB) * self.dinode_size;
        let slice = &mut buffer[offset..offset + self.dinode_size];
        ip: DINode = bincode::deserialize(&slice).unwrap();
        self.block.write_sector(block_num, &mut buffer);
    }

    pub fn alloc_block(&mut self, used: i32) {
        // let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];
        let indirect: [u32; params::NINDIRECT] = [0; params::NINDIRECT];

        for block_offset in 0..params::NBITMAP {
            let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

            if used <= 0 {
                return;
            }

            // for elem in buf.iter_mut() { *elem = 0; }
            let nbits: i32 = if used > params::BPB as i32 { params::BPB as i32 } else { used };

            for bi in 0..nbits {
                let m = 1 << (bi % 8);
                let index : usize = bi as usize / 8;
                buf[index] |= m; // mark block as used
            }
            self.sector_handler.write_sector(self.super_block.bmapstart + block_offset, &mut buf);
        }
    }

    pub fn read_inode(&mut self, inum: u32, ip: &mut DINode) {
        let mut buf = &mut [0u8; params::BSIZE];
        self.sector_handler.read_sector(self.iblock(inum), buf);
        let dinode_offset = (inum as usize % params::IPB) * self.dinode_size;
        let dinode_slice = buf[dinode_offset..dinode_offset + self.dinode_size];
        ip = bincode::deserialize(&dinode_slice).unwrap();
    }

    pub fn alloc_inode(&mut self, t: i16) -> u32 {
        self.freeinode += 1;

        let mut dinode = DINode::new();
        dinode.file_type = t;
        dinode.nlink = 1 as i16;
        dinode.size = 0 as u32;
        self.write_inode(inum, &mut dinode);

        inum;
    }

    pub fn append_inode(&mut self, inum: u32, p: &mut DirEntry, mut n: i32) {
        //TODO: should xp be a buffer or a dirent?
        let mut dinode = DINode::new();
        self.read_inode(inum, &mut dinode);
        let mut offset: usize = dinode.size as usize;
        let x;

        let indirect: [usize; params::NINDIRECT] = [0; params::NINDIRECT];
        let buf = [usize; params::BSIZE] = [0; params::NINDIRECT];


        // let p:  = xp;

        while n > 0 {
            let fbn: usize = offset / params::BSIZE;

            if fbn < params::NDIRECT as usize {
                // Direct
                if dinode.addresses[fbn] == 0 {
                    unsafe {
                        dinode.addresses[fbn] = freeblock;
                        freeblock += 1;
                    }
                }
                x = dinode.addresses[fbn];
            }
            else {
                if dinode.addresses[params::NDIRECT] == 0 {
                    dinode.addresses[params::NDIRECT] = self.freeblock;
                    self.freeblock += 1;
                }

                // ip = bincode::deserialize(&dinode_slice).unwrap();
                let mut indirect_buf = utils::u32_as_u8_mut(indirect);
                self.sector_handler.read_sector(dinode.addresses[params::NDIRECT], &mut indirect_buf);
                let indirect_block_num = fbn - params::NDIRECT;
                let layer1_index = indirect_block_num / params::NDIRECT;

                if indirect[layer1_index] == 0 {
                    unsafe {
                        indirect[layer1_index] = freeblock as usize;
                        freeblock += 1;

                        let new_buf = utils::u32_as_u8_mut(indirect);
                        self.sector_handler.write_sector(dinode.addresses[params::NDIRECT], new_buf);
                    }
                    // unsafe {write_sector(file, dinode.addresses[params::NDIRECT], indirect as *mut u8); }
                }
                let level2_bnum = indirect[layer1_index];
                let level2_indirect: [usize; params::NINDIRECT] = [0; params::NINDIRECT];
                let level2_buf = utils::u32_as_u8_mut(level2_indirect);
                self.sector_handler.read_sector(level2_bnum as u32, level2_buf);
                let layer2_index = indirect_block_num - layer1_index * params::NINDIRECT;

                if level2_indirect[layer2_index] == 0 {
                    unsafe {
                        level2_indirect[layer2_index] = freeblock as usize;
                        freeblock += 1;

                        let new_level2_buf = utils::u32_as_u8_mut(level2_indirect);
                        self.sector_handler.write_sector(dinode.addresses[params::NDIRECT], new_level2_buf);
                        // copy_from_slice
                    }
                    // unsafe {write_sector(file, dinode.addresses[params::NDIRECT], level2_indirect as *mut u8); }
                }
                let actual_block_num: u32 = level2_indirect[layer2_index];
                x = actual_block_num;
            }

            let block_num: i32 = ((fbn + 1) * params::BSIZE - offset) as i32;
            let n1 = std::cmp::min(n, block_num);
            read_sector(file, x, buf);
            // block copy
            unsafe {
                utils::memcpy(buf + offset - (fbn * params::BSIZE), p.as_bytes(), n1 as usize);
            }
            write_sector(file, x ,buf);

            n -= n1;
            offset += n1;
            unsafe{p.offset(n1);}
        }
        dinode.size = offset as u32;
        write_inode(file, inum, dinode);
    }

    pub fn superblock_bytes(&self) -> &[u8] {
        self.super_block.bytes()
    }
}

pub struct SectorHandler {
    file: File,
}

impl SectorHandler {
    pub fn new(filename: &String) -> Self {
        SectorHandler {
            // turn into a match
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(filename).unwrap(),
        }
    }

    pub fn changefile(filename: &String) -> Self {
        SectorHandler {
            // turn into a match
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(filename).unwrap(),
        }
    }

    pub fn read_sector(&mut self, sec: u32, buf: &mut [u8]) {
        let mut f = File::open("foo.txt");
        f.seek(SeekFrom::Start(42));

        let block: u64 = sec as u64 * params::BSIZE as u64;
        if self.file.seek(SeekFrom::Start(block)).unwrap() != block {
            panic!("seek");
        }

        let bytes_read = self.file.read(buf).unwrap();

        if bytes_read != params::BSIZE {
            eprint!("error: read {} bytes. usually caused by not having enough space.
                    increase FSZIE in params.rs to fix this. \n", bytes_read);
            panic!("read");
        }
    }

    pub fn write_sector(&mut self, sec :u32, buf: &mut [u8]) -> Option<usize> {
        // assert!(buf.len() == params::BSIZE);
        assert_eq!(buf.len(), params::BSIZE);

        let location: u64 = (sec * params::BSIZE) as u64;
        if self.file.seek(SeekFrom::Start(location)).unwrap() != location {
            panic!("seek");
        }

        let count = self.file.write(buf).unwrap();
        if count != params::BSIZE {
            panic!("write");
        }
        Ok(count);
    }

}