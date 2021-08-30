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

use crate::defs::{VirtioBackendQueue, MAX_SUPPORTED_QUEUES};
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
use virtio_device::VirtioPciCommonConfig;
use virtio_net_mmio_device::VirtioNetworkDeviceConfig;

/// Increment this counter whenever you wish to notify the device
const DEVICE_NOTIFY: *mut usize = (0x100000 - size_of::<usize>()) as *mut usize;

const MMIO_ADDRESS: *mut VirtioPciCommonConfig = 0x100000 as *mut VirtioPciCommonConfig;
struct VirtioBackendInner {
    device_config: VirtioPciCommonConfig,
    backend_queues: Vec<Option<VirtioBackendQueue>>,
}

/// Call this function anytime the frontend modifies device config and backend needs to update
// pub fn virtio_device_config_modified() {
//     let mut backend = unsafe { VIRTIO_BACKEND.borrow_mut().lock() };

//     unsafe {
//         // Update the backend's info on the queues
//         if backend.device_config.queue_enable == 1 {
//             if backend.device_config.queue_select >= MAX_SUPPORTED_QUEUES {
//                 panic!("Virtio Backend Supports at most {} queues but the device has a queue at index {}",
//                 MAX_SUPPORTED_QUEUES,
//                 backend.device_config.queue_select);
//             } else {
//                 // Update the queue information
//                 let queue = VirtioBackendQueue {
//                     queue_index: backend.device_config.queue_select,
//                     queue_enable: true,
//                     queue_size: backend.device_config.queue_size,
//                     queue_descriptor: backend.device_config.queue_desc,
//                     queue_device: backend.device_config.queue_device,
//                     queue_driver: backend.device_config.queue_driver,

//                     device_idx: 0,
//                     driver_idx: 0,
//                 };

//                 let index = queue.queue_index;
//                 backend.backend_queues[index as usize] = Some(queue);
//             }
//         }

//         println!(
//             "virtio_device_config_modified {:#?}",
//             &backend.backend_queues
//         );
//     }
// }

extern "C" fn virtio_backend() {
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
                println!("{:#?}", read_volatile(MMIO_ADDRESS));
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
