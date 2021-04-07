#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;

use core::panic::PanicInfo;

use interface::rref::RRefVec;
use syscalls::{Heap, Syscall};
use interface::rv6::Rv6;
use interface::vfs::{DirectoryEntry, DirectoryEntryRef, FileMode, INodeFileType};
use usrlib::println;
use usrlib::syscalls::{sys_close, sys_open_slice_slow, sys_read, sys_seek, sys_write};

const ONE_MS: u64 = 2_400_000;
const TEN_MS: u64 = 10 * ONE_MS;
const ONE_SEC: u64 = 2_400_000_000;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone_rv6().unwrap());
    println!("Starting rv6 benchfs with args: {}", args);

    let mut args = args.split_whitespace();
    args.next().unwrap();
    let test = args.next().unwrap_or("throughput");
    let options = args.next().unwrap_or("r");
    let file = args.next().unwrap_or("large");

    match test {
        "throughput" => bench_throughput(&*rv6, options, file),
        "restart" => bench_restart(&*rv6, options, file),
        _ => panic!("{}", test),
    }
}

fn bench_throughput(_rv6: &dyn Rv6, options: &str, file: &str) {
    let sizes = [
        512,
        1024,
        4096,
        8192,
        16 * 1024,
        256 * 1024,
        1024 * 1024,
        4 * 1024 * 1024,
        16 * 1024 * 1024,
        64 * 1024 * 1024,
    ];

    for bsize in sizes.iter() {
        let mut buffer = RRefVec::new(123u8, *bsize);

        if options.contains('w') {
            let fd = sys_open_slice_slow(file, FileMode::WRITE | FileMode::CREATE).unwrap();

            // warm up
            buffer = sys_write(fd, buffer).unwrap().1;

            let start = libtime::get_rdtsc();
            let mut total_size = 0;
            for _ in 0..1024 {
                if total_size > 64 * 1024 * 1024 {
                    break;
                }
                let (size, buffer_back) = sys_write(fd, buffer).unwrap();
                buffer = buffer_back;
                total_size += size;
            }
            println!(
                "Write: buffer size: {}, total bytes: {}, cycles: {}",
                bsize,
                total_size,
                libtime::get_rdtsc() - start
            );

            sys_close(fd).unwrap();
        }

        if options.contains('r') {
            let fd = sys_open_slice_slow(file, FileMode::READ).unwrap();

            // warm up
            buffer = sys_read(fd, buffer).unwrap().1;

            let start = libtime::get_rdtsc();
            let mut total_size = 0;
            loop {
                let (size, buffer_back) = sys_read(fd, buffer).unwrap();
                buffer = buffer_back;
                if size == 0 {
                    break;
                }
                total_size += size;
            }
            println!(
                "Read: buffer size: {}, total bytes: {}, cycles: {}",
                bsize,
                total_size,
                libtime::get_rdtsc() - start
            );

            sys_close(fd).unwrap();
        }
    }
}

