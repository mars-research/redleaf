// I/O APIC
// Reference
//  - https://wiki.osdev.org/IOAPIC
//  - https://github.com/mit-pdos/xv6-public/blob/master/ioapic.c

use core::ptr;
use super::InterruptIndex;

// FIXME: This is bad. We should use ACPI to get a list of IOAPICs
const IOAPIC: u32 = 0xfec00000;
const IOAPIC_VER: u32 = 0x01;
const IOAPIC_REDTBL: u32 = 0x10;

const IOAPIC_INT_DISABLED: u32 = 0x00010000;

const IRQ_OFFSET: u32 = super::IRQ_OFFSET as u32;

unsafe fn ioapicr(register: u32) -> u32 {
    ptr::write_volatile(IOAPIC as *mut u32, register);
    ptr::read_volatile((IOAPIC + 0x10) as *const u32)
}

unsafe fn ioapicw(register: u32, value: u32) {
    ptr::write_volatile(IOAPIC as *mut u32, register);
    ptr::write_volatile((IOAPIC + 0x10) as *mut u32, value);
}

pub fn init() {
    unsafe {
        let maxintr = (ioapicr(IOAPIC_VER) >> 16) & 0xff;

        // https://github.com/mit-pdos/xv6-public/blob/master/ioapic.c
        for i in 0..maxintr {
            // disabled
            ioapicw(IOAPIC_REDTBL + 2 * i, IOAPIC_INT_DISABLED | (IRQ_OFFSET + i));
            // not routed to any CPUs
            ioapicw(IOAPIC_REDTBL + 2 * i + 1, 0);
        }
    }
}

pub unsafe fn irqen(irq: u32, cpu: u32) {
    ioapicw(IOAPIC_REDTBL + 2 * irq, IRQ_OFFSET + irq);
    ioapicw(IOAPIC_REDTBL + 2 * irq + 1, cpu << 24);
}