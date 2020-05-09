// See https://github.com/mit-pdos/xv6-public/blob/master/log.c
use alloc::sync::Arc;
use core::mem::size_of;
use spin::{Once, Mutex};

use libsyscalls::sync::CondVar;

use crate::bcache::{BCACHE, BufferGuard};
use crate::fs::SuperBlock;
use crate::log::log_header::LogHeader;
pub use crate::log::Transaction;
use crate::params;

// We only have one device 
pub static LOG: Once<Log> = Once::new();

pub struct Log {
    log: Arc<(Mutex<LogInternal>, CondVar)>,
}

impl Log {
    pub fn new(dev: u32, superblock: &SuperBlock) -> Self {
        Self {
            log: Arc::new((Mutex::new(LogInternal::new(dev, superblock)), CondVar::new())),
        }
    }

    pub fn begin_transaction(&self) -> Transaction {
        Transaction::new(self.log.clone())
    }
}

#[derive(Debug)]
pub struct LogInternal {
    start: u32,
    size: u32,
    outstanding: u32, // how many FS sys calls are executing.
    committing: bool,  // in commit(), please wait.
    dev: u32,
    logheader: LogHeader,
}


impl LogInternal {
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
                n: 123_456,
                block_nums: [123_456; params::LOGSIZE]
            },
        };
        log.recover_from_log();
        log
    }

    // Copy committed blocks from log to their home location
    fn install_trans(&mut self) {
        for tail in 0..self.logheader.n {
            // console::println!("committing {} to {}", self.start + tail + 1, self.logheader.block_nums[tail as usize]);
            let mut lbuf = BCACHE.force_get().read(self.dev, self.start + tail + 1);
            let mut dbuf = BCACHE.force_get().read(self.dev, self.logheader.block_nums[tail as usize]);
            {
                let mut locked_dbuf = dbuf.lock();
                ***locked_dbuf = ***lbuf.lock();
                BCACHE.force_get().write(dbuf.block_number(), &mut locked_dbuf);  // write dst to disk
            }
            dbuf.unpin();
        }
    }

    // Read the log header from disk into the in-memory log header
    fn read_head(&mut self) {
        let mut buf = BCACHE.force_get().read(self.dev, self.start);
        self.logheader.from_buffer_block(&buf.lock());
                console::println!("Log::read_head: {:?}", self);
    }

    // Write in-memory log header to disk.
    // This is the true point at which the
    // current transaction commits.
    fn write_head(&self) {
        let mut buf = BCACHE.force_get().read(self.dev, self.start);
        {
            let mut locked_buf = buf.lock();
            self.logheader.to_buffer_block(&mut locked_buf);
            BCACHE.force_get().write(buf.block_number(), &mut locked_buf);
        }
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
        // console::println!("try_begin_op; {:?}", self);
        if self.committing {
            return false;
        }
        if self.logheader.n + (self.outstanding+1)*params::MAXOPBLOCKS as u32 > params::LOGSIZE as u32 {
            return false;
        }
        // console::println!("op begin");
        self.outstanding += 1;
        true
    }

    // called at the end of each FS system call.
    // commits if this was the last outstanding operation.
    pub fn end_op(&mut self) {
        let mut do_commit: bool = false;

        // console::println!("end_op; outstanding {}", self.outstanding);

        self.outstanding -= 1;
        assert!(!self.committing, "log.commiting");
        if self.outstanding == 0 {
            do_commit = true;
            self.committing = true;
        }

        if do_commit {
            self.commit();
            self.committing = false;
        }
    }

    // Copy modified blocks from cache to log.
    fn write_log(&mut self) {
        for tail in 0..self.logheader.n {
            // console::println!("logging {} to {}", self.logheader.block_nums[tail as usize], self.start + tail + 1);
            let mut to = BCACHE.force_get().read(self.dev, self.start + tail + 1); // log block
            let mut from = BCACHE.force_get().read(self.dev, self.logheader.block_nums[tail as usize]); // cache block
            {
                let mut locked_to = to.lock();
                ***locked_to = ***from.lock();
                BCACHE.force_get().write(to.block_number(), &mut locked_to);  // write the log
            }
        }
    }

    fn commit(&mut self) {
        if self.logheader.n > 0 {
            // console::println!("committing");
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
    pub fn log_write(&mut self, buffer: &BufferGuard) {
        assert!(self.logheader.n < params::LOGSIZE as u32 || self.logheader.n < self.size - 1,
            "too big a transaction");
        assert!(self.outstanding >= 1, "log_write outside of trans");

        // console::println!("writing {} to transaction", buffer.block_number());
        // Find the index that the block should belong to.
        // Log absorbtion: if the block is already in the log, don't need to do anything.
        //  Else, add the new block to the log
        self.logheader.block_nums[self.logheader.n as usize] = buffer.block_number();
        let current_blocks = &self.logheader.block_nums[0..self.logheader.n as usize];
        let i = current_blocks.iter().position(|&x| x == buffer.block_number());
        if i.is_none() {
            buffer.pin();
            self.logheader.n += 1;
        }
    }
}
