use alloc::vec::Vec;
use alloc::boxed::Box;
use libsyscalls::errors::Result;

use self::disk_ata::DiskATA;
use self::hba::HbaPortType;
use self::hba::{Hba, HbaPort};

use alloc::sync::Arc;
use spin::Mutex;

use ahci::{AhciBarRegion, AhciRegs};

use console::{print, println};

use core::mem::MaybeUninit;

pub mod disk_ata;
pub mod fis;
pub mod hba;

pub trait Disk {
    fn id(&self) -> usize;
    fn size(&mut self) -> u64;
    fn read(&mut self, block: u64, buffer: &mut [u8]);
    fn write(&mut self, block: u64, buffer: &[u8]);
    fn block_length(&mut self) -> Result<u32>;
    fn submit(&mut self, block: u64, write: bool, buffer: Box<[u8]>) -> Result<u32>;
    fn poll(&mut self, slot: u32) -> Result<Option<Box<[u8]>>>;
}

pub fn disks(bar: Box<dyn AhciBarRegion>) -> Vec<Box<dyn Disk>> {
    let base: usize = bar.get_base() as usize;
    let name: &str = "rlahci";

    let hbaarc = Arc::new(Mutex::new(Hba::new(bar)));

    let pi = {
        let hba = hbaarc.lock();
        hba.init();
        hba.bar.read_reg(AhciRegs::Pi)
    };

    let disks: Vec<Box<dyn Disk>> = (0..32)
          .filter(|&i| pi & 1 << i as i32 == 1 << i as i32)
          .filter_map(|i| {
              let mut port = HbaPort::new(hbaarc.clone(), i as u64);
              let port_type = port.probe();
              print!("{}-{}: {:?}\n", name, i, port_type);

              let disk: Option<Box<dyn Disk>> = match port_type {
                  HbaPortType::SATA => {
                      match DiskATA::new(i, port) {
                          Ok(disk) => Some(Box::new(disk)),
                          Err(err) => {
                              print!("{}: {}\n", i, err);
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
