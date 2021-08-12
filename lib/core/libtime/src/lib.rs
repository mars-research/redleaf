#![no_std]
static NS_IN_TIMER_TICK: u64 = 10_000_000;
static NS_IN_RDTSC: u64 = 3;

use console::println;
use libsyscalls::syscalls::sys_yield;

pub fn get_rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn get_ns_time() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() / NS_IN_RDTSC }
}

fn loop_sleep(ns: u64) {
    let target_ns = get_ns_time() + ns;
    loop {
        if get_ns_time() > target_ns {
            break;
        }
    }
}

pub fn sys_ns_sleep(ns: u64) {
    let target_ns = get_ns_time() + ns;
    loop {
        let current_ns = get_ns_time();
        if current_ns > target_ns {
            break;
        }

        let left_to_wait_ns = target_ns - current_ns;
        if left_to_wait_ns < NS_IN_TIMER_TICK {
            //println!("sys_ns_sleep: loopsleep, left to wait: {}", left_to_wait_ns);
            loop_sleep(left_to_wait_ns);
            break;
        }

        //println!("sys_ns_sleep: yield, left to wait:{}", left_to_wait_ns);
        sys_yield();
        //println!("sys_ns_sleep: back from yield");
    }
}

pub fn sys_ns_loopsleep(ns: u64) {
    let target_ns = get_ns_time() + ns;
    let current_ns = get_ns_time();

    if current_ns > target_ns {
        return;
    }

    let left_to_wait_ns = target_ns - current_ns;
    loop_sleep(left_to_wait_ns);
}
