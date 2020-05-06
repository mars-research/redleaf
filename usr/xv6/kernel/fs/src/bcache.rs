// Based on bio.c from xv6.
// The entire ownership system is a mess and error-prone(no one is the owner).
// Need to revisit this and fix it one day.

use crate::params::{NBUF, BSIZE, SECTOR_SIZE};

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use console::println;
use core::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use spin::{Mutex, Once};

use utils::list2;
use rref::RRef;
use usr_interface::bdev::BDev;

pub static BCACHE: Once<BufferCache> = Once::new();

pub type BufferBlock = RRef<[u8; BSIZE]>;

pub struct BufferBlockWrapper(Option<BufferBlock>);

impl BufferBlockWrapper {
    fn take(&mut self) -> BufferBlock {
        self.0.take().unwrap()
    }
}

impl Deref for BufferBlockWrapper {
    type Target = BufferBlock;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl DerefMut for BufferBlockWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

pub struct BufferGuard {
    dev: u32,
    block_number: u32,
    index: i32,
    data: Arc<Mutex<BufferBlockWrapper>>,
}

impl BufferGuard {
    pub fn dev(&self) -> u32{
        self.dev
    }

    pub fn block_number(&self) -> u32 {
        self.block_number
    }

    pub fn pin(&self) {
        // unimplemented!()
    }

    pub fn unpin(&self) {
        // unimplemented!()
    }
}

// I could've get a reference to the bcache and do a brelse explicitly when the guard is dropped.
// But I don't want to deal with the lifetime for now. Might do it later
impl Drop for BufferGuard {
    fn drop(&mut self) {
        assert!(self.index < 0, "You forgot to release the buffer back to the bcache");
    }
}

impl Deref for BufferGuard {
    type Target = Arc<Mutex<BufferBlockWrapper>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

struct Buffer {
    // Pointers to prev/next blocks
    prev: i32,
    next: i32,
    // Metadata about this block
    dev: u32,
    block_number: u32,
    reference_count: u32,
    // The actual data
    // TODO: use a sleep mutex
    data: Arc<Mutex<BufferBlockWrapper>>,
}

impl core::fmt::Debug for Buffer {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_struct("BufferBlock")
           .field("prev", &self.prev)
           .field("next", &self.next)
           .field("dev", &self.dev)
           .field("block_number", &self.block_number)
           .field("reference_count", &self.reference_count)
           .finish()
    }
}

impl Buffer {
    pub fn new(index: usize) -> Self {
        Self {
            dev: 0,
            block_number: 0,
            reference_count: 0,
            prev: index as i32 - 1,
            next: index as i32 + 1,
            data: Arc::new(Mutex::new(BufferBlockWrapper(Some(RRef::new([0u8; BSIZE]))))),
        }
    }
}

#[derive(Debug)]
pub struct BufferCacheInternal {
    buffers: Vec<Buffer>,
    head: usize,
    map: HashMap<(u32, u32), usize>,
}

impl BufferCacheInternal {
    pub fn new() -> Self {
        let mut buffers = vec![];
        for i in 0..NBUF {
            buffers.push(Buffer::new(i));
        }
        buffers[0].prev = NBUF as i32 - 1;
        buffers[NBUF - 1].next = 0;
        Self {
            buffers,
            head: 0,
            map: HashMap::new(),
        }
    }

    // look through buffer cache, return the buffer
    // If the block does not exist, we preempt a not-in-use one
    // We let the caller to lock the buffer when they need to use it
    fn get(&mut self, dev: u32, block_number: u32) -> (bool, BufferGuard) {
        // println!("{:?} {:?}", &(dev, block_number), self.map.get(&(dev, block_number)));
        match self.map.get(&(dev, block_number)) {
            Some(index) => {
                let buffer = &mut self.buffers[*index];
                buffer.reference_count += 1;
                (true, BufferGuard {
                    dev: buffer.dev,
                    block_number: buffer.block_number,
                    index: *index as i32,
                    data: buffer.data.clone(),
                })
            },
            None => {
                // Not cached; recycle an unused buffer.
                let mut curr = self.buffers[self.head].prev;
                for _ in 0..NBUF {
                    let buffer = &mut self.buffers[curr as usize];
                    if buffer.reference_count == 0 {
                        // Move it out from the map
                        if buffer.block_number != 0 {
                            assert!(self.map.remove(&(buffer.dev, buffer.block_number)).is_some());
                        }

                        // Clear the buffer and return it
                        buffer.dev = dev;
                        buffer.block_number = block_number;
                        buffer.reference_count = 1;
                        assert!(self.map.insert((dev, block_number), curr as usize).is_none());
                        return (false, BufferGuard {
                            dev: buffer.dev,
                            block_number: buffer.block_number,
                            index: curr,
                            data: buffer.data.clone(),
                        });
                    }
                    curr = buffer.prev;
                }
                println!("{:?}", self);
                panic!("No free block in bcache");
            },
        }
    }

    fn release(&mut self, index: usize) {
        self.buffers[index].reference_count -= 1;
        if self.buffers[index].reference_count == 0 {
            // Move to the head
            let prev = self.buffers[index].prev as usize;
            let next = self.buffers[index].next as usize;
            self.buffers[next].prev = self.buffers[index].prev;
            self.buffers[prev].next = self.buffers[index].next;
            self.buffers[index].next = self.head as i32;
            self.buffers[index].prev = self.buffers[self.head].prev;
            self.buffers[self.head].prev = index as i32;
            self.head = index;
        }
    }

}

pub struct BufferCache {
    internal: Mutex<BufferCacheInternal>,
    bdev: Box<dyn BDev + Send + Sync>,
}

impl BufferCache {
    pub fn new(bdev: Box<dyn BDev + Send + Sync>) -> Self {
        Self {
            internal: Mutex::new(BufferCacheInternal::new()),
            bdev,
        }
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
        // println!("bread dev#{} block#{}", device, block_number);
        let (valid, buffer) = self.internal.lock().get(device, block_number);
        if !valid {
            let sector = buffer.block_number() * (BSIZE / SECTOR_SIZE) as u32;
            let mut guard = buffer.lock();
            *guard = BufferBlockWrapper(Some(self.bdev.read(sector, guard.take())));
        }
        buffer
    }

    // Write b's contents to disk 
    // Return a locked buf with the contents of the indicated block.
    // This is not very safe since the user could pass in a `block_number` that
    // doesn't match with the `buffer_data`.
    // TODO: address the issue above by refactoring the `BufferGuard`
    pub fn write(&self, block_number: u32, buffer_data: &mut BufferBlockWrapper) {
        // println!("bwrite block#{}", block_number);
        let sector = block_number * (BSIZE / SECTOR_SIZE) as u32;
        *buffer_data = BufferBlockWrapper(Some(self.bdev.write(block_number, buffer_data.take())));
    }

    // This is confusing since it doesn't match xv6's brelse exactly so there could be a bug.
    // Check xv6 for details
    // TODO(tianjiao): fix this
    pub fn release(&self, guard: &mut BufferGuard) {
        self.internal.lock().release(guard.index as usize);
        guard.index = -1;
    }

}

impl core::fmt::Debug for BufferCache {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(fmt, "{:?}", *self.internal.lock())
    }
}
