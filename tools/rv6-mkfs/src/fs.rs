use crate::{params, inode::DINode, layer::{Layer, LayerType}};
use std::{slice, mem, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}};
use std::path::Path;

#[derive(Debug, Copy, Clone)]
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
    pub bmapstart: u32, // Block number of first free map block
}

impl SuperBlock {
    pub fn new() -> SuperBlock {
        SuperBlock {
            size: 0u32,
            nblocks: 0u32,
            ninodes: 0u32,
            nlog: 0u32,
            logstart: 0u32,
            inodestart: 0u32,
            bmapstart: 0u32
        }
    }
    pub fn init() -> SuperBlock {
        let offset = 2;
        let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
        let nblocks: usize = params::FSSIZE - nmeta;

        SuperBlock {
            size: params::FSSIZE as u32,
            nblocks: nblocks as u32,
            ninodes: params::NINODES as u32,
            nlog: params::LOGSIZE as u32,
            logstart: offset,
            inodestart: offset + params::LOGSIZE as u32,
            bmapstart: offset + params::LOGSIZE as u32 + params::NINODEBLOCKS as u32,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const SuperBlock as *const u8, mem::size_of::<SuperBlock>())
                as &[u8]
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

    pub fn bytes(&mut self) -> & mut [u8] {
        unsafe {
            slice::from_raw_parts_mut( self as *mut DirEntry as *mut u8, mem::size_of::<DirEntry>())
                as &mut [u8]
        }
    }
}

#[derive(Debug)]
pub struct FSHandler {
    super_block: SuperBlock,
    sector_handler: SectorHandler,
    dinode_size: usize,
    pub(crate) freeblock: u32,
    freeinode: u32,
}

impl FSHandler {
    pub fn new(s: &String) -> Self {
        FSHandler {
            super_block: SuperBlock::init(),
            sector_handler: SectorHandler::new(s),
            dinode_size: mem::size_of::<DINode>(),
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

        let mut indirect = Layer::new(LayerType::Indirect);
        let mut buf = Layer::new(LayerType::Block);

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

                self.read_file(dinode.addresses[params::NDIRECT], indirect.as_mut_slice());

                let indirect_block_num = fbn - params::NDIRECT;
                let layer1_index = indirect_block_num / params::NINDIRECT;

                // TODO: Change for u8
                if indirect.is_block_empty(layer1_index) {
                    indirect.set(self.freeblock, layer1_index);
                    self.freeblock += 1;
                    self.write_file(dinode.addresses[params::NDIRECT], indirect.as_mut_slice());
                }

                let level2_bnum = indirect.get(layer1_index);

                // Layer 2 indirect
                let mut level2_indirect = Layer::new(LayerType::Indirect);
                self.read_file(level2_bnum as u32, level2_indirect.as_mut_slice());

                let layer2_index = indirect_block_num - layer1_index * params::NINDIRECT;

                if level2_indirect.is_block_empty(layer2_index) {
                    level2_indirect.set(self.freeblock, layer2_index);
                    self.freeblock += 1;
                    self.write_file(level2_bnum as u32, level2_indirect.as_mut_slice());
                }

                let actual_block_num: u32 =  level2_indirect.get(layer2_index);
                x = actual_block_num;

            }

            let block_num: i32 = ((fbn + 1) * params::BSIZE - offset) as i32;
            let n1 = std::cmp::min(n, block_num);
            self.read_file(x, buf.as_mut_slice());

            unsafe {
                std::ptr::copy_nonoverlapping(
                            p.offset(ptr_offset),
                            buf.as_mut_ptr().offset((offset - (fbn * params::BSIZE)) as isize),
                            n1 as usize);
            }

            self.write_file(x, buf.as_mut_slice());

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