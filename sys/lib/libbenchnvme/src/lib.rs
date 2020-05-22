#![no_std]
#![feature(const_int_pow)]

extern crate alloc;
extern crate core;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use b2histogram::Base2Histogram;
use console::{print, println};
use libtime::get_rdtsc as rdtsc;
use rref::{RRef, RRefDeque};
use usr::bdev::{BlkReq, NvmeBDev};

static mut seed: u64 = 123456789;
static pow: u64 = 2u64.pow(31);

pub fn get_rand_block() -> u64 {
    unsafe {
        seed = (1103515245 * seed + 12345) % pow;
        seed % 781422768
    }
}

pub fn rand_test(num_iter: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..num_iter {
        let rand = get_rand_block();
        //println!("rand {}", rand);
        sum += rand;
    }
    sum
}

pub fn run_blocktest_rref(dev: &mut dyn NvmeBDev, block_sz: usize, is_write: bool, is_random: bool)
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
        let (ret, mut submit_, mut collect_) = dev.submit_and_poll_rref(submit.take().unwrap(),
                                                collect.take().unwrap(), is_write);
        submit_elapsed += rdtsc() - submit_start;

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
            //println!("Alloc new batch at {}", count);
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

    // println!("Polling .... last_sq {} last_cq {} sq_id {}", last_sq, last_cq, sq_id);
    println!("Polling ....");

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

