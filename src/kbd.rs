use x86::io::{outb, inb};
use super::drivers::Driver;
use crate::redsys::IRQRegistrar;
use pc_keyboard::{Keyboard, ScancodeSet1, DecodedKey, layouts};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::sync::Arc;
use super::cb::{CircularBuffer,CbError};
use super::interrupt::get_irq_registrar;

/* Constants */
const OP_BUF_BIT: u8 = 0;
const OP_BUF_STATUS: u8 = (1 << OP_BUF_BIT);
const IN_BUF_BIT: u8 = 1;
const IN_BUF_STATUS: u8 = (1 << IN_BUF_BIT);
const PORT_TRANSLATION_BIT: u8 = 6;
const PORT_TRANSLATION_STATUS: u8 = (1 << PORT_TRANSLATION_BIT);

// commands
const READ_CONFIG_CMD: u8 = 0x20;
const WRITE_CONFIG_CMD: u8 = 0x60;
const SELF_TEST_CMD: u8 = 0xAA;

// responses
const SELF_TEST_PASSED: u8 = 0x55;

pub struct Kbd_ctrlr {
    cmd_status_port: u16,
    data_port: u16,
    kbd: Keyboard<layouts::Us104Key, ScancodeSet1>,
    key_buf : CircularBuffer<u8>,
}

impl Kbd_ctrlr {

    fn init(&self) {
        // 1. read the configuration byte from kb controller
        println!("Initializing keyboard controller...");

        self.write_command(READ_CONFIG_CMD);
        let mut config_byte : u8 = self.read_data();
        println!("configuration byte: 0x{:x}", config_byte);

        if config_byte & PORT_TRANSLATION_STATUS == 0  {
            println!("Enable translation \n");
            // Keep translation enabled for simpler scancode processing
            config_byte |= PORT_TRANSLATION_STATUS;
            self.write_command(WRITE_CONFIG_CMD);
            self.write_data(config_byte);
        }

        // 2. Run a self test
        self.write_command(SELF_TEST_CMD);
        if self.read_data() == SELF_TEST_PASSED {
            println!("Self test passed");
        } else {
            println!("Self test failed!!");
        }

        println!("Done");
    }

    fn write_command(&self, cmd: u8) {
        // wait until the input buffer is ready to accept more data
        while self.read_status() & IN_BUF_STATUS != 0 {};
        unsafe {outb(self.cmd_status_port, cmd)}
    }

    fn read_status(&self) -> u8 {
        unsafe {inb(self.cmd_status_port)}
    }

    fn write_data(&self, data: u8) {
        while self.read_status() & IN_BUF_STATUS != 0 {};
        unsafe {
            outb(self.data_port, data)
        }
    }

    fn read_data(&self) -> u8 {
        // Wait until output buffer has some data in it
        while self.read_status() & OP_BUF_STATUS == 0 {};
        unsafe { inb(self.data_port) }
    }

}

impl Kbd_ctrlr {

    pub fn new() -> Kbd_ctrlr {
        let kb = Kbd_ctrlr {
            cmd_status_port : 0x64,
            data_port : 0x60,
            kbd: Keyboard::new(layouts::Us104Key, ScancodeSet1),
            key_buf : CircularBuffer::<u8>::new_with_size(256)
        };
        kb.init();
        return kb;
    }

    pub fn kbd_irq_handler(&mut self) {
        // Ignore the error for now
        self.key_buf.push(self.read_data());
    }

    pub fn readch(&mut self) -> Result<DecodedKey, &'static str> {
        match self.key_buf.pop() {
            Ok(scancode) => {
                if let Ok(Some(key_event)) = self.kbd.add_byte(scancode) {
                    if let Some(key) = self.kbd.process_keyevent(key_event) {
                        Ok(key)
                    } else {
                        Err("Keyevent error")
                    }
                } else {
                    Err("Decode error")
                }
            },
            Err(CbError::QueueIsEmpty) => Err("Empty Buffer"),
            _ => Err("Read Error")
        }
    }
}

/* Register Interrupt handler for the Kbd_ctrlr controller */
impl Driver for Kbd_ctrlr {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<Kbd_ctrlr>) {
        registrar.request_irq(1, Kbd_ctrlr::kbd_irq_handler);
    }
}

lazy_static! {
    pub static ref KBDCTRL: Arc<Mutex<Kbd_ctrlr>> = {
        Arc::new(Mutex::new(Kbd_ctrlr::new()))
    };
}