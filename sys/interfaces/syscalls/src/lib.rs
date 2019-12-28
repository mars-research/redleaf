#![no_std]

extern crate alloc;
use alloc::boxed::Box;

//#[derive(Copy, Clone)]
//pub struct BootSyscall {
//    pub sys_boot_syscall: fn() -> Box<dyn Syscall>,
//}

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_println(&self, s: &str);
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>;
    fn sys_alloc(&self) -> *mut u8;
    fn sys_free(&self, p: *mut u8);
    fn sys_alloc_huge(&self, sz: u64) -> *mut u8;
    fn sys_free_huge(&self, p: *mut u8);
}

/// RedLeaf thread interface
pub trait Thread {
    fn set_affinity(&self, affinity: u64);
}

/// RedLeaf PCI bus driver interface
pub trait PCI {
}

/// Virtual file system interface
/// Currently implemented by xv6 file system
pub trait VFS {
}

/// RedLeaf block device interface
pub trait BDev {
}

/// RedLeaf Domain interface
pub trait Domain {

}   

/// Xv6 system calls
pub trait Xv6 {
}   

pub trait CreatePCI {
    fn create_domain_pci(&self, pci_resource: Box<dyn PciResource>) -> (Box<dyn Domain>, Box<dyn PCI>);
    fn get_pci_resource(&self) -> Box<dyn PciResource>;
}

pub trait CreateAHCI {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateXv6FS {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>);
}   

pub trait CreateXv6Usr {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn Xv6>) -> Box<dyn Domain>;
} 

pub trait CreateXv6 {
    fn create_domain_xv6kernel(&self, 
                                create_xv6fs: Box<dyn CreateXv6FS>, 
                                create_xv6usr: Box<dyn CreateXv6Usr>,
                                bdev: Box<dyn BDev>) -> Box<dyn Domain>;
}   

pub trait PciResource {
    fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32;
    fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32);
}
