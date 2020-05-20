#![no_std]
#![feature(
    asm,
    core_intrinsics,
)]
mod nvme_cmd;
mod queue;
mod nvme_regs;
mod array_init;

extern crate alloc;

use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::boxed::Box;
use console::{println, print};
use core::{mem, ptr, fmt};
use libdma::Dma;
use libdma::nvme::{allocate_dma, NvmeCommand, NvmeCompletion};
use platform::PciBarAddr;
use libtime::sys_ns_loopsleep;
use alloc::format;
pub use nvme_regs::{NvmeRegs32, NvmeRegs64};
use nvme_regs::NvmeArrayRegs;
use queue::{NvmeCommandQueue, NvmeCompletionQueue};
pub use libsyscalls::errors::Result;
use rref::{RRef, RRefDeque};
use usr::bdev::BlkReq;

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;
pub (crate) const NUM_LBAS: u64 = 781422768;

pub struct BlockReq {
    pub block: u64,
    num_blocks: u16,
    data: Vec<u8>,
    cid: u16,
}

impl BlockReq {
    pub fn new(block:u64 , num_blocks: u16, data: Vec<u8>) -> BlockReq {
        BlockReq {
            block,
            num_blocks,
            data,
            cid: 0,
        }
    }
    fn from(&mut self) -> Self {
        Self {
            block: self.block,
            num_blocks: self.num_blocks,
            data: unsafe {
                Vec::from_raw_parts(self.data.as_mut_ptr(), self.data.len(), self.data.capacity())
            },
            cid: self.cid,
        }
    }
}

/*impl From<Request> for BlockReq {
    fn from(req: Request) -> BlockReq {
        BlockReq {
            block: req.block,
            num_blocks: req.num_blocks,
            data: Vec::from_raw_parts(req.data as *mut _ as *mut u8,
        }
    }
}*/

impl Clone for BlockReq {
    fn clone(&self) -> Self {
       Self {
            block: self.block,
            num_blocks: self.num_blocks,
            data: self.data.clone(),
            cid: self.cid,
       }
    }
}
struct NvmeNamespace {
    pub id: u32,
    pub blocks: u64,
    pub block_size: u64,
}

pub struct NvmeStats {
    completed: u64,
    submitted: u64,
}

impl NvmeStats {
    pub fn get_stats(&self) -> (u64, u64) {
        (self.submitted, self.completed)
    }
    pub fn reset_stats(&mut self) {
        self.submitted = 0;
        self.completed = 0;
    }
}

impl fmt::Display for NvmeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "submitted {} completed {}", self.submitted, self.completed)
    }
}

use queue::QUEUE_DEPTH;

pub struct NvmeDevice {
    submission_queues: [NvmeCommandQueue; 2],
    completion_queues: [NvmeCompletionQueue; 2],
    bar: PciBarAddr,
    namespaces: Vec<NvmeNamespace>,
    dstrd: u16,
    pub stats: NvmeStats,
    //rrequests: [Option<Vec<u8>>; QUEUE_DEPTH],
}

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

impl NvmeDevice {
    pub fn new(bar: PciBarAddr) -> NvmeDevice {
        NvmeDevice {
            bar,
            submission_queues: [NvmeCommandQueue::new().unwrap(), NvmeCommandQueue::new().unwrap()],
            completion_queues: [NvmeCompletionQueue::new().unwrap(), NvmeCompletionQueue::new().unwrap()],
            namespaces: Vec::new(),
            dstrd: {
                unsafe {
                    ((ptr::read_volatile((bar.get_base() + NvmeRegs64::CAP as u32) as *const u64) >> 32) & 0b1111) as u16
                }
            },
            stats: NvmeStats { submitted: 0, completed: 0 },
        }
    }

    #[inline(always)]
    pub fn read_reg32(&self, reg: NvmeRegs32) -> u32 {
        unsafe {
            ptr::read_volatile((self.bar.get_base() + reg as u32) as *const u32)
        }
    }

    #[inline(always)]
    pub fn read_reg64(&self, reg: NvmeRegs64) -> u64 {
        unsafe {
            ptr::read_volatile((self.bar.get_base() + reg as u32) as *const u64)
        }
    }

    #[inline(always)]
    pub fn write_reg32(&self, reg: NvmeRegs32, val: u32) {
        unsafe {
            ptr::write_volatile((self.bar.get_base() + reg as u32) as *mut u32, val as u32);
        }
    }

    #[inline(always)]
    pub fn write_reg64(&self, reg: NvmeRegs64, val: u64) {
        unsafe {
            ptr::write_volatile((self.bar.get_base() + reg as u32) as *mut u64, val as u64);
        }
    }

