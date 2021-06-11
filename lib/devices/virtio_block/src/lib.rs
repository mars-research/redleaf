#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra
)]

pub mod pci;
extern crate alloc;

use core::{u16, usize};

use alloc::sync::Arc;
use console::println;
use hashbrown::HashMap;
use interface::rref::{RRef, RRefDeque};
use libtime;
use spin::Mutex;
use virtio_device::defs::{
    VirtQueue, VirtqAvailable, VirtqDescriptor, VirtqUsed, VirtqUsedElement, VirtualQueues,
    DESCRIPTOR_COUNT,
};
use virtio_device::{Mmio, VirtioDeviceStatus};

#[derive(Debug)]
#[repr(C, packed)]

struct BlockBufferHeader {
    /// IN: 0, OUT: 1, FLUSH: 4, DISCARD: 11, WRITE_ZEROES: 13
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

#[derive(Debug)]
#[repr(C, packed)]
struct BlockBufferStatus {
    /// OK: 0, IOERR: 1, UNSUPP: 2
    pub status: u8,
}
#[derive(Debug)]
#[repr(C, packed)]
struct BlockBufferData {
    pub data: [u8; 512],
}

pub struct VirtioBlockInner {
    mmio: Mmio,
    request_queue: VirtQueue,

    /// Tracks which descriptors on the queue are free
    free_descriptors: [bool; DESCRIPTOR_COUNT],

    // The last index (of the used ring) that was checked by the driver
    request_last_idx: u16,
    // rx_buffers: HashMap<u64, RRef<NetworkPacketBuffer>>,
}

impl VirtioBlockInner {
    /// Returns an initialized VirtioBlock from a base address.
    unsafe fn new(mmio_base: usize) -> Self {
        let mmio = Mmio::new(mmio_base);

        let request_queue = VirtQueue {
            descriptors: [VirtqDescriptor {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0,
            }; DESCRIPTOR_COUNT],
            available: VirtqAvailable {
                flags: 0,
                idx: 0,
                ring: [0; DESCRIPTOR_COUNT],
            },
            used: VirtqUsed {
                flags: 0,
                idx: 0,
                ring: [VirtqUsedElement { id: 0, len: 0 }; DESCRIPTOR_COUNT],
            },
        };

        let free_descriptors = [true; DESCRIPTOR_COUNT];

        let virtio_inner = Self {
            mmio,
            request_queue,

            free_descriptors,
            request_last_idx: 0,
        };

        virtio_inner
    }

    pub fn init(&mut self) {
        println!("Initializing Virtio Block Device");

        // VIRTIO DEVICE INIT
        // https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-920001
        //
        // Reset the device.
        // Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
        // Set the DRIVER status bit: the guest OS knows how to drive the device.
        // Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device. During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check that it can support the device before accepting it.
        // Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
        // Re-read device status to ensure the FEATURES_OK bit is still set: otherwise, the device does not support our subset of features and the device is unusable.
        // Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup, reading and possibly writing the device’s virtio configuration space, and population of virtqueues.
        // Set the DRIVER_OK status bit. At this point the device is “live”.

        // Acknowledge Device
        unsafe {
            self.mmio
                .update_device_status(VirtioDeviceStatus::Acknowledge);
            self.mmio.update_device_status(VirtioDeviceStatus::Driver); // But do we really know how to drive the device?
        }

        // self.negotiate_features();

        // Tell the Device that feature Negotiation is complete
        unsafe {
            self.mmio
                .update_device_status(VirtioDeviceStatus::FeaturesOk);
        }

        // Check that Features OK Bit is still set!
        // self.print_device_status();
        if (self.mmio.accessor.read_device_status() & VirtioDeviceStatus::FeaturesOk.value()) == 0 {
            panic!("Failed to negotiate Virtio Block features!");
        }

        // Setup Virtual Queues
        self.initialize_virtual_queue(0, &(self.request_queue));

        // Tell the Device we're all done, even though we aren't
        unsafe { self.mmio.update_device_status(VirtioDeviceStatus::DriverOk) };

        self.mmio.accessor.write_queue_select(0);

        println!("VIRTIO BLOCK READY!");

        self.print_device_config();
    }

    pub fn print_device_config(&mut self) {
        let mut cfg = unsafe { self.mmio.read_common_config() };
        println!("{:#?}", cfg);
    }

