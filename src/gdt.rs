/* The crate for TSS and GDT, see the source code here
   https://github.com/rust-osdev/x86_64/blob/master/src/structures/gdt.rs */

/*
use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
*/

use x86_64::VirtAddr;

use core::mem;
//use x86::current::segmentation::set_cs;
use x86::current::task::TaskStateSegment;
use x86::dtables::{DescriptorTablePointer};
use x86_64::structures::gdt::{Descriptor, SegmentSelector};
use x86::task;

use x86_64::instructions::segmentation;
use x86_64::instructions::segmentation::set_cs;
use x86_64::instructions::segmentation::load_ds;
use x86_64::instructions::segmentation::load_es;
use x86_64::instructions::segmentation::load_fs;
use x86_64::instructions::segmentation::load_gs;
use x86_64::instructions::segmentation::load_ss;
use x86_64::PrivilegeLevel::{Ring0};

use x86::controlregs;

//#[cfg(not(feature = "large_mem"))]
//use x86::msr::{IA32_FS_BASE, wrmsr};

//use crate::paging::PAGE_SIZE;
pub const PAGE_SIZE: usize = 4096;

pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const DOUBLE_FAULT_IST_INDEX: u16 = 2;
pub const NMI_IST_INDEX: u16 = 3;

pub const IST_STACK_SIZE: usize = PAGE_SIZE*4096;

#[thread_local]
pub static mut IST_PF_STACK: [u8; IST_STACK_SIZE] = [0; IST_STACK_SIZE];

#[thread_local]
pub static mut IST_DF_STACK: [u8; IST_STACK_SIZE] = [0; IST_STACK_SIZE];

#[thread_local]
pub static mut IST_NMI_STACK: [u8; IST_STACK_SIZE] = [0; IST_STACK_SIZE];

/*
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        /* Create an IST stack for the double fault handler */        
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        /* Create an IST stack for the NMI  handler */        
        tss.interrupt_stack_table[NMI_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        tss
    };
}
*/

/*
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        /* Create two selectors (one for kernel code and one for TSS)
           https://docs.rs/x86_64/0.7.5/x86_64/structures/gdt/struct.GlobalDescriptorTable.html */
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        set_cs(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
*/

/***********************************************************/

pub const GDT_NULL: usize = 0;
pub const GDT_KERNEL_CODE: usize = 1;
pub const GDT_KERNEL_DATA: usize = 2;
pub const GDT_KERNEL_TLS: usize = 3;
pub const GDT_USER_CODE: usize = 4;
pub const GDT_USER_DATA: usize = 5;
pub const GDT_USER_TLS: usize = 6;
pub const GDT_TSS: usize = 7;
pub const GDT_TSS_HIGH: usize = 8;

pub const GDT_A_PRESENT: u8 = 1 << 7;
pub const GDT_A_RING_0: u8 = 0 << 5;
pub const GDT_A_RING_1: u8 = 1 << 5;
pub const GDT_A_RING_2: u8 = 2 << 5;
pub const GDT_A_RING_3: u8 = 3 << 5;
pub const GDT_A_SYSTEM: u8 = 1 << 4;
pub const GDT_A_EXECUTABLE: u8 = 1 << 3;
pub const GDT_A_CONFORMING: u8 = 1 << 2;
pub const GDT_A_PRIVILEGE: u8 = 1 << 1;
pub const GDT_A_DIRTY: u8 = 1;

pub const GDT_A_TSS_AVAIL: u8 = 0x9;
pub const GDT_A_TSS_BUSY: u8 = 0xB;

pub const GDT_F_PAGE_SIZE: u8 = 1 << 7;
pub const GDT_F_PROTECTED_MODE: u8 = 1 << 6;
pub const GDT_F_LONG_MODE: u8 = 1 << 5;

/// Init-time GDT descriptor loaded into GDTR
static mut INIT_GDT_DESC: DescriptorTablePointer<Descriptor> = DescriptorTablePointer {
    limit: 0,
    base: 0 as *const Descriptor
};

/// Init-time GDT table (we need it before we set up per-CPU variables)
static mut INIT_GDT: [GdtEntry; 4] = [
    // Null
    GdtEntry::new(0, 0, 0, 0),
    // Kernel code
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_EXECUTABLE | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // Kernel data
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // Kernel TLS
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_3 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE)
];

