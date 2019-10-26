// See https://github.com/mit-pdos/xv6-public/blob/master/log.c

use core::mem::size_of;

use crate::filesystem::fs::SuperBlock;
use crate::filesystem::params;

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
struct LogHeader {
    n: u32,
    block_num: [u32; params::LOGSIZE],
}

pub struct Log {
    start: u32,
    size: u32,
    outstanding: u32, // how many FS sys calls are executing.
    committing: u32,  // in commit(), please wait.
    dev: u32,
    logheader: LogHeader,
}

impl Log {
    fn new(dev: u32, superblock: SuperBlock) -> Self {
        assert!(
            size_of::<LogHeader>() < params::BSIZE,
            "initlog: too big logheader"
        );
        let log = Self {
            start: superblock.logstart,
            size: superblock.nlog,
            outstanding: 0,
            committing: 0,
            dev,
            logheader: LogHeader{
                n: 123456,
                block_num: [123456; params::LOGSIZE]
            },
        };
        log.recover_from_log();
        return log;
    }

    // Copy committed blocks from log to their home location
    fn install_trans(&mut self) {

    }

    // Read the log header from disk into the in-memory log header
    fn read_head(&mut self) {

    }

    // Write in-memory log header to disk.
    // This is the true point at which the
    // current transaction commits.
    fn write_head(&mut self) {

    }

    fn recover_from_log(&mut self) {

    }

    // called at the start of each FS system call.
    fn begin_op(&mut self) {

    }

    // called at the end of each FS system call.
    // commits if this was the last outstanding operation.
    fn end_op(&mut self) {

    }

    // Copy modified blocks from cache to log.
    fn write_log(&mut self) {

    }

    fn commit(&mut self) {

    }

    // Caller has modified b->data and is done with the buffer.
    // Record the block number and pin in the cache by increasing refcnt.
    // commit()/write_log() will do the disk write.
    //
    // log_write() replaces bwrite(); a typical use is:
    //   bp = bread(...)
    //   modify bp->data[]
    //   log_write(bp)
    //   brelse(bp)
    fn log_write(&mut self) {

    }



}
