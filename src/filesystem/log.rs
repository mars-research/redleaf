// See https://github.com/mit-pdos/xv6-public/blob/master/log.c
use crate::filesystem::params::LOGSIZE;

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
struct LogHeader {
  n: u32,
  block_num: [u32; LOGSIZE],
}

pub struct Log {
    start: u32,
    size: u32,
    outstanding: u32, // how many FS sys calls are executing.
    committing: u32,  // in commit(), please wait.
    device: u32,
    header: u32,
}



