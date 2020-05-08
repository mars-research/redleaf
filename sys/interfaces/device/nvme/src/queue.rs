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

pub struct Request {
    pub block: u64,
    pub num_blocks: u16,
    pub data: u64,
}

const QUEUE_DEPTH: usize = 512;

pub (crate) struct NvmeCommandQueue {
    pub data: Dma<[NvmeCommand; QUEUE_DEPTH]>,
    pub i: usize,
    pub requests: [Option<Request>; QUEUE_DEPTH],
    pub brequests: [Option<BlockReq>; QUEUE_DEPTH],
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
            req_slot: [false; QUEUE_DEPTH],
            block: 0,
        };
        Ok(module)
    }

    pub fn submit(&mut self, entry: NvmeCommand) -> usize {
        self.data[self.i] = entry;
        self.data[self.i].cid = self.i as u16;
        self.i = (self.i + 1) % self.data.len();
        self.i
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

            //println!("Submitting block {} at slot {}", self.block, cur_idx);

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

    pub fn submit_brequest(&mut self, entry: NvmeCommand, breq: BlockReq) -> Option<usize> {
        let cur_idx = self.i;
        if self.req_slot[cur_idx] == false {
            self.data[cur_idx] = entry;
            self.data[cur_idx].cid = cur_idx as u16;

            //println!("Submitting block {} at slot {}", self.block, cur_idx);

            self.brequests[cur_idx] = Some(breq);
            self.data[cur_idx].cdw10 = self.block as u32;
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
                unsafe { asm!("pause"); }
            }
        }
    }
}
