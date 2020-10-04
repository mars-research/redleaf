//! A very simple but safe hashmap
//! This code is more or less a direct reuse of https://github.com/HarmedChronogram/CIndex
//! which was originally written by https://github.com/HarmedChronogram
//! It does not have a license file.

#![allow(unused)] // For now

pub mod hash;

use self::hash::*;

use core::fmt;
use core::hash::{BuildHasher, Hash};

use alloc::vec::Vec;
use alloc::format;
use core::alloc::Layout;

use console::println;

// Quick'n'dirty TSC
use b2histogram::Base2Histogram;

static mut TSC_INSERT_HISTOGRAM: Option<Base2Histogram> = None;
static mut TSC_INSERT_TOTAL: u64 = 0;

static mut TSC_GET_HISTOGRAM: Option<Base2Histogram> = None;
static mut TSC_GET_TOTAL: u64 = 0;

static mut TSC_HASH_HISTOGRAM: Option<Base2Histogram> = None;
static mut TSC_HASH_TOTAL: u64 = 0;

static mut TSC_FIND_HISTOGRAM: Option<Base2Histogram> = None;
static mut TSC_FIND_TOTAL: u64 = 0;

static mut COLLISIONS: u64 = 0;
static mut HASH_ACC: usize = 0;

pub fn print_collisions() {
    println!("{}", unsafe { COLLISIONS });
}

macro_rules! record_hist {
    ($hist: ident, $total: ident, $val: expr) => {
        unsafe {
            if let None = $hist {
                $hist = Some(Base2Histogram::new());
            }

            let hist = $hist.as_mut().unwrap();
            hist.record($val);
            $total += $val;
        }
    };
}

macro_rules! print_stat {
    ($hist: ident, $total: ident) => {
        unsafe {
            println!("{}", core::stringify!($hist));

            let mut count = 0;

            for bucket in $hist.as_ref().unwrap().iter().filter(|b| b.count > 0) {
                count += bucket.count;
                println!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
            }

            println!("Average: {}", $total / count);
        }
    };
}

// The optimizer can very well do its job if we
// simply add the feature gate in grow() itself,
// but we are doing this for the sake of reducing
// variables.
macro_rules! grow {
    () => {
        #[cfg(feature = "grow")]
        {
            self.grow();
        }
        #[cfg(not(feature = "grow"))]
        {
            panic!("Hash table saturated");
        }
    };
}

pub fn print_stats() {
    println!("Insert/Get = Hash + Find");
    print_stat!(TSC_INSERT_HISTOGRAM, TSC_INSERT_TOTAL);
    print_stat!(TSC_GET_HISTOGRAM, TSC_GET_TOTAL);
    print_stat!(TSC_HASH_HISTOGRAM, TSC_HASH_TOTAL);
    print_stat!(TSC_FIND_HISTOGRAM, TSC_FIND_TOTAL);
}

const DEFAULT_MAX_LOAD: f64 = 0.7;
const DEFAULT_GROWTH_POLICY: f64 = 2.0;
const DEFAULT_PROBING: fn(usize, usize) -> usize = |hash, i| hash + i + i * i;

const DEFAULT_INITIAL_CAPACITY: usize = 1; // not handling zero sized

/// Alias for handling buckets.
pub type Bucket<K, V> = Option<(K, V)>;

/// Alias for handling results of a lookup with the `find` method.
type Find<K, V> = (Option<(K, V)>, Option<usize>);

/// Parameters needed in the configuration
/// of an [`CIndex`] hash table.
///
/// # Example
///
/// ```
/// use std::collections::hash_map::RandomState;
/// use index::{CIndex, Parameters};
///
/// let params = Parameters {
///     max_load: 0.7,
///     growth_policy: 2.0,
///     hasher_builder: RandomState::new(),
///     probe: |hash, i| (hash as f64 + (i as f64 / 2.0) + ((i*i) as f64 / 2.0)) as usize,
/// };
///
/// let mut index = CIndex::with_capacity_and_parameters(10, params);
///
/// index.insert("key", "value");
/// ```
///
/// [`CIndex`]: struct.CIndex.html
#[derive(Debug, Clone)]
pub struct Parameters<S> {
    /// Maximum load factor accepted before the table is resized. Default is `0.7`.
    pub max_load: f64,

    /// Ratio by which the table's capacity is grown. Default is `2`.
    pub growth_policy: f64,

