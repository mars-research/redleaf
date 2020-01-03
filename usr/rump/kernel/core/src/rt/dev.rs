use super::{c_int, c_uint, c_ulong, c_void};

use alloc::boxed::Box;
use core::alloc::Layout;
use core::cell::RefMut;
use core::fmt;
use core::ptr;

use log::trace;
use x86::current::paging::{PAddr, VAddr};
use x86::io;

use log::{error, info, warn};

static PCI_CONF_ADDR: u16 = 0xcf8;
static PCI_CONF_DATA: u16 = 0xcfc;

#[inline]
fn pci_bus_address(bus: u32, dev: u32, fun: u32, reg: i32) -> u32 {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_iospace_init() -> c_int {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_confread(
    bus: c_uint,
    dev: c_uint,
    fun: c_uint,
    reg: c_int,
    value: *mut c_uint,
) -> c_int {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_confwrite(
    bus: c_uint,
    dev: c_uint,
    fun: c_uint,
    reg: c_int,
    value: c_uint,
) -> c_int {
    unimplemented!()
}

#[derive(Debug, Copy, Clone)]
struct RumpIRQ {
    tuple: (c_uint, c_uint, c_uint),
    vector: c_int,
    cookie: c_uint,
    handler: Option<unsafe extern "C" fn(arg: *mut c_void) -> c_int>,
    arg: *mut c_void,
}

static mut IRQS: [RumpIRQ; 32] = [RumpIRQ {
    tuple: (0, 0, 0),
    vector: 0,
    cookie: 0,
    handler: None,
    arg: ptr::null_mut(),
}; 32];

//int rumpcomp_pci_irq_map(unsigned bus, unsigned device, unsigned fun, int intrline, unsigned cookie)
#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_irq_map(
    bus: c_uint,
    dev: c_uint,
    fun: c_uint,
    vector: c_int,
    cookie: c_uint,
) -> c_int {
    unimplemented!()
}

#[allow(unused)]
pub unsafe extern "C" fn irq_handler(_arg1: *mut u8) -> *mut u8 {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_irq_establish(
    cookie: c_uint,
    handler: Option<unsafe extern "C" fn(arg: *mut c_void) -> c_int>,
    arg: *mut c_void,
) -> *mut c_void {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_map(addr: c_ulong, len: c_ulong) -> *mut c_void {
    unimplemented!()
}

// Return PAddr for VAddr
#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_virt_to_mach(vaddr: *mut c_void) -> c_ulong {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_dmalloc(
    size: usize,
    alignment: usize,
    pptr: *mut c_ulong,
    vptr: *mut c_ulong,
) -> c_int {
    unimplemented!()
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_dmafree(addr: c_ulong, size: usize) {
    error!("rumpcomp_pci_dmafree {:#x} {:#x}", addr, size);
    unimplemented!()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct rumpcomp_pci_dmaseg {
    pub ds_pa: c_ulong,
    pub ds_len: c_ulong,
    pub ds_vacookie: c_ulong,
}

impl fmt::Debug for rumpcomp_pci_dmaseg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "rumpcomp_pci_dmaseg {{ ds_pa: {:#x}, ds_len: {}, ds_vacookie: {:#x} }}",
            self.ds_pa, self.ds_len, self.ds_vacookie
        )
    }
}

#[no_mangle]
pub unsafe extern "C" fn rumpcomp_pci_dmamem_map(
    dss: *mut rumpcomp_pci_dmaseg,
    nseg: usize,
    totlen: usize,
    vap: *mut *mut c_void,
) -> c_int {
    unimplemented!()
}
