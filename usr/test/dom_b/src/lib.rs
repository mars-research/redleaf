#![no_std]
extern crate malloc;
extern crate alloc;
use syscalls::{Syscall, Heap};
use libsyscalls;
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use rref::{RRef, RRefDeque};
use usr::dom_a::DomA;
use libtime::get_rdtsc as rdtsc;

fn test_submit_and_poll(dom_a: &mut Box<dyn DomA>) {
    let mut packets = RRefDeque::<[u8; 100], 32>::default();
    let mut reap_queue = RRefDeque::<[u8; 100], 32>::default();
    for i in 0..32 {
        packets.push_back(RRef::<[u8;100]>::new([i;100]));
    }

    let ops = 10_000_000;

    let start = rdtsc();
    let mut packets = Some(packets);
    let mut reap_queue = Some(reap_queue);
    for i in 0..ops {

        // need options as a workaround to destructured assignment
        // https://github.com/rust-lang/rfcs/issues/372
        let (num, mut packets_, mut reap_queue_) = dom_a.tx_submit_and_poll(packets.take().unwrap(), reap_queue.take().unwrap());

        packets.replace(reap_queue_);
        reap_queue.replace(packets_);
    }
    let end = rdtsc();
    println!("ops: {}, delta: {}, delta/ops: {}", ops, end - start, (end - start) / ops);

//    let mut packets = packets.take().unwrap();
//    let mut reap_queue = reap_queue.take().unwrap();
//    for i in 0..32 {
//        if let Some(rref) = packets.pop_front() {
//            drop(rref);
//        }
//        if let Some(rref) = reap_queue.pop_front() {
//            drop(rref);
//        }
//    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_a: Box<dyn DomA>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("In domain B, id: {}", libsyscalls::syscalls::sys_get_current_domain_id());

    {
        println!("rref drop test");
        let rref1 = RRef::new(10usize);
        let rref2 = RRef::new(rref1); // RRef<RRef<usize>>
        println!("dropping rref2, should print drop_t::RRef<RRef<usize>> then drop_t::RRef<usize>");
        drop(rref2);
    }

    let mut dom_a = dom_a;
    test_submit_and_poll(&mut dom_a);
/*
    let mut buffer = RRef::<[u8; 1024]>::new([0;1024]);
    for i in 0..1024 {
        buffer[i] = (i % 256) as u8;
    }
    println!("before pingpong");
    println!("---------------");
    for i in 0..1024 {
        println!("buffer[{}]: {}", i, buffer[i]);
    }
    println!("---------------");
    buffer = dom_a.ping_pong(buffer);
    println!("after pingpong");
    println!("---------------");
    for i in 0..1024 {
        println!("buffer[{}]: {}", i, buffer[i]);
    }
    println!("---------------");
    */
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain B panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
