use core::ptr::read_volatile;

use alloc::collections::VecDeque;
use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use console::println;
use hashbrown::HashMap;
use interface::{
    net::Net,
    rref::{RRef, RRefDeque},
};
use virtio_backend_trusted::defs::{
    Buffer, BufferPtr, BATCH_SIZE, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS, SHARED_MEMORY_REGION_PTR,
};
use virtio_device::defs::VirtqUsedElement;

use crate::{defs::VirtioQueueConfig, virtual_queue::VirtualQueue};

pub struct VirtioBackend {
    /// A copy of the queue_config's device status
    device_status: u8,

    queue_information: Vec<Option<VirtioQueueConfig>>,
    virtual_queues: Vec<Option<VirtualQueue>>,
    net: Box<dyn Net>,

    // *** The below variables are used to simplify communicating with RRef Net Interface ***
    /// We have to copy the arrays in order to satisfy the interface so this keeps track of which RRef
    /// corresponds to which buffer
    buffer_rref_map: HashMap<u64, u64>,

    /// Used so that we don't have to create RRefDeques when calling submit_and_poll_rref()
    /// 0 is packets and 1 is collect
    rref_queues: (Option<RRefDeque<Buffer, 32>>, Option<RRefDeque<Buffer, 32>>),
}

impl VirtioBackend {
    pub fn new(net: Box<dyn Net>) -> Self {
        VirtioBackend {
            device_status: 0,
            queue_information: vec![None, None],
            virtual_queues: vec![None, None],
            net,

            rref_queues: (
                Some(RRefDeque::new([None; 32])),
                Some(RRefDeque::new([None; 32])),
            ),
            buffer_rref_map: HashMap::new(),
        }
    }

    /// According to the VirtIO Net Spec, Even Queues are used for RX and Odd Queues are used for TX
    const fn is_rx_queue(queue_idx: usize) -> bool {
        queue_idx % 2 == 0
    }

    pub fn device_enabled(&self) -> bool {
        self.device_status == 15
    }

    /// Call this function anytime the frontend modifies device config and backend needs to update
    pub fn handle_device_config_update(&mut self) {
        let device_config = unsafe { read_volatile(MMIO_ADDRESS) };

        self.device_status = device_config.device_status;

        // Update the backend's info on the queues
        if device_config.queue_enable == 1 {
            if device_config.queue_select >= MAX_SUPPORTED_QUEUES {
                panic!("Virtio Backend Supports at most {} queues but the device has a queue at index {}",
                MAX_SUPPORTED_QUEUES,
               { device_config.queue_select});
            }

            // Update the queue information
            let queue = VirtioQueueConfig {
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
            self.queue_information[index] = Some(queue);
            self.virtual_queues[index] = Some(VirtualQueue::new(
                device_config.queue_desc,
                device_config.queue_device,
                device_config.queue_driver,
                device_config.queue_size,
                Self::is_rx_queue(index),
            ))
        }

        println!(
            "virtio_device_config_modified {:#?}",
            &self.queue_information
        );
    }

    /// Handles Queue Notifications submitted by the frontend
    pub fn handle_queue_notify(&mut self) {
        if !self.device_enabled() {
            return;
        }

        // Since there's currently no way of knowing which queue was updated check them all
        for i in 0..self.virtual_queues.len() {
            self.notify_virtual_queue(i);
        }
    }

    fn notify_virtual_queue(&mut self, queue_idx: usize) {
        self.move_buffers_from_frontend_to_rref_deque(queue_idx);

        self.call_submit_and_poll_rref(!Self::is_rx_queue(queue_idx));

        self.move_buffers_from_rref_deque_to_frontend(queue_idx);
    }

    fn move_buffers_from_frontend_to_rref_deque(&mut self, queue_idx: usize) {
        // Create new RRefs and add them to the queue
        let queue = self.virtual_queues[queue_idx].as_mut().unwrap();
        let buffers = queue.fetch_new_buffers();
        let packets = self.rref_queues.0.as_mut().unwrap();

        let rx = Self::is_rx_queue(queue_idx);

        while buffers.len() > 0 && packets.len() < BATCH_SIZE {
            let buffer = buffers.pop_front().unwrap();

            let rref = RRef::new([0; 1514]);

            if !rx {
                // If it's tx we need to copy the buffer's contents into the RRef
                unsafe {
                    core::ptr::copy(buffer, rref.as_ptr() as *mut Buffer, 1);
                }
            }

            self.buffer_rref_map
                .insert(rref.as_ptr() as u64, buffer as u64);
            packets.push_back(rref);
        }
    }

    fn move_buffers_from_rref_deque_to_frontend(&mut self, queue_idx: usize) {
        let rx = Self::is_rx_queue(queue_idx);
        self.call_submit_and_poll_rref(!rx);

        let queue = self.virtual_queues[queue_idx].as_mut().unwrap();
        let collect = self.rref_queues.1.as_mut().unwrap();
        assert!(
            self.rref_queues.0.as_ref().unwrap().len() == 0,
            "Packets queue should be flushed completely!"
        );

        // Move buffers from collect queue
        while let Some(rref) = collect.pop_front() {
            if let Some(buffer) = self.buffer_rref_map.remove(&(rref.as_ptr() as u64)) {
                if rx {
                    unsafe {
                        core::ptr::copy(rref.as_ptr() as *mut Buffer, buffer as *mut Buffer, 1);
                    }
                }

                queue.mark_buffers_as_complete(&[buffer as BufferPtr]);
            } else {
                panic!(
                    "RRef address must have changed! FAILED RREF ADDR: {:#?}, EXPECTED: {:#?}",
                    rref.as_ptr() as u64,
                    self.buffer_rref_map
                );
            }
        }
    }

    pub fn update_queues(&mut self) {
        if !self.device_enabled() {
            return;
        }

        for queue_idx in 0..self.virtual_queues.len() {
            self.move_buffers_from_rref_deque_to_frontend(queue_idx);
        }
    }

    /// Returns the number of packets added to the collect queue by the device
    fn call_submit_and_poll_rref(&mut self, tx: bool) -> usize {
        if let Ok(Ok((pkt_count, packets, collect))) = self.net.submit_and_poll_rref(
            self.rref_queues.0.take().unwrap(),
            self.rref_queues.1.take().unwrap(),
            tx,
            1514,
        ) {
            self.rref_queues.0.replace(packets);
            self.rref_queues.1.replace(collect);
            return pkt_count;
        } else {
            panic!("Communication with backend device failed!");
        }
    }

    fn print_descriptor_chain(queue: &VirtualQueue, chain_header_idx: u16) {
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
