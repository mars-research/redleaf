#![no_std]
// #![forbid(unsafe_code)]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use rref::{RRef, RRefDeque};
use alloc::vec::Vec;
use usr::bdev::{BDev, BSIZE};
use usr::rpc::RpcResult;
use create::CreateMemBDev;
use spin::Mutex;

#[derive(Debug)]
struct Stats {
    // Number of restarts
    restart_count: usize,
    // Number of runs that there's no restart
    norestart_count: usize,
    // Cumulative time of api calls when there's a restart
    restart_time: usize,
    // Cumulative time of api cals when there's no-restart
    norestart_time: usize,
    // The time of restart itself, excluding unwinding and retrying
    raw_restart_time: usize,
}

impl Stats {
    fn new() -> Self {
        Self {
            restart_count: 0,
            norestart_count: 0,
            restart_time: 0,
            norestart_time: 0,
            raw_restart_time: 0,
        }
    }
}

impl Drop for Stats {
    fn drop(&mut self) {
        println!("{:?}", self);
    }
}

struct ShadowInternal {
    create: Arc<dyn CreateMemBDev>,
    bdev: Box<dyn BDev>,
    dom: Option<Box<dyn syscalls::Domain>>,
}

impl ShadowInternal {
    unsafe fn new(create: Arc<dyn CreateMemBDev>) -> Self {
        let (dom, bdev) = create.create_domain_membdev(libmembdev::get_memdisk());
        Self {
            create,
            bdev,
            dom: Some(dom),
        }
    }

    unsafe fn restart_bdev(&mut self) {
        let old_domain = self.dom.take().unwrap();
        let (domain, bdev) = self.create.recreate_domain_membdev(old_domain, libmembdev::get_memdisk());
        self.dom = Some(domain); 
        self.bdev = bdev;
    }

    fn read(&mut self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        loop {
            let r = self.bdev.read(block, data);
            if let Err(e) = r {
                println!("bdev.read ncounter error: {:?}; restarting membdev", e);
                unsafe{self.restart_bdev()};

                /* restart invocation on the new domain */
                println!("membdev restarted, retrying bdev.read");
                data = RRef::new([0u8; BSIZE]);
                continue;
            }
            break r;
        }
    }

    fn write(&mut self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        loop {
            let r = self.bdev.write(block, data);
            if let Err(e) = r {
                println!("bdev.write encounter error: {:?}; restarting membdev", e);
                unsafe{self.restart_bdev()};

                /* restart invocation on the new domain */
                println!("membdev restarted, retrying bdev.write");
                continue;
            }
            break r;
        }
    }
}

struct Shadow {
    shadow: Mutex<ShadowInternal>,
}

impl Shadow {
    fn new(create: Arc<dyn CreateMemBDev>) -> Self {
        Self {
            shadow: Mutex::new(unsafe{ShadowInternal::new(create)}),
        }
    }
}

impl BDev for Shadow {
    fn read(&self, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        self.shadow.lock().read(block, data)
        
    }

    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        self.shadow.lock().write(block, data)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, create_bdev: Arc<dyn CreateMemBDev>) -> Box<dyn BDev> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init bdev shadow domain");

    Box::new(Shadow::new(create_bdev))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain bdev shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
