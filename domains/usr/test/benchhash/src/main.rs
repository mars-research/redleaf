#![no_std]
#![no_main]
#![feature(const_int_pow,optimize_attribute)]
#![feature(llvm_asm)]

extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::{println, print};
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use alloc::vec::Vec;
use sashstore_redleaf::SashStore;
use hashbrown::HashMap;
use sashstore_redleaf::indexmap::Index;
use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use core::cell::RefCell;
use fnv::FnvHasher;
use twox_hash::XxHash;
use libtime::get_rdtsc as rdtsc;

const DOMAIN_NAME: &str = "benchhash";
const CACHE_SIZE: usize = 1 << 22;
static mut HIT_COUNT: usize = 0;
static mut HASHMAP_TOTAL: usize = 0;

type FnvHashFactory = BuildHasherDefault<FnvHasher>;
type XxHashFactory = BuildHasherDefault<XxHash>;

struct HashTable {
    // pub cache: RefCell<LruCache<usize, usize>>, // hash -> backend
    // pub cache: RefCell<HashMap<usize, usize, FnvHashFactory>>,
    pub cache: Index<usize, usize>,
}

impl HashTable {
    fn new() -> Self {
        Self {
            cache: Index::with_capacity(CACHE_SIZE),
        }
    }

    fn new_with_capacity(capacity: usize) -> Self {
        Self {
            cache: Index::with_capacity(capacity),
        }
    }

    pub fn dump_stats(&self) {
        unsafe {
            println!("Hits: {}, total: {}", HIT_COUNT, HASHMAP_TOTAL);
        }
        sashstore_redleaf::indexmap::print_stats();
    }
}

fn test_hashmap_with_load(capacity: usize, load: usize) {
    let mut ht = HashTable::new_with_capacity(capacity);

    let load_factor = load as f64 * 0.01;
    let NUM_INSERTIONS = (capacity as f64 * load_factor) as usize;

    println!("======== HT test {{ capacity: {} load factor {} (insertions {}) }}=======", capacity, load_factor, NUM_INSERTIONS);

    let start = rdtsc();
    for i in 0..NUM_INSERTIONS {
        ht.cache.insert(i, i);
    }
    let elapsed = rdtsc() - start;

    println!("{} insertions took {} cycles (avg {})", NUM_INSERTIONS, elapsed, elapsed / NUM_INSERTIONS as u64);

    let start = rdtsc();
    for i in (0..NUM_INSERTIONS) {
        //for _ in 0..10 {
            ht.cache.get(&i);
        //}
    }

    let elapsed = rdtsc() - start;
    println!("{} lookups took {} cycles (avg {})", NUM_INSERTIONS, elapsed, elapsed / NUM_INSERTIONS as u64);
    println!("-----------------------------------------------------------");

}

fn test_hashmap_with_cap(capacity: usize) {
    for load in (10..=70).step_by(10) {
        test_hashmap_with_load(capacity, load);
    }
}

fn test_serial(size_mb: usize) {
    let capacity = (1 << 20) * size_mb;

    let layout = Layout::from_size_align(capacity, 4096)
        .map_err(|e| panic!("Layout error: {}", e)).unwrap();

    let buf = unsafe {alloc::alloc::alloc(layout) as *mut u8 };
    let buf2 = buf as *mut _ as u64;
    let mut v: Vec<u8> = unsafe { Vec::from_raw_parts(buf, capacity, capacity) };

    for i in 0..capacity {
        v[i] = 0;
    }

    let start = unsafe { core::arch::x86_64::_rdtsc() };

    unsafe {
        llvm_asm!(
            "xor %rax, %rax
            1: mov %al, (%rcx, %rax)
            inc %rax
            cmp %rax, %rbx
            jne 1b"
            :: "{rcx}"(buf2), "{rbx}"(capacity) : : "volatile");
    }

    /*for i in 0..capacity {
        v[i] = i as u8;
    }*/

    let diff = unsafe { core::arch::x86_64::_rdtsc() } - start;

    println!("test_serial: size {} MB tsc {}", size_mb, diff);
}

static mut seed: u64 = 123456789;
const pow: u64 = 2u64.pow(31);

#[no_mangle]
pub extern "C" fn get_rand(cap: u64) -> u64 {
    unsafe {
        seed = (1103515245 * seed + 12345) % pow;
        seed & (cap - 1)
    }
}


