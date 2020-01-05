#![no_std]
#![feature(associated_type_defaults)]

extern crate alloc;
use alloc::boxed::Box;

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_println(&self, s: &str);
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>;
    fn sys_current_thread(&self) -> Box<dyn Thread>;
    fn sys_alloc(&self) -> *mut u8;
    fn sys_free(&self, p: *mut u8);
    fn sys_alloc_huge(&self, sz: u64) -> *mut u8;
    fn sys_free_huge(&self, p: *mut u8);
}

#[derive(Clone,Copy,Debug)]
pub enum ThreadState {
    Runnable = 1,
    Paused = 2,
    Waiting = 3, 
}

/// RedLeaf thread interface
pub trait Thread {
    fn get_id(&self) -> u64;
    fn set_affinity(&self, affinity: u64);
    fn set_priority(&self, prio: u64);
    fn set_state(&self, state: ThreadState);
}

/// RedLeaf PCI bus driver interface
pub trait PCI {
    fn pci_register_driver(&self, pci_driver: &mut dyn pci_driver::PciDriver);// -> bar_regions::BarRegions; 
    /// Boxed trait objects cannot be cloned trivially!
    /// https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/6
    fn pci_clone(&self) -> Box<dyn PCI>;
}

/// Virtual file system interface
/// Currently implemented by xv6 file system
pub trait VFS {
}

/// RedLeaf block device interface
pub trait BDev {
    fn read(self, block: u32, data: &mut [u8; 512]);
    fn write(self, block: u32, data: &[u8; 512]);
}
pub type BDevPtr = Box<dyn BDev>;

/// RedLeaf network interface
pub trait Net {
}

/// RedLeaf Domain interface
pub trait Domain {

}   

/// Xv6 system calls
pub trait Xv6 {
}   

pub trait CreatePCI {
    fn create_domain_pci(&self, pci_resource: Box<dyn PciResource>,
                         pci_bar: Box<dyn PciBar>) -> (Box<dyn Domain>, Box<dyn PCI>);
    fn get_pci_resource(&self) -> Box<dyn PciResource>;
    fn get_pci_bar(&self) -> Box<dyn PciBar>;
}

pub trait CreateAHCI {
    fn create_domain_ahci(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn BDev>);
}

pub trait CreateIxgbe {
    fn create_domain_ixgbe(&self, pci: Box<dyn PCI>) -> (Box<dyn Domain>, Box<dyn Net>);
}

pub trait CreateXv6FS {
    fn create_domain_xv6fs(&self, bdev: Box<dyn BDev>) ->(Box<dyn Domain>, Box<dyn VFS>);
}   

pub trait CreateXv6Usr {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn Xv6>) -> Box<dyn Domain>;
} 

pub trait CreateXv6 {
    fn create_domain_xv6kernel(&self, 
                                ints: Box<dyn Interrupt>,
                                create_xv6fs: Box<dyn CreateXv6FS>, 
                                create_xv6usr: Box<dyn CreateXv6Usr>,
                                bdev: Box<dyn BDev>) -> Box<dyn Domain>;
}   

pub static IRQ_TIMER: u8 = 32; 

pub trait Interrupt {
    // Recieve an interrupt
    fn sys_recv_int(&self, int: u8);
    fn int_clone(&self) -> Box<dyn Interrupt>; 
}

pub trait PciResource {
    fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32;
    fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32);
}

pub trait PciBar {
    fn get_bar_region(&self, base: u64, size: usize,
                            pci_driver: pci_driver::PciDrivers) ->  pci_driver::BarRegions;

}
