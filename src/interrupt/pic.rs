// Use the PIC 8259 crate 
// https://docs.rs/crate/pic8259_simple/0.1.1/source/src/lib.rs
use pic8259_simple::ChainedPics;
use spin;

// Map first PIC line to interrupt 32 
const PIC1_OFFSET: u8 = super::IRQ_OFFSET;
const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

// See https://os.phil-opp.com/hardware-interrupts/ 
static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC1_OFFSET, PIC2_OFFSET) });

pub fn init() {
    unsafe {
        PICS.lock().initialize();
    }
}

pub fn end_of_interrupt(interrupt: u8) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(interrupt);
    }
}