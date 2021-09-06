use core::ptr::read_volatile;

use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use console::println;
use interface::{
    net::Net,
    rref::{RRef, RRefDeque},
};
use virtio_backend_trusted::defs::{
    BATCH_SIZE, BUFFER, BUFFER_PTR, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS,
};

use crate::{defs::VirtioBackendQueue, virtual_queue::VirtualQueues};

pub struct VirtioBackendInner {
    backend_queues: Vec<Option<VirtioBackendQueue>>,
    virtual_queues: Vec<Option<VirtualQueues>>,
    net: Box<dyn Net>,

    /// Used to hold the buffer pointers internally when moving them from the virtual queues to the device
    buffers: [Vec<BUFFER_PTR>; MAX_SUPPORTED_QUEUES as usize],

    /// Used so that we don't have to create RRefDeques when calling submit_and_poll_rref()
    /// 0 is packets and 1 is collect
    rref_queues: [Option<RRefDeque<BUFFER, 32>>; 2],
}

impl VirtioBackendInner {
    pub fn new(net: Box<dyn Net>) -> Self {
        VirtioBackendInner {
            backend_queues: vec![None, None, None],
            virtual_queues: vec![None, None, None],
            net,

            buffers: [
                Vec::with_capacity(BATCH_SIZE),
                Vec::with_capacity(BATCH_SIZE),
                Vec::with_capacity(BATCH_SIZE),
            ],

            rref_queues: [
                Some(RRefDeque::new([None; 32])),
                Some(RRefDeque::new([None; 32])),
            ],
        }
    }

    const fn is_rx_queue(queue_idx: usize) -> bool {
        queue_idx % 2 == 0
    }

    /// Call this function anytime the frontend modifies device config and backend needs to update
    pub fn handle_device_config_update(&mut self) {
        let device_config = unsafe { read_volatile(MMIO_ADDRESS) };

        // Update the backend's info on the queues
        if device_config.queue_enable == 1 {
            if device_config.queue_select >= MAX_SUPPORTED_QUEUES {
                panic!("Virtio Backend Supports at most {} queues but the device has a queue at index {}",
                MAX_SUPPORTED_QUEUES,
                device_config.queue_select);
            }

            // Update the queue information
            let queue = VirtioBackendQueue {
                queue_index: device_config.queue_select,
                queue_enable: true,
                queue_size: device_config.queue_size,
                queue_descriptor: device_config.queue_desc,
                queue_device: device_config.queue_device,
                queue_driver: device_config.queue_driver,

                device_idx: 0,
                driver_idx: 0,
            };

            let index = queue.queue_index as usize;
            self.backend_queues[index] = Some(queue);
            self.virtual_queues[index] = Some(VirtualQueues::new(
                device_config.queue_desc,
                device_config.queue_device,
                device_config.queue_driver,
                device_config.queue_size,
            ))
        }

        println!("virtio_device_config_modified {:#?}", &self.backend_queues);
    }

    pub fn handle_queue_notify(&mut self) {
        // Since there's currently no way of knowing which queue was updated check them all
        for i in 0..2 {
            self.process_virtual_queue(i);
        }
    }

    /// Finds applicable buffers in the queue and moves them to the buffer_vec
    fn fetch_buffers_from_queue(queue: &mut VirtualQueues, buffer_vec: &mut Vec<BUFFER_PTR>) {
        // Check for new requests in available / driver queue
        let driver_idx = *queue.driver_queue.idx();
        while queue.driver_queue.previous_idx != driver_idx && buffer_vec.len() < BATCH_SIZE {
            println!(
                "Available Chains: {}",
                driver_idx - queue.driver_queue.previous_idx
            );

            let descriptors = queue.descriptor_queue.get_descriptors();

            // Get the index for the descriptor head
            let mut current_idx =
                (queue.driver_queue.previous_idx % queue.descriptor_queue.queue_size()) as usize;

            // Do actual processing here
            loop {
                let descriptor = descriptors[current_idx];

                if descriptor.len == 1514 {
                    // Add it to the device and break
                    buffer_vec.push(descriptor.addr as BUFFER_PTR);
                    break;
                } else {
                    // Try the next descriptor
                    if (descriptor.flags & 0b1) == 0b1 {
                        current_idx = descriptor.next.into();
                    } else {
                        break;
                    }
                }
            }

            // Move to the next chain
            queue.driver_queue.previous_idx = queue.driver_queue.previous_idx.wrapping_add(1);
        }
    }

    fn move_buffers_to_rref_deque(
        buffer_vec: &mut Vec<BUFFER_PTR>,
        rref_deque: &mut RRefDeque<BUFFER, 32>,
    ) {
        while buffer_vec.len() != 0 && rref_deque.len() < 32 {
            let ptr = buffer_vec.remove(0);
            let r = RRef::new(unsafe { *ptr });
            rref_deque.push_back(r);
        }
    }

    fn process_virtual_queue(&mut self, queue_idx: usize) {
        Self::fetch_buffers_from_queue(
            self.virtual_queues[queue_idx].as_mut().unwrap(),
            &mut self.buffers[queue_idx],
        );

        println!(
            "Submitting {} buffers to device",
            self.buffers[queue_idx].len()
        );
        self.submit_to_device(queue_idx);
    }

    fn submit_to_device(&mut self, queue_idx: usize) {
        Self::move_buffers_to_rref_deque(
            &mut self.buffers[queue_idx],
            self.rref_queues[queue_idx].as_mut().unwrap(),
        );

        // Give all collected buffers to the device
        if let Ok(Ok((_, packets, collect))) = self.net.submit_and_poll_rref(
            self.rref_queues[0].take().unwrap(),
            self.rref_queues[1].take().unwrap(),
            !Self::is_rx_queue(queue_idx),
            1514,
        ) {
            self.rref_queues[0].replace(packets);
            self.rref_queues[1].replace(collect);
        } else {
            panic!("Communication with backend device failed!");
        }
    }

    fn print_descriptor_chain(queue: &VirtualQueues, chain_header_idx: u16) {
        let mut current_idx: usize = chain_header_idx.into();
        let descriptors = queue.descriptor_queue.get_descriptors();

        println!("---CHAIN {} START---", chain_header_idx);

        loop {
            // Get and print the descriptor
            let descriptor = descriptors[current_idx];
            println!("{:#?}", &descriptor);

            if (descriptor.flags & 0b1) == 0b1 {
                // Goto Next
                current_idx = descriptor.next.into();
            } else {
                break;
            }
        }

        println!("---CHAIN {} END---", chain_header_idx);
    }
}