fn test_random(size_mb: usize) {
    let capacity = (1 << 20) * size_mb;

    let layout = Layout::from_size_align(capacity, 4096)
        .map_err(|e| panic!("Layout error: {}", e)).unwrap();

    let buf = unsafe {alloc::alloc::alloc(layout) as *mut u8 };
    let buf2 = buf as *mut _ as u64;

    let mut v: Vec<u8> = unsafe { Vec::from_raw_parts(buf, capacity, capacity) };

    for i in 0..capacity {
        v[i] = 0;
    }

    let start = unsafe { core::arch::x86_64::_rdtsc() };

    /*for i in 0..capacity {
        let idx = get_rand(capacity as u64) as usize;
        v[idx] = idx as u8;
    }*/

    // r12 - old seed
    // r13 - new seed
    // r14 - temp cap (for downsizing)
    unsafe {
        llvm_asm!(
            "xor %rdx, %rdx
            mov $$123456789, %r12
        1:  imul $$0x41c64e6d, %r12, %r13
            add $$0x3039, %r13
            and $$0x7fffffff,%r13
            mov %r13, %r12
            mov %rdi, %r14
            dec %r14
            and %r14, %r13
            mov %r13, %rax
            mov %al, (%rcx, %rax)
            inc %rdx
            cmp %rdx, %rdi
            jne 1b"
            :: "{rcx}"(buf2), "{rdi}"(capacity)
            : "rdx", "rax", "r12", "r13", "r14" : "volatile");
    }

    let diff = unsafe { core::arch::x86_64::_rdtsc() } - start;

    println!("test_random: size {} MB, tsc {}", size_mb, diff);
}

fn test_hashmap() {

    /*for i in 10..30 {
        test_hashmap_with_cap(1 << i);
    }*/

    for i in 12..27 {
        run_bench(i, (i - 2));
    }
        /*
       println!("===> start");
        let start = unsafe { core::arch::x86_64::_rdtsc()} ;
        let mut sum: u64 = 0;
        for i in 0..(1u64 << 35) {
            if i % 2 == 0 {
                sum = sum.wrapping_add(i);
            } else {
                sum = sum.wrapping_sub(i);
            }
        }
        let delta = unsafe {core::arch::x86_64::_rdtsc() - start};
        println!("===> end");
     println!("delta {} sum {}", delta, sum); */
    /*
    for i in 2..12 {
        test_serial(1 << i);
    }

    for i in 2..12 {
        test_random(1 << i);
    }*/

    panic!("");
}

fn run_bench(_capacity: usize, _keys: usize) {
    let capacity: usize = 2usize.pow(_capacity as u32);
    let keys = 2usize.pow(_keys as u32);
    let gets = keys;

    let mut ht = Index::with_capacity(capacity);

    let start_tsc_insert = rdtsc();

    for i in 1..(keys+1) {
        ht.insert(i, i);
    }
    let total_tsc_insert = rdtsc() - start_tsc_insert;
    let cycles_per_insert = total_tsc_insert / keys as u64;
    let mil_inserts_per_sec: f64 = 1000f64 / (cycles_per_insert as f64 / 2.6f64);

    let mut sum = 0usize;
    let start_tsc_lookup = rdtsc();

    for i in 1..(gets+1) {
        match ht.get(&i) {
            Some(val) => {  sum += *val; },
            None => println!("key {} not found", i),
        }
    }

    let total_tsc_lookup = rdtsc() - start_tsc_lookup;
    let cycles_per_lookup = total_tsc_lookup / gets as u64;

    println!("{}, {}, {}, {}, {}, {}, *{}*", _capacity, _keys, total_tsc_insert, cycles_per_insert, total_tsc_lookup, cycles_per_lookup, sum);
    //indexmap::print_collisions();
    /*
    println!("Keys inserted: {}", keys);
    println!("Total TSC: {}", total_tsc);
    println!("Average cycles per insert: {}", cycles_per_insert);
    println!("Million inserts per second: {}", mil_inserts_per_sec);
    */
}

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Initalizing domain: {}", DOMAIN_NAME);

    test_hashmap();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain {} panic: {:?}", DOMAIN_NAME, info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