    /// Hasher builder (see [`BuildHasher`]). Default is [`CIndexHasherBuilder`]
    ///
    /// [`CIndexHasherBuilder`]: hash/struct.CIndexHasherBuilder.html
    /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
    pub hasher_builder: S,

    /// Open addressing probing policy. Default is quadratic probing: `hash + i + i*i`
    pub probe: fn(hash: usize, i: usize) -> usize,
}

/// Simple implementation of a hash table using safe-rust.
///
/// The collisions are resolved through open adressing with
/// quadratic probing (although it is possible to use linear probing or other types
/// when specifying parameters).
///
/// # Example
///
/// ```
/// use index::CIndex;
///
/// let mut index = CIndex::new();
///
/// assert_eq!(index.len(), 0);
/// assert_eq!(index.capacity(), 1);
///
/// index.insert("salutation", "Hello, world!");
/// index.insert("ferris", "https://www.rustacean.net/more-crabby-things/dancing-ferris.gif");
/// index.insert("did you know ?", "Rust is kinda cool guys !");
/// index.insert("key", "value");
///
/// println!("{}", index.get("salutation").unwrap());
///
/// assert_eq!(index.len(), 4);
/// assert_eq!(index.capacity(), 8);
/// ```
#[derive(Clone)]
pub struct CIndex<K, V, S = CIndexHasherBuilder> {
    params: Parameters<S>,
    capacity: usize,
    len: usize,
    table: Vec<Bucket<K, V>>,
}

impl<K, V> CIndex<K, V, CIndexHasherBuilder>
where
    K: Hash + Eq + Copy,
    V: Copy,
{
    /// Creates an empty `CIndex` with default initial capacity and default parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::new();
    /// ```
    pub fn new() -> CIndex<K, V, CIndexHasherBuilder> {
        Self::with_capacity(DEFAULT_INITIAL_CAPACITY)
    }

    /// Creates an empty `CIndex` with specified capacity and default parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::with_capacity(1312);
    /// ```
    pub fn with_capacity(capacity: usize) -> CIndex<K, V, CIndexHasherBuilder> {
        CIndex::with_capacity_and_parameters(
            capacity,
            Parameters {
                max_load: DEFAULT_MAX_LOAD,
                growth_policy: DEFAULT_GROWTH_POLICY,
                hasher_builder: CIndexHasherBuilder {},
                probe: DEFAULT_PROBING,
            },
        )
    }
}

impl<K, V, S> CIndex<K, V, S> {
    /// Returns the maximum load factor accepted before the table is resized.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::new();
    ///
    /// assert_eq!(index.max_load(), 0.7); // default max load
    /// ```
    pub fn max_load(&self) -> f64 {
        self.params.max_load
    }

    /// Returns the ratio by which the table's capacity is grown.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::new();
    ///
    /// assert_eq!(index.growth_policy(), 2.0); // default growth policy
    /// ```
    pub fn growth_policy(&self) -> f64 {
        self.params.growth_policy
    }

    /// Returns a reference to the hasher builder used in the `CIndex`.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    /// use index::hash::CIndexHasherBuilder;
    /// use std::any::{Any, TypeId};
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::new();
    ///
    /// assert_eq!(index.hasher().type_id(), TypeId::of::<CIndexHasherBuilder>()) // default hasher builder
    /// ```
    pub fn hasher(&self) -> &S {
        &self.params.hasher_builder
    }

    /// Returns the probing function pointer of the `CIndex`.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<String, Vec<i32>> = CIndex::new();
    ///
    /// let p = |h: usize, i: usize| h + i + i*i; // default prober
    ///
    /// assert_eq!((index.probe())(45, 2), p(45, 2));
    /// ```
    pub fn probe(&self) -> fn(usize, usize) -> usize {
        self.params.probe
    }

    /// Returns the capacity of the `CIndex`.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<&str, &str> = CIndex::with_capacity(6);
    ///
    /// assert_eq!(index.len(), 0);
    /// assert_eq!(index.capacity(), 6);
    /// ```
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of elements in the `CIndex`.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<&str, i32> = CIndex::with_capacity(6);
    ///
    /// index.insert("one", 1);
    /// index.insert("two", 2);
    /// index.insert("three", 3);
    ///
    /// assert_eq!(index.len(), 3);
    /// assert_eq!(index.capacity(), 6);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the `CIndex` contains no elements.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<&str, &str> = CIndex::with_capacity(10);
    ///
    /// assert!(index.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the current load factor of the `CIndex`.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index: CIndex<&str, i32> = CIndex::with_capacity(6);
    ///
    /// index.insert("one", 1);
    /// index.insert("two", 2);
    /// index.insert("three", 3);
    ///
    /// assert_eq!(index.load(), 0.5);
    /// ```
    pub fn load(&self) -> f64 {
        (self.len as f64) / (self.capacity as f64)
    }
}

