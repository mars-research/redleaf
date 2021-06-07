// Intel SATA AHCI v1.3.1
// Section 9.3.1.1.5.1
// Non-queued commands may use any command slot. The HBA guarantees that command issue order is
// preserved, so software does not need to ensure any ordering of command slots.
// To issue a non-queued command, the host should:
// 1. Select an unused command slot
// 2. Build the command table and command header
// 3. Set PxFBS.DEV to the value of the Port Multiplier port field in the command header
// 4. Set the bit in PxCI that corresponds to the command slot being used

// One Port could have several devices attached to it if there's a multiplexer.
// For now we can assume that there's only one device on a port.

// Each port has a command list containing serveral(32) command header slots.
//  This command list can be used by system software and the HBA even when non-queued commands
//  need to be transferred. System software can still place multiple commands in the list, whether DMA, PIO,
//  or ATAPI, and the HBA will walk the list transferring them.

// Each command header points to a variable sized(up to 65,535) Physical Region Descriptor Table(PRDT)

// Each entry(item) in PRDT contains a pointer(physical address) to the buffer(up to 4MB) that the device can DMA to

extern crate alloc;

use alloc::boxed::Box;

// use console::println;
use libsyscalls::errors::Result;
use libsyscalls::errors::{Error, EBUSY, EINVAL};

use super::disk::Disk;
use super::hba::HbaPort;
use libdma::ahci::allocate_dma;
use libdma::ahci::{HbaCmdHeader, HbaCmdTable};
use libdma::Dma;

use interface::bdev::BlkReq;
use rref::{RRef, RRefDeque};

// Maximun number of sectors per PRDT entry
pub const MAX_SECTORS_PER_PRDT_ENTRY: usize = 8192;
// The size of a sector(some call it block) of the disk in bytes
pub const SECTOR_SIZE: usize = 512;
// Maximun number of bytes per PRDT entry
pub const MAX_BYTES_PER_PRDT_ENTRY: usize = MAX_SECTORS_PER_PRDT_ENTRY * SECTOR_SIZE;
// Maximun number of PRDT entries in a PRDTable
pub const MAX_PRDT_ENTRIES: usize = 65_535;

struct Request {
    address: usize,
    start_sector: u64,
    total_sectors: u64,
    buffer: Box<[u8]>,
    start_time: u64,
}

pub struct AhciStats {
    completed: u64,
    submitted: u64,
}

impl AhciStats {
    pub fn get_stats(&self) -> (u64, u64) {
        (self.submitted, self.completed)
    }
    pub fn reset_stats(&mut self) {
        self.submitted = 0;
        self.completed = 0;
    }
}

pub struct DiskATA {
    id: usize,
    port: HbaPort,
    size: u64,
    // requests_opt: [Option<BlkReq>; 32],
    requests_opt: [Option<Request>; 32],
    blkreqs_opt: [Option<RRef<BlkReq>>; 32],
    // request_opt: Option<Request>,
    clb: Dma<[HbaCmdHeader; 32]>,
    ctbas: [Dma<HbaCmdTable>; 32],
    _fb: Dma<[u8; 256]>,
    pub stats: AhciStats,
}

impl DiskATA {
    pub fn new(id: usize, mut port: HbaPort) -> Result<Self> {
        let mut clb = allocate_dma()?;
        let mut ctbas = [
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
            allocate_dma()?,
        ];
        let mut fb = allocate_dma()?;

        port.init(&mut clb, &mut ctbas, &mut fb);

        let size = port.identify(&mut clb, &mut ctbas).unwrap_or(0);

        Ok(DiskATA {
            id,
            port,
            size,
            // request_opt: None,
            requests_opt: array_init::array_init(|_| None),
            blkreqs_opt: array_init::array_init(|_| None),
            clb,
            ctbas,
            _fb: fb,
            stats: AhciStats {
                submitted: 0,
                completed: 0,
            },
        })
    }
}

