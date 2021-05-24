#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    option_expect_none,
    untagged_unions
)]

// #[macro_use]
// // extern crate bitflags;
// extern crate byteorder;
// #[macro_use]
// extern crate serde_derive;

// mod ahci;
// mod ata;
// mod disk;
// mod fis;
// mod hba;

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use byteorder::{ByteOrder, LittleEndian};
use core::panic::PanicInfo;
use interface::bdev::{BDev, BlkReq, NvmeBDev, BSIZE};
use interface::error::{ErrorKind, Result};
use interface::rpc::RpcResult;
use spin::Mutex;

use console::println;
// use libsyscalls::errors::Result;
use libsyscalls::syscalls::{sys_backtrace, sys_yield};
use pci_driver::{BarRegions, DeviceBarRegions, PciClass};
use rref::{traits::CustomCleanup, RRef, RRefDeque};
use spin::Once;
use syscalls::Syscall;

mod benchmark;
use ahci_device::disk;
use ahci_regs::AhciBarRegion;
use benchmark::benchmark_sync_ahci;

struct Ahci {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    disks: Mutex<Vec<Option<Box<dyn disk::Disk + Send>>>>,
    // submitted blkreq + slot
    // completed blkreq + slot
}

impl Ahci {
    fn new() -> Ahci {
        Ahci {
            // Dummy values. We will use class based matching
            // so vendor_id and device_id won't be used
            vendor_id: 0x1234,
            device_id: 0x1234,
            driver: pci_driver::PciDrivers::AhciDriver,
            disks: Mutex::new(Vec::new()),
        }
    }

    // TODO: return a Err if the disk is not found
    fn with_disk<F, R>(&self, id: usize, f: F) -> R
    where
        F: FnOnce(&mut dyn disk::Disk) -> R,
    {
        // Take the disk from `disks` so we can release the lock
        let mut disk = loop {
            let mut disk = self.disks.lock()[id].take();
            match disk {
                None => {
                    // The disk is currently being used by another thread
                    // Wait and retry
                    sys_yield();
                    continue;
                }
                Some(disk) => break disk,
            }
        };

        // Do something with the disk
        let rtn = f(&mut *disk);

        // Put the disk back after we are done using it
        if self.disks.lock()[id].replace(disk).is_some() {
            panic!(
                "Disk<{}> is accessed by another thread while this thread is using it",
                id
            );
        }
        rtn
    }
}

impl pci_driver::PciDriver for Ahci {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        println!("probe() called");

        let bar = match bar_region {
            DeviceBarRegions::Ahci(bar) => {
                // bar
                unsafe { ahci_device::ahci::AhciBar::new(bar.get_base() as u64, bar.get_size()) }
            }
            _ => {
                panic!("Got unknown BAR region");
            }
        };

        let bar: Box<dyn AhciBarRegion + Send + Sync> = Box::new(bar);

        // println!("Initializing with base = {:x}", bar.get_base());

        let mut disks = disk::create_disks(bar);
        // Filter out all disk that already has an OS on it
        let have_magic_number: Vec<bool> = disks
            .iter_mut()
            .map(|d| {
                let mut buf = [0u8; 512];
                const MBR_MAGIC: u16 = 0xAA55;
                d.read(0, &mut buf);
                println!("{}", LittleEndian::read_u16(&buf[510..]));
                LittleEndian::read_u16(&buf[510..]) == MBR_MAGIC
            })
            .collect();
        let disks = disks
            .into_iter()
            .zip(have_magic_number)
            .filter_map(
                |(d, has_magic_num)| {
                    if has_magic_num {
                        None
                    } else {
                        Some(Some(d))
                    }
                },
            )
            .collect();
        self.disks = Mutex::new(disks);

        for (i, disk) in self.disks.lock().iter().enumerate() {
            println!("Disk {}: {}", i, disk.as_ref().unwrap().size());
        }

