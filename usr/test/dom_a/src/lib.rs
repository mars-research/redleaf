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
    transmit_buffers: [Option<RRef<[u8; 100]>>; 20],
    transmit_index: usize,
    pass_num: usize,
}

impl DomA {
    fn new() -> Self {
        Self {
            transmit_buffers: Default::default(),
            transmit_index: 0,
            pass_num: 0,
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
        mut packets: RRefDeque<RRef<[u8; 100]>, 32>,
        mut reap_queue: RRefDeque<RRef<[u8; 100]>, 32>) -> (
            usize,
            RRefDeque<RRef<[u8; 100]>, 32>,
            RRefDeque<RRef<[u8; 100]>, 32>
        ) {

        let mut read = 0;

        if self.pass_num % 2 == 0 {
            for i in 0..10 {
                let front: RRef<[u8; 100]> = packets.pop_front().unwrap();
                self.transmit_buffers[self.transmit_index] = Some(front);
                self.transmit_index += 1;
            }
        } else {
            for i in 0..self.transmit_index {
                let buff = match self.transmit_buffers[i].take() {
                    Some(buffer) => buffer,
                    None => break,
                };
                if reap_queue.push_back(buff).is_some() {
                    println!("pushing to full reap_queue");
                }
            }
            self.transmit_index = 0;
            read = 10;
        }

        self.pass_num = self.pass_num.wrapping_add(1);

        return (read, packets, reap_queue)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::dom_a::DomA> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("In domain A");

    Box::new(DomA::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain A panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
