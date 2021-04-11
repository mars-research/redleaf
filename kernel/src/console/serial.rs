use super::ns16550a::PioSerial;
use spin::Mutex;

const COM1_PORT: usize = 0x3F8;
const COM2_PORT: usize = 0x2F8;
const COM3_PORT: usize = 0x3E8;
const COM4_PORT: usize = 0x2E8;

pub static SERIALS: [Mutex<PioSerial>; 4] = [
    Mutex::new(unsafe {PioSerial::new(COM1_PORT)}),
    Mutex::new(unsafe {PioSerial::new(COM2_PORT)}),
    Mutex::new(unsafe {PioSerial::new(COM3_PORT)}),
    Mutex::new(unsafe {PioSerial::new(COM4_PORT)}),
];
