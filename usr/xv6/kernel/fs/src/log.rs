// See https://github.com/mit-pdos/xv6-public/blob/master/log.c

use core::mem::size_of;

use spin::Once;

use utils::bytearray;
use crate::fs::SuperBlock;
use crate::params;
use crate::bcache::{BCACHE, BufferBlock, BufferGuard};

// We only have one device 
pub static LOG: Once<Log> = Once::new();

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
struct LogHeader {
    n: u32,
    block_nums: [u32; params::LOGSIZE],
}

impl LogHeader {
    fn from_buffer_block(&mut self, buffer: &BufferBlock) {
        let mut offset = 0;
        self.n = bytearray::to_u32(&buffer[offset..offset+4]);
        offset += 4;

        for block_num in &mut self.block_nums {
            *block_num = bytearray::to_u32(&buffer[offset..offset+4]);
            offset += 4;
        }
    }

    fn to_buffer_block(&self, buffer: &mut BufferBlock) {
        let mut offset = 0;
        bytearray::from_u32(&mut buffer[offset..offset+4], self.n);
        offset += 4;

        for block_num in &self.block_nums {
            bytearray::from_u32(&mut buffer[offset..offset+4], *block_num);
            offset += 4;
        }
    }
}

pub struct Log {
    start: u32,
    size: u32,
    outstanding: u32, // how many FS sys calls are executing.
    committing: bool,  // in commit(), please wait.
    dev: u32,
    logheader: LogHeader,
}

impl Log {
    pub fn new(dev: u32, superblock: &SuperBlock) -> Self {
        assert!(
            size_of::<LogHeader>() < params::BSIZE,
            "initlog: too big logheader"
        );
        let mut log = Self {
            start: superblock.logstart,
            size: superblock.nlog,
            outstanding: 0,
            committing: false,
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
            let mut lbuf = BCACHE.read(self.dev, self.start + tail + 1);
            let mut dbuf = BCACHE.read(self.dev, self.logheader.block_nums[tail as usize]);
            {
                let mut locked_dbuf = dbuf.lock();
                locked_dbuf.data = lbuf.lock().data;
                BCACHE.write(&mut locked_dbuf);  // write dst to disk
            }
            // Pin this buffer if using the riscv one
            BCACHE.release(&mut lbuf);
            BCACHE.release(&mut dbuf);
        }
    }

    // Read the log header from disk into the in-memory log header
    fn read_head(&mut self) {
        let mut buf = BCACHE.read(self.dev, self.start);
        self.logheader.from_buffer_block(&buf.lock().data);
        BCACHE.release(&mut buf);
    }

    // Write in-memory log header to disk.
    // This is the true point at which the
    // current transaction commits.
    fn write_head(&self) {
        let mut buf = BCACHE.read(self.dev, self.start); 
        {
            let mut locked_buf = buf.lock();
            self.logheader.to_buffer_block(&mut locked_buf.data);
            BCACHE.write(&mut locked_buf);
        }
        BCACHE.release(&mut buf);
    }

    fn recover_from_log(&mut self) {
        self.read_head();
        self.install_trans();
        self.logheader.n = 0;
        self.write_head();
    }

    // called at the start of each FS system call.
    // Caller should repeatly call this function until this function returns true.
    // A better implementation of this function would be that
    // this functions returns a guard that can be used to write_log.
    // And end_op will be called when the guard is dropped.
    // TODO(tianjiao): fix this
    pub fn try_begin_op(&mut self) -> bool {
        if self.committing {
            return false;
        }
        if self.logheader.n + (self.outstanding+1)*params::MAXOPBLOCKS as u32 > params::LOGSIZE as u32 {
            return false;
        }
        self.outstanding += 1;
        return true;
    }

    // called at the end of each FS system call.
    // commits if this was the last outstanding operation.
    // Caller should repeatly call this function until this function returns true.
    fn end_op(&mut self) {
        let mut do_commit: bool = false;

        self.outstanding -= 1;
        assert!(!self.committing, "log.commiting");
        if self.outstanding == 0 {
            do_commit = true;
            self.committing = true;
        } else {
            // We dont need this wake up because currently we spin instead of sleep.
            // begin_op() may be waiting for log space,
            // and decrementing log.outstanding has decreased
            // the amount of reserved space.
            // wakeup(&log);
        }

        // This part is EXTREMELY inefficient and weird, but it should work
        // Basically we just hold the lock till we finish commiting.
        if do_commit {
            // call commit w/o holding locks, since not allowed
            // to sleep with locks.
            // TODO(tianjiao): fix this
            self.commit();
            self.committing = false;
            // wakeup(&log);
        }
        unimplemented!();
    }

    // Copy modified blocks from cache to log.
    fn write_log(&mut self) {
        for tail in 0..self.logheader.n {
            let mut to = BCACHE.read(self.dev, self.start + tail + 1); // log block
            let mut from = BCACHE.read(self.dev, self.logheader.block_nums[tail as usize]); // cache block
            {
                let mut locked_to = to.lock();
                locked_to.data = from.lock().data;
                BCACHE.write(&mut locked_to);  // write the log
            }
            BCACHE.release(&mut from);
            BCACHE.release(&mut to);
        }
    }

    fn commit(&mut self) {
        if self.logheader.n > 0 {
            self.write_log();       // Write modified blocks from cache to log
            self.write_head();      // Write header to disk -- the real commit
            self.install_trans();   // Now install writes to home locations
            self.logheader.n = 0;
            self.write_head();      // Erase the transaction from the log
        }
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
    fn log_write(&mut self, buffer: BufferGuard) {
        assert!(self.logheader.n < params::LOGSIZE as u32 || self.logheader.n < self.size - 1,
            "too big a transaction");
        assert!(self.outstanding >= 1, "log_write outside of trans");

        // Find the index that the block should belong to.
        // Log absorbtion: if the block is already in the log, don't need to do anything.
        //  Else, add the new block to the log
        let current_blocks = &self.logheader.block_nums[0..self.logheader.n as usize];
        let i = current_blocks.iter().position(|&x| x == buffer.block_number());
        i.map(|_| {
            buffer.pin();
            self.logheader.n += 1;
        });
    }



}
