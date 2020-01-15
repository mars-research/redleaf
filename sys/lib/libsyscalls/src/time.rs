
static NS_IN_TIMER_TICK: u64 = 10000000;
static NS_IN_RDTSC: u64 = 2;

pub fn get_rdtsc() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

pub fn get_ns_time() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc() / NS_IN_RDTSC
    }
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
        let left_to_wait_ns = target_ns - current_ns; 
        if left_to_wait_ns < NS_IN_TIMER_TICK {
            crate::syscalls::sys_println("sys_ns_sleep: loopsleep");
            loop_sleep(left_to_wait_ns);
            break;
        }

        crate::syscalls::sys_println("sys_ns_sleep: yield");
        crate::syscalls::sys_yield(); 
    }
}
