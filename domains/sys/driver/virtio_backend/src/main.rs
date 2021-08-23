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

extern crate alloc;
extern crate malloc;

mod defs;
mod virtio_net;

use crate::{
    defs::{VirtioBackendQueue, MAX_SUPPORTED_QUEUES},
    virtio_net::VirtioNetInner,
};
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use console::println;
use core::{
    borrow::{Borrow, BorrowMut},
    panic::PanicInfo,
};
use libsyscalls::syscalls::{sys_backtrace, sys_create_thread, sys_yield};
use spin::{Mutex, MutexGuard, Once};
use syscalls::{Heap, Syscall};
use virtio_device::VirtioPciCommonConfig;
struct VirtioBackendInner {
    device_config: VirtioPciCommonConfig,
    backend_queues: Vec<Option<VirtioBackendQueue>>,
}

struct VirtioBackend(Once<Arc<Mutex<VirtioBackendInner>>>);

impl VirtioBackend {
    pub const fn new() -> Self {
        VirtioBackend(Once::new())
    }

    pub fn init(&mut self) {
        self.0.call_once(|| {
            Arc::new(Mutex::new(VirtioBackendInner {
                device_config: VirtioPciCommonConfig {
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
                backend_queues: vec![None, None, None],
            }))
        });
    }

    pub fn lock(&mut self) -> MutexGuard<VirtioBackendInner> {
        self.0.wait().unwrap().lock()
    }
}

static mut VIRTIO_BACKEND: VirtioBackend = VirtioBackend::new();

/// Call this function anytime the frontend modifies device config and backend needs to update
pub fn virtio_device_config_modified() {
    let mut backend = unsafe { VIRTIO_BACKEND.borrow_mut().lock() };

    unsafe {
        // Update the backend's info on the queues
        if backend.device_config.queue_enable == 1 {
            if backend.device_config.queue_select >= MAX_SUPPORTED_QUEUES {
                panic!("Virtio Backend Supports at most {} queues but the device has a queue at index {}", 
                MAX_SUPPORTED_QUEUES, 
                backend.device_config.queue_select);
            } else {
                // Update the queue information
                let queue = VirtioBackendQueue {
                    queue_index: backend.device_config.queue_select,
                    queue_enable: true,
                    queue_size: backend.device_config.queue_size,
                    queue_descriptor: backend.device_config.queue_desc,
                    queue_device: backend.device_config.queue_device,
                    queue_driver: backend.device_config.queue_driver,

                    device_idx: 0,
                    driver_idx: 0,
                };

                let index = queue.queue_index;
                backend.backend_queues[index as usize] = Some(queue);
            }
        }

        println!(
            "virtio_device_config_modified {:#?}",
            &backend.backend_queues
        );
    }
}

extern "C" fn virtio_frontend() {
    let backend = unsafe { VIRTIO_BACKEND.borrow_mut() };

    unsafe {
        let mut VirtioInner = VirtioNetInner::new(
            &backend.lock().device_config as *const VirtioPciCommonConfig as usize,
        );

        VirtioInner.init();
    }
}

fn virtio_backend() {
    let backend = unsafe { VIRTIO_BACKEND.borrow_mut() };

    loop {
        println!("{:#?}", backend.lock().device_config);
        println!("{:#?}", backend.lock().backend_queues);

        for q in &mut backend.lock().backend_queues {
            if let Some(queue) = q.as_mut() {
                println!("Driver Queue: {:#?}", queue.get_driver_queue());
            }
        }

        println!("Virtio Backend Yield");
        sys_yield();
    }
}

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    unsafe {
        VIRTIO_BACKEND.init();
    }

    sys_create_thread("virtio_frontend", virtio_frontend);
    virtio_backend();
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
