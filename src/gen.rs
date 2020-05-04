use syscalls;
use create;
use proxy;
use usr;

use crate::domain::load_domain;
use crate::syscalls::{PDomain, Interrupt, Mmap};
use crate::heap::PHeap;
use crate::interrupt::{disable_irq, enable_irq};

use spin::Mutex;
use alloc::sync::Arc;
use alloc::boxed::Box;

impl create::CreatePCI for PDomain {
    fn create_domain_pci(&self)
                         -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::PCI>) {
        disable_irq();
        let r = create_domain_pci();
        enable_irq();
        r
    }
}

impl create::CreateAHCI for PDomain {
    fn create_domain_ahci(&self,
                          pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {
        disable_irq();
        let r = create_domain_ahci(pci);
        enable_irq();
        r
    }
}

impl create::CreateMemBDev for PDomain {
    fn create_domain_membdev(&self) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {
        disable_irq();
        let r = create_domain_membdev();
        enable_irq();
        r
    }
}

impl create::CreateIxgbe for PDomain {
    fn create_domain_ixgbe(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::Net>) {
        disable_irq();
        let r = create_domain_ixgbe(pci);
        enable_irq();
        r
    }
}

impl create::CreateXv6 for PDomain {
    fn create_domain_xv6kernel(&self,
                               ints: Box<dyn syscalls::Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn usr::bdev::BDev + Send + Sync>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_xv6kernel(ints,
                                                                      create_xv6fs,
                                                                      create_xv6usr,
                                                                      bdev);
        enable_irq();
        r
    }
}

impl create::CreateXv6FS for PDomain {
    fn create_domain_xv6fs(&self, bdev: Box<dyn usr::bdev::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS + Send>) {
        disable_irq();
        let r = create_domain_xv6fs(bdev);
        enable_irq();
        r
    }
}

impl create::CreateXv6Usr for PDomain {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>, &'static str> {
        disable_irq();
        let r = create_domain_xv6usr(name, xv6, blob, args);
        enable_irq();
        r
    }
}

impl create::CreateDomA for PDomain {
    fn create_domain_dom_a(&self) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>) {
        disable_irq();
        let r = create_domain_dom_a();
        enable_irq();
        r
    }
}

impl create::CreateDomB for PDomain {
    fn create_domain_dom_b(&self, dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = create_domain_dom_b(dom_a);
        enable_irq();
        r
    }
}

impl proxy::CreateProxy for PDomain {
    fn create_domain_proxy(
        &self,
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_membdev: Arc<dyn create::CreateMemBDev>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>,
        create_dom_a: Arc<dyn create::CreateDomA>,
        create_dom_b: Arc<dyn create::CreateDomB>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
        disable_irq();
        let r = create_domain_proxy(
            create_pci,
            create_ahci,
            create_membdev,
            create_ixgbe,
            create_xv6fs,
            create_xv6usr,
            create_xv6,
            create_dom_a,
            create_dom_b);
        enable_irq();
        r
    }
}

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

pub fn create_domain_pci() -> (Box<dyn syscalls::Domain>,
                               Box<dyn syscalls::PCI>) {

    extern "C" {
        fn _binary_sys_driver_pci_build_pci_start();
        fn _binary_sys_driver_pci_build_pci_end();
    }

    let binary_range = (
        _binary_sys_driver_pci_build_pci_start as *const u8,
        _binary_sys_driver_pci_build_pci_end as *const u8
    );

    create_domain_pci_bus("pci", binary_range)
}

pub fn create_domain_ahci(pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {

    // extern "C" {
    //     fn _binary_sys_dev_ahci_driver_build_ahci_driver_start();
    //     fn _binary_sys_dev_ahci_driver_build_ahci_driver_end();
    // }

    // let binary_range = (
    //     _binary_sys_dev_ahci_driver_build_ahci_driver_start as *const u8,
    //     _binary_sys_dev_ahci_driver_build_ahci_driver_end as *const u8
    // );

    // create_domain_bdev("ahci", binary_range, pci)
    unimplemented!()
}

pub fn create_domain_ixgbe(pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::Net>) {

    extern "C" {
        fn _binary_sys_driver_ixgbe_build_ixgbe_start();
        fn _binary_sys_driver_ixgbe_build_ixgbe_end();
    }

    let binary_range = (
        _binary_sys_driver_ixgbe_build_ixgbe_start as *const u8,
        _binary_sys_driver_ixgbe_build_ixgbe_end as *const u8
    );

    create_domain_net("ixgbe_driver", binary_range, pci)
}

pub fn create_domain_membdev() -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {

    extern "C" {
        fn _binary_sys_driver_membdev_build_membdev_start();
        fn _binary_sys_driver_membdev_build_membdev_end();
    }

    let binary_range = (
        _binary_sys_driver_membdev_build_membdev_start as *const u8,
        _binary_sys_driver_membdev_build_membdev_end as *const u8
    );

    create_domain_bdev_mem("membdev", binary_range)
}

pub fn create_domain_xv6kernel(ints: Box<dyn syscalls::Interrupt>,
                               create_xv6fs: Arc<dyn create::CreateXv6FS>,
                               create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                               bdev: Box<dyn usr::bdev::BDev + Send + Sync>) -> Box<dyn syscalls::Domain> {
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

pub fn create_domain_xv6fs(bdev: Box<dyn usr::bdev::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS + Send>) {

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
pub fn create_domain_xv6usr(name: &str, xv6: Box<dyn usr::xv6::Xv6>, blob: &[u8], args: &str) -> Result<Box<dyn syscalls::Domain>, &'static str> {
    // TODO: verify that the blob is signed
    // if !signed(blob) return Err("Not signed")

    Ok(build_domain_xv6usr(name, xv6, blob, args))
}

pub fn create_domain_dom_a() -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>) {
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

pub fn create_domain_dom_b(dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
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
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
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
        create_membdev,
        create_ixgbe,
        create_xv6fs,
        create_xv6usr,
        create_xv6,
        create_dom_a,
        create_dom_b)
}

pub fn create_domain_pci_bus(name: &str,
                             binary_range: (*const u8, *const u8))
                             -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::PCI>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>,
                       Box<dyn syscalls::Mmap>,
                       Box<dyn syscalls::Heap>,
    ) -> Box<dyn syscalls::PCI>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let mmap = Box::new(Mmap::new());
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let pci = user_ep(pdom, mmap, pheap);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), pci)
}


