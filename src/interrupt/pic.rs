// PIC
// RedLeaf does not support PIC. It only knows how to disable the PIC by masking all
// interrupts.

use x86::io::outb;

const PIC1_DATA: u16     = 0x21;
const PIC2_DATA: u16     = 0xa1;

pub fn disable() {
    // Mask everything
    unsafe {
        outb(PIC1_DATA, 0xff);
        outb(PIC2_DATA, 0xff);
    }
}