    fn read_reg_idx(&self, reg: NvmeArrayRegs, qid: u16) -> u32 {
        match reg {
            NvmeArrayRegs::SQyTDBL => {
                unsafe {
                    ptr::read_volatile((self.bar.get_base() + 0x1000 +
                                         ((4 << self.dstrd) * (2 * qid)) as u32) as *mut u32)
                }
            },

            NvmeArrayRegs::CQyHDBL => {
                unsafe {
                    ptr::read_volatile((self.bar.get_base() + 0x1000 +
                                         ((4 << self.dstrd) * (2 * qid + 1)) as u32) as *mut u32)
                }
            },
        }
    }

    fn write_reg_idx(&self, reg: NvmeArrayRegs, qid: u16, val: u32) {
        match reg {
            NvmeArrayRegs::SQyTDBL => {
                unsafe {
                    ptr::write_volatile((self.bar.get_base() + 0x1000 +
                                         ((4 << self.dstrd) * (2 * qid)) as u32) as *mut u32, val);
                }
            },

            NvmeArrayRegs::CQyHDBL => {
                unsafe {
                    ptr::write_volatile((self.bar.get_base() + 0x1000 +
                                         ((4 << self.dstrd) * (2 * qid + 1)) as u32) as *mut u32, val);
                }
            },
        }
    }

    fn submission_queue_tail(&mut self, qid: u16, tail: u16) {
        self.write_reg_idx(NvmeArrayRegs::SQyTDBL, qid, tail as u32);
    }

    fn completion_queue_head(&mut self, qid: u16, head: u16) {
        self.write_reg_idx(NvmeArrayRegs::CQyHDBL, qid, head as u32);
    }


    pub fn configure_admin_queue(&self) {
        let acq = &self.completion_queues[0];
        let asq = &self.submission_queues[0];

        self.write_reg32(NvmeRegs32::AQA,
                         ((acq.data.len() as u32 - 1) << 16) | (asq.data.len() as u32 - 1));
        self.write_reg64(NvmeRegs64::ASQ, asq.data.physical() as u64);
        self.write_reg64(NvmeRegs64::ACQ, acq.data.physical() as u64);
    }

    pub fn identify_controller(&mut self) {
        let data: Dma<[u8; 4096]> = allocate_dma().unwrap();

        // println!("  - Attempting to identify controller");
        {
            let qid = 0;
            let queue = &mut self.submission_queues[qid];
            let cid = queue.i as u16;
            let entry = nvme_cmd::identify_controller(
                cid, data.physical()
            );
            let tail = queue.submit(entry);
            self.submission_queue_tail(qid as u16, tail as u16);
        }

        // println!("  - Waiting to identify controller");
        {
            let qid = 0;
            let queue = &mut self.completion_queues[qid];
            let (head, entry, _) = queue.complete_spin();
            self.completion_queue_head(qid as u16, head as u16);
        }

        // println!("  - Dumping identify controller");

        let mut serial = String::new();
        for &b in &data[4..24] {
            if b == 0 {
                break;
            }
            serial.push(b as char);
        }

        let mut model = String::new();
        for &b in &data[24..64] {
            if b == 0 {
                break;
            }
            model.push(b as char);
        }

        let mut firmware = String::new();
        for &b in &data[64..72] {
            if b == 0 {
                break;
            }
            firmware.push(b as char);
        }

        println!(
            "  - Model: {} Serial: {} Firmware: {}",
            model.trim(),
            serial.trim(),
            firmware.trim()
        );
    }

    pub fn identify_ns_list(&mut self) {
        let mut nsids = Vec::new();
        {
            //TODO: Use buffer
            let data: Dma<[u32; 1024]> = allocate_dma().unwrap();

            println!("  - Attempting to retrieve namespace ID list");
            {
                let qid = 0;
                let queue = &mut self.submission_queues[qid];
                let cid = queue.i as u16;
                let entry = nvme_cmd::identify_namespace_list(
                    cid, data.physical(), 1
                );
                let tail = queue.submit(entry);
                self.submission_queue_tail(qid as u16, tail as u16);
            }

            println!("  - Waiting to retrieve namespace ID list");
            {
                let qid = 0;
                let queue = &mut self.completion_queues[qid];
                let (head, entry, _) = queue.complete_spin();
                self.completion_queue_head(qid as u16, head as u16);
            }

            println!("  - Dumping namespace ID list");
            for &nsid in data.iter() {
                if nsid != 0 {
                    nsids.push(nsid);
                }
            }
        }
        println!("nsids len {}", nsids.len());
        for nsid in nsids {
            println!("nsid: {:x}", nsid);
        }
    }

