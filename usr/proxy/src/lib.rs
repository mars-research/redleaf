#![no_std]

extern crate malloc;
extern crate alloc;
use rref::RRef;
use syscalls;
use libsyscalls;
use syscalls::Syscall;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[derive(Clone)]
struct Proxy {
    bdev: Arc<Option<Box<dyn usr::bdev::BDev + Send + Sync>>>,
}

impl Proxy {
    fn new(bdev: Arc<Option<Box<dyn usr::bdev::BDev + Send + Sync>>>) -> Proxy {
        Proxy {
            bdev
        }
    }
}

impl usr::proxy::Proxy for Proxy {
    fn proxy_clone(&self) -> Box<dyn usr::proxy::Proxy> {
        // TODO: is this safe? Box is allocated on proxy's heap
        Box::new((*self).clone())
    }

    fn bdev_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]> {
        println!("Called Proxy::new_value");
        // TODO: get domain id
        let rref = RRef::new(0, data);
        println!("Created new value");
        rref
    }
    fn bdev_drop_data(&self, data: RRef<[u8; 512]>) {
        RRef::drop(data);
    }

    fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>) {
        let bdev = self.bdev.as_deref().expect("BDev interface not initialized.");
        bdev.read(block, data)
    }
    fn bdev_write(&self, block: u32, data: &[u8; 512]) {
        let bdev = self.bdev.as_deref().expect("BDev interface not initialized.");
        bdev.write(block, data)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn syscalls::Heap + Send + Sync>,
            bdev: Arc<Option<Box<dyn usr::bdev::BDev + Send + Sync>>>) -> Box<dyn usr::proxy::Proxy + Send + Sync> {
    libsyscalls::syscalls::init(s);
    libsyscalls::heap::init(heap);

    println!("entered proxy!");

    let rref = RRef::<u64>::new(0, 10);
    println!("RRef's value: {}", *rref);
    RRef::drop(rref);
    println!("Dropped RRef");

    Box::new(Proxy::new(bdev))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("proxy panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
