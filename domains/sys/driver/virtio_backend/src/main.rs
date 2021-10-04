#![no_std]
#![no_main]
// #![forbid(unsafe_code)]
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

mod virtio_backend;

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
use virtio_backend::VirtioBackend;
use virtio_backend_trusted::{
    defs::{
        Buffer, DeviceNotificationType, BATCH_SIZE, DEVICE_NOTIFY, MAX_SUPPORTED_QUEUES,
        MMIO_ADDRESS,
    },
    get_device_notifications, get_thread_arguments, VirtioBackendThreadArguments, THREAD_ARGUMENTS,
};
use virtio_backend_trusted::{initialize_device_config_space, virtual_queue::VirtualQueue};
use virtio_device::{defs::VirtQueue, VirtioPciCommonConfig};
use virtio_net_mmio_device::VirtioNetworkDeviceConfig;

pub extern "C" fn virtio_backend() {
    // Retrieve Thread Arguments
    let args = get_thread_arguments();

    initialize_device_config_space();
    process_notifications(args.net);
}

fn process_notifications(net: Box<dyn Net>) -> ! {
    let mut backend = VirtioBackend::new(net);

    loop {
        // Check device for processed buffers and move to queues
        backend.update_queues();

        let notification = get_device_notifications();

        match notification {
            DeviceNotificationType::DeviceConfigurationUpdated => {
                backend.handle_device_config_update();
            }
            DeviceNotificationType::QueueUpdated => {
                backend.handle_queue_notify();
            }
            DeviceNotificationType::None => {}
        }

        sys_yield();
    }
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
