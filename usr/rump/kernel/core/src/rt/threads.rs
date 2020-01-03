use super::RumpError;
use core::ops::Add;
use cstr_core::CStr;
use lineup::tls::Environment;
use rawtime::{Duration, Instant};

use console::{println, print};
use log::{error, info, trace};

#[allow(non_camel_case_types)]
pub type rumplwpop = u32;

pub const RUMPLWPOP_RUMPUSER_LWP_CREATE: rumplwpop = 0;
pub const RUMPLWPOP_RUMPUSER_LWP_DESTROY: rumplwpop = 1;
pub const RUMPLWPOP_RUMPUSER_LWP_SET: rumplwpop = 2;
pub const RUMPLWPOP_RUMPUSER_LWP_CLEAR: rumplwpop = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct lwp {
    _unused: [u8; 0],
}

/// Create a schedulable host thread context. The rump kernel will call
/// this interface when it creates a kernel thread.
/// The scheduling policy for the new thread is defined by the hypervisor.
/// In case the hypervisor wants to optimize the scheduling of the threads,
/// it can perform heuristics on the thrname, priority and cpuidx parameters.
#[no_mangle]
pub unsafe extern "C" fn rumpuser_thread_create(
    fun: Option<unsafe extern "C" fn(arg1: *mut u8) -> *mut u8>,
    arg: *mut u8,
    name: *const u8,
    mustjoin: i64,
    priority: i64,
    cpuidx: i64,
    cookie: *mut *mut u8,
) -> i64 {
    let tname = CStr::from_ptr(name as *const i8)
        .to_str()
        .unwrap_or("[unknown]");
    trace!(
        "rumpuser_thread_create {} {:?} {:p} join={} prio={} cpu={} cookie={:p}",
        tname,
        fun,
        arg,
        mustjoin,
        priority,
        cpuidx,
        cookie
    );

    let s = lineup::tls::Environment::thread();
    s.spawn(fun, arg);

    0
}

/// Called when a thread created with rumpuser_thread_create() exits.
#[no_mangle]
pub unsafe extern "C" fn rumpuser_thread_exit() {
    let t = lineup::tls::Environment::thread();
    loop {
        info!("rumpuser_thread_exit {:?}", lineup::tls::Environment::tid());
        t.block();
        unreachable!("rumpuser_thread_exit");
    }
}

/// Wait for a joinable thread to exit. The cookie matches the value from rumpuser_thread_create().
#[no_mangle]
pub unsafe extern "C" fn rumpuser_thread_join(_cookie: *mut u8) -> i64 {
    unreachable!("rumpuser_thread_join");
}

#[no_mangle]
pub unsafe extern "C" fn rumpuser_curlwpop(op: rumplwpop, lwp: *const lwp) -> i64 {
    trace!(
        "{:?} rumpuser_curlwpop op={} lwp={:p}",
        lineup::tls::Environment::tid(),
        op,
        lwp
    );
    let t = lineup::tls::Environment::thread();

    if op == RUMPLWPOP_RUMPUSER_LWP_SET {
        t.set_lwp(lwp as *const u64);
    }
    if op == RUMPLWPOP_RUMPUSER_LWP_CLEAR {
        assert!(t.rump_lwp == lwp as *const u64);
        t.set_lwp(core::ptr::null());
    }

    0
}

/*
979148367 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=SET lwp=0xffffffff803a0b40
980654579 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=CREAT lwp=0xffffffff8273c000
982041302 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=CLEAR lwp=0xffffffff803a0b40
983427129 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=SET lwp=0xffffffff8273c000
984934736 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=CREAT lwp=0xffffffff8273d800
986337116 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=CLEAR lwp=0xffffffff8273c000
987740425 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=SET lwp=0xffffffff8273d800
989263375 [ERROR] - bespin::rumprt::threads: ThreadId(1) rumpuser_curlwpop op=DESTROY lwp=0xffffffff8273c000
*/

#[no_mangle]
pub unsafe extern "C" fn rumpuser_curlwp() -> *mut lwp {
    //debug!("rumpuser_curlwp");
    let t = lineup::tls::Environment::thread();
    t.rump_lwp as *mut lwp
}

/// int rumpuser_clock_sleep(int enum_rumpclock, int64_t sec, long nsec)
///
/// enum_rumpclock   In case of RUMPUSER_CLOCK_RELWALL, the sleep should last
/// at least as long as specified.  In case of
/// RUMPUSER_CLOCK_ABSMONO, the sleep should last until the
/// hypervisor monotonic clock hits the specified absolute
/// time.
#[no_mangle]
pub unsafe extern "C" fn rumpuser_clock_sleep(enum_rumpclock: u32, sec: i64, nanos: u64) -> isize {
    trace!(
        "{:?} rumpuser_clock_sleep({}, {}, {})",
        Environment::tid(),
        enum_rumpclock,
        sec,
        nanos
    );
    // TODO: ignored _enum_rumpclock

    let mut nlocks = 0;
    super::rumpkern_unsched(&mut nlocks, None);

    let (until, retval) = match enum_rumpclock as u64 {
        super::RUMPUSER_CLOCK_ABSMONO => {
            // TODO: this will negative overflow panic on bad timed irq
            (
                Instant::from_nanos((sec as u128) * 1_000_000_000 + nanos as u128) - Instant::now(),
                0,
            )
        }
        super::RUMPUSER_CLOCK_RELWALL => (
            Duration::from_secs(sec as u64).add(Duration::from_nanos(nanos)),
            0,
        ),
        _ => (Duration::from_secs(0), RumpError::EINVAL as isize),
    };

    let t = Environment::thread();
    t.sleep(until);

    super::rumpkern_sched(&nlocks, None);

    retval
}

#[no_mangle]
pub unsafe extern "C" fn rumpuser_seterrno(errno: isize) {
    println!("rumpuser_seterrno {}", errno);
}
