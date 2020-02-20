use alloc::vec::Vec;
use alloc::boxed::Box;
use libsyscalls::errors::Result;

use self::disk_ata::DiskATA;
use self::hba::Hba;
use self::hba_port::{HbaPort, HbaPortType};

use alloc::sync::Arc;


use ahci::{AhciBarRegion, AhciRegs};

use console::dbg;



pub mod disk_ata;
pub mod fis;
pub mod hba;
pub mod hba_port;

pub trait Disk {
    fn id(&self) -> usize;
    fn size(&mut self) -> u64;
    fn read(&mut self, block: u64, buffer: &mut [u8]);
    fn write(&mut self, block: u64, buffer: &[u8]);
    fn block_length(&mut self) -> Result<u32>;
    fn submit(&mut self, block: u64, write: bool, buffer: Box<[u8]>) -> Result<u32>;
    fn poll(&mut self, slot: u32) -> Result<Option<Box<[u8]>>>;
}

pub fn create_disks(bar: Box<dyn AhciBarRegion>) -> Vec<Box<dyn Disk>> {
    let _base: usize = bar.get_base() as usize;
    let name: &str = "rlahci";

    let hba = Arc::new(Hba::new(bar));
    hba.init();
    let pi = hba.bar.read_reg(AhciRegs::Pi);

    let disks: Vec<Box<dyn Disk>> = (0..32)
          .filter(|&i| pi & 1 << i as i32 != 0)
          .filter_map(|i| {
              let port = HbaPort::new(hba.clone(), i as u64);
              let port_type = port.probe();
              dbg!("HBA port {}-{}: {:?}", name, i, port_type);

              let disk: Option<Box<dyn Disk>> = match port_type {
                  HbaPortType::SATA => {
                      match DiskATA::new(i, port) {
                          Ok(disk) => Some(Box::new(disk)),
                          Err(err) => {
                              dbg!("Failed to create disk for port#{}: {}", i, err);
                              None
                          }
                      }
                  }
                  _ => None,
              };

              disk
          })
          .collect();

    disks
}
