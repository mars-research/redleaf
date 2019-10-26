// https://github.com/mit-pdos/xv6-public/blob/master/param.h

pub const NPROC: usize =        64;     // maximum number of processes
pub const KSTACKSIZE: usize = 4096;     // size of per-process kernel stack
pub const NCPU: usize =          8;     // maximum number of CPUs
pub const NOFILE: usize =       16;     // open files per process
pub const NFILE: usize =       100;     // open files per system
pub const NINODE: usize =       50;     // maximum number of active i-nodes
pub const NDEV: usize =         10;     // maximum major device number
pub const ROOTDEV: usize =       1;     // device number of file system root disk
pub const MAXARG: usize =       32;     // max exec arguments
pub const MAXOPBLOCKS: usize =  10;     // max # of blocks any FS op writes
pub const LOGSIZE: usize =      (MAXOPBLOCKS*3);  // max data blocks in on-disk log
pub const NBUF: usize =         (MAXOPBLOCKS*3);  // size of disk block cache
pub const BSIZE: usize =        1024;   // block size
pub const FSSIZE: usize =       1000;   // size of file system in blocks

// Inodes per block.
// TODO: fix this, it should be (BSIZE / sizeof(struct dinode))
pub const IPB: usize =           BSIZE / 64;