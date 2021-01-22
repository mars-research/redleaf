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
use core::mem;

use arrayvec::ArrayVec;

use log::trace;
use console::{println,print};

pub mod indexmap;
pub mod cindexmap;

mod memb;

use memb::serialize::{RequestHeader, SetRequest, GetRequest, GetResponse};

use memb::{serialize::buf_encode, serialize::Decoder, ClientValue, ServerValue};
use fnv::FnvHasher;

use core::cell::RefCell;

use b2histogram::Base2Histogram;

type FnvHashFactory = BuildHasherDefault<FnvHasher>;

pub use memb::serialize::KEY_SIZE;
pub use memb::serialize::VALUE_SIZE;

pub type KVKey = ArrayVec<[u8; KEY_SIZE]>;
pub type KVVal =  ArrayVec<[u8; VALUE_SIZE]>;
pub type KVal = KVVal;
//pub type KVVal = (u32, KVal);

static mut TOTAL_SET: usize = 0;
static mut TOTAL_STORED: usize = 0;
static mut TOTAL_NOT_STORED: usize = 0;
static mut TOTAL_GET: usize = 0;
static mut TOTAL_RETRIEVED: usize = 0;
static mut TOTAL_NOT_FOUND: usize = 0;

#[repr(C,packed)]
#[derive(Debug,Clone)]
pub struct KVPair {
    pub key: [u8; KEY_SIZE],
    pub val: [u8; VALUE_SIZE],
}

impl KVPair {
    const fn empty() -> Self {
        Self {
            key: [0u8; KEY_SIZE],
            val: [0u8; VALUE_SIZE],
        }
    }
}

