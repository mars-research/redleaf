/// The number of Descriptors (must be a multiple of 2), called "Queue Size" in documentation
pub const DESCRIPTOR_COUNT: usize = 256; // Maybe change this to 256, was 8 before

#[derive(Debug)]
#[repr(C, align(16))]
pub struct VirtualQueues {
    pub receive_queue: VirtQueue,
    pub transmit_queue: VirtQueue,
}

// 2.6.12 Virtqueue Operation
// There are two parts to virtqueue operation: supplying new available buffers to the device, and processing used buffers from the device.
// Note: As an example, the simplest virtio network device has two virtqueues: the transmit virtqueue and the receive virtqueue.
// The driver adds outgoing (device-readable) packets to the transmit virtqueue, and then frees them after they are used.
// Similarly, incoming (device-writable) buffers are added to the receive virtqueue, and processed after they are used.

#[derive(Debug)]
#[repr(C, align(16))]
pub struct VirtQueue {
    pub descriptors: [VirtqDescriptor; DESCRIPTOR_COUNT],
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

    pub flags: u16,

    /// Next field if flags contains NEXT
    pub next: u16,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed(2))]
pub struct VirtqAvailable {
    pub flags: u16,

    /// Index into VirtqDescriptor Array. Count of Descriptor Chain Heads
    pub idx: u16,

    /// The index of the head of the descriptor chain in the descriptor table
    pub ring: [u16; DESCRIPTOR_COUNT],
}

impl VirtqAvailable {
    pub fn default() -> VirtqAvailable {
        VirtqAvailable {
            flags: 0,
            idx: 0,
            ring: [0; DESCRIPTOR_COUNT],
        }
    }
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
pub struct VirtqUsed {
    pub flags: u16,

    /// Index into VirtqDescriptor Array
    pub idx: u16,

    pub ring: [VirtqUsedElement; DESCRIPTOR_COUNT],
}

impl VirtqUsed {
    pub fn default() -> VirtqUsed {
        VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElement { id: 0, len: 0 }; DESCRIPTOR_COUNT],
        }
    }
}
