#![no_std]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra
)]

pub mod defs;
pub mod virtual_queue;
extern crate alloc;

use alloc::boxed::Box;
use core::ptr::{read_volatile, write_volatile};
use defs::{
    Buffer, BufferPtr, DeviceNotificationType, DEVICE_NOTIFY, MAX_SUPPORTED_QUEUES, MMIO_ADDRESS,
};
use interface::{net::Net, rref::RRef};
use libsyscalls::syscalls::sys_yield;
use virtio_device::VirtioPciCommonConfig;

pub struct VirtioBackendThreadArguments {
    pub net: Box<dyn Net>,
}

pub static mut THREAD_ARGUMENTS: Option<VirtioBackendThreadArguments> = None;

pub fn get_thread_arguments() -> VirtioBackendThreadArguments {
    unsafe { THREAD_ARGUMENTS.take().unwrap() }
}

pub fn device_notify(notification_type: DeviceNotificationType) {
    let value = notification_type.value();

    unsafe {
        write_volatile(DEVICE_NOTIFY, value);

        const WAIT_VALUE: usize = DeviceNotificationType::None.value();

        // Wait for acknowledgement
        while read_volatile(DEVICE_NOTIFY) != WAIT_VALUE {
            sys_yield();
        }
    }
}

pub fn initialize_device_config_space() {
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

pub fn get_device_notifications() -> DeviceNotificationType {
    let dn = unsafe { read_volatile(DEVICE_NOTIFY) };

    if dn != DeviceNotificationType::None.value() {
        // Clear the notification
        unsafe {
            write_volatile(DEVICE_NOTIFY, 0);
        }
    }

    DeviceNotificationType::from_value(dn)
}

pub fn copy_buffer_into_rref(buffer: &BufferPtr, rref: &RRef<Buffer>) {
    unsafe {
        core::ptr::copy(*buffer, rref.as_ptr() as *mut Buffer, 1);
    }
}

pub fn copy_rref_into_buffer(rref: &RRef<Buffer>, buffer: BufferPtr) {
    unsafe {
        core::ptr::copy(rref.as_ptr() as *mut Buffer, buffer as *mut Buffer, 1);
    }
}