fn bench_restart(_rv6: &dyn Rv6, options: &str, file: &str) {
    let file_size = 128 * 1024 * 1024;

    // let buffer_sizes = [512, 1024, 4096, 8192, 16 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024, 16 * 1024 * 1024, 64 * 1024 * 1024];
    let buffer_sizes = [4 * 1024];

    for bsize in buffer_sizes.iter() {
        let bsize = *bsize;
        let mut buffer = RRefVec::new(123u8, bsize);

        // 4GB
        let total_size = 4 * 1024 * 1024 * 1024;
        assert!(total_size % bsize == 0);
        if options.contains('w') {
            println!("begin fs write benchmark");
            let fd = sys_open_slice_slow(file, FileMode::WRITE | FileMode::CREATE).unwrap();

            // warm up
            buffer = sys_write(fd, buffer).unwrap().1;
            sys_seek(fd, 0).unwrap();

            let mut recording: [(u64, f64); 100_000] = [(0, 0.0); 100_000];
            let mut recording_index = 0;
            let mut curr_size = 0;
            let mut seek_count = 0;
            let start = libtime::get_rdtsc();
            let mut intervel_start = start;
            let mut interval_read = 0;
            for offset in (bsize..total_size + bsize).step_by(bsize) {
                let curr_time = libtime::get_rdtsc();
                if curr_time >= intervel_start + TEN_MS {
                    let elapse = curr_time - intervel_start;
                    // prints bytes per second
                    recording[recording_index] = (
                        curr_time,
                        interval_read as f64 / elapse as f64 * ONE_SEC as f64,
                    );
                    recording_index += 1;
                    intervel_start = curr_time;
                    curr_size += interval_read;
                    interval_read = 0;
                }
                if offset % file_size == 0 {
                    sys_seek(fd, 0).unwrap();
                    seek_count += 1;
                }
                let (bytes_read, buffer_back) = sys_write(fd, buffer).unwrap();
                buffer = buffer_back;
                interval_read += bytes_read;
            }
            let curr_time = libtime::get_rdtsc();
            let elapse = curr_time - intervel_start;
            curr_size += interval_read;
            recording[recording_index] = (
                curr_time,
                interval_read as f64 / elapse as f64 * ONE_SEC as f64,
            );
            let elapse = libtime::get_rdtsc() - start;

            {
                println!("timestamp(s),throughput(MB/s),");
                let start = recording[0].0;
                for (time_stamp, throughput) in &recording[0..recording_index + 1] {
                    println!(
                        "{},{},",
                        (time_stamp - start) as f64 / ONE_SEC as f64,
                        throughput / 1_000_000.0
                    );
                }
            }

            println!(
                "Write: buffer size: {}, total bytes: {}, cycles: {}, seek count: {}",
                bsize, total_size, elapse, seek_count
            );
            assert_eq!(curr_size, total_size);

            sys_close(fd).unwrap();
        }

        // 30GB
        let total_size = 30 * 1024 * 1024 * 1024;
        assert!(total_size % bsize == 0);
        if options.contains('r') {
            println!("begin fs read benchmark");
            let fd = sys_open_slice_slow(file, FileMode::READ).unwrap();

            // warm up
            buffer = sys_read(fd, buffer).unwrap().1;
            sys_seek(fd, 0).unwrap();

            let mut recording: [(u64, f64); 100_000] = [(0, 0.0); 100_000];
            let mut recording_index = 0;
            let mut curr_size = 0;
            let mut seek_count = 0;
            let start = libtime::get_rdtsc();
            let mut intervel_start = start;
            let mut interval_read = 0;
            for offset in (bsize..total_size + bsize).step_by(bsize) {
                let curr_time = libtime::get_rdtsc();
                if curr_time >= intervel_start + TEN_MS {
                    let elapse = curr_time - intervel_start;
                    // prints bytes per second
                    recording[recording_index] = (
                        curr_time,
                        interval_read as f64 / elapse as f64 * ONE_SEC as f64,
                    );
                    recording_index += 1;
                    intervel_start = curr_time;
                    curr_size += interval_read;
                    interval_read = 0;
                }
                if offset % file_size == 0 {
                    sys_seek(fd, 0).unwrap();
                    seek_count += 1;
                }
                let (bytes_read, buffer_back) = sys_read(fd, buffer).unwrap();
                buffer = buffer_back;
                interval_read += bytes_read;
            }
            let curr_time = libtime::get_rdtsc();
            let elapse = curr_time - intervel_start;
            curr_size += interval_read;
            recording[recording_index] = (
                curr_time,
                interval_read as f64 / elapse as f64 * ONE_SEC as f64,
            );
            let elapse = libtime::get_rdtsc() - start;

            {
                println!("timestamp(s),throughput(MB/s),");
                let start = recording[0].0;
                for (time_stamp, throughput) in &recording[0..recording_index + 1] {
                    println!(
                        "{},{},",
                        (time_stamp - start) as f64 / ONE_SEC as f64,
                        throughput / 1_000_000.0
                    );
                }
            }

            println!(
                "Read: buffer size: {}, total bytes: {}, cycles: {}, seek count: {}",
                bsize, total_size, elapse, seek_count
            );
            assert_eq!(curr_size, total_size);

            sys_close(fd).unwrap();
        }
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchfs panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