/// Per-CPU GDT descriptor
#[thread_local]
pub static mut GDT_DESC: DescriptorTablePointer<Descriptor> = DescriptorTablePointer {
    limit: 0,
    base: 0 as *const Descriptor
};

/// Per-CPU GDT that has a private per-CPU fs segment for per-CPU variables
#[thread_local]
pub static mut GDT: [GdtEntry; 9] = [
    // Null
    GdtEntry::new(0, 0, 0, 0),
    // Kernel code
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_EXECUTABLE | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // Kernel data
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // Kernel TLS
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // User code
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_3 | GDT_A_SYSTEM | GDT_A_EXECUTABLE | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // User data
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_3 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // User TLS
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_3 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // TSS
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_3 | GDT_A_TSS_AVAIL, 0),
    // TSS must be 16 bytes long, twice the normal size
    GdtEntry::new(0, 0, 0, 0),
];

/// Per-CPU TSS (keeps per-CPU interrupt stacks, IST and simple kernel stacks)
#[thread_local]
pub static mut TSS: TaskStateSegment = TaskStateSegment {
    reserved: 0,
    rsp: [0; 3],
    reserved2: 0,
    ist: [0; 7],
    reserved3: 0,
    reserved4: 0,
    iomap_base: 0xFFFF
};

/*
pub unsafe fn set_tcb(pid: usize) {
    GDT[GDT_USER_TLS].set_offset((crate::USER_TCB_OFFSET + pid * PAGE_SIZE) as u32);
}
*/

pub unsafe fn set_tss_stack(stack: usize) {
    TSS.rsp[0] = stack as u64;
}

