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

extern "C" {
    fn _binary___________usr_mkfs_build_fs_img_start();
    fn _binary___________usr_mkfs_build_fs_img_end(); 
}

struct ShadowInternal {
    create: Arc<dyn CreateMemBDev>,
    bdev: Box<dyn BDev>,
    dom: Option<Box<dyn syscalls::Domain>>,
}

impl ShadowInternal {
    unsafe fn new(create: Arc<dyn CreateMemBDev>) -> Self {
        let start = _binary___________usr_mkfs_build_fs_img_start;
        let end = _binary___________usr_mkfs_build_fs_img_end;
        let size = end as usize - start as usize;
        let memdisk = core::slice::from_raw_parts_mut(start as *mut u8, size);

        let (dom, bdev) = create.create_domain_membdev(memdisk);
        Self {
            create,
            bdev,
            dom: Some(dom),
        }
    }

    unsafe fn restart_bdev(&mut self) {
        let start = _binary___________usr_mkfs_build_fs_img_start;
        let end = _binary___________usr_mkfs_build_fs_img_end;
        let size = end as usize - start as usize;
        let memdisk = core::slice::from_raw_parts_mut(start as *mut u8, size);

        let old_domain = self.dom.take().unwrap();
        let (domain, bdev) = self.create.recreate_domain_membdev(old_domain, memdisk);
        self.dom = Some(domain); 
        self.bdev = bdev;
    }

    fn read(&mut self, block: u32, mut data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        loop {
            let r = self.bdev.read(block, data);
            if let Err(e) = r {
                println!("Encounter error: {:?}; restarting membdev", e);
                unsafe{self.restart_bdev()};

                /* restart invocation on the new domain */
                println!("membdev restarted, retrying bdev.read");
                data = RRef::new([0u8; BSIZE]);
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
        self.shadow.lock().bdev.write(block, data)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, create_bdev: Arc<dyn CreateMemBDev>) -> Box<dyn BDev> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

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
