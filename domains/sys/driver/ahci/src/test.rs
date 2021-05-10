
extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use byteorder::{ByteOrder, LittleEndian};
use core::panic::PanicInfo;
use interface::bdev::{BDev, BSIZE};
use interface::rpc::RpcResult;
use spin::Mutex;

use console::println;
use libsyscalls::errors::Result;
use libsyscalls::syscalls::{sys_backtrace, sys_yield};
use pci_driver::{BarRegions, DeviceBarRegions, PciClass};
use rref::RRef;
use spin::Once;
use syscalls::Syscall;

use ahci_device::disk;
use ahci_regs::AhciBarRegion;



fn verify_write(bdev: &Box<dyn interface::bdev::SyncBDev>) {
    let disk_offset = 10000;
    let buff = RRef::new([123u8; 4096]);
    bdev.write(disk_offset, &buff);

    let mut buff = RRef::new([222u8; 4096]);
    // bdev.read(disk_offset, &mut buff);
    bdev.read(disk_offset, buff);
    for i in buff.iter() {
        assert!(*i == 123u8);
    }
}

fn verify_write(bdev: &Box<dyn BDev>) {
    let disk_offset = 10000;
    let buff = RRef::new([123u8; 4096]);
    bdev.write(disk_offset, &buff);

    let mut buff = RRef::new([222u8; 4096]);
    // bdev.read(disk_offset, &mut buff);
    bdev.read(disk_offset, buff);
    for i in buff.iter() {
        assert!(*i == 123u8);
    }
}

//TODO: impl with RRefs
fn benchmark_ahci(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
   assert!(blocks_to_read % blocks_per_patch == 0);
   assert!(blocks_per_patch <= 0xFFFF);
   let mut buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];

   let start = libtime::get_rdtsc();
   for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
       bdev.read_contig(i, &mut buf);
   }
   let end = libtime::get_rdtsc();
   println!("AHCI benchmark: reading {} blocks, {} blocks at a time, takes {} cycles", blocks_to_read, blocks_per_patch, end - start);
}

//TODO: impl with RRefs
fn benchmark_ahci_async(bdev: &Box<dyn usr::bdev::BDev>, blocks_to_read: u32, blocks_per_patch: u32) {
   println!("starting bencharl async {}", blocks_to_read);

   assert!(blocks_to_read % blocks_per_patch == 0);
   assert!(blocks_per_patch <= 0xFFFF);
   let mut buffers: Vec<Box<[u8]>> = Vec::new();
   for _ in 0..32 {
       let buf = alloc::vec![0 as u8; 512 * blocks_per_patch as usize];
       buffers.push(buf.into_boxed_slice());
   }
   let mut pending = Vec::<u32>::new();

   let start = libtime::get_rdtsc();
   for i in (0..blocks_to_read).step_by(blocks_per_patch as usize) {
       while buffers.is_empty() {
           assert!(!pending.is_empty());
           pending = pending
               .into_iter()
               .filter(|slot|  {
                   if let Some(buf) = bdev.poll(*slot).unwrap() {
                       buffers.push(buf);
                       false
                   } else {
                       true
                   }
               })
               .collect();
       }

       pending.push(bdev.submit(i as u64, false, buffers.pop().unwrap()).unwrap());
   }

   for p in pending {
       while bdev.poll(p).unwrap().is_none() {
           // spin
       }
   }
   let end = libtime::get_rdtsc();
   println!("AHCI async benchmark: reading {} blocks, {} blocks at a time, takes {} cycles", blocks_to_read, blocks_per_patch, end - start);
}