// Based on bio.c from xv6.
// The entire ownership system is a mess and error-prone(no one is the owner).
// Need to revisit this and fix it one day.

use crate::params::{NBUF, BSIZE};

use alloc::sync::Arc;
use console::println;
use core::ops::Deref;
use spin::{Mutex};
use utils::list2;

const B_DIRTY: u32 = 1 << 0;
const B_VALID: u32 = 1 << 1;

fn iderw(buffer: &mut BufferData, write: bool) {
    if write {
        buffer.data[0] = 2;
    }
}

pub type BufferBlock = [u8; BSIZE];

pub struct BufferData {
    flags: u32,
    pub data: BufferBlock,
}

impl BufferData {
    fn new() -> Self {
        Self {
            flags: 0,
            data: [0; BSIZE],
        }
    }
}

pub struct BufferGuard {
    dev: u32,
    block_number: u32,
    node: list2::Link<Buffer>,
    data: Arc<Mutex<BufferData>>,
}

impl BufferGuard {
    pub fn dev(&self) -> u32{
        self.dev
    }

    pub fn block_number(&self) -> u32 {
        self.block_number
    }

    // This is nasty. Fix this
    pub fn pin(&self) {
        self.node.as_ref().take().unwrap().lock().elem.reference_count += 1;
    }

    pub fn unpin(&self) {
        self.node.as_ref().take().unwrap().lock().elem.reference_count -= 1;
    }
}

// I could've get a reference to the bcache and do a brelse explicitly when the guard is dropped.
// But I don't want to deal with the lifetime for now. Might do it later
impl Drop for BufferGuard {
    fn drop(&mut self) {
        assert!(self.node.is_none(), "You forgot to release the buffer back to the bcache");
    }
}

impl Deref for BufferGuard {
    type Target = Arc<Mutex<BufferData>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

struct Buffer {
    // Metadata about this block
    dev: u32,
    block_number: u32,
    reference_count: u32,
    flags: u32,
    // The actual data
    // Maybe it will be more efficient if we allocate it in the heap?
    data: Arc<Mutex<BufferData>>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            dev: 0,
            block_number: 0,
            reference_count: 0,
            flags: 0,
            data: Arc::new(Mutex::new(BufferData::new())),
        }
    }
}

pub struct BufferCache {
    list: Mutex<list2::List<Buffer>>,
}

impl BufferCache {
    pub fn new() -> Self {
        let mut list = list2::List::<Buffer>::new();
        for _ in 0..NBUF {
            list.push_back(Buffer::new());
        }
        Self {
            list: Mutex::new(list),
        }
    }

    // look through buffer cache, return the buffer
    // If the block does not exist, we preempt a not-in-use one
    // We let the caller to lock the buffer when they need to use it
    fn get(&self, dev: u32, block_number: u32) -> BufferGuard {
        // we probably don't need a lock here since there's a outer lock for
        // the shared `BCACHE` object.
        for mutex in self.list.lock().iter() {
            let mut node = mutex.lock();
            let mut buffer = &mut node.elem;
            if buffer.dev == dev && buffer.block_number == block_number {
                buffer.reference_count += 1;
                return BufferGuard {
                    dev: buffer.dev,
                    block_number: buffer.block_number,
                    node: Some(mutex.clone()),
                    data: buffer.data.clone(),
                };
            }
        }

        // Not cached; recycle an unused buffer.
        for mutex in self.list.lock().iter().rev() {
            let mut node = mutex.lock();
            let mut buffer = &mut node.elem;
            if buffer.reference_count == 0 && (buffer.flags & B_DIRTY) == 0 {
                buffer.dev = dev;
                buffer.block_number = block_number;
                buffer.flags = 0;
                buffer.reference_count = 1;
                return BufferGuard {
                    dev: buffer.dev,
                    block_number: buffer.block_number,
                    node: Some(mutex.clone()),
                    data: buffer.data.clone(),
                };
            }
        }
        panic!("Not reusable block in bcache");
    }

    // Return a unlocked buffer with the contents of the indicated block.
    // In xv6, we get a locked buffer from `bget` and it stays locked
    // after it's returned from this function.
    // Since it's hard to pass a locked buffer around in Rust, we choose to
    // get an unlocked buffer from `bget`, lock the buffer and sync it with the disk,
    // then unlock it and return it to the caller.
    // This is okay because the buffer will become valid only if it is a reused buffer.
    // We can also merge `bread` with `bget` since `bget` is only a helper for `bread`
    pub fn read(&self, device: u32, block_number: u32) -> BufferGuard {
        println!("bread dev{} block{}", device, block_number);
        let buffer = self.get(device, block_number);
        {
            let mut guard = buffer.lock();
            if (guard.flags & B_VALID) == 0 {
                // iderw will set the buffer to valid
                // Note that this is different from xv6-risvc 
                iderw(&mut guard, false);
            }
        }
        return buffer;
    }

    // Write b's contents to disk 
    // Return a locked buf with the contents of the indicated block.
    pub fn write(&self, buffer_data: &mut BufferData) {
        iderw(buffer_data, true);
    }

    // This is confusing since it doesn't match xv6's brelse exactly so there could be a bug.
    // Check xv6 for details
    // TODO(tianjiao): fix this
    pub fn release(&self, guard: &mut BufferGuard) {
        println!("brlse dev{} block{}", guard.dev, guard.block_number);
        let node = guard.node.take().expect("Buffer is not initialized or already released.");
        let mut list = self.list.lock();
        node.lock().elem.reference_count -= 1;
        list.move_front(node);
    }

}

lazy_static! {
    pub static ref BCACHE: BufferCache = BufferCache::new();
}
