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
struct BlockBuffer {
    /// IN: 0, OUT: 1, FLUSH: 4, DISCARD: 11, WRITE_ZEROES: 13
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
    pub data: [u8; 512],

    /// OK: 0, IOERR: 1, UNSUPP: 2
    pub status: u8,
}

static mut TEMP_BUFFER: BlockBuffer = BlockBuffer {
    request_type: 0,
    reserved: 0,
    sector: 0,
    data: [0x11; 512],
    status: 0,
};

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

    pub fn read(&mut self) {
        // self.submit_read_request();
        // Poll until ready
    }

    pub fn submit_read_request(&mut self, sector_number: u64) {
        unsafe {
            println!("{:x?}", TEMP_BUFFER);
        }

        if let Ok(free_descriptor) = Self::get_free_descriptor(&mut self.free_descriptors) {
            let addr: u64 = 0;

            unsafe {
                addr = Self::get_addr(&TEMP_BUFFER);
            }

            self.request_queue.descriptors[free_descriptor as usize] = VirtqDescriptor {
                addr: Self::get_addr(&TEMP_BUFFER),
                len: 529,
                flags: 2,
                next: 0,
            };

            unsafe {
                println!("TEMP_BUFFER ADDR: {:}", Self::get_addr(&TEMP_BUFFER));
            }

            self.request_queue.available.ring[self.request_queue.available.idx as usize] =
                free_descriptor;
            self.request_queue.available.idx += 1;

            unsafe {
                self.mmio.queue_notify(0, 0);
            }
        } else {
            println!("Virtio Block: No free descriptors, request dropped");
        }

        for i in 0..5 {
            println!("Sleep {:}", i);
            libtime::sys_ns_loopsleep(1_000_000_000);
        }

        println!("{:#?}", self.request_queue.used.idx);
        unsafe {
            println!("{:x?}", TEMP_BUFFER);
        }
    }
}
