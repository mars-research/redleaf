// Based on https://github.com/redox-os/drivers/blob/master/nvmed/src/nvme.rs
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
use libdma::Dma;
use libdma::DmaAllocator;
use libdma::nvme::allocate_dma;
use libdma::nvme::{NvmeCommand, NvmeCompletion};
use crate::{Result, BlockReq};
use crate::array_init;
use console::{println, print};
use interface::rref::{RRef, RRefDeque};
use alloc::vec::Vec;
use interface::bdev::BlkReq;

pub struct Request {
    pub block: u64,
    pub num_blocks: u16,
    pub data: u64,
}

pub const QUEUE_DEPTH: usize = 1024;

use crate::NUM_LBAS;

struct Rand {
    seed: u64,
    pow: u64,
}

impl Rand {
    fn new() -> Rand {
        Rand {
            seed: 123456789,
            pow: 2u64.pow(31),
        }
    }

    #[inline(always)]
    fn get_rand_block(&mut self) -> u64 {
        self.seed = (1103515245 * self.seed + 12345) % self.pow;
        self.seed % NUM_LBAS
    }
}

fn dump_cmd_entry(entry: &NvmeCommand) {
    print!("opc {} fuse {} cid {} nsid {} mptr {:x?} prp1 {:x?} prp2 {} cdw10 \
           {} cdw11 {} cdw12 {} cdw13 {} cdw14 {} cdw15 {}\n",
           entry.opcode, entry.flags, entry.cid, entry.nsid, entry.mptr, entry.dptr[0],
           entry.dptr[1], entry.cdw10, entry.cdw11, entry.cdw12, entry.cdw13, entry.cdw14, entry.cdw15);
}

pub (crate) struct NvmeCommandQueue {
    pub data: Dma<[NvmeCommand; QUEUE_DEPTH]>,
    rand: Rand,
    pub i: usize,
    pub requests: [Option<Request>; QUEUE_DEPTH],
    pub brequests: [Option<BlockReq>; QUEUE_DEPTH],
    pub rrequests: [Option<Vec<u8>>; QUEUE_DEPTH],
    pub raw_requests: [Option<u64>; QUEUE_DEPTH],
    pub blkreq_rrefs: [Option<RRef<BlkReq>>; QUEUE_DEPTH],
    pub req_slot: [bool; QUEUE_DEPTH],
    block: u64,
}

impl NvmeCommandQueue {
    pub fn new() -> Result<NvmeCommandQueue> {
        let module = Self {
            data: allocate_dma()?,
            i: 0,
            requests: array_init::array_init(|_| None),
            brequests: array_init::array_init(|_| None),
            rrequests: array_init::array_init(|_| None),
            raw_requests: array_init::array_init(|_| None),
            blkreq_rrefs: array_init::array_init(|_| None),
            req_slot: [false; QUEUE_DEPTH],
            block: 0,
            rand: Rand::new(),
        };
        Ok(module)
    }

    pub fn submit(&mut self, entry: NvmeCommand) -> usize {
        self.data[self.i] = entry;
        self.data[self.i].cid = self.i as u16;
        self.i = (self.i + 1) % self.data.len();
        self.i
    }

    pub fn is_submittable(&self) -> bool {
        !self.req_slot[self.i]
    }

    pub fn submit_from_slot(&mut self, entry: NvmeCommand, slot: usize) -> Option<usize> {
        let slot = self.i;
        if self.req_slot[slot] == false {
            self.data[slot] = entry;
            //println!("setting cid {} for slot", slot);
            self.data[slot].cid = slot as u16;
            self.data[slot].cdw10 = self.block as u32;
            {
                use crate::NUM_LBAS;
                self.block = (self.block + 1) % NUM_LBAS;
            }

            if let Some(req) = &mut self.requests[slot] {
                req.block = self.data[slot].cdw10 as u64;
                req.data = entry.dptr[0] as u64;
                req.num_blocks = entry.cdw12 as u16;
                println!("Submitting {} at slot {}", req.block, slot);
            }
            self.req_slot[slot] = true;
            self.i = (self.i + 1) % self.data.len();
            Some(self.i)
        } else {
            None
        }
    }

    pub fn submit_request(&mut self, entry: NvmeCommand) -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;

            println!("Submitting block {} at slot {}", self.block, cur_idx);

            self.data[cur_idx].cdw10 = self.block as u32;

