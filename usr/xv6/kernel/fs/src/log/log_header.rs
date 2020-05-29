use byteorder::{ByteOrder, LittleEndian};

use crate::bcache::BufferBlock;
use crate::params;

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
#[derive(Debug)]
pub struct LogHeader {
    pub n: u32,
    pub block_nums: [u32; params::LOGSIZE],
}

impl LogHeader {
    pub fn from_buffer_block(&mut self, buffer: &BufferBlock) {
        let mut offset = 0;
        self.n = LittleEndian::read_u32(&buffer[offset..offset + 4]);
        offset += 4;

        for block_num in &mut self.block_nums {
            *block_num = LittleEndian::read_u32(&buffer[offset..offset + 4]);
            offset += 4;
        }
    }

    pub fn to_buffer_block(&self, buffer: &mut BufferBlock) {
        let mut offset = 0;
        LittleEndian::write_u32(&mut buffer[offset..offset + 4], self.n);
        offset += 4;

        for block_num in &self.block_nums {
            LittleEndian::write_u32(&mut buffer[offset..offset + 4], *block_num);
            offset += 4;
        }
    }
}
