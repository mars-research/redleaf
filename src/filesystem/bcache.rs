//! Double-linked-list buffer cache, adopted heavily from xv6
//!
//! https://github.com/mit-pdos/xv6-riscv

use alloc::collections::LinkedList;
use spin::{Mutex, MutexGuard};
use alloc::boxed::Box;
use alloc::sync::Arc;

const BUFFER_SIZE: usize = 1024;
const MAX_OP_BLOCKS: u32 = 10; // max number of blocks any fs op writes
const NUM_BUFFERS: u32 = MAX_OP_BLOCKS * 3; // size of the disk block cache

struct BufferData {
    flags: u32,
    data: [u8; BUFFER_SIZE],
}

impl BufferData {
    fn new() -> Self {
        Self {
            flags: 0,
            data: [0; BUFFER_SIZE],
        }
    }
}

struct Buffer {
    // Metadata about this block
    device: u32,
    block_number: u32,
    reference_count: u32,
    // The actual data
    // Maybe it will be more efficient if we allocate it in the heap?
    data: Arc<Mutex<BufferData>>,
}


impl Buffer {
    pub fn new() -> Self {
        Self {
            device: 0,
            block_number: 0,
            reference_count: 0,
            data: Arc::new(Mutex::new(BufferData::new())),
        }
    }
}

pub struct BufferCache {
    list: Mutex<LinkedList<Buffer>>,
}

impl BufferCache {
    fn new() -> BufferCache {
        let mut list = LinkedList::<Buffer>::new();
        for i in 0..NUM_BUFFERS {
            list.push_back(Buffer::new());
        }
        BufferCache {
            list: Mutex::new(list),
        }
    }

    // look through buffer cache, return the buffer
    // If the block does not exist, we preempt a not-in-use one 
    // We let the caller to lock the buffer when they need to use it
    pub fn get<F>(&mut self, device: u32, block_number: u32) -> MutexGuard<BufferData> {
        for buffer in self.list.lock().iter() {
            if buffer.device == device && buffer.block_number == block_number {
                buffer.reference_count += 1;
                return buffer.data.lock();
            }
        }

        // Not cached; recycle an unused buffer.
        // In xv6, the bcache is kinda like a LRU cache so it looks backward when looking
        // for an unused buffer.
        // TODO: iter the linked list backward
        for buffer in self.list.lock().iter() {
            if(buffer.reference_count == 0 && (buffer.flags & B_DIRTY) == 0) {
                buffer.dev = dev;
                buffer.block_number = block_number;
                buffer.flags = 0;
                buffer.reference_count = 1;
                return buffer.data.lock();
            }
        }
        panic!("Not reusable block in bcache");
    }
}

lazy_static! {
    pub static ref BCACHE: Mutex<BufferCache> = {
        Mutex::new(BufferCache::new())
    };
}
