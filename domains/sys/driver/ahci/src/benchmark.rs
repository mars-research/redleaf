extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use byteorder::{ByteOrder, LittleEndian};
// use embedded_time::{clock, duration, Timer};

use core::panic::PanicInfo;
use interface::bdev::{BDev, BSIZE};
use interface::rpc::RpcResult;
use spin::Mutex;

use console::println;
use libsyscalls::errors::Result;
use libsyscalls::syscalls::{sys_backtrace, sys_yield};
use libtime::get_rdtsc;
use pci_driver::{BarRegions, DeviceBarRegions, PciClass};
use rref::RRef;
use spin::Once;
use syscalls::Syscall;

use ahci_device::disk;
use ahci_regs::AhciBarRegion;

pub fn benchmark_sync_ahci(
    bdev: &Box<dyn BDev + Send + Sync>,
    blocks_to_read: u32,
    blocks_per_patch: u32,
) {
    assert!(blocks_to_read % blocks_per_patch == 0);
    assert!(blocks_per_patch <= 0xFFFF);

    let start = libtime::get_rdtsc();

    let mut data = RRef::<[u8; 4096]>::new([0; 4096]);
    for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
        data = bdev.read(i, data).unwrap();
    }

    let end = libtime::get_rdtsc();
    println!(
        "AHCI benchmark: reading {} blocks, {} blocks at a time, takes {} cycles",
        blocks_to_read,
        blocks_per_patch,
        end - start
    );
}

// pub fn timed_sync_ahci(bdev: &Box<dyn BDev + Send + Sync>, time_in_sec: u32) {
//     let duration = duration::Seconds::new(time_in_sec);

//     let timer = Clock::new_timer(&self, duration);
//     let mut block_count: u32 = 0;
//     let mut data = RRef::<[u8; 4096]>::new([0; 4096]);

//     loop {}

//     // loop {
//     //     match now.elapsed() {
//     //         OK(elapsed) => {
//     //             // TODO: use more accurate measure than sec
//     //             if elapsed.as_secs() >= time {
//     //                 break;
//     //             }
//     //         }
//     //         Err(e) => {
//     //             println!("AHCI Benchmark Error: {}", e);
//     //         }
//     //     }
//     //     data = bdev.read(block_count, data).unwrap();
//     //     block_count += 1;
//     // }

//     // println!("");
// }