        println!("probe() finished");
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

// impl interface::bdev::SyncBDev for Ahci {
//     fn read(&self, block: u32, data: &mut [u8]) {
//         self.with_disk(0, |d| d.read(block as u64, data))
//     }
//     fn write(&self, block: u32, data: &[u8]) {
//         self.with_disk(0, |d| d.write(block as u64, data))
//     }
// }

// TODO: impl with RRefs
//    fn submit(&self, block: u64, write: bool, buf: Box<[u8]>) -> Result<u32> {
//        self.disks.borrow_mut()[0].submit(block, write, buf)
//    }
//
//    fn poll(&self, slot: u32) -> Result<Option<Box<[u8]>>> {
//        self.disks.borrow_mut()[0].poll(slot)
//    }
// }

impl BDev for Ahci {
    fn read(&self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        // let mut buffer: Box<u8> = Box::<u8>::new(0);
        let mut value: [u8; BSIZE] = [0; BSIZE];
        self.with_disk(0, |d| d.read(block as u64, &mut value));
        // data.copy_from_slice();
        data.copy_from_slice(&value);
        Ok(data)
    }
    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        // call write from disk
        self.with_disk(0, |d| d.write(block as u64, &**data));
        Ok(())
    }
}

impl NvmeBDev for Ahci {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)>> {
        // let mut submit = Some(submit);
        // let mut collect = Some(collect);
        let (submit_num, submit_, collect_) =
            self.with_disk(0, |d| d.submit_and_poll_rref(submit, collect, write));
        Ok(Ok((submit_num, submit_, collect_)))
    }

    fn poll_rref(
        &self,
        mut collect: RRefDeque<BlkReq, 1024>,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        let (num, collect_) = self.with_disk(0, |d| d.poll_rref(collect));
        Ok(Ok((num, collect_)))
    }

    fn get_stats(&self) -> RpcResult<Result<(u64, u64)>> {
        let (submitted, collected) = self.with_disk(0, |d| d.get_stats());
        Ok(Ok((submitted, collected)))
    }
}

