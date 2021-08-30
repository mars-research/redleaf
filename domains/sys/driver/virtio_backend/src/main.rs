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

use crate::defs::VirtioBackendQueue;
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use console::println;
use core::{
    intrinsics::size_of,
    panic::PanicInfo,
    ptr::{read_volatile, write_volatile},
};
use libsyscalls::syscalls::{sys_backtrace, sys_create_thread, sys_yield};
use libtime::sys_ns_sleep;
use spin::{Mutex, MutexGuard, Once};
use syscalls::{Heap, Syscall};
use virtio_backend_trusted::defs::{DEVICE_NOTIFY, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS};
use virtio_device::VirtioPciCommonConfig;
use virtio_net_mmio_device::VirtioNetworkDeviceConfig;

struct VirtioBackendInner {
    backend_queues: Vec<Option<VirtioBackendQueue>>,
}

impl VirtioBackendInner {
    /// Call this function anytime the frontend modifies device config and backend needs to update
    pub fn update_virtio_device_queue_config(&mut self) {
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

            let index = queue.queue_index;
            self.backend_queues[index as usize] = Some(queue);
        }

        println!("virtio_device_config_modified {:#?}", &self.backend_queues);
    }
}

extern "C" fn virtio_backend() {
    let mut backend = VirtioBackendInner {
        backend_queues: vec![None, None, None],
    };

    // Initialize the device config
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

    loop {
        // println!("Virtio Backend!");
        unsafe {
            let dn = read_volatile(DEVICE_NOTIFY);
            // println!("DEVICE_NOTIFY: {:#?}", &dn);

            if dn != 0 {
                backend.update_virtio_device_queue_config();
                write_volatile(DEVICE_NOTIFY, 0);
            }
        }
        sys_yield();
    }
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
