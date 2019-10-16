#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions,
    naked_functions,
    panic_info_message
)]

extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;
extern crate slabmalloc;
extern crate alloc;

extern crate backtracer;


#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod redsys;
mod drivers;
mod filesystem;
pub mod gdt;

mod multibootv2;
mod memory;
mod prelude;
pub mod arch;

mod tls;
//mod common; 
mod thread;
mod panic; 

use x86::cpuid::CpuId;
use crate::arch::init_buddy;
use spin::Mutex;
use core::alloc::{GlobalAlloc, Layout};
use memory::{BespinSlabsProvider, PhysicalAllocator};
use slabmalloc::{PageProvider, ZoneAllocator};
use crate::memory::buddy::BUDDY;
use thread::{Scheduler, Thread};
use core::cell::{UnsafeCell, RefCell};
use alloc::boxed::Box;
use crate::thread::switch;

#[no_mangle]
pub static mut cpu1_stack: u32 = 0;

extern "C" {
    #[no_mangle]
    static _bootinfo: usize;
}

/// We use this static variable to temporarely save the stack of the 
/// boot function (rust_main_ap()) 
static mut AP_INIT_STACK: *mut usize = 0x0 as *mut usize;

/// Stack size for the kernel main thread
const KERNEL_STACK_SIZE: usize = 4096 * 16;

/// Per-CPU scheduler
#[thread_local]
static SCHED: RefCell<Scheduler> = RefCell::new(Scheduler::new()); 

/// Per-CPU current thread
#[thread_local]
static CURRENT: RefCell<Option<Box<Thread>>> = RefCell::new(None); 

#[allow(dead_code)]
static PAGER: Mutex<BespinSlabsProvider> = Mutex::new(BespinSlabsProvider::new());

