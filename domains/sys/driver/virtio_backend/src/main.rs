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

mod backend;
mod defs;
mod virtual_queue;

use crate::{backend::VirtioBackendInner, defs::VirtioBackendQueue};
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use console::{print, println};
use core::{
    borrow::BorrowMut,
    intrinsics::size_of,
    panic::PanicInfo,
    ptr::{read_volatile, write_volatile},
};
use interface::{
    net::Net,
    rref::{RRef, RRefDeque},
};
use libsyscalls::syscalls::{sys_backtrace, sys_create_thread, sys_yield};
use libtime::sys_ns_sleep;
use spin::{Mutex, MutexGuard, Once};
use syscalls::{Heap, Syscall};
use virtio_backend_trusted::defs::{
    DeviceNotificationType, BATCH_SIZE, BUFFER, DEVICE_NOTIFY, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS,
};
use virtio_device::{defs::VirtQueue, VirtioPciCommonConfig};
use virtio_net_mmio_device::VirtioNetworkDeviceConfig;
use virtual_queue::VirtualQueues;

struct VirtioBackendThreadArguments {
    net: Box<dyn Net>,
}

static mut THREAD_ARGUMENTS: Option<VirtioBackendThreadArguments> = None;

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

fn process_notifications(net: Box<dyn Net>) -> ! {
    let mut backend = VirtioBackendInner::new(net);

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
    // Retrieve Thread Arguments
    let args = unsafe { THREAD_ARGUMENTS.take().unwrap() };

    initialize_device_config_space();
    process_notifications(args.net);
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    net: Box<dyn Net>,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    // Prepare thread arguments
    unsafe {
        THREAD_ARGUMENTS = Some(VirtioBackendThreadArguments { net });
    }

    sys_create_thread("virtio_backend", virtio_backend);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
