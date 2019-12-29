use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::{VSpace, ResourceType};
use crate::memory::paddr_to_kernel_vaddr;
use x86::bits64::paging::BASE_PAGE_SIZE;
use alloc::boxed::Box; 
use spin::Mutex;
use alloc::sync::Arc; 
use crate::domain::domain::{Domain}; 
use syscalls::{Thread,PciResource};

//use crate::domain::domain::BOOTING_DOMAIN; 

extern crate syscalls; 

macro_rules! round_up {
    ($num:expr, $s:expr) => {
        (($num + $s - 1) / $s) * $s
    };
}

//pub static BOOT_SYSCALL: BootSyscall = BootSyscall {
//    sys_boot_syscall,
//};

//// AB: I was not able to pass Box<dyn Syscall> as an argument 
//// to user_ep() (maybe it's possible, I didn't have time to 
//// figure it out
//pub fn sys_boot_syscall() -> Box<dyn Syscall> {
//    let pdom = BOOTING_DOMAIN.replace(None);
//
//    enable_irq(); 
//    return pdom.unwrap();
//}

pub struct PDomain {
    domain: Arc<Mutex<Domain>>
}

impl PDomain {
    pub const fn new(dom: Arc<Mutex<Domain>>) -> PDomain {
        PDomain {
            domain: dom,
        }
    }
}

impl syscalls::Domain for PDomain { }

impl syscalls::Syscall for PDomain {

    // Print a string 
    fn sys_print(&self, s: &str) {
        disable_irq();
        print!("{}", s);
        enable_irq(); 
    }
    
    // Print a string and a newline
    fn sys_println(&self, s: &str) {
        disable_irq();
        println!("{}", s);
        enable_irq(); 
    }

    fn sys_alloc(&self) -> *mut u8 {
        disable_irq();
        let paddr: PAddr = VSpace::allocate_one_page();
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    fn sys_alloc_huge(&self, sz: u64) -> *mut u8 {
        let how_many = round_up!(sz as usize, BASE_PAGE_SIZE as usize) / BASE_PAGE_SIZE;
        disable_irq();
        let paddr: PAddr = VSpace::allocate_pages(how_many, ResourceType::Memory);
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc_huge: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    // todo: implement free!
    fn sys_free(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // todo: implement free!
    fn sys_free_huge(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // Yield to any thread
    fn sys_yield(&self) {

        disable_irq();
        println!("sys_yield"); 
        do_yield();
        enable_irq(); 
    }

    // Create a new thread
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>  {

        disable_irq();
        println!("sys_create_thread"); 
        let pt = create_thread(name, func);

        let t = pt.thread.clone(); 
    
        let mut d = self.domain.lock();
        d.add_thread(t); 

        println!("Created thread {} for domain {}", pt.thread.borrow().name, d.name); 

        // Drop before re-enabling interrupts
        drop(d); 

        enable_irq();
        return pt;
    }
}

impl syscalls::CreatePCI for PDomain {
    fn create_domain_pci(&self, pci_resource: Box<dyn syscalls::PciResource>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::PCI>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_pci(pci_resource);
        enable_irq();
        r
    }

    fn get_pci_resource(&self) -> Box<dyn PciResource> {
        use crate::dev::pci_resource::PCI_RESOURCE;
        disable_irq();
        let pci_r = Box::new(PCI_RESOURCE);
        enable_irq();
        pci_r
    }
}

impl syscalls::CreateAHCI for PDomain {
    fn create_domain_ahci(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::BDev>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_ahci(pci);
        enable_irq();
        r
    }
}

impl syscalls::CreateIxgbe for PDomain {
    fn create_domain_ixgbe(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::Net>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_ixgbe(pci);
        enable_irq();
        r
    }
}

impl syscalls::CreateXv6 for PDomain {
    fn create_domain_xv6kernel(&self,
                                create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
                                create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
                                bdev: Box<dyn syscalls::BDev>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6kernel(create_xv6fs, create_xv6usr, bdev);
        enable_irq();
        r
    }
}   

impl syscalls::CreateXv6FS for PDomain {
    fn create_domain_xv6fs(&self, bdev: Box<dyn syscalls::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn syscalls::VFS>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6fs(bdev);
        enable_irq();
        r
    }
}   

impl syscalls::CreateXv6Usr for PDomain {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn syscalls::Xv6>) -> Box<dyn syscalls::Domain> 
    {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6usr(name, xv6);
        enable_irq();
        r
    }
} 
