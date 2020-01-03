use elfloader::ElfBinary;
use super::Domain;
use syscalls::{Syscall, BDev, PCI, VFS, PciResource, Net, PciBar};
use crate::syscalls::{PDomain, Interrupt};
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use alloc::boxed::Box; 
//use syscalls::BootSyscall;
//use crate::domain::domain::BOOTING_DOMAIN; 
//use crate::syscalls::BOOT_SYSCALL; 


pub fn create_domain_init() -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_sys_init_build_init_start();
        fn _binary_sys_init_build_init_end();
    }

    let binary_range = (
        _binary_sys_init_build_init_start as *const u8,
        _binary_sys_init_build_init_end as *const u8
    );

    return build_domain_init("sys_init", binary_range);
}

pub fn create_domain_pci(pci_resource: Box<dyn PciResource>,
                         pci_bar: Box<dyn PciBar>) -> (Box<dyn syscalls::Domain>,
                                                       Box<dyn PCI>) {

    extern "C" {
        fn _binary_sys_dev_pci_build_pci_start();
        fn _binary_sys_dev_pci_build_pci_end();
    }

    let binary_range = (
        _binary_sys_dev_pci_build_pci_start as *const u8,
        _binary_sys_dev_pci_build_pci_end as *const u8
    );

    create_domain_pci_bus("pci", binary_range, pci_resource, pci_bar)
}

pub fn create_domain_ahci(pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn BDev>) {

    extern "C" {
        fn _binary_sys_dev_ahci_build_ahci_start();
        fn _binary_sys_dev_ahci_build_ahci_end();
    }

    let binary_range = (
        _binary_sys_dev_ahci_build_ahci_start as *const u8,
        _binary_sys_dev_ahci_build_ahci_end as *const u8
    );

    create_domain_bdev("ahci", binary_range, pci)
}

pub fn create_domain_ixgbe(pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn Net>) {

    extern "C" {
        fn _binary_sys_dev_ixgbe_driver_build_ixgbe_driver_start();
        fn _binary_sys_dev_ixgbe_driver_build_ixgbe_driver_end();
    }

    let binary_range = (
        _binary_sys_dev_ixgbe_driver_build_ixgbe_driver_start as *const u8,
        _binary_sys_dev_ixgbe_driver_build_ixgbe_driver_end as *const u8
    );

    create_domain_net("ixgbe_driver", binary_range, pci)
}

pub fn create_domain_xv6kernel(ints: Box<dyn syscalls::Interrupt>,
                               create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
                               create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
                               bdev: Box<dyn BDev>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_start();
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_end();
    }

    let binary_range = (
        _binary_usr_xv6_kernel_core_build_xv6kernel_start as *const u8,
        _binary_usr_xv6_kernel_core_build_xv6kernel_end as *const u8
    );

    build_domain_xv6kernel("xv6kernel", binary_range, ints, 
                            create_xv6fs, create_xv6usr, bdev)
}

pub fn create_domain_rumpkernel() -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_usr_rump_kernel_core_build_rumprt_start();
        fn _binary_usr_rump_kernel_core_build_rumprt_end();
    }

    let binary_range = (
        _binary_usr_rump_kernel_core_build_rumprt_start as *const u8,
        _binary_usr_rump_kernel_core_build_rumprt_end as *const u8
    );

    build_domain_rumpkernel("rumpkernel", binary_range)
}

pub fn create_domain_xv6fs(bdev: Box<dyn BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn VFS>) {

    extern "C" {
        fn _binary_usr_xv6_kernel_fs_build_xv6fs_start();
        fn _binary_usr_xv6_kernel_fs_build_xv6fs_end();
    }

    let binary_range = (
        _binary_usr_xv6_kernel_fs_build_xv6fs_start as *const u8,
        _binary_usr_xv6_kernel_fs_build_xv6fs_end as *const u8
    );

    build_domain_fs("xv6fs", binary_range, bdev)
}

 
// AB: We have to split ukern syscalls into some that are 
// accessible to xv6 user, e.g., memory management, and the rest 
// which is hidden, e.g., create_thread, etc.
pub fn create_domain_xv6usr(name: &str, xv6: Box<dyn syscalls::Xv6>) -> Box<dyn syscalls::Domain> {

    let binary_range = match name {
        "shell" => {
            extern "C" {
                fn _binary_usr_xv6_usr_shell_build_shell_start();
                fn _binary_usr_xv6_usr_shell_build_shell_end();
            }

            let binary_range = (
                _binary_usr_xv6_usr_shell_build_shell_start as *const u8,
                _binary_usr_xv6_usr_shell_build_shell_end as *const u8
            );
            binary_range
        },
        _ => {
            (0 as *const u8, 0 as *const u8)
        }
    };

    build_domain_xv6usr(name, binary_range, xv6)
}


// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall 
//                    + syscalls::CreateXv6 + syscalls::CreateXv6FS /* + CreateXv6User */
//                    + syscalls::CreatePCI + syscalls::CreateAHCI + Send + Sync>) 
// See
//   rustc --explain E0225
//
// We have to re-write in an ugly way

pub fn build_domain_init(name: &str, 
                            binary_range: (*const u8, *const u8)) 
                                -> Box<dyn syscalls::Domain> 
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, 
                         Box<dyn syscalls::Interrupt>,
                         Box<dyn syscalls::CreateXv6>,
                         Box<dyn syscalls::CreateXv6FS>,
                         Box<dyn syscalls::CreateXv6Usr>,
                         Box<dyn syscalls::CreatePCI>,
                         Box<dyn syscalls::CreateIxgbe>,
                         Box<dyn syscalls::CreateAHCI>,
                         Box<dyn syscalls::CreateRump>);

    let (dom, entry) = unsafe { 
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(Interrupt::new()),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom))),
            Box::new(PDomain::new(Arc::clone(&dom)))); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}


pub fn create_domain_pci_bus(name: &str, 
                                binary_range: (*const u8, *const u8),
                                pci_resource: Box<dyn PciResource>,
                                pci_bar: Box<dyn PciBar>)
                            -> (Box<dyn syscalls::Domain>, Box<dyn PCI>) 
{
    type UserInit = fn(Box<dyn Syscall>,
                        Box<dyn PciResource>,
                        Box<dyn PciBar>) -> Box<dyn PCI>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let pci = user_ep(pdom, pci_resource, pci_bar);
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), pci)
}


pub fn create_domain_bdev(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn BDev>) {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn PCI>) -> Box<dyn BDev>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pci); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)     
}

pub fn create_domain_net(name: &str,
                                 binary_range: (*const u8, *const u8),
                                 pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn Net>) {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn PCI>) -> Box<dyn Net>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let net = user_ep(pdom, pci);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), net)
}

pub fn build_domain_fs(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 bdev: Box<dyn BDev>) -> (Box<dyn syscalls::Domain>, Box<dyn VFS>) 
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn BDev>) -> Box<dyn VFS>;
    
    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let vfs = user_ep(pdom, bdev); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), vfs)     
}

pub fn build_domain_xv6kernel(name: &str, 
                                 binary_range: (*const u8, *const u8),
                                 ints: Box<dyn syscalls::Interrupt>,
                                 create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
                                 create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
                                 bdev: Box<dyn BDev>) -> Box<dyn syscalls::Domain> 
{
    type UserInit = fn(Box<dyn Syscall>,
                       Box<dyn syscalls::Interrupt>,
                       Box<dyn syscalls::CreateXv6FS>,
                       Box<dyn syscalls::CreateXv6Usr>,
                       Box<dyn BDev>);
    
    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe{
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, ints, create_xv6fs, create_xv6usr, bdev); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_rumpkernel(name: &str, 
    binary_range: (*const u8, *const u8)) -> Box<dyn syscalls::Domain> 
{
    type UserInit = fn(Box<dyn Syscall>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe{
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_xv6usr(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 xv6: Box<dyn syscalls::Xv6>) -> Box<dyn syscalls::Domain> 
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn syscalls::Xv6>);
    
    let (dom, entry) = unsafe { 
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, xv6); 
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
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

    println!("domain/{}: .text starts at {:x}", 
        name, loader.offset + domain_elf.file.find_section_by_name(".text").unwrap().address());

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

