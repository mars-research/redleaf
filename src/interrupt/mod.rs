use lazy_static::lazy_static;
use x86::cpuid::CpuId;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{gdt, println, entryother};

mod lapic;
mod ioapic;
mod pic;

pub const IRQ_OFFSET: u8 = 32;

static mut use_apic: bool = true;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    NMI = 2,
    Breakpoint = 3,
    Timer = IRQ_OFFSET,
    Keyboard,
    ApicError = 19,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_u32(self) -> u32 {
        self as u32
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        /* NMI fault hanler executes on the IST stack */
        unsafe {
            idt.non_maskable_interrupt
               .set_handler_fn(nmi_handler)
               .set_stack_index(gdt::NMI_IST_INDEX); 
        }

        /* Double fault hanler executes on the IST stack -- just in 
            case the kernel stack is already full and triggers a pagefault, 
           that in turn (since the hardware will not be able to push the 
              exception fault on the stack will trigger a tripple fault */
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); 
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

pub unsafe fn init_cpu(cpu: u32, stack: [u8; 4096], code: u64) {
    let destination: *mut u8 = 0x7000 as *mut u8;
    let stackp: u32 = (&stack as *const u8) as u32;

    let mut pgdir: u64 = 0;
    asm!("mov $0, cr3" : "=r"(pgdir) ::: "intel");

    entryother::copy_binary_to(destination);
    entryother::init_args(destination, stackp + 4096, pgdir as u32, code);

    lapic::start_ap(cpu, destination);
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_irqs_local() {
    unsafe {
        use_apic = detect_apic();
        if !use_apic {
            panic!("APIC is required to run RedLeaf");
            // println!("Initializing PIC");
            // pic::init();
        }
        lapic::init();
    }
}

pub fn init_irqs() {
    unsafe {
        ioapic::init();
        ioapic::irqen(1, 0);
    }
}

fn end_of_interrupt(interrupt: u8) {
    // The unsafe is not too evil, as the variable is only set once
    if unsafe { use_apic } {
        lapic::end_of_interrupt();
    } else {
        pic::end_of_interrupt(interrupt);
    }
}

fn detect_apic() -> bool {
    let cpuid = CpuId::new();

    match cpuid.get_feature_info() {
        Some(feat) => feat.has_apic(),
        None => false,
    }
}

// 2: NMI
extern "x86-interrupt" fn nmi_handler(stack_frame: &mut InterruptStackFrame) {
    println!("nmi:\n{:#?}", stack_frame); 
    end_of_interrupt(InterruptIndex::NMI.as_u8());
}

// 3: #BP
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("breakpoint:\n{:#?}", stack_frame);
    end_of_interrupt(InterruptIndex::Breakpoint.as_u8());
}

// 8: #DF
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("double fault:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 11: #NP
extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("segment not present:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 12: #SS
extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("stack segment fault:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 13: #GP
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("general protection fault:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 14: #PF
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::halt();
}

// 17: #AC
extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("alignment check:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 18: #MC
extern "x86-interrupt" fn machine_check_handler(stack_frame: &mut InterruptStackFrame) {
    println!("machine check:\n{:#?}", stack_frame);
    crate::halt(); 
}

// IRQ 0: Timer
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    end_of_interrupt(InterruptIndex::Timer.as_u8());
}

// IRQ 1: Keyboard
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
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
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    end_of_interrupt(InterruptIndex::Keyboard.as_u8());
}
