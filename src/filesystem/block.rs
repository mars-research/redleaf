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

    // Allocate a zeroed disk block.
    // Returns None if out of blocks
    // xv6 equivalent: balloc
    pub fn alloc(device: u32) -> Option<u32> {
        let super_block = get_super_block();

        for b in (0..super_block.size).step_by(params::BPB) {
            let mut bguard = BCACHE.read(device, block_num_for_node(b as u32, &super_block));
            let mut buffer = bguard.lock();

            let mut bi = 0;
            while bi < params::BPB && b + bi < super_block.size {
                let m = 1 << (bi % 8);
                if buffer.data[bi as usize / 8] & m == 0 {
                    buffer.data[bi as usize / 8] |= m; // mark block as used
                    // TODO: log_write here

                    drop(buffer);
                    BCACHE.release(&mut bguard);

                    Block::zero(device, (b + bi) as u32);
                    return Some((b + bi) as u32);
                }
                bi += 1;
            }

            drop(buffer);
            BCACHE.release(&mut bguard);
        }

        // out of blocks
        None
    }

    // Zero a block
    // xv6 equivalent: bzero
    pub fn zero(device: u32, block_number: u32) {
        let mut bguard = BCACHE.read(device, block_number);
        let mut buffer = bguard.lock();

        for v in buffer.data.iter_mut() {
            *v = 0;
        }

        // TODO: log_write here
        drop(buffer);
        BCACHE.release(&mut bguard);
    }
}
