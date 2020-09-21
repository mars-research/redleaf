use elfloader::ElfBinary;
use super::Domain;
use syscalls::Syscall;
use crate::syscalls::{PDomain};
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use alloc::boxed::Box; 
//use syscalls::BootSyscall;
//use crate::domain::domain::BOOTING_DOMAIN; 
//use crate::syscalls::BOOT_SYSCALL; 

fn sys_init_binary_range() -> (u64, u64) {
    extern "C" {
        static _binary_sys_init_build_init_start: u8;
        static _binary_sys_init_build_init_end: u8;
    }

    unsafe {
        (
            & _binary_sys_init_build_init_start as *const _ as u64,
            & _binary_sys_init_build_init_end as *const _ as u64
        )
    }
}

pub unsafe fn load_sys_init() {
    let (binary_start, binary_end) = sys_init_binary_range();
    let binary_start: *const u8 = binary_start as *const u8;
    let binary_end: *const u8 = binary_end as *const u8;
    //type UserInit = fn(BootSyscall);
    type UserInit = fn(Box<dyn Syscall>);

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("start: {:x} end : {:x} ", binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let sys_init_elf = ElfBinary::new("sys_init", core::slice::from_raw_parts(binary_start, num_bytes)).expect("Got ELF file");

    // Create a domain for the to-be-loaded elf file
    let dom = Arc::new(Mutex::new(Domain::new("sys_init")));

    // Create a domain for the to-be-loaded elf file
    let mut loader = dom.lock();

    // load the binary
    sys_init_elf.load(&mut *loader).expect("Cannot load binary");

    // print its entry point for now
    println!("entry point at {:x}", loader.offset + sys_init_elf.entry_point());

    let user_ep: UserInit = {
        let mut entry: *const u8 = (*loader).offset.as_ptr();
        entry = entry.offset(sys_init_elf.entry_point() as isize);
        let _entry = entry as *const ();
        transmute::<*const(), UserInit>(_entry)
    };

    // Drop the lock so if domain starts creating threads we don't
    // deadlock
    drop(loader);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    //BOOTING_DOMAIN.replace(Some(pdom));
    //user_ep(BOOT_SYSCALL);

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom);
    disable_irq(); 

    println!("Hello back");
}
