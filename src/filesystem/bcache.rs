use crate::filesystem::params::{NBUF, BSIZE};
use crate::common::list2;
use alloc::sync::Arc;
use alloc::rc::Rc;
use core::cell::{Ref, RefMut, RefCell};
use core::ops::Deref;
use spin::{Mutex};

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
    node: list2::Link<Buffer>,
    data: Arc<Mutex<BufferData>>,
}

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
    fn new() -> BufferCache {
        let mut list = list2::List::<Buffer>::new();
        for i in 0..NBUF {
            list.push_back(Buffer::new());
        }
        BufferCache {
            list: Mutex::new(list),
        }
    }

    // look through buffer cache, return the buffer
    // If the block does not exist, we preempt a not-in-use one
    // We let the caller to lock the buffer when they need to use it
    fn get(&self, dev: u32, block_number: u32) -> Arc<Mutex<BufferData>> {
        // we probably don't need a lock here since there's a outer lock for
        // the shared `BCACHE` object.
        for mutex in self.list.lock().iter() {
            let mut node = mutex.lock();
            let mut buffer = &mut node.elem;
            if buffer.dev == dev && buffer.block_number == block_number {
                buffer.reference_count += 1;
                return buffer.data.clone();
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
                return buffer.data.clone();
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
    pub fn read(&self, device: u32, block_number: u32) -> Arc<Mutex<BufferData>> {
        let buffer = self.get(device, block_number);
        let mut guard = buffer.lock();
        if (guard.flags & B_VALID) == 0 {
            // iderw will set the buffer to valid
            // Note that this is different from xv6-risvc 
            iderw(&mut guard, false);
        }
        return buffer.clone();
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
        let node = guard.node.take().expect("Buffer is not initialized or already released.");
        let mut list = self.list.lock();
        node.lock().elem.reference_count -= 1;
        list.move_front(node);
    }

}

lazy_static! {
    pub static ref BCACHE: BufferCache = BufferCache::new();
}