pub fn create_domain_bdev(name: &str,
                          binary_range: (*const u8, *const u8),
                          pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn syscalls::PCI>) -> Box<dyn usr::bdev::BDev + Send + Sync>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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

pub fn create_domain_bdev_mem(name: &str,
                              binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn usr::bdev::BDev + Send + Sync>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn usr::bdev::BDev + Send + Sync>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let bdev = user_ep(pdom, pheap);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), bdev)
}

pub fn create_domain_net(name: &str,
                         binary_range: (*const u8, *const u8),
                         pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::Net>) {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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
                       Arc<dyn create::CreateAHCI>,
                       Arc<dyn create::CreateMemBDev>,
                       Arc<dyn create::CreateDomA>,
                       Arc<dyn create::CreateDomB>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))),
            Arc::new(PDomain::new(Arc::clone(&dom))));
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_fs(
    name: &str,
    binary_range: (*const u8, *const u8),
    bdev: Box<dyn usr::bdev::BDev>) -> (Box<dyn syscalls::Domain>, Box<dyn usr::vfs::VFS + Send>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::bdev::BDev>) -> Box<dyn usr::vfs::VFS + Send>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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
    create_membdev: Arc<dyn create::CreateMemBDev>,
    create_ixgbe: Arc<dyn create::CreateIxgbe>,
    create_xv6fs: Arc<dyn create::CreateXv6FS>,
    create_xv6usr: Arc<dyn create::CreateXv6Usr>,
    create_xv6: Arc<dyn create::CreateXv6>,
    create_dom_a: Arc<dyn create::CreateDomA>,
    create_dom_b: Arc<dyn create::CreateDomB>) -> (Box<dyn syscalls::Domain>, Arc<dyn proxy::Proxy>) {
    type UserInit = fn(
        Box<dyn syscalls::Syscall>,
        create_pci: Arc<dyn create::CreatePCI>,
        create_ahci: Arc<dyn create::CreateAHCI>,
        create_membdev: Arc<dyn create::CreateMemBDev>,
        create_ixgbe: Arc<dyn create::CreateIxgbe>,
        create_xv6fs: Arc<dyn create::CreateXv6FS>,
        create_xv6usr: Arc<dyn create::CreateXv6Usr>,
        create_xv6: Arc<dyn create::CreateXv6>,
        create_dom_a: Arc<dyn create::CreateDomA>,
        create_dom_b: Arc<dyn create::CreateDomB>) -> Arc<dyn proxy::Proxy>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let proxy = user_ep(
        pdom,
        create_pci,
        create_ahci,
        create_membdev,
        create_ixgbe,
        create_xv6fs,
        create_xv6usr,
        create_xv6,
        create_dom_a,
        create_dom_b);
    disable_irq();

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), proxy)
}

pub fn build_domain_xv6kernel(name: &str,
                              binary_range: (*const u8, *const u8),
                              ints: Box<dyn syscalls::Interrupt>,
                              create_xv6fs: Arc<dyn create::CreateXv6FS>,
                              create_xv6usr: Arc<dyn create::CreateXv6Usr + Send + Sync>,
                              bdev: Box<dyn usr::bdev::BDev + Send + Sync>) -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn syscalls::Syscall>,
                       Box<dyn syscalls::Heap>,
                       Box<dyn syscalls::Interrupt>,
                       Arc<dyn create::CreateXv6FS>,
                       Arc<dyn create::CreateXv6Usr>,
                       Box<dyn usr::bdev::BDev + Send + Sync>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe{
        core::mem::transmute::<*const(), UserInit>(entry)
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
                           xv6: Box<dyn usr::xv6::Xv6>,
                           blob: &[u8],
                           args: &str) -> Box<dyn syscalls::Domain>
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::xv6::Xv6>, &str);

    let begin = blob.as_ptr();
    let end = unsafe { begin.offset(blob.len() as isize) };
    let (dom, entry) = unsafe {
        load_domain(name, (begin, end))
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
    };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom, pheap, xv6, args);
    disable_irq();

    println!("domain/{}: returned from entry point with return code", name);
    Box::new(PDomain::new(Arc::clone(&dom)))
}

pub fn build_domain_dom_a(name: &str,
                          binary_range: (*const u8, *const u8)) -> (Box<dyn syscalls::Domain>, Box<dyn usr::dom_a::DomA>)
{
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn usr::dom_a::DomA>;

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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
                          dom_a: Box<dyn usr::dom_a::DomA>) -> Box<dyn syscalls::Domain> {
    type UserInit = fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>, Box<dyn usr::dom_a::DomA>);

    let (dom, entry) = unsafe {
        load_domain(name, binary_range)
    };

    let user_ep: UserInit = unsafe {
        core::mem::transmute::<*const(), UserInit>(entry)
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
