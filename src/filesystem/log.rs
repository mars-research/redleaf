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

    fn install_trans(&mut self) {

    }

    fn read_head(&mut self) {

    }

    fn write_head(&mut self) {

    }

    fn recover_from_log(&mut self) {

    }

    fn begin_op(&mut self) {

    }

    fn end_op(&mut self) {

    }

    fn write_log(&mut self) {

    }

    fn commit(&mut self) {

    }

    fn log_write(&mut self) {
        
    }



}
