use crate::filesystem::params;
use crate::filesystem::bcache::{BCACHE, BufferBlock};
use crate::filesystem::fs::{block_num_for_node, get_super_block};

pub struct Block {}

impl Block {
    // Frees a disk block
    // xv6 equivalent: bfree
    pub fn free(device: u32, block: u32) {
        let super_block = get_super_block();

        let mut bguard = BCACHE.read(device, block_num_for_node(block, &super_block));
        let mut buffer = bguard.lock();
        let bi = (block as usize) % params::BPB;
        let m = 1 << (bi % 8);
        if buffer.data[bi / 8] & m == 0 {
            panic!("freeing freed block");
        }
        buffer.data[bi / 8] &= !m;
        // TODO: log_write here
        drop(buffer);
        BCACHE.release(&mut bguard);
    }
}
