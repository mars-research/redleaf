use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;

use pc_keyboard::DecodedKey;

use console::{print, println};
use libsyscalls::syscalls::{sys_readch_kbd, sys_yield};
use usr_interface::xv6::Device;

struct ConsoleDeviceInternal {
    // The buffer is supposed be in the driver. I'm just too lazy to do the refactoring.
    buffer: VecDeque<u8>,
    reached_eol: bool,
}

impl ConsoleDeviceInternal {
    fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(1024),
            reached_eol: false,
        }
    }

    fn populate_buffer_until_eol(&mut self) {
        loop {
           let key = match sys_readch_kbd() {
                Err(e) => {
                    // println!("{}", e);
                    sys_yield();
                    continue;
                }
                Ok(key) => key,
            };
            match key {
                None => {
                    sys_yield();
                    continue;
                }
                Some(DecodedKey::Unicode(key)) => {
                    if !key.is_control() {
                        self.buffer.push_back(key as u8)
                    } else {
                        let key = key as u8;
                        match key {
                            b'\x08' => {
                                if self.buffer.pop_back().is_some() {
                                    self.write(&[key]);
                                }
                            },
                            b'\n' => {
                                self.write(&[key]);
                                self.buffer.push_back(key);
                                self.reached_eol = true;
                                return;
                            }
                            _ => {},
                        }
                    }
                },
                Some(DecodedKey::RawKey(key)) => {
                    console::println!("Skipping raw key {:?}", key);
                    continue;
                }
            }
        }
    }

    // Block until end-of-line
    // TODO: use cv to reduce spinning
    fn read(&mut self, data: &mut [u8]) -> usize {
        if !self.reached_eol {
            println!("please wait");
            self.populate_buffer_until_eol();
        }

        println!("you got this");
        for (i, d) in data.iter_mut().enumerate() {
            match self.buffer.pop_front() {
                Some(c) => *d = c,
                None => {
                    self.reached_eol = false;
                    return i + 1;
                },
            }
        }

        if self.buffer.len() == 0 {
            self.reached_eol = false;
        }

        data.len()
    }

    fn write(&mut self, data: &[u8]) -> usize {
        for d in data.iter() {
            print!("{}", *d as char);
        }
        data.len()
    }
}

pub struct ConsoleDevice(Mutex<ConsoleDeviceInternal>);

impl ConsoleDevice {
    fn new() -> Self {
        Self(Mutex::new(ConsoleDeviceInternal::new()))
    }
}

impl Device for ConsoleDevice {
    fn read(&self, data: &mut [u8]) -> usize {
        self.0.lock().read(data)
    }

    fn write(&self, data: &[u8]) -> usize {
        self.0.lock().write(data)
    }
}

lazy_static! {
    // xv6 equivalent: devsw
    pub static ref DEVICES: Vec<Option<Box<dyn Device + Send + Sync>>> = alloc::vec![None, Some(box ConsoleDevice::new())];
}
