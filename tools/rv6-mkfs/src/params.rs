use std::mem;
use crate::fs;

// https://github.com/mit-pdos/xv6-public/blob/master/param.h

pub const NINODES : usize = 200;
pub const BSIZE: usize = 4096;
pub const NFILE: usize = 100;
pub const NOFILE: usize = 16; // open files per process
pub const NINODE: usize = 50; // maximum number of active i-nodes
pub const NDEV: i16 = 10; // maximum major device number
pub const ROOTDEV: u32 = 1; // device number of file system root disk
pub const MAXOPBLOCKS: usize = 10; // max # of blocks any FS op writes
pub const LOGSIZE: usize = MAXOPBLOCKS * 3; // max data blocks in on-disk log
pub const NBUF: usize = MAXOPBLOCKS * 3; // size of disk block cache
pub const SECTOR_SIZE: usize = 512;

pub const BPB: usize = BSIZE * 8; // bits per block
pub const FSSIZE: usize = 1000; // size of file system in blocks

// https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/fs.h
pub const ROOTINO: u16 = 1; // root i-number
pub const FSMAGIC: usize = 0x1020_3040;
pub const NDIRECT: usize = 12;
pub const NINDIRECT: usize = BSIZE / mem::size_of::<u32>();
pub const MAXFILE: usize = NDIRECT + NINDIRECT * NINDIRECT;

pub const IPB: usize = BSIZE / mem::size_of::<fs::DINode>();
pub const DIRSIZ: usize = 14;
pub const NINODEBLOCKS: usize = NINODES / IPB + 1;
pub const NBITMAP: usize = FSSIZE / (BSIZE*8) + 1; 