    pub fn identify_ns(&mut self, nsid: u32) {
        let data: Dma<[u8; 4096]> = allocate_dma().unwrap();

        println!("  - Attempting to identify namespace {}", nsid);
        {
            let qid = 0;
            let queue = &mut self.submission_queues[qid];
            let cid = queue.i as u16;
            let entry = nvme_cmd::identify_namespace(
                cid, data.physical(), nsid
            );
            let tail = queue.submit(entry);
            self.submission_queue_tail(qid as u16, tail as u16);
        }

        // println!("  - Waiting to identify namespace {}", nsid);
        {
            let qid = 0;
            let queue = &mut self.completion_queues[qid];
            let (head, entry, _) = queue.complete_spin();
            self.completion_queue_head(qid as u16, head as u16);
        }

        // println!("  - Dumping identify namespace");

        unsafe {

            let size = *(data.as_ptr().offset(0) as *const u64);
            let capacity = *(data.as_ptr().offset(8) as *const u64);
            println!(
                "    - ID: {} Size: {} Capacity: {}",
                nsid,
                size * 512,
                capacity * 512,
            );

            //TODO: Read block size
            self.namespaces.push(NvmeNamespace {
                id: nsid,
                blocks: size,
                block_size: 512, // TODO
            });

        }
    }

    pub fn create_io_queues(&mut self) {
        for io_qid in 1..self.completion_queues.len() {
            let (ptr, len) = {
                let queue = &self.completion_queues[io_qid];
                (queue.data.physical(), queue.data.len())
            };

            println!("  - Attempting to create I/O completion queue {} with phys {:x}", io_qid, ptr);
            {
                let qid = 0;
                let queue = &mut self.submission_queues[qid];
                let cid = queue.i as u16;
                let entry = nvme_cmd::create_io_completion_queue(
                    cid, io_qid as u16, ptr, (len - 1) as u16
                );
                let tail = queue.submit(entry);
                self.submission_queue_tail(qid as u16, tail as u16);
            }

            // println!("  - Waiting to create I/O completion queue {}", io_qid);
            {
                let qid = 0;
                let queue = &mut self.completion_queues[qid];
                let (head, entry, _) = queue.complete_spin();
                self.completion_queue_head(qid as u16, head as u16);
            }
        }

        for io_qid in 1..self.submission_queues.len() {
            let (ptr, len) = {
                let queue = &self.submission_queues[io_qid];
                (queue.data.physical(), queue.data.len())
            };

            println!("  - Attempting to create I/O submission queue {} with phys {:x}", io_qid, ptr);
            {
                let qid = 0;
                let queue = &mut self.submission_queues[qid];
                let cid = queue.i as u16;
                //TODO: Get completion queue ID through smarter mechanism
                let entry = nvme_cmd::create_io_submission_queue(
                    cid, io_qid as u16, ptr, (len - 1) as u16, io_qid as u16
                );
                let tail = queue.submit(entry);
                self.submission_queue_tail(qid as u16, tail as u16);
            }

            // println!("  - Waiting to create I/O submission queue {}", io_qid);
            {
                let qid = 0;
                let queue = &mut self.completion_queues[qid];
                let (head, entry, _) = queue.complete_spin();
                self.completion_queue_head(qid as u16, head as u16);
            }
        }
    }

    pub fn submit(&mut self, breq: BlockReq, write: bool) {
        let (ptr0, ptr1) = (breq.data.as_ptr() as u64, 0);
        let qid = 1;
        let queue = &mut self.submission_queues[qid];
        let block = breq.block;
        let num_blocks = (breq.data.len() + 511) / 512;
        let mut entry;

        if write {
            entry = nvme_cmd::io_write(qid as u16,
                    1, // nsid
                    block, // block to read
                    num_blocks as u16,
                    ptr0,
                    ptr1,
                    );
        } else {
            entry = nvme_cmd::io_read(qid as u16,
                    1, // nsid
                    block, // block to read
                    num_blocks as u16,
                    ptr0,
                    ptr1,
                    );
        }

        if let Some(tail) = queue.submit_brequest_cid(entry, breq) {
            self.submission_queue_tail(qid as u16, tail as u16);
        }
        self.stats.submitted += 1;
    }

