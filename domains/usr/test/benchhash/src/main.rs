#![no_std]
#![no_main]
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

fn test_hashmap() {

    for i in 10..30 {
        test_hashmap_with_cap(1 << i);
    }
    panic!("");
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
