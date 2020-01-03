#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    c_variadic,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_recv_int};
use console::println;

mod rt;

struct RumpSyscalls {}

impl RumpSyscalls {
    fn new() -> RumpSyscalls {
        RumpSyscalls{}
    }
}

impl syscalls::Xv6 for RumpSyscalls {}

extern fn xv6_kernel_test_th() {
   loop {
        println!("rump kernel test th"); 
        sys_yield(); 
   }
}

extern fn timer_thread() {
    println!("Registering rump timer thread"); 
    
    loop {
         sys_recv_int(syscalls::IRQ_TIMER);
         println!("rump: got a timer interrupt"); 
    }
}


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>) 
{
    libsyscalls::syscalls::init(s);
    
    use cstr_core::CStr;
    
    #[repr(C)]
    struct tmpfs_args {
        ta_version: u64, // c_int
        /* Size counters. */
        ta_nodes_max: u64, // ino_t			ta_nodes_max;
        ta_size_max: i64,  // off_t			ta_size_max;
        /* Root node attributes. */
        ta_root_uid: u32,  // uid_t			ta_root_uid;
        ta_root_gid: u32,  // gid_t			ta_root_gid;
        ta_root_mode: u32, // mode_t		ta_root_mode;
    }

    extern "C" {
        fn rump_boot_setsigmodel(sig: usize);
        fn rump_init() -> u64;
        fn mount(typ: *const i8, path: *const i8, n: u64, args: *const tmpfs_args, argsize: usize);
        fn open(path: *const i8, opt: u64) -> i64;
        fn read(fd: i64, buf: *mut i8, bytes: u64) -> i64;
        fn write(fd: i64, buf: *const i8, bytes: u64) -> i64;
    }

    let up = lineup::Upcalls {
        curlwp: rt::rumpkern_curlwp,
        deschedule: rt::rumpkern_unsched,
        schedule: rt::rumpkern_sched,
    };

    let mut scheduler = lineup::Scheduler::new(up);
    scheduler.spawn(
        32 * 4096,
        |_yielder| unsafe {
            let start = rawtime::Instant::now();
            rump_boot_setsigmodel(0);
            let ri = rump_init();
            assert_eq!(ri, 0);
            println!("rump_init({}) done in {:?}", ri, start.elapsed());

            const TMPFS_ARGS_VERSION: u64 = 1;

            let tfsa = tmpfs_args {
                ta_version: TMPFS_ARGS_VERSION,
                ta_nodes_max: 0,
                ta_size_max: 1 * 1024 * 1024,
                ta_root_uid: 0,
                ta_root_gid: 0,
                ta_root_mode: 0o1777,
            };

            let path = CStr::from_bytes_with_nul(b"/tmp\0");
            let tmpfs_ident = CStr::from_bytes_with_nul(b"tmpfs\0");
            println!("mounting tmpfs");

            let r = mount(
                tmpfs_ident.unwrap().as_ptr(),
                path.unwrap().as_ptr(),
                0,
                &tfsa,
                core::mem::size_of::<tmpfs_args>(),
            );
            println!("mounted tmpfs {:?}", r);

            let path = CStr::from_bytes_with_nul(b"/tmp/bla\0");
            println!("before open /tmp/bla");
            let fd = open(path.unwrap().as_ptr(), 0x00000202);
            println!("opened[1] /tmp/bla {}", fd);
            assert_eq!(fd, 3, "Proper FD was returned");
            println!("opened /tmp/bla");

            let wbuf: [i8; 12] = [0xa; 12];
            let bytes_written = write(fd, wbuf.as_ptr(), 12);
            assert_eq!(bytes_written, 12, "Write successful");
            println!("bytes_written: {:?}", bytes_written);

            let path = CStr::from_bytes_with_nul(b"/tmp/bla\0");
            let fd = open(path.unwrap().as_ptr(), 0x00000002);
            let mut rbuf: [i8; 12] = [0x00; 12];
            let read_bytes = read(fd, rbuf.as_mut_ptr(), 12);
            assert_eq!(read_bytes, 12, "Read successful");
            assert_eq!(rbuf[0], 0xa, "Read matches write");
            println!("bytes_read: {:?}", read_bytes);
        },
        core::ptr::null_mut(),
    );

    scheduler.run();
    // TODO: Don't drop the scheduler for now,
    // so we don't panic because of unfinished generators:
    core::mem::forget(scheduler);
    println!("rump tmpfs DONE");
    println!("init rump/core");
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
