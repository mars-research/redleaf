// Based on bio.c from xv6.
// The entire ownership system is a mess and error-prone(no one is the owner).
// Need to revisit this and fix it one day.

use crate::params::{NBUF, BSIZE, SECTOR_SIZE};

use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::string::String;
use console::println;
use core::ops::Deref;
use spin::{Mutex, Once};
use utils::list2;
use rref::RRef;
use usr_interface::bdev::BDev;

pub static BCACHE: Once<BufferCache> = Once::new();

pub type BufferBlock = [u8; BSIZE];

pub struct BufferGuard {
    dev: u32,
    block_number: u32,
    node: list2::Link<Buffer>,
    data: Arc<Mutex<BufferBlock>>,
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
        let mut node = self.node.as_ref().take().unwrap().lock();
        // println!("bpin {} {}", self.block_number, node.reference_count);
        node.reference_count += 1;
    }

    pub fn unpin(&self) {
        let mut node = self.node.as_ref().take().unwrap().lock();
        // println!("bunpin {} {}", self.block_number, node.reference_count);
        node.reference_count -= 1;
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
    type Target = Arc<Mutex<BufferBlock>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

struct Buffer {
    // Metadata about this block
    dev: u32,
    block_number: u32,
    reference_count: u32,
    // The actual data
    // Maybe it will be more efficient if we allocate it in the heap?
    data: Arc<Mutex<BufferBlock>>,
}

impl core::fmt::Debug for Buffer {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_struct("BufferBlock")
           .field("dev", &self.dev)
           .field("block_number", &self.block_number)
           .field("reference_count", &self.reference_count)
           .finish()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            dev: 0,
            block_number: 0,
            reference_count: 0,
            data: Arc::new(Mutex::new([0u8; BSIZE])),
        }
    }
}

pub struct BufferCache {
    list: Mutex<list2::List<Buffer>>,
    bdev: Box<dyn BDev + Send + Sync>,
}

impl BufferCache {
    pub fn new(bdev: Box<dyn BDev + Send + Sync>) -> Self {
        let mut list = list2::List::<Buffer>::new();
        for _ in 0..NBUF {
            list.push_back(Buffer::new());
        }
        Self {
            list: Mutex::new(list),
            bdev,
        }
    }

    // look through buffer cache, return the buffer
    // If the block does not exist, we preempt a not-in-use one
    // We let the caller to lock the buffer when they need to use it
    fn get(&self, dev: u32, block_number: u32) -> (bool, BufferGuard) {
        // we probably don't need a lock here since there's a outer lock for
        // the shared `BCACHE` object.
        for mutex in self.list.lock().iter() {
            let buffer = &mut **mutex.lock();
            if buffer.dev == dev && buffer.block_number == block_number {
                // println!("bcache hit: {:?}", buffer);
                buffer.reference_count += 1;
                return (true, BufferGuard {
                    dev: buffer.dev,
                    block_number: buffer.block_number,
                    node: Some(mutex.clone()),
                    data: buffer.data.clone(),
                });
            }
        }

        // println!("bcache not hit: {} {}", dev, block_number);
        // Not cached; recycle an unused buffer.
        for mutex in self.list.lock().rev() {
            let buffer = &mut **mutex.lock();
            if buffer.reference_count == 0 {
                buffer.dev = dev;
                buffer.block_number = block_number;
                buffer.reference_count = 1;
                return (false, BufferGuard {
                    dev: buffer.dev,
                    block_number: buffer.block_number,
                    node: Some(mutex.clone()),
                    data: buffer.data.clone(),
                });
            }
        }
        println!("{:?}", self);
        panic!("No free block in bcache");
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
        let (valid, buffer) = self.get(device, block_number);
        if !valid {
            let sector = buffer.block_number() * (BSIZE / SECTOR_SIZE) as u32;
            let mut guard = buffer.lock();
            let mut buf = RRef::<[u8; BSIZE]>::new(*guard);
            self.bdev.read(sector, &mut buf);
            *guard = *buf;
        }
        buffer
    }

    // Write b's contents to disk 
    // Return a locked buf with the contents of the indicated block.
    // This is not very safe since the user could pass in a `block_number` that
    // doesn't match with the `buffer_data`.
    // TODO: address the issue above by refactoring the `BufferGuard`
    pub fn write(&self, block_number: u32, buffer_data: &mut BufferBlock) {
        let sector = block_number * (BSIZE / SECTOR_SIZE) as u32;
        self.bdev.write(block_number, buffer_data);
    }

    // This is confusing since it doesn't match xv6's brelse exactly so there could be a bug.
    // Check xv6 for details
    // TODO(tianjiao): fix this
    pub fn release(&self, guard: &mut BufferGuard) {
        // println!("brlse dev{} block{}", guard.dev, guard.block_number);
        let node = guard.node.take().expect("Buffer is not initialized or already released.");
        let mut list = self.list.lock();
        // println!("refcnt {}", node.lock().reference_count);
        node.lock().reference_count -= 1;
        list.move_front(node);
    }

}


impl core::fmt::Debug for BufferCache {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.list.lock().iter()
                        .map(|b| writeln!(fmt, "{:?}", **b.lock()))
                        .fold(Ok(()), core::fmt::Result::and)
    }
}