impl<K, V, S> CIndex<K, V, S>
where
    K: Hash + Eq + Copy,
    V: Copy,
    S: BuildHasher + Clone,
{
    // static

    /// Creates an empty `CIndex` with specified capacity and parameters.
    ///
    /// See [`Parameters`] for details.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::hash_map::RandomState;
    /// use index::{CIndex, Parameters};
    ///
    /// let params = Parameters {
    ///     max_load: 0.7,
    ///     growth_policy: 2.0,
    ///     hasher_builder: RandomState::new(),
    ///     probe: |hash, i| (hash as f64 + (i as f64 / 2.0) + ((i*i) as f64 / 2.0)) as usize,
    /// };
    ///
    /// let mut index = CIndex::with_capacity_and_parameters(10, params);
    ///
    /// index.insert("key", "value");
    /// ```
    ///
    /// [`Parameters`]: struct.Parameters.html
    pub fn with_capacity_and_parameters(capacity: usize, params: Parameters<S>) -> CIndex<K, V, S> {
        let capacity = if capacity == 0 {
            DEFAULT_INITIAL_CAPACITY
        } else {
            capacity
        };

        #[cfg(feature = "aligned-mem")]
        let mut index = { 
            let num_bytes = core::mem::size_of::<Bucket<K, V>>() * capacity;

            // println!("Creating a layout of {} * {} = {}",
            //                        core::mem::size_of::<Bucket<K,V>>(),
            //                        capacity, num_bytes);

            let layout = Layout::from_size_align(num_bytes, 4096)
                    .map_err(|e| panic!("Layout error: {}", e)).unwrap();


            let buf = unsafe {alloc::alloc::alloc(layout) as *mut Bucket<K,V> };
            let mut v: Vec<Bucket<K,V>> = unsafe { Vec::from_raw_parts(buf, capacity, capacity)} ;
            //println!("vec len {} cap {}", v.len(), v.capacity());
            CIndex {
                params,
                capacity,
                len: 0,
                table: v
            }
        };

        #[cfg(not(feature = "aligned-mem"))]
        let mut index = CIndex {
            params,
            capacity,
            len: 0,
            table: Vec::with_capacity(capacity),
        };

        Self::init_table(&mut index.table, index.capacity);

        index
    }

    /// Initializes inner table with empty buckets according to specified capacity.
    fn init_table(table: &mut Vec<Bucket<K, V>>, capacity: usize) {
        for i in 0..capacity {
            #[cfg(feature = "aligned-mem")]
            { table[i] = Bucket::None; }

            #[cfg(not(feature = "aligned-mem"))]
            { table.push(Bucket::None); }
        }

        // useless but that paranoia
        assert_eq!(capacity, table.len());
        assert_eq!(capacity, table.capacity());
    }

    // methods

    /// Resizes `CIndex` with new capacity by allocating a new `CIndex`
    /// and moving entries from the old one to the new one by using insert to
    /// rehash the entries (if the new capacity is to small, the insert operation will grow
    /// the new `CIndex` automatically).
    fn resize(&mut self, new_capacity: usize) {
        let mut new_index = Self::with_capacity_and_parameters(new_capacity, self.params.clone());

       /* for (key, value) in self.drain() {
            new_index.insert(key, value);
        }*/

        *self = new_index;
    }

    /// Grows `CIndex` according to growth policy.
    fn grow(&mut self) {
        let new_cap = (self.capacity as f64 * self.params.growth_policy) as usize;
        self.resize(new_cap);
    }

    /// Searches for an entry according to specified hash and discriminating closure.
    ///
    /// See alias definition of `Find<'a, K, V>` at the top of this file for more details.
    fn find<F>(&self, hash: usize, f: F) -> Find<K, V>
    where
        F: Fn((K, V)) -> bool,
    {
        for i in 0..self.capacity {
            let probe = (hash + i) & (self.capacity - 1);

            /*if (i < self.capacity - 2) {
                let p1 = (hash + (i + 1) + (i + 1) * (i + 1)) & (self.capacity - 1);
                let p2 = (hash + (i + 2) + (i + 2) * (i + 2)) & (self.capacity - 1);
                unsafe { core::intrinsics::prefetch_read_data(&self.table[p1], 3) };
                unsafe { core::intrinsics::prefetch_read_data(&self.table[p2], 3) };
            }*/


            match &self.table[probe] {
                Some(pair) if f(*pair) => return (Some(*pair), Some(probe)), // found matching bucket
                None => return (None, Some(probe)), // found empty bucket
                Some(_) => {
                    /*unsafe {
                        COLLISIONS += 1;
                    }*/
                    continue;
                },
            }
        }

        (None, None) // found nothing
    }

    /// Inserts key-value pair in the `CIndex`.
    ///
    /// If it encounters an occupied bucket with the same key, it will replace the
    /// entry according to the new value and return the old bucket.
    ///
    /// The function also verifies before anything else that the load factor is lesser
    /// than the maximum accepted load, if not it will grow the `CIndex` before proceeding to the insertion.
    ///
    /// If the lookup returns no valid result, the insertion is considered impossible and
    /// the function will grow the `CIndex` and retry to insert the pair.
    ///
    /// # Example
    ///
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index = CIndex::with_capacity(2);
    ///
    /// index.insert("key", "value");
    ///
    /// assert_eq!(*index.get("key").unwrap(), "value");
    ///
    /// index.insert("key", "new value");
    ///
    /// assert_eq!(*index.get("key").unwrap(), "new value");
    ///
    /// assert_eq!(index.len(), 1);
    /// assert_eq!(index.capacity(), 2);
    ///
    /// index.insert("salutation", "Hello, world!");
    /// index.insert("ferris", "https://www.rustacean.net/more-crabby-things/dancing-ferris.gif");
    /// index.insert("did you know ?", "Rust is kinda cool guys !");
    ///
    /// assert_eq!(index.len(), 4);
    /// assert_eq!(index.capacity(), 8);
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Bucket<K, V> {
        // /* PERF */ let insert_start = unsafe { core::arch::x86_64::_rdtsc() };

        // /* PERF */ let hash_start = unsafe { core::arch::x86_64::_rdtsc() };
        // let hash = make_hash(&self.params.hasher_builder, &key) as usize;
        let hash = fnv(&key) as usize;
        // /* PERF */ let hash_end = unsafe { core::arch::x86_64::_rdtsc() };
        // /* PERF */ record_hist!(TSC_HASH_HISTOGRAM, TSC_HASH_TOTAL, hash_end - hash_start);
        /*unsafe {
          HASH_ACC += hash;
        }*/

        #[cfg(feature = "grow")]
        {
            if self.load() >= self.params.max_load {
                self.grow();
            }
        }

        // We have two styles of insert()
        // - C-style, similar to insert() in the C version
        // - Idiomatic Rust
        #[cfg(feature = "c-style-insert")]
        {
            // C-style
            /*
            for i in 0..self.capacity {
                let probe = (hash + i + i * i) % self.capacity;

                #[cfg(feature = "grow")]
                {
                    if self.table[probe].is_none() {
                        self.len += 1;
                    }
                }

                if self.table[probe].is_none() || self.table[probe].as_ref().unwrap().borrow().0 == key {
                    self.table[probe] = Bucket::Some(RefCell::new((key, value)));
                    return;
                }

                unsafe {
                    COLLISIONS += 1;
                }
            }
            */

            for i in 0..self.capacity {
                let probe = (hash + i) & (self.capacity - 1);

                match &self.table[probe] {
                    Some(pair) if pair.0 == key => {
                        core::mem::replace(&mut self.table[probe], Bucket::Some((key, value)));
                        return Bucket::None;
                    },
                    None => {
                        core::mem::replace(&mut self.table[probe], Bucket::Some((key, value)));
                        return Bucket::None;
                    },
                    Some(_) => {
                        /*unsafe {
                            COLLISIONS += 1;
                        }*/
                        continue;
                    },
                }
            }

            // Failed to find a cell
            grow!();
            self.insert(key, value)
        }
        #[cfg(not(feature = "c-style-insert"))]
        {
            //this_will_not_compile();

            // Idiomatic Rust
            match self.find(hash, |p| key.eq(p.0)) {
                (Some(_), Some(i)) => {
                    return core::mem::replace(&mut self.table[i], Bucket::Some((key, value)));
                }
                (None, Some(i)) => {
                    self.table[i] = Bucket::Some((key, value));

                    #[cfg(feature = "grow")]
                    {
                        self.len += 1;
                    }
                    return Bucket::None;
                    // return;
                }
                _ => {
                    grow!();
                    self.insert(key, value)
                }
            }
        }

        // /* PERF */ let insert_end = unsafe { core::arch::x86_64::_rdtsc() };
        // /* PERF */ record_hist!(TSC_INSERT_HISTOGRAM, TSC_INSERT_TOTAL, insert_end - insert_start);
        // /* PERF */ eprintln!("1,{},{},{},{}", self.len, self.capacity, self.load(), insert_end - insert_start);
    }

    // pub fn remove_entry<Q>(&mut self, key: &Q) -> Bucket<K, V> where K: Borrow<Q>, Q: Hash + Eq + ?Sized
    /*
        Problem: removing entry can corrupt lookup integrity
                 (find may encounter empty bucket before searched value)

        Solutions:
            - use find_match and find_empty
                Problem: find_match will always have to be used for remove and get operations
                         to ensure lookup integrity and will have O(n) complexity if key isnt in table (because wont return first empty bucket found)
            - use flag array for present, empty, removed values ?

        Same problem arises when modifying keys through an IterMut
    */

    /// Returns a reference to the value associated with the specified key
    /// if the lookup found a match, else it returns `None`.
    ///
    /// # Example
    ///  
    /// ```
    /// use index::CIndex;
    ///
    /// let mut index = CIndex::with_capacity(10);
    ///
    /// index.insert("salutation", "Hello, world!");
    /// index.insert("ferris", "https://www.rustacean.net/more-crabby-things/dancing-ferris.gif");
    /// index.insert("did you know ?", "Rust is kinda cool !");
    ///
    /// assert_eq!(*index.get("salutation").unwrap(), "Hello, world!");
    /// ```
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Copy,
    {
        /* PERF */ //let get_start = unsafe { core::arch::x86_64::_rdtsc() };

        /* PERF */ //let hash_start = unsafe { core::arch::x86_64::_rdtsc() };
        //let hash = make_hash(self.hasher(), &key) as usize;
        //let k = unsafe { *(key as *const _ as *const usize) };
        let hash = fnv(key) as usize;
        /* PERF */ //let hash_end = unsafe { core::arch::x86_64::_rdtsc() };
        /* PERF */ //record_hist!(TSC_HASH_HISTOGRAM, TSC_HASH_TOTAL, hash_end - hash_start);

        #[cfg(not(feature="c-style-insert"))]
        {
            this_wont_compile();
        
            return self.find(hash, |p| key.eq(p.0))
                .0.map(|p| p.1);
        }
        /* PERF */ //let find_start = unsafe { core::arch::x86_64::_rdtsc() };
        //let r = self.find(hash, |p| key.borrow().eq(p.0.borrow()))
        //    .0
        //    .map(|pair| Ref::map(pair.borrow(), |p| &p.1));

        #[cfg(feature="c-style-insert")]
        { 
            for i in 0..self.capacity {
                let probe = (hash + i) & (self.capacity - 1);

                if let Some(pair) = &self.table[probe] {
                    if pair.0 == *key {
                        return Some(pair.1);
                    }
                } else {
                    return None;
                }
            }
            None
        }
        /* PERF */ //let find_end = unsafe { core::arch::x86_64::_rdtsc() };
        /* PERF */ //record_hist!(TSC_FIND_HISTOGRAM, TSC_FIND_TOTAL, find_end - find_start);

        /* PERF */ //let get_end = unsafe { core::arch::x86_64::_rdtsc() };
        /* PERF */ //record_hist!(TSC_GET_HISTOGRAM, TSC_GET_TOTAL, get_end - get_start);

        // /* PERF */ eprintln!("0,{},{},{},{}", self.len, self.capacity, self.load(), get_end - get_start);

        //r
    }
}

impl<K, V, S> fmt::Debug for CIndex<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = format!(
            "CIndex {{\n\tparams: {:?}\t\ncapacity: {:?}\n\tlen: {:?}\n\ttable:\n\t[",
            self.params, self.capacity, self.len
        );

        for (i, entry) in self.table.iter().enumerate() {
            s = format!(
                "{}\n\t\t{} : {:?},",
                s,
                i,
                if let Some(pair) = entry {
                    Some(pair)
                } else {
                    None
                }
            );
        }
        s = format!("{}\n\t]\n}}", s);

        write!(f, "{}", s)
    }
}

impl<K, V> Default for CIndex<K, V, CIndexHasherBuilder>
where
    K: Hash + Eq + Copy,
    V: Copy,
{
    fn default() -> Self {
        Self::new()
    }
}
