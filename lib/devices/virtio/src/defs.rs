use alloc::{
<<<<<<< HEAD
    alloc::{alloc, dealloc},
    boxed::Box,
    vec::Vec,
};
=======
    alloc::{alloc, alloc_zeroed, dealloc},
    boxed::Box,
    vec::Vec,
};
use console::println;
>>>>>>> virtio_blk_bench
use core::{alloc::Layout, mem::size_of, usize};

// 2.6.12 Virtqueue Operation
// There are two parts to virtqueue operation: supplying new available buffers to the device, and processing used buffers from the device.
// Note: As an example, the simplest virtio network device has two virtqueues: the transmit virtqueue and the receive virtqueue.
// The driver adds outgoing (device-readable) packets to the transmit virtqueue, and then frees them after they are used.
// Similarly, incoming (device-writable) buffers are added to the receive virtqueue, and processed after they are used.

#[repr(C, align(16))]
pub struct VirtQueue {
    pub descriptors: Vec<VirtqDescriptor>,
    pub available: VirtqAvailable,
    pub used: VirtqUsed,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed(16))]
pub struct VirtqDescriptor {
    /// Address (guest-physical) to Virtio Net Packet Header
    pub addr: u64,
    /// Length
    pub len: u32,

    /// 1: NEXT, 2: DEVICE WRITABLE
    pub flags: u16,

    /// Next field if flags contains NEXT
    pub next: u16,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed)]
pub struct VirtqUsedElement {
    /// Index of start of used descriptor chain
    pub id: u32,
    /// Total length of the descriptor chain used
    pub len: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed(4))]
pub struct VirtqUsedPacked {
    pub flags: u16,

    /// Index into VirtqDescriptor Array
    pub idx: u16,

<<<<<<< HEAD
    pub ring: [VirtqUsedElement; 0], // Will have size queue_size
=======
    ring: [VirtqUsedElement; 0], // Will have size queue_size
>>>>>>> virtio_blk_bench
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed(2))]
pub struct VirtqAvailablePacked {
    pub flags: u16,

    /// Index into VirtqDescriptor Array. Count of Descriptor Chain Heads
    pub idx: u16,

    /// The index of the head of the descriptor chain in the descriptor table
<<<<<<< HEAD
    pub ring: [u16; 0], // Will have size queue_size
=======
    ring: [u16; 0], // Will have size queue_size
>>>>>>> virtio_blk_bench
}

pub struct VirtqAvailable {
    pub data: Box<VirtqAvailablePacked>,
    queue_size: u16,
}

impl VirtqAvailable {
    pub unsafe fn new(queue_size: u16) -> Self {
        let layout = Self::get_layout(queue_size);
<<<<<<< HEAD
        let ptr = alloc(layout);
=======
        let ptr = alloc_zeroed(layout);

        // println!("VirtqAvailable Allocated At: {}", ptr as u64);
        // println!("Layout: {:#?}", &layout);
>>>>>>> virtio_blk_bench

        Self {
            data: Box::from_raw(ptr as *mut VirtqAvailablePacked),
            queue_size,
<<<<<<< HEAD
        }
    }

    pub fn ring(&mut self, index: u16) -> &mut u16 {
        if index < self.queue_size {
            unsafe { self.data.ring.get_unchecked_mut(index as usize) }
        } else {
            panic!(
                "Ring Index Out Of Bounds! index: {}, queue_size: {}",
                index, self.queue_size
            );
        }
    }

=======
        }
    }

    pub fn ring(&mut self, index: u16) -> &mut u16 {
        if index < self.queue_size {
            unsafe { self.data.ring.get_unchecked_mut(index as usize) }
        } else {
            panic!(
                "Ring Index Out Of Bounds! index: {}, queue_size: {}",
                index, self.queue_size
            );
        }
    }

>>>>>>> virtio_blk_bench
    fn get_layout(queue_size: u16) -> Layout {
        let size = size_of::<VirtqAvailablePacked>() + (queue_size as usize) * size_of::<u16>();
        Layout::from_size_align(size, 2).unwrap()
    }
}

impl Drop for VirtqAvailable {
    fn drop(&mut self) {
<<<<<<< HEAD
=======
        println!("VirtqAvailable.drop()");
>>>>>>> virtio_blk_bench
        let layout = Self::get_layout(self.queue_size);
        unsafe {
            dealloc(
                self.data.as_mut() as *mut VirtqAvailablePacked as *mut u8,
                layout,
            );
        }
    }
}

pub struct VirtqUsed {
    pub data: Box<VirtqUsedPacked>,
    queue_size: u16,
}

impl VirtqUsed {
    pub unsafe fn new(queue_size: u16) -> Self {
        let layout = Self::get_layout(queue_size);
<<<<<<< HEAD
        let ptr = alloc(layout);

        Self {
            data: Box::from_raw(ptr as *mut VirtqUsedPacked),
            queue_size,
        }
    }

=======
        let ptr = alloc_zeroed(layout);

        Self {
            data: Box::from_raw(ptr as *mut VirtqUsedPacked),
            queue_size,
        }
    }

>>>>>>> virtio_blk_bench
    pub fn ring(&mut self, index: u16) -> &mut VirtqUsedElement {
        if index < self.queue_size {
            unsafe { self.data.ring.get_unchecked_mut(index as usize) }
        } else {
            panic!(
                "Ring Index Out Of Bounds! index: {}, queue_size: {}",
                index, self.queue_size
            );
        }
    }

    fn get_layout(queue_size: u16) -> Layout {
        let size =
            size_of::<VirtqUsedPacked>() + (queue_size as usize) * size_of::<VirtqUsedElement>();
        Layout::from_size_align(size, 4).unwrap()
    }
}

impl Drop for VirtqUsed {
    fn drop(&mut self) {
<<<<<<< HEAD
=======
        println!("VirtqAvailable.drop()");
>>>>>>> virtio_blk_bench
        let layout = Self::get_layout(self.queue_size);
        unsafe {
            dealloc(
                self.data.as_mut() as *mut VirtqUsedPacked as *mut u8,
                layout,
            );
        }
    }
}
