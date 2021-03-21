use crate::{params, utils};
// use serde::{Deserialize, Serialize};
use std::{mem::{size_of}, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}, ops, slice, mem};
use crate::fs::{DINode, SuperBlock, DirEntry};
use std::path::Path;
use byteorder::ByteOrder;

// use nix::dir::Dir;

#[derive(Debug)]
pub struct FSHandler {
    super_block: SuperBlock,
    sector_handler: SectorHandler,
    dinode_size: usize,
    pub freeblock: u32,
    freeinode: u32,
}

impl FSHandler {
    pub fn new(s: &String) -> Self {
        FSHandler {
            super_block: SuperBlock::init(),
            sector_handler: SectorHandler::new(s),
            dinode_size: size_of::<DINode>(),
            freeinode: 1,
            freeblock: 0,
        }
    }

    fn iblock(&self, i: u32) -> u32 {
        i / params::IPB as u32 + self.super_block.inodestart
    }


    pub fn write_inode(&mut self, inum: u32, ip: &DINode) {
        let mut buffer = [0u8; params::BSIZE];

        let block_num = self.iblock(inum);
        self.read_file(block_num, &mut buffer);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();


        let offset = (inum as usize % params::IPB) * DINODE_SIZE;
        let slice: &mut [u8] = &mut buffer[offset..offset + DINODE_SIZE];

        ip.to_bytes(slice);

        self.write_file(block_num, &mut buffer);
    }

    pub fn alloc_disk_block(&mut self, used: i32) {
        let indirect: [u32; params::NINDIRECT] = [0; params::NINDIRECT];

        for block_offset in 0..params::NBITMAP {
            let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

            if used <= 0 {
                return;
            }

            let nbits: i32 = if used > params::BPB as i32 { params::BPB as i32 } else { used };

            for bi in 0..nbits {
                let m = 1 << (bi % 8);
                let index : usize = bi as usize / 8;
                buf[index] |= m; // mark block as used
            }
            self.write_file(self.super_block.bmapstart + block_offset as u32, &mut buf);
        }
    }

    pub fn read_inode(&mut self, inum: u32, ip: &mut DINode) {
        let buf: &mut [u8; params::BSIZE] = &mut [0u8; params::BSIZE];
        self.read_file(self.iblock(inum), buf);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();
        let dinode_offset = (inum as usize % params::IPB) * DINODE_SIZE;

        unsafe {
            let dinode_slice = &buf[dinode_offset..dinode_offset + DINODE_SIZE];
            *ip = DINode::from_bytes(&dinode_slice);
        }
    }

    pub fn set_block(&mut self, idx: usize, buffer: &mut [u8]) {
        let index = idx * 4;
        byteorder::LittleEndian::write_u32(&mut buffer[index..index+4], self.freeblock);
        self.freeblock += 1;
    }

    pub fn get_block(&self, idx: usize, buffer: &[u8]) -> u32 {
        let index = idx * 4;
        byteorder::LittleEndian::read_u32(&buffer[index..index+4])
    }

    pub fn alloc_inode(&mut self, t: i16) -> u32 {
        let inum = self.freeinode;
        self.freeinode += 1;

        let mut dinode: DINode = DINode::new();
        dinode.file_type = t;
        dinode.nlink = 1 as i16;
        dinode.size = 0 as u32;
        self.write_inode(inum, &mut dinode);

        inum
    }

