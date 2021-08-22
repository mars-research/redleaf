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

use alloc::{boxed::Box, vec, vec::Vec};
use console::println;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_backtrace, sys_create_thread, sys_yield};
use syscalls::{Heap, Syscall};
use virtio_device::VirtioPciCommonConfig;
use virtio_network_device::VirtioNetInner;

#[derive(Debug)]
struct VirtioBackendQueue {
    queue_index: u16,
    queue_size: u16,
    queue_enable: bool,
    queue_descriptor: u64,
    queue_driver: u64,
    queue_device: u64,
}

const MAX_SUPPORTED_QUEUES: u16 = 3;

static mut VIRTIO_DEVICE_CONFIG: VirtioPciCommonConfig = VirtioPciCommonConfig {
    /* About the whole device. */
    device_feature_select: 0,         /* read-write */
    device_feature: 0,                /* read-only for driver */
    driver_feature_select: 0,         /* read-write */
    driver_feature: 0,                /* read-write */
    msix_config: 0,                   /* read-write */
    num_queues: MAX_SUPPORTED_QUEUES, /* read-only for driver */
    device_status: 0,                 /* read-write */
    config_generation: 0,             /* read-only for driver */

    /* About a specific virtqueue. */
    queue_select: 0, /* read-write */

    /// Maximum number of items in Descriptor Queue
    queue_size: 256, /* read-write */
    queue_msix_vector: 0, /* read-write */
    queue_enable: 0,      /* read-write */
    queue_notify_off: 0,  /* read-only for driver */
    queue_desc: 0,        /* read-write */

    /// Available Ring
    queue_driver: 0, /* read-write */

    /// Used Ring
    queue_device: 0, /* read-write */
};

extern "C" fn virtio_frontend() {
    unsafe {
        let mut VirtioInner =
            VirtioNetInner::new(&VIRTIO_DEVICE_CONFIG as *const VirtioPciCommonConfig as usize);

        sys_yield();

        VirtioInner.init();
    }
}

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Virtio Backend!");

    let virtio_frontend_thread = sys_create_thread("virtio_frontend", virtio_frontend);

    let mut backend_queues: Vec<Option<VirtioBackendQueue>> = vec![None, None, None];

    loop {
        unsafe {
            println!("{:#?}", VIRTIO_DEVICE_CONFIG);

            // Update the backend's info on the queues
            if VIRTIO_DEVICE_CONFIG.queue_enable == 1 {
                if VIRTIO_DEVICE_CONFIG.queue_select >= MAX_SUPPORTED_QUEUES {
                    panic!("Virtio Backend Supports at most {} queues but the device has a queue at index {}", MAX_SUPPORTED_QUEUES, VIRTIO_DEVICE_CONFIG.queue_select);
                }

                // Update the queue information
                let queue = VirtioBackendQueue {
                    queue_index: VIRTIO_DEVICE_CONFIG.queue_select,
                    queue_enable: true,
                    queue_size: VIRTIO_DEVICE_CONFIG.queue_size,
                    queue_descriptor: VIRTIO_DEVICE_CONFIG.queue_desc,
                    queue_device: VIRTIO_DEVICE_CONFIG.queue_device,
                    queue_driver: VIRTIO_DEVICE_CONFIG.queue_driver,
                };

                backend_queues[VIRTIO_DEVICE_CONFIG.queue_select as usize] = Some(queue);
            }

            println!("{:#?}", &backend_queues);
        }
        sys_yield();
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
