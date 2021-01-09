// Hello driver
//
// This simple driver listens to IRQ 1 (keyboard) for keystrokes,
// and implements a rudimentary shell.

use super::Driver;
use crate::redsys::IRQRegistrar;
use alloc::string::String;

pub struct Hello {
    current_command: String,
}

impl Driver for Hello {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<Hello>) {
        // Request IRQ 1 (Keyboard)
        registrar.request_irq(1, Hello::keyboard_handler).unwrap();
    }
}

impl Hello {
    pub fn new() -> Hello {
        let mut buf = String::new();
        buf.reserve(256);

        Hello {
            current_command: buf,
        }
    }

    pub fn keyboard_handler(&mut self) {
        use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
        use spin::Mutex;
        use x86_64::instructions::port::Port;

        lazy_static! {
            static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
                Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
        }

        let mut keyboard = KEYBOARD.lock();
        let mut port = Port::new(0x60);

        let scancode: u8 = unsafe { port.read() };
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        if character == '\n' {
                            println!();
                            self.run_command();
                        } else if character != '\x08' {
                            self.current_command.push(character);
                            print!("{}", character);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn run_command(&mut self) {
        match self.current_command.as_ref() {
            "hello" => println!("--> Hi there!"),
            "hi" => println!("--> Hello!"),
            "make me a sandwich" => println!("--> What? Make it yourself."),
            "sudo make me a sandwich" => println!("--> Okay."),
            "xyzzy" => println!("-- A hollow voice says, \"Fool!\""),
            _ => println!(
                "-- You hear a distant echo saying \"{}\"",
                self.current_command
            ),
        }
        self.current_command.clear();
    }
}
