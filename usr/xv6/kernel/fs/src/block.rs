use crate::params;
use crate::bcache::{BCACHE};
use crate::fs::{SUPER_BLOCK, SuperBlock};
use crate::log::Transaction;

// Frees a disk block
// xv6 equivalent: bfree
pub fn free(trans: &mut Transaction, device: u32, block: u32) {
    let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");

    let mut bguard = BCACHE.r#try().unwrap().read(device, block_to_bitmap_block(block, super_block));
    let mut buffer = bguard.lock();
    let bi = (block as usize) % params::BPB;
    let m = 1 << (bi % 8);
    if buffer[bi / 8] & m == 0 {
        panic!("freeing freed block");
    }
    buffer[bi / 8] &= !m;
    trans.write(&bguard);
    drop(buffer);
}

// Allocate a zeroed disk block.
// Returns None if out of blocks
// xv6 equivalent: balloc
pub fn alloc(trans: &mut Transaction, device: u32) -> Option<u32> {
    let super_block = SUPER_BLOCK.r#try().expect("fs not initialized");

    for b in (0..super_block.size).step_by(params::BPB) {
        let mut bguard = BCACHE.r#try().unwrap().read(device, block_to_bitmap_block(b, super_block));
        let mut buffer = bguard.lock();

        let mut bi = 0;
        while bi < params::BPB && b + (bi as u32) < super_block.size {
            let m = 1 << (bi % 8);
            if buffer[bi / 8] & m == 0 {
                buffer[bi / 8] |= m; // mark block as used
                trans.write(&bguard);

                drop(buffer);
                
                zero(trans, device, b + bi as u32);
                return Some(b + bi as u32);
            }
            bi += 1;
        }

        drop(buffer);
            }

    // out of blocks
    None
}

// Zero a block
// xv6 equivalent: bzero
fn zero(trans: &mut Transaction, device: u32, block_number: u32) {
    let mut bguard = BCACHE.r#try().unwrap().read(device, block_number);
    let mut buffer = bguard.lock();

    for v in buffer.iter_mut() {
        *v = 0;
    }

    trans.write(&bguard);
    drop(buffer);
    }

// Block of free map containing bit for block b
// xv6 equivalent: BBLOCK
fn block_to_bitmap_block(block_number: u32, sb: &SuperBlock) -> u32 {
    block_number / params::BPB as u32 + sb.bmapstart
}
