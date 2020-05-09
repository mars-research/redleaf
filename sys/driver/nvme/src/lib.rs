#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message,
    maybe_uninit_extra,
    core_intrinsics,
)]
#![forbid(unsafe_code)]

extern crate malloc;
extern crate alloc;

mod device;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
#[macro_use]
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall, PCI, Heap};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::{println, print};
use pci_driver::DeviceBarRegions;
pub use libsyscalls::errors::Result;
use core::cell::RefCell;
use alloc::sync::Arc;
use spin::Mutex;
use libtime::get_rdtsc as rdtsc;
use crate::device::NvmeDev;
pub use nvme_device::BlockReq;

#[macro_use]
use b2histogram::Base2Histogram;

struct Nvme {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    device_initialized: bool,
    device: RefCell<Option<NvmeDev>>
}

impl Nvme {
    fn new() -> Nvme {
        Nvme {
            vendor_id: 0x8086,
            device_id: 0x0953,
            driver: pci_driver::PciDrivers::NvmeDriver,
            device_initialized: false,
            device: RefCell::new(None)
        }
    }

    fn active(&self) -> bool {
        self.device_initialized
    }
}

impl pci_driver::PciDriver for Nvme {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        match bar_region {
            DeviceBarRegions::Nvme(bar) => {
                println!("got nvme bar region");
                if let Ok(nvme_dev) = NvmeDev::new(bar) {
                    self.device_initialized = true;
                    self.device.replace(Some(nvme_dev));
                }
            }
            _ => { println!("Got unknown bar region") }
        }
    }

    fn get_vid(&self) -> u16 {
        self.vendor_id
    }

    fn get_did(&self) -> u16 {
        self.device_id
    }

    fn get_driver_type(&self) -> pci_driver::PciDrivers {
        self.driver
    }
}

fn perf_test(dev: &Nvme, runtime: u64, batch_sz: u64, is_write: bool) {

    let mut buffer: Vec<u8>;
    if is_write {
        buffer = alloc::vec![0xccu8; 4096];
    } else {
        buffer = alloc::vec![0u8; 4096];
    }

    let block_size = buffer.len();
    let mut breq: BlockReq = BlockReq::new(0, 8, buffer);
    let mut submit: VecDeque<BlockReq> = VecDeque::with_capacity(batch_sz as usize);
    let mut collect: VecDeque<BlockReq> = VecDeque::new();
    let mut batch_size = batch_sz;

    let mut block_num: u64 = 0;

    for i in 0..batch_size {
        let mut breq = breq.clone();
        breq.block = block_num;
        block_num = block_num.wrapping_add(1);
        submit.push_back(breq.clone());
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut NvmeDev = device;

        let mut submit_start = 0;
        let mut submit_elapsed = 0;
        let mut poll_start = 0;
        let mut poll_elapsed = 0;
        let mut count = 0;

        let mut submit_hist = Base2Histogram::new();
        let mut poll_hist = Base2Histogram::new();
        let mut ret = 0;

        let tsc_start = rdtsc();
        let tsc_end = tsc_start + runtime * 2_600_000_000;

        /*loop {
            count += 1;
            submit_start = rdtsc();
            ret = dev.submit_io(&mut submit, is_write);
            submit_elapsed += rdtsc() - submit_start;

            submit_hist.record(ret as u64);

            poll_start = rdtsc();
            dev.poll(16, &mut collect, false);
            poll_elapsed += rdtsc() - poll_start;

            poll_hist.record(collect.len() as u64);

            submit.append(&mut collect);

            if rdtsc() > tsc_end {
                break;
            }
        }*/

        dev.submit_io(&mut submit, is_write);

        loop {

            //println!("checking");
            dev.check_io(batch_sz, is_write);

            let cur_tsc = rdtsc();

            if cur_tsc > tsc_end {
                break;
            }
        }

        let (sub, comp) = dev.get_stats();
        println!("runtime {} submitted {} IOPS completed {} IOPS", runtime, sub as f64 / runtime as f64 / 1 as f64,
                      comp as f64 / runtime as f64 / 1 as f64);
        println!("loop {} submit took {} cycles (avg {} cycles), poll took {} cycles (avg {} cycles)",
                                count, submit_elapsed, submit_elapsed / count, poll_elapsed, poll_elapsed / count);

        for hist in alloc::vec![submit_hist, poll_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }

    }
}

#[no_mangle]
pub fn nvme_init(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) {
    libsyscalls::syscalls::init(s);

    println!("nvme_init: starting nvme driver domain");
    let mut nvme = Nvme::new();
    if let Err(_) = pci.pci_register_driver(&mut nvme, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    perf_test(&nvme, 30, 128, false);
    //perf_test(&nvme, 30, 32, false);
    Box::new(nvme);
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
