#![no_std]
#![no_main]
extern crate alloc;
use alloc::boxed::Box;
use core::panic::PanicInfo;
use usrlib::println;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    rv6: Box<dyn interface::rv6::Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone_rv6().unwrap());

    usrlib::benchnvme::main(rv6, args)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnvme panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