    pub fn append_inode(&mut self, inum: u32, xp: &mut [u8], mut n: i32) {
        let mut dinode: DINode = DINode::new();
        // println!("inum: {:?}", inum);
        self.read_inode(inum, &mut dinode);
        let mut offset: usize = dinode.size.clone() as usize;
        let mut ptr_offset = 0;
        let mut x;

        let p: *mut u8 = xp.as_mut_ptr();

        // start out with u8 buffer transform into u32 after reading from file
        let mut indirect: [u8; params::NINDIRECT * 4] = [0; params::NINDIRECT * 4];
        let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

        while n > 0 {
            let fbn: usize = offset / params::BSIZE;

            if fbn < params::NDIRECT as usize {
                // Direct
                if dinode.addresses[fbn] == 0 {
                    dinode.addresses[fbn] = self.freeblock;
                    self.freeblock += 1;
                }
                x = dinode.addresses[fbn];
            }
            else {
                // Layer 1 indirect
                if dinode.addresses[params::NDIRECT] == 0 {
                    dinode.addresses[params::NDIRECT] = self.freeblock;
                    self.freeblock += 1;
                }

                self.read_file(dinode.addresses[params::NDIRECT], &mut indirect);

                let indirect_block_num = fbn - params::NDIRECT;
                let layer1_index = indirect_block_num / params::NINDIRECT;

                // TODO: Change for u8
                if self.get_block(layer1_index, &indirect) == 0 {
                    self.set_block(layer1_index, &mut indirect);
                    self.write_file(dinode.addresses[params::NDIRECT], &mut indirect);
                }

                let level2_bnum = self.get_block(layer1_index, &indirect);
                // println!("{:?} | {:?} | {:?}", level2_bnum, indirect_block_num, self.freeblock);

                // Layer 2 indirect
                let mut level2_indirect: [u8; params::NINDIRECT * 4] = [0; params::NINDIRECT * 4];
                self.read_file(level2_bnum as u32, &mut level2_indirect);

                let layer2_index = indirect_block_num - layer1_index * params::NINDIRECT;

                if self.get_block(layer2_index, &level2_indirect) == 0 {
                    self.set_block(layer2_index, &mut level2_indirect);
                    self.write_file(level2_bnum as u32, &mut level2_indirect);
                }

                let actual_block_num: u32 = self.get_block(layer2_index, & level2_indirect);
                x = actual_block_num;

            }

            let block_num: i32 = ((fbn + 1) * params::BSIZE - offset) as i32;
            let n1 = std::cmp::min(n, block_num);
            self.read_file(x, &mut buf);

            unsafe {
                std::ptr::copy_nonoverlapping(
                            p.offset(ptr_offset),
                            buf.as_mut_ptr().offset((offset - (fbn * params::BSIZE)) as isize),
                            n1 as usize);
            }

            self.write_file(x, &mut buf);

            n -= n1;
            offset += (n1 as usize);
            ptr_offset += n1 as isize;

        }
        dinode.size = offset as u32;
        self.write_inode(inum, &dinode);
    }

    pub fn superblock_bytes(&self) -> &[u8] {
        self.super_block.bytes()
    }

    pub fn write_file(&mut self, sec :u32, buf: &mut [u8]) {
        self.sector_handler.write_sector(sec, buf);
    }
    pub fn read_file(&mut self, sec: u32, buf: &mut [u8]) {
        self.sector_handler.read_sector(sec, buf);
    }
}

#[derive(Debug)]
pub struct SectorHandler {
    file: File,
}

impl SectorHandler {
    pub fn new(filename: &String) -> Self {
        let p = Path::new(&filename);

        SectorHandler {
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p).unwrap(),
        }
    }

    pub fn read_sector(&mut self, sec: u32, buf: &mut [u8]) {
        let offset: u64 = sec as u64 * params::BSIZE as u64;

        if self.file.seek(SeekFrom::Start(offset)).unwrap() != offset {
            panic!("seek");
        }

        let bytes_read = self.file.read(buf).unwrap();
        if bytes_read != params::BSIZE {
            eprint!("error: read {} bytes. usually caused by not having enough space.
                    increase FSZIE in params.rs to fix this. \n", bytes_read);
            panic!("read");
        }
    }

    pub fn write_sector(&mut self, sec :u32, buf: &mut [u8])  {
        assert_eq!(buf.len(), params::BSIZE);

        let location: u64 = (sec as usize* params::BSIZE) as u64;
        if self.file.seek(SeekFrom::Start(location)).unwrap() != location {
            panic!("seek");
        }

        let count = self.file.write(buf).unwrap();
        if count != params::BSIZE {
            panic!("write");
        }
    }

}
