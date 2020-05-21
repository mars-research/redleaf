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
#![feature(const_int_pow)]
//#![forbid(unsafe_code)]

extern crate malloc;
extern crate alloc;

mod device;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
#[macro_use]
use alloc::vec::Vec;
use core::panic::PanicInfo;
use usr::pci::PCI;
use syscalls::{Syscall, Heap};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::{println, print};
use pci_driver::DeviceBarRegions;
pub use libsyscalls::errors::Result;
use core::cell::RefCell;
use alloc::sync::Arc;
use spin::Mutex;
use libtime::get_rdtsc as rdtsc;
use libtime::sys_ns_loopsleep;
use crate::device::NvmeDev;
pub use nvme_device::BlockReq;
use rref::{RRef, RRefDeque};
use usr::bdev::BlkReq;

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

/*impl usr::bdev::BDev for Nvme {
    fn submit_and_poll_rref(
        &self,
        mut submit: RRefDeque<BlkReq, 32>,
        mut collect: RRefDeque<BlkReq, 32>,
        write: bool,
        ) -> (
            usize,
            RRefDeque<BlkReq, 32>,
            RRefDeque<BlkReq, 32>,
        )
    {

   let mut submit = Some(submit);
        let mut collect = Some(collect);


        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut NvmeDev = device;
            let (num, mut submit_, mut collect_) = dev.device.submit_and_poll_rref(submit.take().unwrap(),
                                                    collect.take().unwrap(), write);
            ret = num;

            submit.replace(submit_);
            collect.replace(collect_);
        }

        (ret, submit.unwrap(), collect.unwrap())
    }
}*/

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

fn perf_test_raw(dev: &Nvme, runtime: u64, batch_sz: u64, is_write: bool) {

    let mut buffer: Vec<u8>;
    if is_write {
        buffer = alloc::vec![0xbau8; 4096];
    } else {
        buffer = alloc::vec![0u8; 4096];
    }

    let block_size = buffer.len();
    let mut breq: BlockReq = BlockReq::new(0, 8, buffer);
    let mut req: Vec<u8> = alloc::vec![0xeeu8; 4096];
    let mut submit: VecDeque<BlockReq> = VecDeque::with_capacity(batch_sz as usize);
    let mut submit_vec: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz as usize);
    let mut collect: VecDeque<BlockReq> = VecDeque::new();

    let mut block_num: u64 = 0;

    for i in 0..batch_sz {
        let mut breq = breq.clone();
        breq.block = block_num;
        block_num = block_num.wrapping_add(1);
        submit.push_back(breq.clone());
        submit_vec.push_back(req.clone());
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
        let tsc_end = tsc_start + runtime * 2_400_000_000;

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


        dev.submit_io_raw(&mut submit_vec, is_write);

        loop {

            count += 1;
            //println!("checking");
            let ret = dev.check_io_raw(128, is_write);

            //poll_start = rdtsc();
            //poll_hist.record(ret);
            //poll_elapsed += rdtsc() - poll_start;

            let cur_tsc = rdtsc();

            if cur_tsc > tsc_end {
                break;
            }
            //sys_ns_loopsleep(200);
        }

        let (sub, comp) = dev.get_stats();
        println!("runtime {} submitted {:.2} K IOPS completed {:.2} K IOPS", runtime, sub as f64 / runtime as f64 / 1_000 as f64,
                      comp as f64 / runtime as f64 / 1_000 as f64);
        println!("loop {} poll took {} cycles (avg {} cycles)", count, poll_elapsed, poll_elapsed / count);

        for hist in alloc::vec![poll_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }

    }
}