fn run_blocktest_rref(device: &Ahci, from_block: u64, block_num: u64) {
    assert!(block_num <= 32, "block num must be at most 32");

    // Submit write requests
    let mut submit = RRefDeque::<BlkReq, 128>::default();
    let mut collect = RRefDeque::<BlkReq, 128>::default();
    let mut poll = RRefDeque::<BlkReq, 1024>::default();

    for i in 0..block_num {
        let mut block_req = BlkReq::from_data([((from_block + i) % 255) as u8; 4096]);
        block_req.block = i;
        submit.push_back(RRef::<BlkReq>::new(block_req));
    }

    let (submit_num, submit_, collect_) = device
        .submit_and_poll_rref(submit, collect, true)
        .unwrap()
        .unwrap();

    submit = submit_;
    collect = collect_;

    // Wait for write to finish
    loop {
        let (submit_num, submit_, collect_) = device
            .submit_and_poll_rref(submit, collect, false)
            .unwrap()
            .unwrap();
        submit = submit_;
        collect = collect_;
        if collect.len() == block_num as usize {
            collect.cleanup();
            break;
        }
    }
    assert!(submit.len() == 0, "submit is not finished");

    // Submit read requests
    for i in 0..block_num {
        let mut block_req = BlkReq::new();
        block_req.block = i;
        submit.push_back(RRef::<BlkReq>::new(block_req));
    }

    let (submit_num, submit_, collect_) = device
        .submit_and_poll_rref(submit, collect, false)
        .unwrap()
        .unwrap();

    submit = submit_;
    collect = collect_;

    loop {
        let (submit_num, submit_, collect_) = device
            .submit_and_poll_rref(submit, collect, false)
            .unwrap()
            .unwrap();

        submit = submit_;
        collect = collect_;

        if collect.len() == block_num as usize {
            while let Some(block_req) = collect.pop_front() {
                let value = [((from_block + block_req.block) % 255) as u8; 4096];
                assert_eq!(
                    &value[..],
                    &block_req.data[..],
                    "\nexpected{:?}\ngot{:?}\n",
                    &value[..],
                    &block_req.data[..],
                );
            }
            break;
        }
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn NvmeBDev> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    let mut ahci = Ahci::new();
    if let Err(_) = pci.pci_register_driver(
        &mut ahci,
        /*ABAR index*/ 5,
        Some((PciClass::Storage, /*SATA*/ 0x06)),
    ) {
        println!("WARNING: Failed to register AHCI device");
    }

    // let ahci: Box<dyn interface::bdev::BDev> = Box::new(ahci);
    // let ahci: Box<dyn BDev + Send + Sync> = Box::new(ahci);
    run_blocktest_rref(&ahci, 0, 8);

    let ahci: Box<dyn NvmeBDev + Send> = Box::new(ahci);

    // verify_write(&ahci);

    // benchmark_sync_ahci(&ahci, 512, 1);
    // benchmark_sync_ahci(&ahci, 8192, 1);
    // benchmark_sync_ahci(&ahci, 8192 * 8, 1);

    // timed_sync_ahci(&ahci, 3);
    // benchmark_sync_ahci(&ahci, 0xFFFF * 128, 0xFFFF);

    // benchmark_ahci(&ahci, 1, 1);
    // benchmark_ahci_async(&ahci, 256, 1);
    // benchmark_ahci(&ahci, 8192, 8192);
    // benchmark_ahci_async(&ahci, 8192, 8192);
    // benchmark_ahci(&ahci, 8192 * 128, 8192);
    // benchmark_ahci_async(&ahci, 8192 * 128, 8192);
    // benchmark_ahci(&ahci, 32768, 32768);
    // benchmark_ahci(&ahci, 0xFFFF * 128, 0xFFFF);
    // benchmark_ahci_async(&ahci, 0xFFFF * 128, 0xFFFF);
    ahci
}

// fn verify_write(bdev: &Box<dyn interface::bdev::SyncBDev>) {
//     let disk_offset = 10000;
//     let buff = RRef::new([123u8; 4096]);
//     bdev.write(disk_offset, &buff);

//     let mut buff = RRef::new([222u8; 4096]);
//     // bdev.read(disk_offset, &mut buff);
//     bdev.read(disk_offset, buff);
//     for i in buff.iter() {
//         assert!(*i == 123u8);
//     }
// }

// fn verify_write(bdev: &Box<dyn BDev>) {
//     let disk_offset = 10000;
//     let buff = RRef::new([123u8; 4096]);
//     bdev.write(disk_offset, &buff);

//     let mut buff = RRef::new([222u8; 4096]);
//     // bdev.read(disk_offset, &mut buff);
//     bdev.read(disk_offset, buff);
//     for i in buff.iter() {
//         assert!(*i == 123u8);
//     }
// }

// TODO: impl with RRefs
// fn benchmark_ahci(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
//     assert!(blocks_to_read % blocks_per_patch == 0);
//     assert!(blocks_per_patch <= 0xFFFF);
//     let mut buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];

//     let start = libtime::get_rdtsc();
//     for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
//         bdev.read_contig(i, &mut buf);
//     }
//     let end = libtime::get_rdtsc();
//     println!(
//         "AHCI benchmark: reading {} blocks, {} blocks at a time, takes {} cycles",
//         blocks_to_read,
//         blocks_per_patch,
//         end - start
//     );
// }

// TODO: impl with RRefs
//fn benchmark_ahci_async(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
//    println!("starting bencharl async {}", blocks_to_read);
//
//    assert!(blocks_to_read % blocks_per_patch == 0);
//    assert!(blocks_per_patch <= 0xFFFF);
//    let mut buffers: Vec<Box<[u8]>> = Vec::new();
//    for _ in 0..32 {
//        let buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];
//        buffers.push(buf.into_boxed_slice());
//    }
//    let mut pending = Vec::<u32>::new();
//
//    let start = libtime::get_rdtsc();
//    for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
//        while buffers.is_empty() {
//            assert!(!pending.is_empty());
//            pending = pending
//                .into_iter()
//                .filter(|slot|  {
//                    if let Some(buf) = bdev.poll(*slot).unwrap() {
//                        buffers.push(buf);
//                        false
//                    } else {
//                        true
//                    }
//                })
//                .collect();
//        }
//
//        pending.push(bdev.submit(i as u64, false, buffers.pop().unwrap()).unwrap());
//    }
//
//    for p in pending {
//        while bdev.poll(p).unwrap().is_none() {
//            // spin
//        }
//    }
//    let end = libtime::get_rdtsc();
//    println!("AHCI async benchmark: reading {} blocks, {} blocks at a time, takes {} cycles", blocks_to_read, blocks_per_patch, end - start);
//}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("ahci panicked: {:?}", info);
    sys_backtrace();
    loop {}
}
