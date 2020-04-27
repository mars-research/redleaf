use elfloader::ElfBinary;
use super::Domain;
use syscalls::{Syscall, Heap, PCI, PciResource, Net, PciBar};
use usr::{bdev::BDev, vfs::VFS, dom_a::DomA};
use crate::syscalls::{PDomain, Interrupt};
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};
use spin::Mutex;
use alloc::sync::Arc; 
use alloc::boxed::Box;
use proxy;
use crate::heap::PHeap;
use super::trusted_binary;
use super::trusted_binary::SignatureCheckResult;
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

pub fn create_domain_ahci(pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn BDev + Send + Sync>) {

    extern "C" {
        fn _binary_sys_dev_ahci_driver_build_ahci_driver_start();
        fn _binary_sys_dev_ahci_driver_build_ahci_driver_end();
    }

    let binary_range = (
        _binary_sys_dev_ahci_driver_build_ahci_driver_start as *const u8,
        _binary_sys_dev_ahci_driver_build_ahci_driver_end as *const u8
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
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr>,
                               bdev: Box<dyn BDev + Send + Sync>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_start();
        fn _binary_usr_xv6_kernel_core_build_xv6kernel_end();
    }

    let binary_range = (
        _binary_usr_xv6_kernel_core_build_xv6kernel_start as *const u8,
        _binary_usr_xv6_kernel_core_build_xv6kernel_end as *const u8
    );

    build_domain_xv6kernel("xv6kernel", binary_range, ints, create_xv6fs, create_xv6usr, bdev)
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
pub fn create_domain_xv6usr(name: &str, xv6: Box<dyn usr::xv6::Xv6>) -> Box<dyn syscalls::Domain> {

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

pub fn create_domain_dom_a() -> (Box<dyn syscalls::Domain>, Box<dyn DomA>) {
    extern "C" {
        fn _binary_usr_test_dom_a_build_dom_a_start();
        fn _binary_usr_test_dom_a_build_dom_a_end();
    }

    let binary_range = (
        _binary_usr_test_dom_a_build_dom_a_start as *const u8,
        _binary_usr_test_dom_a_build_dom_a_end as *const u8
    );

    build_domain_dom_a("dom_a", binary_range)
}

pub fn create_domain_dom_b(dom_a: Box<dyn DomA>) -> Box<dyn syscalls::Domain> {
    extern "C" {
        fn _binary_usr_test_dom_b_build_dom_b_start();
        fn _binary_usr_test_dom_b_build_dom_b_end();
    }

    let binary_range = (
        _binary_usr_test_dom_b_build_dom_b_start as *const u8,
        _binary_usr_test_dom_b_build_dom_b_end as *const u8
    );

    build_domain_dom_b("dom_b", binary_range, dom_a)
}

pub fn create_domain_proxy(
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
    extern "C" {
        fn _binary_usr_proxy_build_dom_proxy_start();
        fn _binary_usr_proxy_build_dom_proxy_end();
    }

    let binary_range = (
        _binary_usr_proxy_build_dom_proxy_start as *const u8,
        _binary_usr_proxy_build_dom_proxy_end as *const u8
    );

    build_domain_proxy(
        "dom_proxy",
        binary_range,
        create_pci,
        create_ahci,
        create_ixgbe,
        create_xv6fs,
        create_xv6usr,
        create_xv6)
}

// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall 
//                    + create::CreateXv6 + create::CreateXv6FS /* + CreateXv6User */
//                    + create::CreatePCI + create::CreateAHCI + Send + Sync>)
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
                         Box<dyn proxy::CreateProxy>,
                         Arc<dyn create::CreateXv6>,
                         Arc<dyn create::CreateXv6FS>,
                         Arc<dyn create::CreateXv6Usr>,
                         Arc<dyn create::CreatePCI>,
                         Arc<dyn create::CreateIxgbe>,
                         Arc<dyn create::CreateAHCI>);

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
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))));
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
                        Box<dyn Heap>,
                        Box<dyn PciResource>,
                        Box<dyn PciBar>) -> Box<dyn PCI>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let pci = user_ep(pdom, pheap, pci_resource, pci_bar);
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), pci)
}


pub fn create_domain_bdev(name: &str, 
                                 binary_range: (*const u8, *const u8),
                                 pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn BDev + Send + Sync>) {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>, Box<dyn PCI>) -> Box<dyn BDev + Send + Sync>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pheap, pci);
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)     
}

pub fn create_domain_net(name: &str,
                                 binary_range: (*const u8, *const u8),
                                 pci: Box<dyn PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn Net>) {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>, Box<dyn PCI>) -> Box<dyn Net>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let net = user_ep(pdom, pheap, pci);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), net)
}

pub fn build_domain_fs(
    name: &str,
    binary_range: (*const u8, *const u8),
    bdev: Box<dyn BDev>) -> (Box<dyn syscalls::Domain>, Box<dyn VFS>)
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>, Box<dyn BDev>) -> Box<dyn VFS>;
    
    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());
    
    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let vfs = user_ep(pdom, pheap, bdev);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), vfs)     
}

pub fn build_domain_proxy(
    name: &str,
    binary_range: (*const u8, *const u8),
    create_pci: Arc<dyn create::CreatePCI>,
    create_ahci: Arc<dyn create::CreateAHCI>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
    type UserInit = fn(
        Box<dyn Syscall>,
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>) -> Arc<dyn proxy::Proxy>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let proxy = user_ep(
        pdom,
        create_pci,
        create_ahci,
        create_ixgbe,
        create_xv6fs,
        create_xv6usr,
        create_xv6);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), proxy)
}

pub fn build_domain_xv6kernel(name: &str, 
                                 binary_range: (*const u8, *const u8),
                                 ints: Box<dyn syscalls::Interrupt>,
                                 create_xv6fs: Arc<dyn create::CreateXv6FS>,
                                 create_xv6usr: Arc<dyn create::CreateXv6Usr>,
                                 bdev: Box<dyn BDev + Send + Sync>) -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn Syscall>,
                       Box<dyn Heap>,
                       Box<dyn syscalls::Interrupt>,
                       Arc<dyn create::CreateXv6FS>,
                       Arc<dyn create::CreateXv6Usr>,
                       Box<dyn BDev + Send + Sync>);
    
    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe{
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, ints, create_xv6fs, create_xv6usr, bdev);
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_xv6usr(name: &str, 
                                 binary_range: (*const u8, *const u8), 
                                 xv6: Box<dyn usr::xv6::Xv6>) -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>, Box<dyn usr::xv6::Xv6>);
    
    let (dom, entry) = unsafe { 
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, xv6);
    disable_irq(); 

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_dom_a(name: &str,
                           binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn DomA>)
{
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>) -> Box<dyn DomA>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let dom_a = user_ep(pdom, pheap);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), dom_a)
}

pub fn build_domain_dom_b(name: &str,
                          binary_range: (*const u8, *const u8),
                          dom_a: Box<dyn DomA>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn Syscall>, Box<dyn Heap>, Box<dyn DomA>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, dom_a);
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
    let binary = core::slice::from_raw_parts(binary_start, num_bytes);
    let domain_elf = ElfBinary::new(name, binary).expect("Invalid ELF file");

    // Verify signature in binary
    // FIXME: Actually enforce this
    match trusted_binary::verify(binary) {
        SignatureCheckResult::Unsigned => {
            println!("domain/{}: Binary is unsigned", name);
        },
        SignatureCheckResult::GoodSignature => {
            println!("domain/{}: Binary has good signature", name);
        },
        SignatureCheckResult::BadSignature => {
            println!("domain/{}: Binary has BAD signature", name);
        }
    }

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
