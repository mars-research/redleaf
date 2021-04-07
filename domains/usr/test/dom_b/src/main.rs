#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;
use syscalls::{Heap, Syscall};

use alloc::boxed::Box;

use console::println;

use core::panic::PanicInfo;
use libtime::get_rdtsc as rdtsc;
use interface::rref::{RRef, RRefDeque, Owned};
use interface::dom_a::{DomA, OwnedTest};
use core::ops::Deref;

fn test_submit_and_poll(dom_a: &mut Box<dyn DomA>) {
    let mut packets = RRefDeque::<[u8; 100], 32>::default();
    let reap_queue = RRefDeque::<[u8; 100], 32>::default();
    for i in 0..32 {
        packets.push_back(RRef::<[u8; 100]>::new([i; 100]));
    }

    let ops = 10_000_000;

    let start = rdtsc();
    let mut packets = Some(packets);
    let mut reap_queue = Some(reap_queue);
    for _i in 0..ops {
        // need options as a workaround to destructured assignment
        // https://github.com/rust-lang/rfcs/issues/372
        let (_num, packets_, reap_queue_) =
            dom_a.tx_submit_and_poll(packets.take().unwrap(), reap_queue.take().unwrap());

        packets.replace(reap_queue_);
        reap_queue.replace(packets_);
    }
    let end = rdtsc();
    println!(
        "ops: {}, delta: {}, delta/ops: {}",
        ops,
        end - start,
        (end - start) / ops
    );

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
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    dom_a: Box<dyn DomA>,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!(
        "In domain B, id: {}",
        libsyscalls::syscalls::sys_get_current_domain_id()
    );

    // {
    //     println!("rref drop test");
    //     let rref1 = RRef::new(10usize);
    //     let rref2 = RRef::new(rref1); // RRef<RRef<usize>>
    //     println!("dropping rref2, should print drop_t::RRef<RRef<usize>> then drop_t::RRef<usize>");
    //     drop(rref2);
    // }

    {
        println!("RRef::Owned<T> test");
        let mut outer = RRef::new(OwnedTest {
            owned: Owned::new(RRef::new(0))
        });
        outer = dom_a.test_owned(outer);
        assert_eq!(outer.owned.take().as_deref().unwrap(), &1);
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