impl Disk for DiskATA {
    fn id(&self) -> usize {
        self.id
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn read(&mut self, block: u64, buffer: &mut [u8]) {
        // Synchronous read
        let slot = self
            .submit(block, false, unsafe { Box::from_raw(buffer as *mut [u8]) })
            .unwrap();
        while let None = self.poll(slot).unwrap() {
            // Spin
        }
    }

    fn write(&mut self, block: u64, buffer: &[u8]) {
        // Synchronous read
        let slot = self
            .submit(block, true, unsafe {
                Box::from_raw(buffer as *const [u8] as *mut [u8])
            })
            .unwrap();
        while let None = self.poll(slot).unwrap() {
            // Spin
        }
    }

    fn block_length(&mut self) -> Result<u32> {
        Ok(512)
    }

    fn submit(&mut self, block: u64, write: bool, buffer: Box<[u8]>) -> Result<u32> {
        assert!(
            buffer.len() % 512 == 0,
            "Must read a multiple of block size number of bytes"
        );

        let address = &*buffer as *const [u8] as *const () as usize;
        let total_sectors = buffer.len() as u64 / 512;

        if let Some(slot) = self.port.ata_dma(
            block,
            total_sectors as u16,
            write,
            &mut self.clb,
            &mut self.ctbas,
            &*buffer,
        ) {
            // Submitted, create the corresponding Request in self.requests_opt
            self.port.set_slot_ready(slot, false);
            self.requests_opt[slot as usize] = Some(Request {
                address,
                start_sector: block,
                total_sectors,
                buffer,
                start_time: libtime::get_rdtsc(),
            });
            self.stats.submitted += 1;
            Ok(slot)
        } else {
            // Error
            Err(Error::new(EBUSY))
        }
    }

    fn poll(&mut self, slot: u32) -> Result<Option<Box<[u8]>>> {
        if let None = self.requests_opt[slot as usize] {
            return Err(Error::new(EINVAL));
        }

        if self.port.ata_running(slot) {
            // Still running
            Ok(None)
        } else {
            // Finished (errored or otherwise)
            let req = self.requests_opt[slot as usize].take().unwrap();
            self.port.set_slot_ready(slot, true);
            self.port.ata_stop(slot)?;
            self.stats.completed += 1;
            // println!("Request to {}-{} sectors takes {} cycles", req.start_sector, req.start_sector + req.total_sectors, libtime::get_rdtsc() - req.start_time);
            Ok(Some(req.buffer))
        }
    }

    // fn submit_and_poll_rref(
    //     &mut self,
    //     mut submit: RRefDeque<BlkReq, 128>,
    //     mut collect: RRefDeque<BlkReq, 128>,
    //     write: bool,
    // ) -> (usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>) {
    //     let mut submit_count = 0;

    //     while let Some(mut block_req) = submit.pop_front() {
    //         let block = block_req.block;
    //         // let data = &mut block_req.data[..];

    //         // let buffer = &data[..];
    //         // let buffer = Box::new(buffer);

    //         let buffer;

    //         let data = &mut block_req.data[..];
    //         if write {
    //             buffer = unsafe { Box::from_raw(data as *const [u8] as *mut [u8]) };
    //         } else {
    //             // let data = &block_req.data[..];
    //             buffer = unsafe { Box::from_raw(data as *mut [u8]) };
    //         }

    //         assert!(
    //             buffer.len() % 512 == 0,
    //             "Must read a multiple of block size number of bytes"
    //         );

    //         let address = &*buffer as *const [u8] as *const () as usize;
    //         let total_sectors = buffer.len() as u64 / 512;

    //         if let Some(slot) = self.port.ata_dma(
    //             block,
    //             total_sectors as u16,
    //             write,
    //             &mut self.clb,
    //             &mut self.ctbas,
    //             &*buffer,
    //         ) {
    //             // Submitted, create the corresponding BlkReq in self.blkreqs_opt
    //             self.port.set_slot_ready(slot, false);
    //             self.blkreqs_opt[slot as usize] = Some(block_req);
    //             // self.requests_opt[slot as usize] = Some(Request {
    //             //     address,
    //             //     start_sector: block,
    //             //     total_sectors,
    //             //     buffer,
    //             //     start_time: libtime::get_rdtsc(),
    //             // });
    //             submit_count += 1;
    //             self.stats.submitted += 1;

    //             // Poll
    //             while self.port.ata_running(slot) {
    //                 // Wait
    //             }
    //             // Request finished
    //             // Make sure there's space in collect then do the following
    //             let mut block_req = self.blkreqs_opt[slot as usize].take().unwrap();
    //             // let req = self.requests_opt[slot as usize].take().unwrap();
    //             self.port.set_slot_ready(slot, true);
    //             self.port.ata_stop(slot);
    //             // block_req.data.copy_from_slice(&*req.buffer);
    //             collect.push_back(block_req);
    //             self.stats.completed += 1;
    //         } else {
    //             // No slots available, push back the block_req
    //             // TODO: possibly submit has no space?
    //             submit.push_back(block_req);
    //         }
    //     }

    //     for slot in 0..self.requests_opt.len() {
    //         let slot = slot as u32;
    //         if let None = self.blkreqs_opt[slot as usize] {
    //             continue;
    //         }
    //         if !self.port.ata_running(slot) {
    //             // Make sure there's space in collect then do the following
    //             let block_req = self.blkreqs_opt[slot as usize].take().unwrap();
    //             self.port.set_slot_ready(slot, true);
    //             self.port.ata_stop(slot);
    //             collect.push_back(block_req);
    //             self.stats.completed += 1;
    //         }
    //     }

    //     (submit_count, submit, collect)
    // }

    fn submit_and_poll_rref(
        &mut self,
        mut submit: RRefDeque<BlkReq, 128>,
        mut collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> (usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>) {
        console::println!("Entered submit and poll rref: write = {}", write);
        let mut submit_count = 0;

        while let Some(mut block_req) = submit.pop_front() {
            let block = block_req.block;
            // let mut data = block_req.data;
            // let buffer;

            // if write {
            //     buffer = unsafe { Box::from_raw(&data as *const [u8] as *mut [u8]) };
            // } else {
            //     buffer = unsafe { Box::from_raw(&mut data as *mut [u8]) };
            // }
            let buffer;

            let data = &mut block_req.data[..];
            if write {
                buffer = unsafe { Box::from_raw(data as *const [u8] as *mut [u8]) };
            } else {
                // let data = &block_req.data[..];
                buffer = unsafe { Box::from_raw(data as *mut [u8]) };
            }

            // let buffer = &data[..];
            // let buffer = Box::new(buffer);

            assert!(
                buffer.len() % 512 == 0,
                "Must read a multiple of block size number of bytes"
            );

            // let address = &*buffer as *const [u8] as *const () as usize;
            let total_sectors = buffer.len() as u64 / 512;

            if let Some(slot) = self.port.ata_dma(
                block,
                total_sectors as u16,
                write,
                &mut self.clb,
                &mut self.ctbas,
                &*buffer,
            ) {
                // Submitted, create the corresponding BlkReq in self.blkreqs_opt
                console::println!(
                    "request with block {} now in slot {}",
                    block_req.block,
                    slot
                );
                self.port.set_slot_ready(slot, false);
                self.blkreqs_opt[slot as usize] = Some(block_req);
                submit_count += 1;
                self.stats.submitted += 1;
            } else {
                // No slots available, push back the block_req
                // TODO: possibly submit has no space?
                submit.push_back(block_req);
            }
        }

        for slot in 0..self.requests_opt.len() {
            let slot = slot as u32;
            if let None = self.blkreqs_opt[slot as usize] {
                continue;
            }
            if !self.port.ata_running(slot) {
                // Make sure there's space in collect then do the following
                let block_req = self.blkreqs_opt[slot as usize].take().unwrap();
                self.port.set_slot_ready(slot, true);
                self.port.ata_stop(slot);
                console::println!("slot {} - block {} finished.", slot, block_req.block);
                collect.push_back(block_req);
                self.stats.completed += 1;
            } else {
                console::println!("request at slot {} still running...", slot);
            }
        }

        (submit_count, submit, collect)
    }

    fn poll_rref(
        &mut self,
        mut collect: RRefDeque<BlkReq, 1024>,
    ) -> (usize, RRefDeque<BlkReq, 1024>) {
        let qid = 1;
        let mut count: usize = 0;
        let mut reap_count = 0;
        let mut cur_head = 0;
        let reap_all = false;

        for slot in 0..self.requests_opt.len() {
            let slot = slot as u32;
            if let None = self.blkreqs_opt[slot as usize] {
                continue;
            }
            if !self.port.ata_running(slot) {
                reap_count += 1;
                // Make sure there's space in collect then do the following
                let block_req = self.blkreqs_opt[slot as usize].take().unwrap();
                self.port.set_slot_ready(slot, true);
                self.port.ata_stop(slot);
                collect.push_back(block_req);
                self.stats.completed += 1;
            }
        }

        (reap_count, collect)
        //push it to the collect queue
        //and return the queue
    }

    fn get_stats(&mut self) -> (u64, u64) {
        (self.stats.submitted, self.stats.completed)
    }
}