pub const empty_key: KVPair = KVPair::empty();

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

        /*unsafe {
            let mut vec = ArrayVec::new();
            vec.try_extend_from_slice(&[0u8; 64]);

            //FAKE_VAL = Some(RefCell::new((805306368, vec)));
        }*/
        
        println!("sizeof RequestHeader {}, SetRequest {}, GetRequest {}",
                        mem::size_of::<RequestHeader>(),
                        mem::size_of::<SetRequest>(),
                        mem::size_of::<GetRequest>());

        println!("Sizeof each element in HT {}", mem::size_of::<(KVKey, KVVal)>());
        println!("Sizeof each element in HT {}", mem::size_of::<KVPair>());

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
        //println!("capacity={}, len={}", self.map.capacity(), self.map.len());
        //print_stat!(TSC_PARSE_HISTOGRAM, TSC_PARSE_TOTAL);
        memb::serialize::print_stats();
    }

    pub fn print_stats_simple(&self) {
        println!("key_size {}, value_size {}",
                 KEY_SIZE, VALUE_SIZE);
        unsafe {
            println!("simple_stats: total_set {}, total_stored {} total_not_stored {} \
        total_get {}, total_retrieved {} total_not_found {}",
        TOTAL_SET, TOTAL_STORED, TOTAL_NOT_STORED,
        TOTAL_GET, TOTAL_RETRIEVED, TOTAL_NOT_FOUND);
        }
    }

    pub fn handle_network_request_simple(&mut self, buf: &mut Vec<u8>) {
        assert!(buf.len() >= 8);
        // 0-1 Request ID
        let req_id = [
            buf[0],
            buf[1],
            buf[2],
            buf[3],
        ];

        // HACK: We want 4 bytes for req_id
        let request_id = u32::from_be_bytes(req_id);

        let op = &buf[10..14];
        let set = b"set ";
        let get = b"get ";

        // parse opcode
        if op == set {
                let key: &[u8] = &buf[(10 + 4)..(10 + 4 + KEY_SIZE)];
                let value: &[u8] = &buf[(10 + 4 + KEY_SIZE + 11)..(10 + 4 + KEY_SIZE + 11 + VALUE_SIZE)];

                /*unsafe {
                  print!("set for {:?} {:?}\n", String::from_utf8(slice::from_raw_parts(key.as_ptr(), KEY_SIZE).to_vec())
                  .unwrap(),
                  String::from_utf8(slice::from_raw_parts(value.as_ptr(), VALUE_SIZE).to_vec())
                  .unwrap());
                }*/
                let success = self.map.insert_simple(&key, &value);

                buf.clear();

                buf.extend_from_slice(&u32::to_be_bytes(request_id));
                buf.extend_from_slice(&u16::to_be_bytes(0)); // seq number
                buf.extend_from_slice(&u16::to_be_bytes(1)); // #datagram
                buf.extend_from_slice(&u16::to_be_bytes(0)); // reserved

                if success {
                    buf.extend_from_slice(b"STORED\r\n");
                    unsafe { TOTAL_STORED += 1; }
                } else {
                    unsafe { TOTAL_NOT_STORED += 1; }
                    buf.extend_from_slice(b"NOT_STORED\r\n");
                }
                unsafe { TOTAL_SET += 1; }
               // return Ok(ClientValue::Set(request_id, &key, 0, &value));
        } else if op == get {
            unsafe { TOTAL_GET += 1; }

            let key: &[u8] = &buf[(10 + 4)..(10 + 4 + KEY_SIZE)];

            /*unsafe {
                print!("get for {:?}\n",
                       String::from_utf8(core::slice::from_raw_parts(key.as_ptr(), KEY_SIZE).to_vec())
                       .unwrap());
            }*/

            let (success, pair) = self.map.get_simple(&key);


            if success {
                /*unsafe {
                print!("got k = {:?} v = {:?}\n",
                       String::from_utf8(core::slice::from_raw_parts(pair.key.as_ptr(), KEY_SIZE).to_vec())
                       .unwrap(),
                       String::from_utf8(core::slice::from_raw_parts(pair.val.as_ptr(), VALUE_SIZE).to_vec())
                       .unwrap(),
                       );
                }*/

                
                /*buf.extend_from_slice(&u32::to_be_bytes(request_id));
                buf.extend_from_slice(&u16::to_be_bytes(0)); // seq number
                buf.extend_from_slice(&u16::to_be_bytes(1)); // #datagram
                buf.extend_from_slice(&u16::to_be_bytes(0)); // reserved
                buf.extend_from_slice(b"VALUE ");
                buf.extend_from_slice(&pair.key);
                buf.extend_from_slice(" ".as_bytes());

                buf.extend_from_slice(b"5555");
                buf.extend_from_slice(b"1023\r\n");
                buf.extend_from_slice(b"55551023\r\n");
                buf.extend_from_slice(&pair.val);
                buf.extend_from_slice(b"\r\nEND\r\n");*/

                //let header = &[buf[0], buf[1], buf[2], buf[3], 0, 0, 1, 0, 0, 0];
                buf.clear();
                //buf.extend_from_slice(header);
                //buf.extend_from_slice(b"VALUE ");
                //buf.extend_from_slice(&pair.key);
                //buf.extend_from_slice(&pair);
                //buf.extend_from_slice(b" 55551023\r\n");
                //buf.extend_from_slice(&pair.val);
                //buf.extend_from_slice(b"\r\nEND\r\n");*/
                let resp = GetResponse::new(request_id);

                unsafe {
                    let resp_bytes = &resp as *const _ as *const u8;
                    let resp_len = mem::size_of::<GetResponse>();
                    core::ptr::copy(resp_bytes, buf.as_mut_ptr(), resp_len);
                    core::ptr::copy(&pair.key.as_ptr(), buf.as_mut_ptr().offset(16) as *mut *const u8, KEY_SIZE);
                    core::ptr::copy(&pair.val.as_ptr(), buf.as_mut_ptr().offset(16 + KEY_SIZE as isize + 11) as *mut *const u8, VALUE_SIZE);
                    buf.set_len(resp_len);
                }

                unsafe { TOTAL_RETRIEVED += 1; }

            } else {
                buf.clear();
                buf.extend_from_slice(b"NOT_FOUND\r\n");
                unsafe { TOTAL_NOT_FOUND += 1; }
            }

            //return Ok(ClientValue::Get(request_id, &key));
        } else {
            //return Err(DecodeError::InvalidOpCode);
        }
    }

    /// Execute the content of a packet buffer in our KV store.
    pub fn handle_network_request(&mut self, buf: Vec<u8>) -> Vec<u8> {
        //let reader = VecDeque::from(buf);
        //println!("<= req_buf {:x?} {}", buf.as_ptr(), buf.len());
        let mut decoder = Decoder::new(buf);

        //let start = unsafe { core::arch::x86_64::_rdtsc() };
        //let r = decoder.decode();
        let r = decoder.decode_as_struct();
        //let elapsed = unsafe { core::arch::x86_64::_rdtsc() } - start;
        //record_hist!(TSC_PARSE_HISTOGRAM, TSC_PARSE_TOTAL, elapsed);

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

                /*unsafe {
                        print!("get for {:?} ", String::from_utf8(core::slice::from_raw_parts(key.as_ptr(), KEY_SIZE).to_vec())
                                                    .unwrap());
                }*/

                let mut key_vec: KVKey = ArrayVec::new();
                key_vec.try_extend_from_slice(&key).expect("rua");

                let r = self.map.get(&key_vec);
                let mut ret;
                match r {
                    Some(value) => {
                        // one copy here
                        //let mut key_vec = ArrayVec::new();
                        //key_vec.try_extend_from_slice(key).expect("Key too long");
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

                //let end = unsafe { core::arch::x86_64::_rdtsc() };

                ret
            }
            ClientValue::Set(req_id, key, flags, value) => {
                // HACK
                /*
                return ServerValue::Stored(req_id);
                */

                // println!("Set for {}", core::str::from_utf8(key).unwrap());
                //let start = unsafe { core::arch::x86_64::_rdtsc() };

                let r = if key.len() <= 250 {
                    use core::slice;
                    let mut key_vec: KVKey = ArrayVec::new();
                    let mut value_vec: KVal = ArrayVec::new();

                    key_vec.try_extend_from_slice(&key).expect("rua");
                    value_vec.try_extend_from_slice(&value).expect("rua");

                    /*unsafe {
                        print!("set for {:?} {:?}", String::from_utf8(slice::from_raw_parts(key.as_ptr(), KEY_SIZE).to_vec())
                                                    .unwrap(),
                                                    String::from_utf8(slice::from_raw_parts(value.as_ptr(), VALUE_SIZE).to_vec())
                                                    .unwrap());
                    }*/
                    self.map.insert(key_vec, value_vec);
                    ServerValue::Stored(req_id)
                } else {
                    ServerValue::NotStored(req_id)
                };

                //let end = unsafe { core::arch::x86_64::_rdtsc() };
                // println!("set took {:?}", end - start);

                r
            }
            _ => unreachable!(),
        }
    }
}