fn perf_test_iov(dev: &Nvme, runtime: u64, batch_sz: u64, is_write: bool) {

    let mut buffer: Vec<u8>;
    if is_write {
        buffer = alloc::vec![0xccu8; 4096];
    } else {
        buffer = alloc::vec![0u8; 4096];
    }

    let block_size = buffer.len();
    let mut breq: BlockReq = BlockReq::new(0, 8, buffer);
    let mut req: Vec<u8> = alloc::vec![0u8; 4096];
    let mut submit: VecDeque<BlockReq> = VecDeque::with_capacity(batch_sz as usize);
    let mut submit_vec: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz as usize);
    let mut collect: VecDeque<BlockReq> = VecDeque::new();
    let mut batch_size = batch_sz;

    let mut block_num: u64 = 0;

    for i in 0..batch_size {
        let mut breq = breq.clone();
        breq.block = block_num;
        block_num = block_num.wrapping_add(1);
        submit.push_back(breq.clone());
        submit_vec.push_back(req.clone());
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
        let tsc_end = tsc_start + runtime * 2_400_000_000;

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

        dev.submit_iov(&mut submit_vec, is_write);

        loop {
            count += 1;

            //println!("checking");
            let ret = dev.check_iov(batch_sz, is_write);

            poll_hist.record(ret);

            let cur_tsc = rdtsc();

            if cur_tsc > tsc_end {
                break;
            }
        }

        let (sub, comp) = dev.get_stats();
        println!("runtime {} submitted {} IOPS completed {} IOPS", runtime, sub as f64 / runtime as f64 / 1_000 as f64,
                      comp as f64 / runtime as f64 / 1_000 as f64);
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

fn run_blocktest_rref(dev: &Nvme, block_sz: usize, is_write: bool, is_random: bool)
{
    let mut block_num: u64 = 0;
    let batch_sz = 32;
    let runtime = 30;

    let mut data = [0u8; 4096];
    if is_write {
        data = [0xeeu8; 4096];
    }

    let mut submit = RRefDeque::<BlkReq, 128>::default();
    let mut collect = RRefDeque::<BlkReq, 128>::default();
    let mut poll = RRefDeque::<BlkReq, 1024>::default();

    for i in 0..32 {
        let mut breq = BlkReq::from_data(data.clone());
        if is_random {
            breq.block = get_rand_block();
        } else {
            breq.block = block_num;
            block_num = block_num.wrapping_add(8);
        }
        submit.push_back(RRef::<BlkReq>::new(breq));
    }

    println!("======== Starting {}{} test (rrefs)  ==========",
                                    if is_random { "rand" } else { "" },
                                    if is_write { "write" } else { "read" });

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut NvmeDev = device;

        let mut submit_start = 0;
        let mut submit_elapsed = 0;

        let mut alloc_count = 0;
        let mut alloc_elapsed = 0;

        let mut count: u64 = 0;

        let mut submit = Some(submit);
        let mut collect = Some(collect);
        let mut poll = Some(poll);

        let mut submit_hist = Base2Histogram::new();
        let mut poll_hist = Base2Histogram::new();
        let mut ret = 0;
        let mut last_cq = 0;
        let mut last_sq = 0;
        let mut sq_id = 0;

        let tsc_start = rdtsc();
        let tsc_end = tsc_start + runtime * 2_400_000_000;

        loop {
            count += 1;
            submit_start = rdtsc();
            let (ret, _last_sq, _last_cq, _sq_id, mut submit_, mut collect_) = dev.submit_and_poll_rref(submit.take().unwrap(),
                                                 collect.take().unwrap(), is_write);
            submit_elapsed += rdtsc() - submit_start;

            if ret > 0 {
                last_sq = _last_sq;
            }

            if collect_.len() > 0 {
                last_cq = _last_cq;
                sq_id = _sq_id;
            }

            //println!("submitted {} reqs, collect {} reqs sq: {} cq {}", ret, collect_.len(), last_sq, last_cq);
            submit_hist.record(ret as u64);

            poll_hist.record(collect_.len() as u64);

            while let Some(mut breq) = collect_.pop_front() {
                if is_random {
                    breq.block = get_rand_block();
                } else {
                    breq.block = block_num;
                    block_num = block_num.wrapping_add(8);
                }
                if submit_.push_back(breq).is_some() {
                    println!("submit too full already!");
                    break;
                }
            }

            if submit_.len() == 0  {//&& (alloc_count * batch_sz) < 1024 {
                println!("Alloc new batch at {}", count);
                alloc_count += 1;
                let alloc_rdstc_start = rdtsc();
                for i in 0..batch_sz {
                    let mut breq = BlkReq::from_data(data.clone());
                    if is_random {
                        breq.block = get_rand_block();
                    } else {
                        breq.block = block_num;
                        block_num = block_num.wrapping_add(8);
                    }
                    submit_.push_back(RRef::<BlkReq>::new(breq));
                }
                alloc_elapsed += rdtsc() - alloc_rdstc_start;
            }


            submit.replace(submit_);
            collect.replace(collect_);

            if rdtsc() > tsc_end {
                break;
            }
            //sys_ns_loopsleep(2000);
        }

        let elapsed = rdtsc() - tsc_start;

        let adj_runtime = elapsed as f64 / 2_400_000_000_u64 as f64;

        let (sub, comp) = dev.get_stats();

        println!("Polling .... last_sq {} last_cq {} sq_id {}", last_sq, last_cq, sq_id);

        let (done, poll_) = dev.poll_rref(poll.take().unwrap());

        println!("Poll: Reaped {} requests", done);

        if let Some(mut submit) = submit.take() {
            println!("submit {} requests", submit.len());
        }

        if let Some(mut collect) = collect.take() {
            println!("collect {} requests", collect.len());
        }
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("submitted {:.2} K IOPS completed {:.2} K IOPS",
                             sub as f64 / adj_runtime as f64 / 1_000 as f64,
                             comp as f64 / adj_runtime as f64 / 1_000 as f64);
        println!("submit_and_poll_rref took {} cycles (avg {} cycles)",
                                            submit_elapsed, submit_elapsed / count);

        println!("Number of new allocations {}", alloc_count * batch_sz);


        for hist in alloc::vec![submit_hist, poll_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }
        println!("++++++++++++++++++++++++++++++++++++++++++++++++++++");
    }
}

