#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use console::println;
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};

use libtime::get_rdtsc as rdtsc;
use interface::rref::{RRef, RRefDeque};
use tls::ThreadLocal;
#[macro_use]
use lazy_static::lazy_static;
use interface::rpc::RpcResult;
struct DomA {}

impl DomA {
    fn new() -> Self {
        Self {}
    }
}

use interface::dom_a::OwnedTest;

impl interface::dom_a::DomA for DomA {
    fn ping_pong(&self, mut buffer: RRef<[u8; 1024]>) -> RpcResult<RRef<[u8; 1024]>> {
        println!("[dom_a]: ping pong");
        for i in 0..buffer.len() {
            buffer[i] *= 2_u8;
        }
        Ok(buffer)
    }

    fn test_owned(&self, mut rref: RRef<OwnedTest>) -> RpcResult<RRef<OwnedTest>> {
        Ok(match rref.owned.take() {
            None => rref,
            Some(mut inner) => {
                *inner += 1;
                rref.owned.replace(inner);
                rref
            }
        })
    }
}

lazy_static! {
    pub static ref COUNTER: ThreadLocal<usize> = ThreadLocal::new(|| 0usize);
}

fn bench_tls() {
    let ops = 10_000_000;

    let start = rdtsc();
    for _ in 0..ops {
        COUNTER.with(|counter| {
            *counter += 1;
        });
    }
    let end = rdtsc();
    println!(
        "ops: {}, delta: {}, delta/ops: {}",
        ops,
        end - start,
        (end - start) / ops
    );
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
) -> Box<dyn interface::dom_a::DomA> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!(
        "In domain A, id: {}",
        libsyscalls::syscalls::sys_get_current_domain_id()
    );

    println!("Bench tls");
    for _ in 0..10 {
        bench_tls();
    }

    Box::new(DomA::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain A panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
