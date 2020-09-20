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

    /*for i in 10..30 {
        test_hashmap_with_cap(1 << i);
    }*/

    for i in 12..27 {
        run_bench(i, (i - 2));
    }

    panic!("");
}

fn run_bench(_capacity: usize, _keys: usize) {
    let capacity: usize = 2usize.pow(_capacity as u32);
    let keys = 2usize.pow(_keys as u32);

    let mut ht = Index::with_capacity(capacity);

    let start_tsc = rdtsc();

    for i in 1..(keys+1) {
        ht.insert(i, i);
    }
    let total_tsc = rdtsc() - start_tsc;
    let cycles_per_insert = total_tsc / keys as u64;
    let mil_inserts_per_sec: f64 = 1000f64 / (cycles_per_insert as f64 / 2.6f64);

    println!("{}, {}, {}", _capacity, _keys, total_tsc);
    //indexmap::print_collisions();
    /*
    println!("Keys inserted: {}", keys);
    println!("Total TSC: {}", total_tsc);
    println!("Average cycles per insert: {}", cycles_per_insert);
    println!("Million inserts per second: {}", mil_inserts_per_sec);
    */
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>) {
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