fn run_blocktest_raw(dev: &Nvme, runtime: u64, batch_sz: u64, is_write: bool) {

    let mut req: Vec<u8>;
    if is_write {
        req = alloc::vec![0xbau8; 4096];
    } else {
        req = alloc::vec![0u8; 4096];
    }

    let mut submit: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz as usize);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    let mut block_num: u64 = 0;

    for i in 0..batch_sz {
        submit.push_back(req.clone());
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut NvmeDev = device;

        let mut submit_start = 0;
        let mut submit_elapsed = 0;
        let mut poll_start = 0;
        let mut poll_elapsed = 0;
        let mut count = 0;
        let mut allocated_count = 0;

        let mut submit_hist = Base2Histogram::new();
        let mut poll_hist = Base2Histogram::new();
        let mut ret = 0;

        let tsc_start = rdtsc();
        let tsc_end = tsc_start + runtime * 2_400_000_000;

        loop {
            count += 1;
            submit_start = rdtsc();
            submit_hist.record(submit.len() as u64);
            ret = dev.submit_and_poll_raw(&mut submit, &mut collect, is_write);
            submit_elapsed += rdtsc() - submit_start;

            //submit_hist.record(ret as u64);

            poll_hist.record(collect.len() as u64);

            submit.append(&mut collect);

            if submit.len() == 0 {
                allocated_count += 1;
                println!("allocating new batch at count {}", count);
                for i in 0..batch_sz {
                    submit.push_back(req.clone());
                }
            }

            if rdtsc() > tsc_end {
                break;
            }
            sys_ns_loopsleep(2000);
        }

        let (sub, comp) = dev.get_stats();
        println!("runtime {} submitted {:.2} K IOPS completed {:.2} K IOPS", runtime, sub as f64 / runtime as f64 / 1_000 as f64,
                      comp as f64 / runtime as f64 / 1_000 as f64);
        println!("run_blocktest loop {} submit_and_poll took {} cycles (avg {} cycles)", count,
                                            submit_elapsed, submit_elapsed / count);
        println!("Allocated breqs {}", allocated_count * batch_sz);

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

fn run_blocktest(dev: &Nvme, runtime: u64, batch_sz: u64, is_write: bool) {

    let mut buffer: Vec<u8>;
    if is_write {
        buffer = alloc::vec![0xbau8; 4096];
    } else {
        buffer = alloc::vec![0u8; 4096];
    }

    let block_size = buffer.len();
    let mut breq: BlockReq = BlockReq::new(0, 8, buffer);
    let mut submit: VecDeque<BlockReq> = VecDeque::with_capacity(batch_sz as usize);
    let mut collect: VecDeque<BlockReq> = VecDeque::new();

    let mut block_num: u64 = 0;

    for i in 0..batch_sz {
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
        let tsc_end = tsc_start + runtime * 2_400_000_000;

        loop {
            count += 1;
            submit_start = rdtsc();
            ret = dev.submit_and_poll(&mut submit, &mut collect, is_write);
            submit_elapsed += rdtsc() - submit_start;

            submit_hist.record(ret as u64);

            poll_hist.record(collect.len() as u64);

            submit.append(&mut collect);

            if submit.len() == 0 {
                //println!("allocating new batch");
                for i in 0..batch_sz {
                    let mut breq = breq.clone();
                    breq.block = block_num;
                    block_num = block_num.wrapping_add(1);
                    submit.push_back(breq.clone());
                }
            }

            for b in submit.iter_mut() {
                b.block = block_num;
                block_num = block_num.wrapping_add(1);
            }

            if rdtsc() > tsc_end {
                break;
            }
            sys_ns_loopsleep(2000);
        }

        let (sub, comp) = dev.get_stats();
        println!("runtime {} submitted {:.2} K IOPS completed {:.2} K IOPS", runtime, sub as f64 / runtime as f64 / 1_000 as f64,
                      comp as f64 / runtime as f64 / 1_000 as f64);
        println!("run_blocktest loop {} submit_and_poll took {} cycles (avg {} cycles)", count,
                                            submit_elapsed, submit_elapsed / count);

        for hist in alloc::vec![poll_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }

    }
}

static mut seed: u64 = 123456789;
static pow: u64 = 2u64.pow(31);

fn get_rand_block() -> u64 {
    unsafe {
        seed = (1103515245 * seed + 12345) % pow;
        seed % 781422768
    }
}

fn rand_test(num_iter: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..num_iter {
        let rand = get_rand_block();
        //println!("rand {}", rand);
        sum += rand;
    }
    sum
}


#[no_mangle]
pub fn nvme_init(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn usr::pci::PCI>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("nvme_init: starting nvme driver domain");
    let mut nvme = Nvme::new();
    if let Err(_) = pci.pci_register_driver(&mut nvme, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    println!("starting tests!...");
    /*let num_iter = 10_000_000;
    let rand_start = rdtsc();
    let sum = rand_test(num_iter);
    let rand_elapsed = rdtsc() - rand_start;
    println!("Rand {} test {} iterations took {} cycles (avg {} cycles)", sum, num_iter, rand_elapsed, rand_elapsed as f64 / num_iter as f64);
    */
    //perf_test_raw(&nvme, 60, 8, false);
   // for _ in 1..1024 {
    //    perf_test_iov(&nvme, 30, 8, false);
    //}


    //run_blocktest_raw(&nvme, 30, 32, true);

    run_blocktest_rref(&nvme, 4096, false, false);
    run_blocktest_rref(&nvme, 4096, false, false);

    run_blocktest_rref(&nvme, 4096, false, true);
    run_blocktest_rref(&nvme, 4096, false, true);


    run_blocktest_rref(&nvme, 4096, true, false);
    run_blocktest_rref(&nvme, 4096, true, false);

    run_blocktest_rref(&nvme, 4096, true, true);
    run_blocktest_rref(&nvme, 4096, true, true);

    panic!("nvme dies");
    run_blocktest_rref(&nvme, 4096, false, false);
    run_blocktest_rref(&nvme, 4096, true, false);
    run_blocktest_rref(&nvme, 4096, true, false);
    run_blocktest_rref(&nvme, 4096, true, false);
    run_blocktest_rref(&nvme, 4096, true, false);


    run_blocktest_raw(&nvme, 10, 32, true);
    run_blocktest_raw(&nvme, 10, 32, true);
    run_blocktest_raw(&nvme, 10, 32, true);
    run_blocktest_raw(&nvme, 10, 32, true);
    run_blocktest_raw(&nvme, 10, 32, true);


    run_blocktest_raw(&nvme, 10, 32, false);
    run_blocktest_raw(&nvme, 10, 32, false);
    run_blocktest_raw(&nvme, 10, 32, false);
    run_blocktest_raw(&nvme, 10, 32, false);
    run_blocktest_raw(&nvme, 10, 32, false);

/*
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);


    perf_test_raw(&nvme, 10, 32, true);
    perf_test_raw(&nvme, 10, 32, true);
    perf_test_raw(&nvme, 10, 32, true);
    perf_test_raw(&nvme, 10, 32, true);
    perf_test_raw(&nvme, 10, 32, true);
*/
    /*perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    perf_test_raw(&nvme, 10, 32, false);
    */
    //perf_test_iov(&nvme, 30, 8, true);
    //perf_test(&nvme, 30, 32, false);
    Box::new(nvme);
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
