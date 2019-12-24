use elfloader::ElfBinary;
use super::Domain;
use syscalls::{Syscall};
use crate::syscalls::{PDomain};
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use alloc::boxed::Box; 
use syscalls::BootSyscall;
use crate::domain::domain::BOOTING_DOMAIN; 
use crate::syscalls::BOOT_SYSCALL; 

pub unsafe fn create_domain(name: &'static str, binary_range: (*const u8, *const u8)) {
    let (binary_start, binary_end) = binary_range;
    //type UserInit = fn(BootSyscall);
    type UserInit = fn(Box<dyn Syscall>);

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("domain/{}: Binary start: {:x}, end: {:x} ", name, binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let domain_elf = ElfBinary::new(name, core::slice::from_raw_parts(binary_start, num_bytes)).expect("Invalid ELF file");

    // Create a domain for the to-be-loaded elf file
    let dom = Arc::new(Mutex::new(Domain::new(name)));

    let mut loader = dom.lock();

    // load the binary
    domain_elf.load(&mut *loader).expect("Cannot load binary");

    // print its entry point for now
    println!("domain/{}: Entry point at {:x}", name, loader.offset + domain_elf.entry_point());

    let user_ep: UserInit = {
        let mut entry: *const u8 = (*loader).offset.as_ptr();
        entry = entry.offset(domain_elf.entry_point() as isize);
        let _entry = entry as *const ();
        transmute::<*const(), UserInit>(_entry)
    };

    // Drop the lock so if domain starts creating threads we don't
    // deadlock
    drop(loader);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    //BOOTING_DOMAIN.replace(Some(pdom));

    // We play a little dance with interrupts here. While normally
    // we would enable interrupts as we upcall into user, we can't
    // as we have to ensure atomicity with respect to sys_boot_syscall.
    // We enter user with interrupts off, init immediately calls back into 
    // the kernel with the sys_boot_syscall, which enables interrupts
    // on return to user. 
    //user_ep(BOOT_SYSCALL);
    enable_irq();
    user_ep(pdom); 
    disable_irq(); 

    println!("domain/{}: Returned", name);
}