    #[inline(always)]
    pub fn submit_raw(&mut self, data: u64, cid: u16, write: bool) {
        let (ptr0, ptr1) = (data, 0);
        let qid = 1;
        let queue = &mut self.submission_queues[qid];
        //let block = breq.block;
        //let num_blocks = (breq.data.len() + 511) / 512;
        let num_blocks = 8;
        let mut entry;

        if write {
            entry = nvme_cmd::io_write(cid as u16,
                    1, // nsid
                    0, // block to read
                    (num_blocks - 1) as u16,
                    ptr0,
                    ptr1,
                    );
        } else {
            entry = nvme_cmd::io_read(cid as u16,
                    1, // nsid
                    0, // block to read
                    (num_blocks - 1) as u16,
                    ptr0,
                    ptr1,
                    );
        }

        if let Some(tail) = queue.submit_raw_request_cid(entry) {
            self.submission_queue_tail(qid as u16, tail as u16);
        }
        self.stats.submitted += 1;
    }

    pub fn submit_v(&mut self, vec: Vec<u8>, cid: u16, write: bool) {
        let (ptr0, ptr1) = (vec.as_ptr() as u64, 0);
        let qid = 1;
        let queue = &mut self.submission_queues[qid];
        //let block = breq.block;
        //let num_blocks = (breq.data.len() + 511) / 512;
        let num_blocks = 8;
        let mut entry;

        if write {
            entry = nvme_cmd::io_write(cid as u16,
                    1, // nsid
                    0, // block to read
                    (num_blocks - 1) as u16,
                    ptr0,
                    ptr1,
                    );
        } else {
            entry = nvme_cmd::io_read(cid as u16,
                    1, // nsid
                    0, // block to read
                    (num_blocks - 1) as u16,
                    ptr0,
                    ptr1,
                    );
        }

        if let Some(tail) = queue.submit_rrequest_cid(entry, vec) {
            self.submission_queue_tail(qid as u16, tail as u16);
        }
        self.stats.submitted += 1;
    }

