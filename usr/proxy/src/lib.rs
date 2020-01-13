#![no_std]

extern crate malloc;
extern crate alloc;
use rref::RRef;
use syscalls;
use libsyscalls;
use syscalls::Syscall;
use alloc::boxed::Box;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;

struct Proxy();

impl Proxy {
    fn new() -> Proxy {
        Proxy {}
    }
}

impl syscalls::Proxy for Proxy {
    fn foo(&self) -> usize {
        let ptr = libsyscalls::heap::sys_heap_alloc(10, Layout::new::<u64>());
        unsafe { *(ptr as *mut u64) = 0xf00; } // 3840
        return ptr as usize;
    }
}

impl Proxy {
    fn rref_example(&self, input: u64) -> RRef<u64> {
        println!("input: {}", input);
        RRef::new(0, input)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn syscalls::Heap + Send + Sync>) -> Box<dyn syscalls::Proxy + Send + Sync> {
    libsyscalls::syscalls::init(s);
    libsyscalls::heap::init(heap);

    println!("entered proxy!");

    let rref = RRef::<u64>::new(0, 10);
    println!("RRef's value: {}", *rref);
    drop(rref);
    println!("Dropped RRef");

    Box::new(Proxy::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