    /// Receive Queues must be 2*N and Transmit Queues must be 2*N + 1
    /// For example, Receive Queue must be 0 and Transmit Queue must be 1
    pub fn initialize_virtual_queue(&self, queue_index: u16, virt_queue: &VirtQueue) {
        self.mmio.accessor.write_queue_select(queue_index);

        self.mmio.accessor.write_queue_desc(
            (&virt_queue.descriptors as *const [VirtqDescriptor; DESCRIPTOR_COUNT]) as u64,
        );
        self.mmio
            .accessor
            .write_queue_driver((&virt_queue.available as *const VirtqAvailable) as u64);
        self.mmio
            .accessor
            .write_queue_device((&virt_queue.used as *const VirtqUsed) as u64);
        self.mmio.accessor.write_queue_enable(1);
    }

    fn get_addr<T>(obj: &T) -> u64 {
        (obj as *const T) as u64
    }

    /// Errors if there are no free descriptors
    fn get_free_descriptor(free_descriptors: &mut [bool; DESCRIPTOR_COUNT]) -> Result<u16, ()> {
        for i in 0..free_descriptors.len() {
            if free_descriptors[i] {
                free_descriptors[i] = false;
                return Ok(i as u16);
            }
        }
        Err(())
    }

    /// Errors if there are no free descriptors
    fn get_three_free_descriptor(
        free_descriptors: &mut [bool; DESCRIPTOR_COUNT],
    ) -> Result<(usize, usize, usize), ()> {
        let mut desc = (None, None, None);

        for i in 0..free_descriptors.len() {
            if free_descriptors[i] {
                free_descriptors[i] = false;

                if (desc.0.is_none()) {
                    desc.0 = Some(i);
                } else if (desc.1.is_none()) {
                    desc.1 = Some(i);
                } else if (desc.2.is_none()) {
                    desc.2 = Some(i);

                    return Ok((desc.0.unwrap(), desc.1.unwrap(), desc.2.unwrap()));
                }
            }
        }
        Err(())
    }

    pub fn read(&mut self) {
        // self.submit_read_request();
        // Poll until ready
    }

    pub fn submit_read_request(&mut self, sector_number: u64) {
        // self.request_queue.available.idx += 3;

        let mut blk_header = BlockBufferHeader {
            request_type: 0,
            reserved: 0,
            sector: 1,
        };

        self.free_descriptors[0] = false;
        self.free_descriptors[1] = false;
        self.free_descriptors[2] = false;

        let mut blk_data = BlockBufferData { data: [0x11; 512] };

        let mut blk_status = BlockBufferStatus { status: 0xFF };

        println!("{:#?}", blk_header);
        println!("{:?}", blk_data);
        println!("{:#?}", blk_status);

        if let Ok(desc_idx) = Self::get_three_free_descriptor(&mut self.free_descriptors) {
            self.request_queue.descriptors[desc_idx.0] = VirtqDescriptor {
                addr: Self::get_addr(&blk_header),
                len: 32,
                flags: 1,
                next: desc_idx.1 as u16,
            };

            self.request_queue.descriptors[desc_idx.1] = VirtqDescriptor {
                addr: Self::get_addr(&blk_data),
                len: 512,
                flags: 1 | 2,
                next: desc_idx.2 as u16,
            };

            self.request_queue.descriptors[desc_idx.2] = VirtqDescriptor {
                addr: Self::get_addr(&blk_status),
                len: 1,
                flags: 2,
                next: 0,
            };

            println!("{:#?}", desc_idx);

            self.request_queue.available.ring[self.request_queue.available.idx as usize] =
                desc_idx.0 as u16;
            Mmio::memory_fence();
            self.request_queue.available.idx += 1;

            unsafe {
                self.mmio.queue_notify(0, 0);
            }
        } else {
            println!("Virtio Block: No free descriptors, request dropped");
        }

        for i in 0..10 {
            println!("Sleep {:}", i);
            libtime::sys_ns_loopsleep(1_000_000_000);
        }

        println!("{:#?}", self.request_queue.used.idx);
        println!("{:#?}", blk_header);
        println!("{:?}", blk_data);
        println!("{:#?}", blk_status);
    }
}
