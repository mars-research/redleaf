use alloc::vec::Vec;
use alloc::boxed::Box;
use libsyscalls::errors::Result;

use self::disk_ata::DiskATA;
use self::disk_atapi::DiskATAPI;
use self::hba::{HbaMem, HbaPortType};

use console::print;

pub mod disk_ata;
pub mod disk_atapi;
pub mod fis;
pub mod hba;

pub trait Disk {
    fn id(&self) -> usize;
    fn size(&mut self) -> u64;
    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>>;
    fn write(&mut self, block: u64, buffer: &[u8]) -> Result<Option<usize>>;
    fn block_length(&mut self) -> Result<u32>;
}

// base: 0xfebf1000
pub fn disks(base: usize, name: &str) -> (&'static mut HbaMem, Vec<Box<dyn Disk>>) {
    let hba_mem = unsafe { &mut *(base as *mut HbaMem) };
    hba_mem.init();
    let pi = hba_mem.pi.read();
    let disks: Vec<Box<dyn Disk>> = (0..hba_mem.ports.len())
          .filter(|&i| pi & 1 << i as i32 == 1 << i as i32)
          .filter_map(|i| {
              let port = unsafe { &mut *hba_mem.ports.as_mut_ptr().add(i) };

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
                  HbaPortType::SATAPI => {
                      match DiskATAPI::new(i, port) {
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

    (hba_mem, disks)
}
