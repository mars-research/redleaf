#![no_std]
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

struct DomA {
}

impl DomA {
    fn new() -> Self {
        Self {
        }
    }
}

impl usr::dom_a::DomA for DomA {
    fn ping_pong(&self, mut buffer: RRef<[u8; 1024]>) -> RRef<[u8; 1024]> {
        println!("[dom_a]: ping pong");
        for i in 0..buffer.len() {
            buffer[i] *= 2 as u8;
        }
        buffer
    }

    fn tx_submit_and_poll(
        &mut self,
        mut packets: RRefDeque<[u8; 100], 32>,
        mut reap_queue: RRefDeque<[u8; 100], 32>) -> (
            usize,
            RRefDeque<[u8; 100], 32>,
            RRefDeque<[u8; 100], 32>
        ) {

        let mut read = 0;

        while let Some(buf) = packets.pop_front() {
            reap_queue.push_back(buf);
            read += 1;
        }

        return (read, packets, reap_queue)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::dom_a::DomA> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("In domain A, id: {}", libsyscalls::syscalls::sys_get_current_domain_id());

    Box::new(DomA::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain A panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
