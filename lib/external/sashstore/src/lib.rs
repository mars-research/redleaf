//! A safe key--value store (sashstore)
//#![forbid(unsafe_code)]
#![feature(test,
           core_intrinsics)]
#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::string::String;

use arrayvec::ArrayVec;

use log::trace;
use console::println;

pub mod indexmap;

mod memb;

use memb::{serialize::buf_encode, serialize::Decoder, ClientValue, ServerValue};
use fnv::FnvHasher;

use core::cell::RefCell;

use b2histogram::Base2Histogram;

type FnvHashFactory = BuildHasherDefault<FnvHasher>;

pub type KVKey = ArrayVec<[u8; 8]>;
pub type KVal =  ArrayVec<[u8; 8]>;
pub type KVVal = (u32, KVal);

static mut FAKE_VAL: Option<RefCell<KVVal>> = None;

static mut TSC_PARSE_HISTOGRAM: Option<Base2Histogram> = None;
static mut TSC_PARSE_TOTAL: u64 = 0;

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

pub struct SashStore {
    /// Maps key -> (flags, value)
    map: indexmap::Index<KVKey, KVVal, FnvHashFactory>,
}

impl SashStore {
    /// Initialize a new SashStore instance.
    pub fn with_capacity(capacity: usize) -> Self {
        const DEFAULT_MAX_LOAD: f64 = 0.7;
        const DEFAULT_GROWTH_POLICY: f64 = 2.0;
        const DEFAULT_PROBING: fn(usize, usize) -> usize = |hash, i| hash + i + i * i;

        unsafe {
            let mut vec = ArrayVec::new();
            vec.try_extend_from_slice(&[0u8; 64]);

            FAKE_VAL = Some(RefCell::new((805306368, vec)));
        }
        
        SashStore {
            map: indexmap::Index::with_capacity_and_parameters(
                capacity,
                indexmap::Parameters {
                    max_load: DEFAULT_MAX_LOAD,
                    growth_policy: DEFAULT_GROWTH_POLICY,
                    hasher_builder: Default::default(),
                    probe: DEFAULT_PROBING,
                },
            )
        }
    }

    pub fn print_stats(&self) {
        println!("capacity={}, len={}", self.map.capacity(), self.map.len());
        print_stat!(TSC_PARSE_HISTOGRAM, TSC_PARSE_TOTAL);
    }

    /// Execute the content of a packet buffer in our KV store.
    pub fn handle_network_request(&mut self, buf: Vec<u8>) -> Vec<u8> {
        //let reader = VecDeque::from(buf);
        //println!("<= req_buf {:x?} {}", buf.as_ptr(), buf.len());
        let mut decoder = Decoder::new(buf);

        let start = unsafe { core::arch::x86_64::_rdtsc() };
        let r = decoder.decode();
        let elapsed = unsafe { core::arch::x86_64::_rdtsc() } - start;
        record_hist!(TSC_PARSE_HISTOGRAM, TSC_PARSE_TOTAL, elapsed);

        let response = match r {
            Ok(value) => {
                // trace!("Received value={:?}", value);
                self.execute_cmd(value)
            }
            Err(e) => panic!("Couldn't parse request {:?}", e),
        };
        let mut buf = decoder.destroy();
        buf_encode(&response, &mut buf);
        //println!("=> resp_buf {:x?} {}", resp_buf.as_ptr(), resp_buf.len());
        buf
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd<'req, 'kv>(&'kv mut self, cmd: ClientValue<'req>) -> ServerValue<'kv> {
        match cmd {
            ClientValue::Get(req_id, key) => {
                // HACK
                /*
                let mut key_vec = ArrayVec::new();
                key_vec.try_extend_from_slice(key).expect("Key too long");
                let val = unsafe { FAKE_VAL.as_ref().unwrap() }.borrow();
                return ServerValue::Value(req_id, key_vec, val);
                */

                trace!("Execute .get for {:?}", key);
                // println!("Get for {}", core::str::from_utf8(key).unwrap());

                if key.len() > 64 {
                    // Illegal key
                    panic!("key too long");
                    return ServerValue::NoReply;
                }

                let r = self.map.get(key);
                let mut ret;
                match r {
                    Some(value) => {
                        // one copy here
                        let mut key_vec = ArrayVec::new();
                        key_vec.try_extend_from_slice(key).expect("Key too long");
                        ret = ServerValue::Value(req_id, key_vec, value)
                    },
                    None => {
                        /*
                        for (i, kv) in self.map.iter().enumerate() {
                            println!("{}: {} -> {:?}", i, core::str::from_utf8(&kv.0).unwrap(), core::str::from_utf8(&(kv.1).1).unwrap());
                        }
                        */
                        // println!("No value for {}", core::str::from_utf8(key).unwrap());
                        // unreachable!("didn't find value for key {:?}", key);
                        ret = ServerValue::NoReply
                    },
                }

                let end = unsafe { core::arch::x86_64::_rdtsc() };

                ret
            }
            ClientValue::Set(req_id, key, flags, value) => {
                // HACK
                /*
                return ServerValue::Stored(req_id);
                */

                // println!("Set for {}", core::str::from_utf8(key).unwrap());
                let start = unsafe { core::arch::x86_64::_rdtsc() };

                let r = if key.len() <= 250 {
                    let mut key_vec: KVKey = ArrayVec::new();
                    let mut value_vec: KVal = ArrayVec::new();

                    key_vec.try_extend_from_slice(&key).expect("rua");
                    value_vec.try_extend_from_slice(&value).expect("rua");

                    self.map.insert(key_vec, (flags, value_vec));
                    //println!("set for {:?} {:?}", key, value);
                    ServerValue::Stored(req_id)
                } else {
                    ServerValue::NotStored(req_id)
                };

                let end = unsafe { core::arch::x86_64::_rdtsc() };
                // println!("set took {:?}", end - start);

                r
            }
            _ => unreachable!(),
        }
    }
}