            self.requests[cur_idx] = Some(Request{
                block: self.block as u64,
                num_blocks: entry.cdw12 as u16,
                data: entry.dptr[0] as u64,
            });
            {
                use crate::NUM_LBAS;
                self.block = (self.block + 1) % NUM_LBAS;
            }
            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    #[inline(always)]
    pub fn submit_raw_request_cid(&mut self, entry: NvmeCommand) -> Option<usize> {
        let cur_idx = self.i;
        let cid = entry.cid as usize;
        if !self.req_slot[cid] {
            self.data[cur_idx] = entry;

            //self.block = self.rand.get_rand_block();
            self.block = (self.block + 8) % NUM_LBAS;

            self.data[cur_idx].cdw10 = self.block as u32;
            //self.data[cur_idx].cdw11 = (self.block >> 32) as u32;

            //println!("Submitting block[{}] {} at slot {}", cid, self.block, cur_idx);
            //dump_cmd_entry(&self.data[cur_idx]);


            //self.block = 0;

            self.req_slot[cid] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_rrequest_cid(&mut self, entry: NvmeCommand, req: Vec<u8>) -> Option<usize> {
        let cur_idx = self.i;
        let cid = entry.cid as usize;
        if self.req_slot[cid as usize] == false {
            self.data[cur_idx] = entry;

            //println!("Submitting _cid {} block {} at slot {}", breq.cid, self.block, cur_idx);

            self.rrequests[cid] = Some(req);
            self.data[cur_idx].cdw10 = self.block as u32;
            self.block = (self.block + 1) % NUM_LBAS;

            self.req_slot[cid] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_brequest_cid(&mut self, entry: NvmeCommand, mut breq: BlockReq) -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[breq.cid as usize] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = breq.cid;

            //println!("Submitting _cid {} block {} at slot {}", breq.cid, self.block, cur_idx);

            let breq_cid = breq.cid as usize;
            self.brequests[breq_cid] = Some(breq);
            self.data[cur_idx].cdw10 = self.block as u32;
            {
                use crate::NUM_LBAS;
                self.block = (self.block + 1) % NUM_LBAS;
            }

            self.req_slot[breq_cid] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_request_rref(&mut self, entry: NvmeCommand, breq: RRef<BlkReq>)
                                        -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;
            self.data[cur_idx].cdw10 %= NUM_LBAS as u32;
            //self.block = (self.block + 8) % NUM_LBAS;
            let cid = cur_idx  as u16;

            //println!("Submitting block[{}] {} at slot {}", cid, breq.block, cur_idx);

            self.blkreq_rrefs[cur_idx] = Some(breq);

            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_request_rand_raw(&mut self, entry: NvmeCommand, data: u64)
                                        -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;
            let cid = cur_idx  as u16;

            self.raw_requests[cur_idx] = Some(data);
            self.data[cur_idx].cdw10 = self.block as u32;

            self.block = self.rand.get_rand_block();

            //println!("Submitting block[{}] {} at slot {}", cid, self.block, cur_idx);

            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_request_raw(&mut self, entry: NvmeCommand, data: u64)
                                        -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;
            let cid = cur_idx  as u16;

            self.raw_requests[cur_idx] = Some(data);
            self.data[cur_idx].cdw10 = self.block as u32;

            self.block = (self.block + 8) % NUM_LBAS;

            //println!("Submitting block[{}] {} at slot {}", cid, self.block, cur_idx);

            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_rrequest(&mut self, entry: NvmeCommand, v: Vec<u8>)
                                        -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            //entry.cid = cur_idx as u16;
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;
            //breq.cid = cur_idx as u16;

            //println!("Submitting block {} at slot {} cid {}", self.block, cur_idx, self.data[cur_idx].cid);

            self.rrequests[cur_idx] = Some(v);
            self.data[cur_idx].cdw10 = self.block as u32;
            self.block = (self.block + 1) % NUM_LBAS;

            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }

    pub fn submit_brequest(&mut self, mut entry: NvmeCommand, mut breq: BlockReq) -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            entry.cid = cur_idx as u16;
            self.data[cur_idx] = entry;
            breq.cid = cur_idx as u16;

            //println!("Submitting block {} at slot {} cid {}", entry.cdw10, cur_idx, self.data[cur_idx].cid);

            self.brequests[cur_idx] = Some(breq);
            //self.data[cur_idx].cdw10 = self.block as u32;
            //{
            //    use crate::NUM_LBAS;
            //    self.block = (self.block + 1) % NUM_LBAS;
            //}

            self.req_slot[cur_idx] = true;
            self.i = (cur_idx + 1) % self.data.len();
            Some(self.i)
        } else {
            //println!("No free slot");
            None
        }
    }
}

pub (crate) struct NvmeCompletionQueue {
    pub data: Dma<[NvmeCompletion; QUEUE_DEPTH]>,
    pub i: usize,
    pub phase: bool,
}

impl NvmeCompletionQueue {
    pub fn new() -> Result<Self> {
        Ok(Self {
            data: allocate_dma()?,
            i: 0,
            phase: true,
        })
    }

    pub fn get_cq_head(&self) -> usize {
        self.i
    }

    pub (crate) fn complete(&mut self) -> Option<(usize, NvmeCompletion, usize)> {
        let entry = unsafe {
            core::ptr::read_volatile(self.data.as_ptr().add(self.i))
        };
        let mut cq_entry: usize = 0;
        if ((entry.status & 1) == 1) == self.phase {
            cq_entry = self.i;
            self.i = (self.i + 1) % self.data.len();
            if self.i == 0 {
                self.phase = ! self.phase;
            }
            //println!("=> {:?}", entry);
            Some((self.i, entry, cq_entry))
        } else {
            None
        }
    }

    pub fn is_valid(&self) -> Option<NvmeCompletion> {
        let entry = unsafe {
            core::ptr::read_volatile(self.data.as_ptr().add(self.i))
        };

        if ((entry.status & 1) == 1) == self.phase {
            //println!("idx {} status {} phase {}", self.i, entry.status, self.phase);
            Some(entry)
        } else {
            //println!("None: idx {} status {} phase {}", self.i, entry.status, self.phase);
            None
        }
    }

    pub fn advance(&mut self) {
        self.i = (self.i + 1) % self.data.len();
        if self.i == 0 {
            //println!("switching phase from {} to {}", self.phase, !self.phase);
            self.phase = ! self.phase;
        }
    }

    pub fn complete_spin(&mut self) -> (usize, NvmeCompletion, usize) {
        loop {
            if let Some(some) = self.complete() {
                return some;
            } else {
                unsafe { llvm_asm!("pause"); }
            }
        }
    }
}
