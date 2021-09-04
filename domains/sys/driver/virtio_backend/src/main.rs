#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra,
    core_intrinsics
)]

extern crate alloc;
extern crate malloc;

mod defs;
mod virtual_queue;

use crate::defs::VirtioBackendQueue;
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use console::{print, println};
use core::{
    intrinsics::size_of,
    panic::PanicInfo,
    ptr::{read_volatile, write_volatile},
};
use libsyscalls::syscalls::{sys_backtrace, sys_create_thread, sys_yield};
use libtime::sys_ns_sleep;
use spin::{Mutex, MutexGuard, Once};
use syscalls::{Heap, Syscall};
use virtio_backend_trusted::defs::{
    DeviceNotificationType, DEVICE_NOTIFY, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS,
};
use virtio_device::VirtioPciCommonConfig;
use virtio_net_mmio_device::VirtioNetworkDeviceConfig;
use virtual_queue::VirtualQueues;

struct VirtioBackendInner {
    backend_queues: Vec<Option<VirtioBackendQueue>>,
    virtual_queues: Vec<Option<VirtualQueues>>,
}

impl VirtioBackendInner {
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
        for i in 0..self.virtual_queues.len() {
            if i % 2 == 0 {
                self.process_rx_queue(i);
            }
        }
    }

    pub fn process_rx_queue(&mut self, idx: usize) {
        if self.virtual_queues[idx].is_none() {
            return;
        }

        let queue = self.virtual_queues[idx].as_mut().unwrap();

        // Check for new requests in available / driver queue
        let current_idx = *queue.driver_queue.idx();
        while queue.driver_queue.previous_idx != current_idx {
            // Get the index for the descriptor head
            let chain_header_idx = &mut queue.descriptor_queue.get_descriptors()
                [queue.driver_queue.previous_idx as usize];

            // Do actual processing here
            println!("Chain Descriptor Head: {:#?}", chain_header_idx);

            // Move to the next chain
            queue.driver_queue.previous_idx = queue.driver_queue.previous_idx.wrapping_add(1);
        }
    }

    pub fn process_descriptor(&mut self) {}
}

fn initialize_device_config_space() {
    unsafe {
        write_volatile(DEVICE_NOTIFY, 0);

        write_volatile(
            MMIO_ADDRESS,
            VirtioPciCommonConfig {
                device_feature_select: 0,
                device_feature: 0,
                driver_feature_select: 0,
                driver_feature: 0,
                msix_config: 0,
                num_queues: MAX_SUPPORTED_QUEUES,
                device_status: 0,
                config_generation: 0,
                queue_select: 0,
                queue_size: 256,
                queue_msix_vector: 0,
                queue_enable: 0,
                queue_notify_off: 0,
                queue_desc: 0,
                queue_driver: 0,
                queue_device: 0,
            },
        );
    }
}

fn process_notifications() -> ! {
    let mut backend = VirtioBackendInner {
        backend_queues: vec![None, None, None],
        virtual_queues: vec![None, None, None],
    };

    loop {
        let dn = unsafe { read_volatile(DEVICE_NOTIFY) };

        match DeviceNotificationType::from_value(dn) {
            DeviceNotificationType::DeviceConfigurationUpdated => {
                backend.handle_device_config_update();
            }
            DeviceNotificationType::QueueUpdated => {
                backend.handle_queue_notify();
            }
            DeviceNotificationType::None => {}
        }

        if dn != 0 {
            unsafe {
                write_volatile(DEVICE_NOTIFY, 0);
            }
        }

        sys_yield();
    }
}

extern "C" fn virtio_backend() {
    initialize_device_config_space();
    process_notifications();
}

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    sys_create_thread("virtio_backend", virtio_backend);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
