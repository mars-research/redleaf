use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;

use pc_keyboard::DecodedKey;

use usr_interface::xv6::Device;
use libsyscalls::syscalls::{sys_readch_kbd, sys_yield};
use console::{print, println};

pub struct ConsoleDevice;

impl ConsoleDevice {
    pub fn new() -> Self {
        Self{}
    }
}

impl Device for ConsoleDevice {
    fn read(&self, data: &mut [u8]) {
        for d in data.iter_mut() {
            loop {
                let key = match sys_readch_kbd() {
                    Err(e) => {
                        // println!("{}", e);
                        sys_yield();
                        continue;
                    },
                    Ok(key) => key,
                };
                match key {
                    None => {
                        sys_yield();
                        continue;
                    }
                    Some(DecodedKey::Unicode(key)) => *d = (key as u8),
                    Some(DecodedKey::RawKey(key)) => {
                        console::println!("Skipping raw key {:?}", key);
                        continue;
                    },
                }
                break;
            }
        }
    }

    fn write(&self, data: &[u8]) {
        for d in data.iter() {
            print!("{}", *d as char);
        }
    }
}

lazy_static! {
    // xv6 equivalent: devsw
    pub static ref DEVICES: Vec<Option<Box<dyn Device + Send + Sync>>> = alloc::vec![None, Some(box ConsoleDevice::new())];
}