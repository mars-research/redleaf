extern crate alloc;

// pub mod disk_ata;
// pub mod fis;
// pub mod hba;
// pub mod hba_port;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use libsyscalls::errors::Result;

// use super::ahci::{AhciBarRegion, AhciRegs};
use super::ata::DiskATA;
use super::hba::{Hba, HbaPort, HbaPortType};
use ahci_regs::{AhciBarRegion, AhciRegs};
use interface::bdev::BlkReq;
use rref::{RRef, RRefDeque};

use console::dbg;

pub trait Disk {
    fn id(&self) -> usize;
    fn size(&self) -> u64;
    fn read(&mut self, block: u64, buffer: &mut [u8]);
    fn write(&mut self, block: u64, buffer: &[u8]);
    fn block_length(&mut self) -> Result<u32>;
    fn submit(&mut self, block: u64, write: bool, buffer: Box<[u8]>) -> Result<u32>;
    fn poll(&mut self, slot: u32) -> Result<Option<Box<[u8]>>>;

    fn submit_and_poll_rref(
        &mut self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> (usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>);
    fn poll_rref(&mut self, collect: RRefDeque<BlkReq, 1024>) -> (usize, RRefDeque<BlkReq, 1024>);
    fn get_stats(&mut self) -> (u64, u64);
}

pub fn create_disks(bar: Box<dyn AhciBarRegion + Send + Sync>) -> Vec<Box<dyn Disk + Send>> {
    let _base: usize = bar.get_base() as usize;
    let name: &str = "rlahci";

    let hba = Arc::new(Hba::new(bar));
    hba.init();
    let pi = hba.bar.read_reg(AhciRegs::Pi);

    let disks = (0..32)
        .filter(|&i| pi & 1 << i as i32 != 0)
        .filter_map(|i| {
            let port = HbaPort::new(hba.clone(), i as u64);
            let port_type = port.probe();
            dbg!("HBA port {}-{}: {:?}", name, i, &port_type);

            let disk: Option<Box<dyn Disk + Send>> = match port_type {
                HbaPortType::SATA => match DiskATA::new(i, port) {
                    Ok(disk) => Some(Box::new(disk)),
                    Err(err) => {
                        dbg!("Failed to create disk for port#{}: {}", i, err);
                        None
                    }
                },
                _ => None,
            };

            disk
        })
        .collect();

    disks
}