/// Setup init-time GDT
pub unsafe fn init_global_gdt() {
    // Setup the initial GDT with TLS, so we can setup the TLS GDT (a little confusing)
    // This means that each CPU will have its own GDT, but we only need to define it once as a thread local
    // Configure initial GDT descriptor
    INIT_GDT_DESC.limit = (INIT_GDT.len() * mem::size_of::<GdtEntry>() - 1) as u16;
    INIT_GDT_DESC.base = INIT_GDT.as_ptr() as *const Descriptor;

    // Load the initial GDT, before we have access to thread locals
    x86::dtables::lgdt(&INIT_GDT_DESC);

    // Load the segment descriptors
    set_cs(SegmentSelector::new(GDT_KERNEL_CODE as u16, Ring0));
    load_ds(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    load_es(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    load_fs(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    load_gs(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    load_ss(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
}

#[inline]
pub unsafe fn writefs(fs: u64) {
    llvm_asm!("wrfsbase $0" :: "r"(fs) :: "volatile");
}

/// Initialize GDT with TLS
/// In other words take a TCB offset and load it into the GDT
pub unsafe fn init_percpu_gdt(tcb_offset: u64) {
    // Set the TLS segment to the offset of the Thread Control Block
    //INIT_GDT[GDT_KERNEL_TLS].set_offset(tcb_offset as u32);

    // Load the initial GDT, before we have access to thread locals
    x86::dtables::lgdt(&INIT_GDT_DESC);

    // Enable wrfsbase
    //#[cfg(feature = "large_mem")]
    {
        let mut cr4 = controlregs::cr4();
        cr4 = cr4 | controlregs::Cr4::CR4_ENABLE_FSGSBASE;
        controlregs::cr4_write(cr4);
    }


    // Load fs
   //#[cfg(not(feature = "large_mem"))]
    //wrmsr(IA32_FS_BASE, tcb_offset);

    //#[cfg(feature = "large_mem")]
    writefs(tcb_offset); 

    // Now that we have access to thread locals, setup the AP's individual GDT
    GDT_DESC.limit = (GDT.len() * mem::size_of::<GdtEntry>() - 1) as u16;
    GDT_DESC.base = GDT.as_ptr() as *const Descriptor;

    // Set the TLS segment to the offset of the Thread Control Block
    //#[cfg(not(feature = "large_mem"))]
    //GDT[GDT_KERNEL_TLS].set_offset(tcb_offset as u32);

    // Set the User TLS segment to the offset of the user TCB
    //set_tcb(0);


    //TSS.ist[PAGE_FAULT_IST_INDEX as usize] = VirtAddr::from_ptr(unsafe { &IST_PF_STACK });
    TSS.ist[PAGE_FAULT_IST_INDEX as usize] = &IST_PF_STACK as *const _ as u64;
    TSS.ist[DOUBLE_FAULT_IST_INDEX as usize] = &IST_DF_STACK as *const _ as u64;
    TSS.ist[NMI_IST_INDEX as usize] = &IST_NMI_STACK as *const _ as u64;

    // We can now access our TSS, which is a thread local
    GDT[GDT_TSS].set_offset(&TSS as *const _ as u32);
    GDT[GDT_TSS].set_limit(mem::size_of::<TaskStateSegment>() as u32);

    //#[cfg(feature = "large_mem")]
    {
        GDT[GDT_TSS_HIGH].limitl = (((&TSS as *const _ as u64) >> 32) & 0xFFFF) as u16;
        GDT[GDT_TSS_HIGH].offsetl = (((&TSS as *const _ as u64) >> 48) & 0xFFFF) as u16;
    }

    // Set the stack pointer when coming back from userspace
    // set_tss_stack(stack_offset);

    // Load the new GDT, which is correctly located in thread local storage
    x86::dtables::lgdt(&GDT_DESC);

    // Reload the segment descriptors
    set_cs(SegmentSelector::new(GDT_KERNEL_CODE as u16, Ring0));
    segmentation::load_ds(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    segmentation::load_es(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    segmentation::load_gs(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));
    segmentation::load_ss(SegmentSelector::new(GDT_KERNEL_DATA as u16, Ring0));

    //#[cfg(not(feature = "large_mem"))]
    //segmentation::load_fs(SegmentSelector::new(GDT_KERNEL_TLS as u16, Ring0));

    // Enable wrfsbase
    //#[cfg(feature = "large_mem")]
    {
        let mut cr4 = controlregs::cr4();
        cr4 = cr4 | controlregs::Cr4::CR4_ENABLE_FSGSBASE;
        controlregs::cr4_write(cr4);

        writefs(tcb_offset);
    }


    //#[cfg(not(feature = "large_mem"))]
    //wrmsr(IA32_FS_BASE, tcb_offset);

    // Load the task register
    task::load_tr(x86::segmentation::SegmentSelector::new(GDT_TSS as u16, x86::Ring::Ring0));
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct GdtEntry {
    pub limitl: u16,
    pub offsetl: u16,
    pub offsetm: u8,
    pub access: u8,
    pub flags_limith: u8,
    pub offseth: u8
}

impl GdtEntry {
    pub const fn new(offset: u32, limit: u32, access: u8, flags: u8) -> Self {
        GdtEntry {
            limitl: limit as u16,
            offsetl: offset as u16,
            offsetm: (offset >> 16) as u8,
            access,
            flags_limith: flags & 0xF0 | ((limit >> 16) as u8) & 0x0F,
            offseth: (offset >> 24) as u8
        }
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.offsetl = offset as u16;
        self.offsetm = (offset >> 16) as u8;
        self.offseth = (offset >> 24) as u8;
    }

    pub fn set_limit(&mut self, limit: u32) {
        self.limitl = limit as u16;
        self.flags_limith = self.flags_limith & 0xF0 | ((limit >> 16) as u8) & 0x0F;
    }
}
