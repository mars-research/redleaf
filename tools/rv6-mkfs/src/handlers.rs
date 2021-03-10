use crate::{params, utils};
// use serde::{Deserialize, Serialize};
use std::{mem::{size_of}, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}, ops, slice, mem};
use crate::fs::{DINode, SuperBlock, DirEntry};
use std::path::Path;
// use nix::dir::Dir;

#[derive(Debug)]
pub struct NodeHandler {
    super_block: SuperBlock,
    sector_handler: SectorHandler,
    dinode_size: usize,
    pub freeblock: u32,
    freeinode: u32,
}

impl NodeHandler {
    pub fn new(s: &String) -> Self {
        NodeHandler {
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
        // println!("block num: {:?}", block_num);
        self.sector_handler.read_sector(block_num, &mut buffer);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();


        let offset = (inum as usize % params::IPB) * DINODE_SIZE;
        let slice: &mut [u8] = &mut buffer[offset..offset + DINODE_SIZE];

        // let temp: DINode = DINode::new_from(ip);
       // unsafe {
           // let bytes = utils::any_as_u8_slice(ip);
           // utils::fill(slice, bytes, bytes.len());

           // utils::memcpy(mut* slice, mut* bytes, bytes.len());
           // core::ptr::write(slice, bytes);
       // }
       //  utils::fill(slice, ip.to_bytes(slice), bytes.len());
       //  println!("after: {:p}", slice);

        ip.to_bytes(slice);
        // println!("before: {:p}", slice);

        self.sector_handler.write_sector(block_num, &mut buffer);
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
            self.sector_handler.write_sector(self.super_block.bmapstart + block_offset as u32, &mut buf);
        }
    }

    pub fn read_inode(&mut self, inum: u32, ip: &mut DINode) {
        // TODO investigate bug in reading inode
        let buf: &mut [u8; params::BSIZE] = &mut [0u8; params::BSIZE];
        self.sector_handler.read_sector(self.iblock(inum), buf);
        const DINODE_SIZE: usize = mem::size_of::<DINode>();
        let dinode_offset = (inum as usize % params::IPB) * DINODE_SIZE;
        unsafe {
            let dinode_slice = &buf[dinode_offset..dinode_offset + DINODE_SIZE];
            *ip = DINode::from_bytes(&dinode_slice);
            // println!("ip: {:?}", ip.size);
        }
        // ip = bincode::deserialize(&dinode_slice).unwrap();
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

        // eprintln!("{:p} | {:p}", p, xp);

        let mut indirect: [u32; params::NINDIRECT] = [0; params::NINDIRECT];
        let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];
        let mut good = false;
        // println!("append inum {:?} at off {:?} sz {:?}", inum, offset, n);

        while n > 0 {
            let fbn: usize = offset / params::BSIZE;
            // println!("fbn: {}", fbn);

            if fbn < params::NDIRECT as usize {
                // println!("inside");


                // Direct
                if dinode.addresses[fbn] == 0 {
                    dinode.addresses[fbn] = self.freeblock;
                    self.freeblock += 1;
                }
                x = dinode.addresses[fbn];
            }
            else {
                if dinode.addresses[params::NDIRECT] == 0 {
                    dinode.addresses[params::NDIRECT] = self.freeblock;
                    self.freeblock += 1;
                }
                let mut indirect_buf = utils::u32_as_u8_mut(&mut indirect);

                self.sector_handler.read_sector(dinode.addresses[params::NDIRECT], &mut indirect_buf);

                let indirect_block_num = fbn - params::NDIRECT;
                let layer1_index = indirect_block_num / params::NINDIRECT;

                // println!("ibn: {}, layer1_inidex: {}", indirect_block_num, layer1_index);

                if indirect[layer1_index] == 0 {
                    indirect[layer1_index] = self.freeblock;
                    self.freeblock += 1;

                    let new_buf = utils::u32_as_u8_mut(&mut indirect);

                    self.sector_handler.write_sector(dinode.addresses[params::NDIRECT], new_buf);
                    // println!("\n\n!freeblock\n\t{:?}", self.freeblock);

                }

                let level2_bnum = indirect[layer1_index];


                let mut level2_indirect: [u32; params::NINDIRECT] = [0; params::NINDIRECT];
                let level2_buf = utils::u32_as_u8_mut(&mut level2_indirect);
                self.sector_handler.read_sector(level2_bnum as u32, level2_buf);

                let layer2_index = indirect_block_num - layer1_index * params::NINDIRECT;

                if level2_indirect[layer2_index] == 0 {
                    // println!("inside");
                    level2_indirect[layer2_index] = self.freeblock;
                    self.freeblock += 1;

                    let new_level2_buf = utils::u32_as_u8_mut(&mut level2_indirect);

                    self.sector_handler.write_sector(level2_bnum as u32, new_level2_buf);
                }



                let actual_block_num: u32 = level2_indirect[layer2_index];
                x = actual_block_num;

            }

            let block_num: i32 = ((fbn + 1) * params::BSIZE - offset) as i32;
            let n1 = std::cmp::min(n, block_num);
            self.sector_handler.read_sector(x, &mut buf);

            unsafe {
                utils::memcpy(buf.as_mut_ptr().offset((offset - (fbn * params::BSIZE)) as isize),
                              p.offset(ptr_offset),
                              n1 as usize);
            }

            // if self.freeblock == 65 || self.freeblock == 66 {
            //     println!("x {:?}", x);
            // }
            self.sector_handler.write_sector(x, &mut buf);


            n -= n1;
            offset += (n1 as usize);
            // unsafe {
            //     eprintln!("{:p}", p.offset(ptr_offset));
            // }

            ptr_offset += n1 as isize;

            // eprintln!("{:?}", ptr_offset);

        }
        // println!("offset: {:?}", offset);
        dinode.size = offset as u32;
        // eprintln!("{:?}", dinode);
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
        println!("fname: {:?}", filename);
        println!("path: {:?}", p);

        SectorHandler {
            // TODO: turn into a match
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p).unwrap(),
        }
    }

    pub fn changefile(filename: &String) -> Self {
        let filename_path = Path::new(&filename);
        SectorHandler {
            // TODO: turn into a match
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(filename_path).unwrap(),
        }
    }

    pub fn read_sector(&mut self, sec: u32, buf: &mut [u8]) {
        // let mut f = File::open("foo.txt").unwrap();
        // self.file.seek(SeekFrom::Start((sec * params::BSIZE as u32) as u64));

        let offset: u64 = sec as u64 * params::BSIZE as u64;

        // println!("offset: {:?}, sec: {:?}, BSIZE: {:?}", offset, sec, params::BSIZE);
        if self.file.seek(SeekFrom::Start(offset)).unwrap() != offset {
            panic!("seek");
        }

        let bytes_read = self.file.read(buf).unwrap();
        // println!("{:?}", buf);

        if bytes_read != params::BSIZE {
            eprint!("error: read {} bytes. usually caused by not having enough space.
                    increase FSZIE in params.rs to fix this. \n", bytes_read);
            panic!("read");
        }
    }

    pub fn write_sector(&mut self, sec :u32, buf: &mut [u8])  {
        // assert!(buf.len() == params::BSIZE);
        assert_eq!(buf.len(), params::BSIZE);

        let location: u64 = (sec as usize* params::BSIZE) as u64;
        if self.file.seek(SeekFrom::Start(location)).unwrap() != location {
            panic!("seek");
        }

        let count = self.file.write(buf).unwrap();
        if count != params::BSIZE {
            panic!("write");
        }
        // Ok(count);
    }

}
