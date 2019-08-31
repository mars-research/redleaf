// LAPIC
// Reference:
//  - https://wiki.osdev.org/APIC#Local_APIC_configuration
//  - https://github.com/pdoane/osdev/blob/master/intr/local_apic.c
//  - https://github.com/mit-pdos/xv6-public/blob/master/lapic.c

extern crate raw_cpuid;
use core::ptr;
use raw_cpuid::CpuId;
use x86_64::registers::model_specific::Msr;

static mut apic: u32 = 0;

const LAPIC_ID: u32 = 0x0020;
const LAPIC_VER: u32 = 0x0030;
const LAPIC_TPR: u32 = 0x0080;
const LAPIC_EOI: u32 = 0x00b0;
const LAPIC_SVR: u32 = 0x00f0;
const LAPIC_ESR: u32 = 0x0280;
const LAPIC_TIMER: u32 = 0x0320;
const LAPIC_PCINT: u32 = 0x0340;
const LAPIC_LINT0: u32 = 0x0350;
const LAPIC_LINT1: u32 = 0x0360;
const LAPIC_ERROR: u32 = 0x0370;
const LAPIC_TICR: u32 = 0x0380;
const LAPIC_TDCR: u32 = 0x03e0;

const LAPIC_SVR_ENABLE: u32 = 0x0100;
const LAPIC_TDCR_X1: u32 = 0x0000000b;
const LAPIC_TIMER_PERIODIC: u32 = 0x00020000;
const LAPIC_MASKED: u32 = 0x00010000;

const IRQ_OFFSET: u32 = 32;
const IRQ_SPURIOUS: u32 = 31;

unsafe fn apicr(offset: u32) -> u32 {
    ptr::read_volatile((apic + offset) as *const u32)
    /*
    let mut value: u32 = 0;
    asm!("mov $0, $1" : "=r"(value) : "r"(apic + offset) :: "intel");
    value
    */
}

unsafe fn apicw(offset: u32, value: u32) {
    ptr::write_volatile((apic + offset) as *mut u32, value);
    ptr::read_volatile((apic + LAPIC_ID) as *const u32);
    /*
    asm!("mov $0, $1
          mov eax, $2"
         : // output
         : "r"(apic + offset), "r"(value), "r"(apic + LAPIC_ID) // input
         : "eax" // clobber
         : "intel"
    );
    */
}

fn probe_apic() {
    let cpuid = CpuId::new();

    match cpuid.get_feature_info() {
        Some(feat) => {
            if !feat.has_apic() {
                panic!("lapic: APIC requested but the machine does not support it");
            }
        },
        None => println!("lapic: Processor does not support CPUID. Continuing assuming APIC is present. YMMV"),
    }

    unsafe {
        let msr27: u32 = Msr::new(27).read() as u32;
        apic = msr27 & 0xffff0000;
        println!("APIC @ {:x?}", apic);
    }
}

fn init_lapic() {
    unsafe {
        // Enable LAPIC
        apicw(LAPIC_SVR, LAPIC_SVR_ENABLE | (IRQ_OFFSET + IRQ_SPURIOUS));

        // Timer interrupt
        apicw(LAPIC_TDCR, LAPIC_TDCR_X1);
        apicw(LAPIC_TIMER, LAPIC_TIMER_PERIODIC | (IRQ_OFFSET + 0));
        apicw(LAPIC_TICR, 10000000);

        // Mask logical interrupt lines
        apicw(LAPIC_LINT0, LAPIC_MASKED);
        apicw(LAPIC_LINT1, LAPIC_MASKED);

        // Mask performance counter overflow interrupts
        if ((apicr(LAPIC_VER) >> 16) & 0xff) >= 4 {
            apicw(LAPIC_PCINT, LAPIC_MASKED);
        }

        // Remap error to IRQ 19
        apicw(LAPIC_ERROR, IRQ_OFFSET + 19);

        // Clear error status register
        apicw(LAPIC_ESR, 0);
        apicw(LAPIC_ESR, 0);

        // Ack any outstanding interrupts
        apicw(LAPIC_EOI, 0);

        // Enable interrupts on APIC
        apicw(LAPIC_TPR, 0);

        println!("LAPIC initialization sequence complete");
    }
}

pub fn end_of_interrupt() {
    unsafe {
        apicw(LAPIC_EOI, 0);
    }
}

pub fn init() {
    probe_apic();
    init_lapic();
}
