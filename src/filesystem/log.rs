// See https://github.com/mit-pdos/xv6-public/blob/master/log.c

use core::mem::size_of;

use crate::common::bytearray;
use crate::filesystem::fs::SuperBlock;
use crate::filesystem::params;
use crate::filesystem::bcache::{BCACHE, BufferBlock};

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
struct LogHeader {
    n: u32,
    block_nums: [u32; params::LOGSIZE],
}

impl LogHeader {
    fn from_buffer_block(&mut self, buffer: &BufferBlock) {
        self.n = bytearray::to_u32(&buffer[0..3]);
        let offset = 4;
        for block_num in &mut self.block_nums {
            block_num = bytearray::to_u32(&buffer[offset..offset+3]);
            offset += 4;
        }
    }
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
                block_nums: [123456; params::LOGSIZE]
            },
        };
        log.recover_from_log();
        return log;
    }

    // Copy committed blocks from log to their home location
    fn install_trans(&mut self) {
        for tail in 0..self.logheader.n {
            let lbuf = BCACHE.lock().read(self.dev, self.start + tail + 1);
            let dbuf = BCACHE.lock().read(self.dev, self.logheader.block_nums[tail as usize]);
            let locked_dbuf = dbuf.lock();
            locked_dbuf.data = lbuf.lock().data;
            BCACHE.lock().write(&mut locked_dbuf);  // write dst to disk
            // TODO: implement brelse and finish this function up
            // brelse(lbuf);
            // brelse(dbuf);
        }
    }

    // Read the log header from disk into the in-memory log header
    fn read_head(&mut self) {
        let buf = BCACHE.lock().read(self.dev, self.start);
        self.logheader.from_buffer_block(&buf.lock().data);
        // brelse(buf);
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
