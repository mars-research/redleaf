use lazy_static::lazy_static;
use x86::cpuid::CpuId;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{gdt, println, entryother};

mod lapic;
mod ioapic;
mod pic;

pub const IRQ_OFFSET: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
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
        idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler); 
        idt.device_not_available.set_handler_fn(device_not_avail_handler);

        idt.invalid_tss.set_handler_fn(invalid_tss_handler); 
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);

        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        
        idt.page_fault.set_handler_fn(page_fault_handler);
        
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);

        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);
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

pub unsafe fn init_cpu(cpu: u32, stack: u32, code: u64) {
    let destination: *mut u8 = 0x7000 as *mut u8;

    let mut pgdir: u64 = 0;
    asm!("mov $0, cr3" : "=r"(pgdir) ::: "intel");

    entryother::copy_binary_to(destination);
    entryother::init_args(destination, stack, pgdir as u32, code);

    lapic::start_ap(cpu, destination);
}

pub fn init_idt() {
    IDT.load();

    // Trigger breakpoint interrupt to see that IDT is ok
    //x86_64::instructions::interrupts::int3();
}

pub fn init_irqs_local() {
    unsafe {
        if !detect_apic() {
            panic!("APIC is required to run RedLeaf");
        }
        pic::disable();
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
    lapic::end_of_interrupt();
}

fn detect_apic() -> bool {
    let cpuid = CpuId::new();

    match cpuid.get_feature_info() {
        Some(feat) => feat.has_apic(),
        None => false,
    }
}

// 0: Divide by zero
extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Divide by zero exception:\n{:#?}", stack_frame); 
}

// 1: Debug
extern "x86-interrupt" fn debug_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Debug exception:\n{:#?}", stack_frame); 
}


// 2: NMI
extern "x86-interrupt" fn nmi_handler(stack_frame: &mut InterruptStackFrame) {
    println!("NMI exception:\n{:#?}", stack_frame); 
}

// 3: Breakpoint
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Breakpoint exception:\n{:#?}", stack_frame);
}

// 4: Overflow
extern "x86-interrupt" fn overflow_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Overflow exception:\n{:#?}", stack_frame);
}

// 5: Bound range 
extern "x86-interrupt" fn bound_range_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Bound range exception:\n{:#?}", stack_frame);
    crate::halt();
}

// 6: Invalid opcode
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Invalide opcode exception:\n{:#?}", stack_frame);
    crate::halt();
}

// 7: Device not available
extern "x86-interrupt" fn device_not_avail_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Device not available exception:\n{:#?}", stack_frame);
    crate::halt();
}


// 8: Double fault
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("double fault:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 10: Invalid TSS
extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    println!("Invalid TSS exception:\n{:#?}", stack_frame);
    crate::halt();
}

// 11: Segment not present
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

// 13: General protection
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("general protection fault:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 14: Page fault 
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

// 16: x87 Floating-Point Exception
extern "x86-interrupt" fn x87_floating_point_handler(
    stack_frame: &mut InterruptStackFrame,
) {
    println!("x87 floating point exception:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 17: Alignment check
extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64
) {
    println!("Alignment check exception:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 18: Machine check
extern "x86-interrupt" fn machine_check_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Machine check exception:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 19: SIMD Floating-Point Exception
extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: &mut InterruptStackFrame) {
    println!("SIMD Floating-Point Exception:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 20: Virtualization
extern "x86-interrupt" fn virtualization_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Virtualization exception:\n{:#?}", stack_frame);
    crate::halt(); 
}

// 30: Security 
extern "x86-interrupt" fn security_exception_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64) {
    println!("Security exception:\n{:#?}", stack_frame);
    crate::halt(); 
}



// IRQ 0: Timer
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    end_of_interrupt(InterruptIndex::Timer.as_u8());
    crate::schedule();
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
