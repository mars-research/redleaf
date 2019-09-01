// IOAPIC

use super::InterruptIndex;

// FIXME: This is bad. We should use ACPI to get a list of IOAPICs
static mut lapic: u32 = ;

const IRQ_OFFSET: u32 = super::IRQ_OFFSET as u32;