#[allow(dead_code)]
pub struct SafeZoneAllocator(Mutex<ZoneAllocator<'static>>);

impl SafeZoneAllocator {
    pub const fn new(provider: &'static Mutex<PageProvider>) -> SafeZoneAllocator {
        SafeZoneAllocator(Mutex::new(ZoneAllocator::new(provider)))
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

unsafe impl GlobalAlloc for SafeZoneAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        println!("alloc layout={:?}", layout);
        if layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE {
            let ptr = self.0.lock().allocate(layout);
            println!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
            ptr
        } else {
            let mut ptr = core::ptr::null_mut();

            if let Some(ref mut fmanager) = *BUDDY.lock() {
                let mut f = fmanager.allocate(layout);
                ptr = f.map_or(core::ptr::null_mut(), |mut region| {
                    region.zero();
                    region.kernel_vaddr().as_mut_ptr()
                });
                println!("allocated ptr=0x{:x} layout={:?}", ptr as usize, layout);
                drop(fmanager);
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        println!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
        if layout.size() <= ZoneAllocator::MAX_ALLOC_SIZE {
            //debug!("dealloc ptr = 0x{:x} layout={:?}", ptr as usize, layout);
            self.0.lock().deallocate(ptr, layout);
        } else {
            use arch::memory::{kernel_vaddr_to_paddr, VAddr};
            if let Some(ref mut fmanager) = *BUDDY.lock() {
                fmanager.deallocate(
                    memory::Frame::new(
                        kernel_vaddr_to_paddr(VAddr::from_u64(ptr as u64)),
                        layout.size(),
                    ),
                    layout,
                );
            } else {
                panic!("__rust_allocate: buddy not initialized");
            }
        }
    }
}

#[global_allocator]
static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&PAGER);

// Init AP cpus
pub fn init_ap_cpus() {

    // Allocate CPU stack, write it into a global variable 
    // until the CPU is woken up
    unsafe{
        AP_INIT_STACK = alloc::alloc::alloc(
                    Layout::from_size_align_unchecked(KERNEL_STACK_SIZE, 4096)) as *mut usize;
        
        //println!("Allocated stack for the CPU: {:?}", AP_INIT_STACK); 

        let ap_cpu_stack = AP_INIT_STACK as u32; 
    
        //println!("Waking up CPU with stack: {}", ap_cpu_stack);
        interrupt::init_cpu(1, ap_cpu_stack, rust_main_ap as u64);
    }
}

pub fn init_allocator() {
    unsafe {
        println!("multibootv2 tag found at {:x}", _bootinfo as usize);
        let bootinfo = multibootv2::load(_bootinfo);
        println!("Tags: {:?}", bootinfo);
        init_buddy(bootinfo);
            
/*        unsafe {
//                let ptr = 0x12b000 as *mut u32;
//                unsafe { *ptr = 42; }

            let new_region: *mut u8 =
                alloc::alloc::alloc(Layout::from_size_align_unchecked(256, 256));
            println!(" === > {:?}", new_region);
        } */
    }

}

pub extern fn idle() {
    halt(); 
}

pub extern fn hello1() {
    loop {
        println!("hello 1"); 
    }
}

pub extern fn hello2() {
    loop {
        println!("hello 2"); 
    }
}

fn set_current(mut t: Box<Thread>) {
    CURRENT.replace(Some(t)); 
}

//fn get_current_ref() -> &'static mut Option<Box<Thread>> {
//    unsafe{&mut *CURRENT.get()}
//}

fn get_current() -> Option<Box<Thread>> {
    CURRENT.replace(None)
}


fn init_threads() {

    let mut s = SCHED.borrow_mut();

    let mut idle = Box::new(Thread::new("idle", idle));
    let mut t1 = Box::new(Thread::new("hello 1", hello1));
    let mut t2 = Box::new(Thread::new("hello 2", hello2));

    //s.put_thread(idle); 
    s.put_thread(t1);
    s.put_thread(t2);

    // Make idle the current thread
    set_current(idle);
    
}

// Kicked from the timer IRQ
pub fn schedule() {

    println!("Schedule"); 

    let mut s = SCHED.borrow_mut();
    let mut next_thread = match s.next() {
        Some(t) => t,
        None => {
            // Nothing again, current is the only runnable thread, no need to
            // context switch
            println!("No runnable threads");
            return; 
        }

    };

    let mut c = match get_current() {
        Some(t) => t,
        None => { return; } 
    };

    let prev = &mut *c as *mut Thread; 
    let next = &mut *next_thread as *mut Thread; 


    // Make next thread current
    set_current(next_thread); 

    // put the old thread back in the scheduling queue
    s.put_thread(c);

    drop(s); 

    unsafe {
        switch(prev, next);
    }

}


const MAX_CPUS: u32 = 32;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {

    match CpuId::new().get_vendor_info() {
        Some(vendor) => println!("RedLeaf booting (CPU model: {})", vendor.as_string()),
        None => println!("RedLeaf booting on (CPU model: unknown)"),
    }
    
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;

    unsafe {
        // We don't have per-CPU variables yet, init global gdt
        gdt::init_global_gdt();
    }

    // Init IDT mostly so if we get some exceptions in the allocator 
    // we can see nice crash reports
    interrupt::init_idt();

    // Init memory allocator (normal allocation should work after this) 
    init_allocator();

    // Init per-CPU variables
    unsafe {
        tls::init_per_cpu_area(MAX_CPUS);
    }

    // Initialize LAPIC as BSP
    interrupt::init_irqs_local();
    interrupt::init_irqs();

    // Microkernel runs with interrupts disabled
    // we re-enable them on exits
    //x86_64::instructions::interrupts::enable();
     
    // Spin up other CPUs 
    //init_ap_cpus(); 

    rust_main_ap(); 
}

#[no_mangle]
pub extern "C" fn rust_main_ap() -> ! {
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    println!("Initializing CPU#{}", cpu_id); 

    unsafe {
        gdt::init_global_gdt();
        let tcb_offset = tls::init_per_cpu_vars(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();

    if cpu_id != 0 {
        interrupt::init_irqs_local();
    }
     
    println!("cpu{}: Initialized", cpu_id);

    if cpu_id == 0 {
        let ide = drivers::ide::IDE::new();
        println!("Initializing IDE");
        ide.init();
        println!("IDE Initialized!");

        // Write a block of 5s
        let data: [u32; 512] = [5u32; 512];
        ide.write(20, &data);
        println!("Data written");

        // Read the block back
        let mut rdata: [u32; 512] = [0u32; 512];
        ide.read(20, &mut rdata);
        println!("Data read");
    }

    init_threads(); 
    
    println!("Ready to enable interrupts");

    // Enable interrupts and the timer will schedule the next thread
    x86_64::instructions::interrupts::enable();

    halt(); 
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