    pub fn poll(&mut self, num_reqs: u64, reap: &mut VecDeque<BlockReq>, reap_all: bool) {
        let qid = 1;
        let mut count = 0;
        let mut cur_head = 0;
        {
            for i in 0..num_reqs {
                let queue = &mut self.completion_queues[qid];
                if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                    //println!("Got head {} cq_idx {}", head, cq_idx);
                    let sq = &mut self.submission_queues[qid];
                    if sq.req_slot[cq_idx] == true {
                        if let Some(req) = &mut sq.brequests[cq_idx] {
                           reap.push_front(req.from());
                        }
                        sq.req_slot[cq_idx] = false;
                        count += 1;
                    }
                    cur_head = head;
                    //TODO: Handle errors
                    self.stats.completed += 1;
                }
            }
            if count > 0 {
                self.completion_queue_head(qid as u16, cur_head as u16);
            }
        }
        //reap
    }

    pub fn submit_io_raw(&mut self, submit_queue: &mut VecDeque<Vec<u8>>, write: bool) -> usize {
        let mut count = 0;
        let mut cur_tail = 0;
        let qid = 1;

        while let Some(mut req) = submit_queue.pop_front() {
            let (ptr0, ptr1) = (req.as_ptr() as u64, 0);
            let queue = &mut self.submission_queues[qid];
            //let block = breq.block;

            //let num_blocks = (breq.data.len() + 511) / 512;
            let num_blocks = 8;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if let Some(tail) = queue.submit_request_raw(entry, req.as_ptr() as u64) {
                cur_tail = tail;
                count += 1;
            }

            self.stats.submitted += 1;
        }

        if count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }
        count
    }


    pub fn submit_iov(&mut self, submit_queue: &mut VecDeque<Vec<u8>>, write: bool) -> usize {
        let mut count = 0;
        let mut cur_tail = 0;
        let qid = 1;

        while let Some(mut req) = submit_queue.pop_front() {
            let (ptr0, ptr1) = (req.as_ptr() as u64, 0);
            let queue = &mut self.submission_queues[qid];
            //let block = breq.block;

            //let num_blocks = (breq.data.len() + 511) / 512;
            let num_blocks = 8;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if let Some(tail) = queue.submit_rrequest(entry, req) {
                cur_tail = tail;
                count += 1;
            }

            self.stats.submitted += 1;
        }

        if count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }
        count
    }

    pub fn poll_rref(&mut self, mut collect: RRefDeque<BlkReq, 1024>) ->
            (usize, RRefDeque<BlkReq, 1024>)
    {
        let qid = 0;
        let mut reap_count = 0;

        for i in 0..collect.len() {
            let queue = &mut self.completion_queues[qid];

            if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                //println!("Got head {} cq_idx {}", head, cq_idx);
                let sq = &mut self.submission_queues[qid];
                if sq.req_slot[cq_idx] == true {
                    if let Some(mut req) = sq.blkreq_rrefs[cq_idx].take() {
                        if collect.push_back(req).is_some() {
                            println!("submit_and_poll2: pushing to full collect queue");
                            cur_head = head;
                            sq.req_slot[cq_idx] = false;
                            break;
                        }
                    }
                    sq.req_slot[cq_idx] = false;
                    reap_count += 1;
                }
                cur_head = head;
                //TODO: Handle errors
                self.stats.completed += 1;
            }
        }
        if reap_count > 0 {
            self.completion_queue_head(qid as u16, cur_head as u16);
        }
        (reap_count, collect)
    }

    pub fn submit_and_poll_rref(&mut self, 
                                mut submit: RRefDeque<BlkReq, 128>,
                                mut collect: RRefDeque<BlkReq, 128>,
                                write: bool) ->
            (usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)
    {

        let mut sub_count = 0;
        let mut reap_count = 0;
        let mut cur_tail = 0;
        let mut cur_head = 0;
        let batch_sz = 32;
        let reap_all = false;
        let qid = 1;

        while let Some(breq) = submit.pop_front() {
            let buf_addr = &(*breq).data as *const [u8; 4096] as *const u64 as u64;
            let (ptr0, ptr1) = (buf_addr, 0);
            let queue = &mut self.submission_queues[qid];

            let num_blocks = 8;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        breq.block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        breq.block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if queue.is_submittable() {
                if let Some(tail) = queue.submit_request_rref(entry, breq) {
                    cur_tail = tail;
                    sub_count += 1;

                    let queue = &mut self.completion_queues[qid];
                    if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                        //println!("Got head {} cq_idx {}", head, cq_idx);
                        let sq = &mut self.submission_queues[qid];
                        if sq.req_slot[cq_idx] == true {
                            if let Some(mut req) = sq.blkreq_rrefs[cq_idx].take() {
                                if collect.push_back(req).is_some() {
                                    println!("submit_and_poll1: pushing to full collect queue");
                                }
                            }
                            sq.req_slot[cq_idx] = false;
                            reap_count += 1;
                        }
                        cur_head = head;
                        //TODO: Handle errors
                        self.stats.completed += 1;
                    }

                    self.stats.submitted += 1;
                }
            } else {
                submit.push_back(breq);
                break;
            }

            //self.stats.submitted += 1;
        }

        if sub_count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }

        {
            for i in 0..batch_sz {
                let queue = &mut self.completion_queues[qid];
                if collect.len() == 128 {
                    break;
                }
                if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                    //println!("Got head {} cq_idx {}", head, cq_idx);
                    let sq = &mut self.submission_queues[qid];
                    if sq.req_slot[cq_idx] == true {
                        if let Some(mut req) = sq.blkreq_rrefs[cq_idx].take() {
                            if collect.push_back(req).is_some() {
                                println!("submit_and_poll2: pushing to full collect queue");
                                cur_head = head;
                                sq.req_slot[cq_idx] = false;
                                break;
                            }
                        }
                        sq.req_slot[cq_idx] = false;
                        reap_count += 1;
                    }
                    cur_head = head;
                    //TODO: Handle errors
                    self.stats.completed += 1;
                }
            }
            if reap_count > 0 {
                self.completion_queue_head(qid as u16, cur_head as u16);
            }
        }

        (sub_count, submit, collect)
    }

    pub fn submit_and_poll_raw(&mut self, submit: &mut VecDeque<Vec<u8>>, collect: &mut VecDeque<Vec<u8>>, write: bool) -> usize {
        let mut sub_count = 0;
        let mut reap_count = 0;
        let mut cur_tail = 0;
        let mut cur_head = 0;
        let batch_sz = 32;
        let reap_all = false;
        let qid = 1;

        while let Some(breq) = submit.pop_front() {
            let (ptr0, ptr1) = (breq.as_ptr() as u64, 0);
            let queue = &mut self.submission_queues[qid];

            let num_blocks = 8;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        0, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if queue.is_submittable() {
                if let Some(tail) = queue.submit_request_raw(entry, breq.as_ptr() as u64) {
                    cur_tail = tail;
                    sub_count += 1;

                    let queue = &mut self.completion_queues[qid];
                    if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                        //println!("Got head {} cq_idx {}", head, cq_idx);
                        let sq = &mut self.submission_queues[qid];
                        if sq.req_slot[cq_idx] == true {
                            if let Some(req) = &mut sq.raw_requests[cq_idx] {
                            let vec = unsafe {
                                Vec::from_raw_parts(*req as *mut u8,
                                        4096,
                                        4096)
                                };
 
                                collect.push_front(vec);
                            }
                            sq.req_slot[cq_idx] = false;
                            reap_count += 1;
                        }
                        cur_head = head;
                        //TODO: Handle errors
                        self.stats.completed += 1;
                    }

                    self.stats.submitted += 1;
                }
            } else {
                submit.push_front(breq);
                break;
            }

        }

        if sub_count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }

        {
            for i in 0..batch_sz {
                //if collect.len() == 32 {
                  //  break;
               // }
                let queue = &mut self.completion_queues[qid];
                if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                    //println!("Got head {} cq_idx {}", head, cq_idx);
                    let sq = &mut self.submission_queues[qid];
                    if sq.req_slot[cq_idx] == true {
                        if let Some(req) = &mut sq.raw_requests[cq_idx] {
                            let vec = unsafe {
                                Vec::from_raw_parts(*req as *mut u8,
                                        4096,
                                        4096)
                                };
 
                            collect.push_front(vec);
                        }
                        sq.req_slot[cq_idx] = false;
                        reap_count += 1;
                    }
                    cur_head = head;
                    //TODO: Handle errors
                    self.stats.completed += 1;
                }
            }
            if reap_count > 0 {
                self.completion_queue_head(qid as u16, cur_head as u16);
            }
        }

        sub_count
    }

    pub fn submit_and_poll(&mut self, submit: &mut VecDeque<BlockReq>, collect: &mut VecDeque<BlockReq>, write: bool) -> usize {
        let mut sub_count = 0;
        let mut reap_count = 0;
        let mut cur_tail = 0;
        let mut cur_head = 0;
        let batch_sz = 128;
        let reap_all = false;
        let qid = 1;

        while let Some(breq) = submit.pop_front() {
            let (ptr0, ptr1) = (breq.data.as_ptr() as u64, 0);
            let queue = &mut self.submission_queues[qid];
            let block = breq.block;

            let num_blocks = ((breq.data.len() + 511) / 512) - 1;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if queue.is_submittable() {
                if let Some(tail) = queue.submit_brequest(entry, breq) {
                    cur_tail = tail;
                    sub_count += 1;

                    let queue = &mut self.completion_queues[qid];
                    if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                        //println!("Got head {} cq_idx {}", head, cq_idx);
                        let sq = &mut self.submission_queues[qid];
                        if sq.req_slot[cq_idx] == true {
                            if let Some(req) = &mut sq.brequests[cq_idx] {
                                collect.push_front(req.from());
                            }
                            sq.req_slot[cq_idx] = false;
                            reap_count += 1;
                        }
                        cur_head = head;
                        //TODO: Handle errors
                        self.stats.completed += 1;
                    }

                }
            } else {
                submit.push_front(breq);
                break;
            }

            self.stats.submitted += 1;
        }

        if sub_count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }

        {
            for i in 0..batch_sz {
                let queue = &mut self.completion_queues[qid];
                if let Some((head, entry, cq_idx)) = if reap_all { Some(queue.complete_spin()) } else { queue.complete() } {
                    //println!("Got head {} cq_idx {}", head, cq_idx);
                    let sq = &mut self.submission_queues[qid];
                    if sq.req_slot[cq_idx] == true {
                        if let Some(req) = &mut sq.brequests[cq_idx] {
                           collect.push_front(req.from());
                        }
                        sq.req_slot[cq_idx] = false;
                        reap_count += 1;
                    }
                    cur_head = head;
                    //TODO: Handle errors
                    self.stats.completed += 1;
                }
            }
            if reap_count > 0 {
                self.completion_queue_head(qid as u16, cur_head as u16);
            }
        }

        sub_count
    }

    pub fn submit_io(&mut self, submit_queue: &mut VecDeque<BlockReq>, write: bool) -> usize {
        let mut count = 0;
        let mut cur_tail = 0;
        let qid = 1;

        while let Some(mut breq) = submit_queue.pop_front() {
            let (ptr0, ptr1) = (breq.data.as_ptr() as u64, 0);
            let queue = &mut self.submission_queues[qid];
            let block = breq.block;

            let num_blocks = (breq.data.len() + 511) / 512;
            let mut entry;

            if write {
                entry = nvme_cmd::io_write(qid as u16,
                        1, // nsid
                        block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );

            } else {
                entry = nvme_cmd::io_read(qid as u16,
                        1, // nsid
                        block, // block to read
                        (num_blocks - 1) as u16,
                        ptr0,
                        ptr1,
                        );
            }

            if let Some(tail) = queue.submit_brequest(entry, breq) {
                cur_tail = tail;
                count += 1;
            }

            self.stats.submitted += 1;
        }

        if count > 0 {
            self.submission_queue_tail(qid as u16, cur_tail as u16);
        }
        count
    }

    fn submit_from_slot(&mut self, cid: u16,
                                block: u64,
                                num_blocks: u64,
                                data_ptr: u64, is_write: bool) {
        let qid = 1;

        let (ptr0, ptr1) = (data_ptr, 0);
        let queue = &mut self.submission_queues[qid];

        if is_write {
            let entry = nvme_cmd::io_write(qid as u16,
                                 1, // nsid
                                 block, // block to read
                                 num_blocks as u16,
                                 ptr0,
                                 ptr1,
                                 );

            if let Some(tail) = queue.submit_from_slot(entry, cid as usize) {
                self.submission_queue_tail(qid as u16, tail as u16);
            }
        } else {
            let entry = nvme_cmd::io_read(qid as u16,
                                1, // nsid
                                block, // block to read
                                num_blocks as u16,
                                ptr0,
                                ptr1,
                                );

            if let Some(tail) = queue.submit_from_slot(entry, cid as usize) {
                self.submission_queue_tail(qid as u16, tail as u16);
            }
        }

        self.stats.submitted += 1;
    }

    pub fn check_io(&mut self, batch_sz: u64, is_write: bool) {
        let mut next_is_valid = false;
        let qid = 1;
        let mut next_phase: bool;
        let mut num_completions = 0;
        let mut cq_head: Option<usize> = None;

        loop {
            let mut cq_valid:bool = false;
            let mut cid: u16 = 0;
            {
                let cq = &mut self.completion_queues[qid];
                if !next_is_valid {
                    match cq.is_valid() {
                        Some(_) => (),
                        None => break,
                    }
                }

                let mut next_cq_head: usize;
                if let Some(entry) = cq.is_valid() {
                    /*unsafe {
                        core::intrinsics::prefetch_read_data(
                            &self.submission_queues[1].requests[entry.cid as usize]
                            as *const _ as *const u64, 2);
                    };*/

                    next_cq_head = (cq.i + 1) % cq.data.len();
                    if next_cq_head == 0 {
                        next_phase = !cq.phase;
                    } else {
                        next_phase = cq.phase;
                    }

                    let next_cpl = unsafe {
                        core::ptr::read_volatile(cq.data.as_ptr().add(next_cq_head))
                    };

                    next_is_valid = ((next_cpl.status & 1) == next_phase as u16);
                    /*if next_is_valid {
                        unsafe {
                            core::intrinsics::prefetch_read_data(
                                &self.submission_queues[1].requests[next_cpl.cid as usize]
                                as *const _ as *const u64, 2);
                        };
                    }*/
                    //println!("cq {}", cq.i);
                    cq.advance();
                    cq_valid = true;
                    cid = entry.cid;
                    self.stats.completed += 1;
                    cq_head = Some(cq.i);
                }
            }

            if cq_valid == true {
                let mut slot_valid : bool = false;
                let mut block = 0u64;
                let mut num_blocks = 0u64;
                let mut data = 0u64;
                {
                    let sq = &mut self.submission_queues[qid];
                    //println!("checking cid {}", cid);
                    if sq.req_slot[cid as usize] == true {
                        if let Some(req) = &mut sq.brequests[cid as usize] {
                            let breq = req.from();
                            sq.req_slot[cid as usize] = false;
                            slot_valid = true;
                            //println!("trying to submit cide {} breq.cid {}", cid,
                            //        breq.cid);
                            self.submit(breq, is_write);
                            //block = req.block + 32;
                            //block = get_rand_block();
                           // num_blocks = req.num_blocks as u64;
                           // data = req.data;
                        }
                    } else {
                        println!("slot {} is not submitted", cid);
                    }
                }
                /*if slot_valid {
                    //self.submit_from_slot(cid, block, num_blocks,
                    //                      data, is_write);
                }*/
                num_completions += 1;
            }
            if num_completions > batch_sz {
                break;
            }
        }

        if num_completions > 0 {
            if let Some(head) = cq_head {
                //println!("writing head {}", head);
                self.completion_queue_head(qid as u16, head as u16);
            }
        }
    }

    pub fn check_iov(&mut self, batch_sz: u64, is_write: bool) -> u64 {
        let mut next_is_valid = false;
        let qid = 1;
        let mut next_phase: bool;
        let mut num_completions = 0;
        let mut cq_head: Option<usize> = None;

        loop {
            let mut cq_valid:bool = false;
            let mut cid: u16 = 0;
            {
                let cq = &mut self.completion_queues[qid];
                if !next_is_valid {
                    match cq.is_valid() {
                        Some(_) => (),
                        None => break,
                    }
                }

                let mut next_cq_head: usize;
                if let Some(entry) = cq.is_valid() {
                    /*unsafe {
                        core::intrinsics::prefetch_read_data(
                            &self.submission_queues[1].requests[entry.cid as usize]
                            as *const _ as *const u64, 2);
                    };*/

                    next_cq_head = (cq.i + 1) % cq.data.len();
                    if next_cq_head == 0 {
                        next_phase = !cq.phase;
                    } else {
                        next_phase = cq.phase;
                    }

                    let next_cpl = unsafe {
                        core::ptr::read_volatile(cq.data.as_ptr().add(next_cq_head))
                    };

                    next_is_valid = ((next_cpl.status & 1) == next_phase as u16);
                    /*if next_is_valid {
                        unsafe {
                            core::intrinsics::prefetch_read_data(
                                &self.submission_queues[1].requests[next_cpl.cid as usize]
                                as *const _ as *const u64, 2);
                        };
                    }*/
                    //println!("cq {}", cq.i);
                    cq.advance();
                    cq_valid = true;
                    cid = entry.cid;
                    self.stats.completed += 1;
                    cq_head = Some(cq.i);
                }
            }

            if cq_valid == true {
                let mut slot_valid : bool = false;
                let mut block = 0u64;
                let mut num_blocks = 0u64;
                let mut data = 0u64;
                {
                    let sq = &mut self.submission_queues[qid];
                    //println!("checking cid {}", cid);
                    if sq.req_slot[cid as usize] == true {
                        if let Some(req) = &mut sq.rrequests[cid as usize] {
                            let vec = unsafe {
                                Vec::from_raw_parts(req.as_mut_ptr(),
                                        req.len(),
                                        req.capacity())
                            };
                            sq.req_slot[cid as usize] = false;
                            slot_valid = true;
                            //println!("trying to submit cide {} breq.cid {}", cid,
                            //        breq.cid);
                            self.submit_v(vec, cid, is_write);
                            //block = req.block + 32;
                            //block = get_rand_block();
                           // num_blocks = req.num_blocks as u64;
                           // data = req.data;
                        }
                    } else {
                        println!("slot {} is not submitted", cid);
                    }
                }
                /*if slot_valid {
                    //self.submit_from_slot(cid, block, num_blocks,
                    //                      data, is_write);
                }*/
                num_completions += 1;
            }
            if num_completions > batch_sz {
                break;
            }
        }

        if num_completions > 0 {
            if let Some(head) = cq_head {
                //println!("writing head {}", head);
                self.completion_queue_head(qid as u16, head as u16);
            }
        }
        num_completions
    }

    pub fn check_io_raw(&mut self, batch_sz: u64, is_write: bool) -> u64 {
        let mut next_is_valid = false;
        let qid = 1;
        let mut next_phase: bool;
        let mut num_completions = 0;
        let mut cq_head: Option<usize> = None;

        loop {
            let mut cq_valid:bool = false;
            let mut cid: u16 = 0;
            let cq = &mut self.completion_queues[qid];

            let entry: Option<NvmeCompletion> = cq.is_valid();
            let mut next_cq_head: usize;

            if !next_is_valid && entry.is_none() {
                break;
            }

            let entry = entry.unwrap();
            let cpl_entry = cq.i;
            let sq_head = entry.sq_head;

            next_cq_head = (cq.i + 1) % cq.data.len();
            if cq.i + 1 != cq.data.len() {
                next_cq_head = cq.i + 1;
                next_phase = cq.phase;
            } else {
                next_cq_head = 0;
                next_phase = !cq.phase;
            }

            let next_cpl = unsafe {
                core::ptr::read_volatile(cq.data.as_ptr().add(next_cq_head))
            };

            next_is_valid = ((next_cpl.status & 1) == next_phase as u16);

            //println!("cq {}", cq.i);
            cq.advance();
            cid = entry.cid;
            self.stats.completed += 1;
            cq_head = Some(cq.i);

            let mut slot_valid : bool = false;
            let mut data = 0u64;
            let sq = &mut self.submission_queues[qid];
            //println!("checking cid {}", cid);
            if sq.req_slot[cid as usize] {
                if let Some(req) = &sq.raw_requests[cid as usize] {
                    sq.req_slot[cid as usize] = false;
                    slot_valid = true;
                    //println!("cpl[{}] finished cid {} sq_head {}", cpl_entry, cid, sq_head);
                    data = *req;
                }
            }

            if slot_valid {
                self.submit_raw(data, cid, is_write);
            }

            num_completions += 1;

            if num_completions == batch_sz {
                break;
            }
        }

        if num_completions > 0 {
            if let Some(head) = cq_head {
                //println!("writing head {}", head);
                self.completion_queue_head(qid as u16, head as u16);
            }
        }
        num_completions
    }
}
