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

pub mod defs;
extern crate alloc;

use core::ptr::{read_volatile, write_volatile};

use defs::{DeviceNotificationType, DEVICE_NOTIFY};
use libsyscalls::syscalls::sys_yield;

pub fn device_notify(notification_type: DeviceNotificationType) {
    let value = notification_type.value();

    unsafe {
        write_volatile(DEVICE_NOTIFY, value);

        const wait_value: usize = DeviceNotificationType::None.value();

        // Wait for acknowledgement
        while read_volatile(DEVICE_NOTIFY) != wait_value {
            sys_yield();
        }
    }
}
