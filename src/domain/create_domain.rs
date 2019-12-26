use elfloader::ElfBinary;
use super::Domain;
use syscalls::{Syscall, BDev, PCI, VFS};
use crate::syscalls::{PDomain};
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use alloc::boxed::Box; 
//use syscalls::BootSyscall;
//use crate::domain::domain::BOOTING_DOMAIN; 
//use crate::syscalls::BOOT_SYSCALL; 


pub fn create_domain_init() {
    extern "C" {
        fn _binary_sys_init_build_init_start();
        fn _binary_sys_init_build_init_end();
    }

    let binary_range = (
        _binary_sys_init_build_init_start as *const u8,
        _binary_sys_init_build_init_end as *const u8
    );

    unsafe {
        create_domain("sys_init", binary_range);
    }
}

pub fn create_domain_pci() -> Box<dyn PCI> {

    extern "C" {
        fn _binary_sys_dev_pci_build_pci_start();
        fn _binary_sys_dev_pci_build_pci_end();
    }

    let binary_range = (
        _binary_sys_dev_pci_build_pci_start as *const u8,
        _binary_sys_dev_pci_build_pci_end as *const u8
    );

    unsafe {
        create_domain_pci_bus("pci", binary_range)
    }
}

pub fn create_domain_ahci(pci: Box<dyn PCI>) -> Box<dyn BDev> {

    extern "C" {
        fn _binary_sys_dev_ahci_build_ahci_start();
        fn _binary_sys_dev_ahci_build_ahci_end();
    }

    let binary_range = (
        _binary_sys_dev_ahci_build_ahci_start as *const u8,
        _binary_sys_dev_ahci_build_ahci_end as *const u8
    );

    unsafe {
        create_domain_bdev("ahci", binary_range, pci)
    }
}

pub fn create_domain_xv6kernel() {
    extern "C" {
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_start();
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_end();
    }

    let binary_range = (
        _binary_usr_xv6_kernel_core_build_xv6kernel_start as *const u8,
        _binary_usr_xv6_kernel_core_build_xv6kernel_end as *const u8
    );

    unsafe {
        create_domain("xv6kernel", binary_range);
    }
}

pub fn create_domain_xv6fs(bdev: Box<dyn BDev>) -> Box<dyn VFS> {

    extern "C" {
        fn _binary_usr_xv6_kernel_core_build_xv6fs_start();
        fn _binary_usr_xv6_kernel_core_build_xv6fs_end();
    }

    let binary_range = (
        _binary_usr_xv6_kernel_core_build_xv6fs_start as *const u8,
        _binary_usr_xv6_kernel_core_build_xv6fs_end as *const u8
    );

    unsafe {
        create_domain_fs("xv6fs", binary_range, bdev)
    }
}

pub unsafe fn create_domain_pci_bus(name: &str, 
                                binary_range: (*const u8, *const u8)) -> Box<dyn PCI> 
{
    type UserInit = fn(Box<dyn Syscall>) -> Box<dyn PCI>;

    let (dom, entry) = load_domain(name, binary_range);

    let user_ep: UserInit = transmute::<*const(), UserInit>(entry);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let pci = user_ep(pdom); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);

    pci
}


pub unsafe fn create_domain_bdev(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 pci: Box<dyn PCI>) -> Box<dyn BDev> {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn PCI>) -> Box<dyn BDev>;

    let (dom, entry) = load_domain(name, binary_range);

    let user_ep: UserInit = transmute::<*const(), UserInit>(entry);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pci); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);

    bdev
}

pub unsafe fn create_domain_fs(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 bdev: Box<dyn BDev>) -> Box<dyn VFS> 
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn BDev>) -> Box<dyn VFS>;
    
    let (dom, entry) = load_domain(name, binary_range);

    let user_ep: UserInit = transmute::<*const(), UserInit>(entry);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let vfs = user_ep(pdom, bdev); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);

    vfs
}


pub unsafe fn create_domain(name: &str, binary_range: (*const u8, *const u8)) {
    type UserInit = fn(Box<dyn Syscall>);

    let (dom, entry) = load_domain(name, binary_range);

    let user_ep: UserInit = transmute::<*const(), UserInit>(entry);

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
}

pub unsafe fn load_domain(name: &str, binary_range: (*const u8, *const u8)) -> (Arc<Mutex<Domain>>, *const()) {
    let (binary_start, binary_end) = binary_range;

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("domain/{}: Binary start: {:x}, end: {:x} ", 
        name, binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let domain_elf = ElfBinary::new(name, 
                                core::slice::from_raw_parts(binary_start, num_bytes))
                                .expect("Invalid ELF file");

    // Create a domain for the to-be-loaded elf file
    let dom = Arc::new(Mutex::new(Domain::new(name)));

    let mut loader = dom.lock();

    // load the binary
    domain_elf.load(&mut *loader).expect("Cannot load binary");

    // print its entry point for now
    println!("domain/{}: Entry point at {:x}", 
        name, loader.offset + domain_elf.entry_point());

    let user_ep: *const() = {
        let mut entry: *const u8 = (*loader).offset.as_ptr();
        entry = entry.offset(domain_elf.entry_point() as isize);
        let _entry = entry as *const ();
        _entry
    };

    // Drop the lock so if domain starts creating threads we don't
    // deadlock
    drop(loader);

    (dom, user_ep)
}

