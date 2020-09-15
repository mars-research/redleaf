//! A safe key--value store (sashstore)
#![forbid(unsafe_code)]
#![feature(test)]
#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate test;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use log::trace;
use arrayvec::ArrayVec;

mod indexmap;

mod memb;

use memb::{serialize::buf_encode, serialize::Decoder, ClientValue, ServerValue};

pub struct SashStore {
    /// Maps key -> (flags, value)
    map: indexmap::Index<ArrayVec<[u8; 256]>, (u32, Vec<u8>)>,
}

impl SashStore {
    /// Initialize a new SashStore instance.
    pub fn with_capacity(cap: usize) -> Self {
        SashStore {
            map: indexmap::Index::with_capacity(cap),
        }
    }

    /// Execute the content of a packet buffer in our KV store.
    pub fn handle_network_request(&mut self, buf: Vec<u8>) -> Vec<u8> {
        //let reader = VecDeque::from(buf);
        //println!("<= req_buf {:x?} {}", buf.as_ptr(), buf.len());
        let mut decoder = Decoder::new(buf);
        let response = match decoder.decode() {
            Ok(value) => {
                // trace!("Received value={:?}", value);
                self.execute_cmd(value)
            }
            Err(e) => panic!("Couldn't parse request {:?}", e),
        };
        let buf = decoder.destroy();
        // buf_encode(&response, &mut buf);
        //println!("=> resp_buf {:x?} {}", resp_buf.as_ptr(), resp_buf.len());
        buf
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd<'req, 'kv>(&'kv mut self, cmd: ClientValue<'req>) -> ServerValue<'kv> {
        match cmd {
            ClientValue::Get(req_id, key) => {
                trace!("Execute .get for {:?}", key);
                if key.len() > 250 {
                    // Illegal key
                    return ServerValue::NoReply;
                }

                let r = self.map.get(key);
                match r {
                    Some(value) => {
                        // one copy here
                        let mut copied_key: [u8; 250] = [0; 250];
                        let copied_length = key.len();
                        copied_key[0..copied_length].clone_from_slice(&key[0..copied_length]);
                        ServerValue::Value(req_id, copied_key, copied_length, value)
                    },
                    None => {
                        unreachable!("didn't find value for key {:?}", key);
                        ServerValue::NoReply
                    }
                }
            }
            ClientValue::Set(req_id, key, flags, value) => {
                trace!("Set for {:?} {:?}", key, value);
                if key.len() <= 250 {
                    self.map.insert(key.to_vec(), (flags, value.to_vec()));
                    ServerValue::Stored(req_id)
                } else {
                    ServerValue::NotStored(req_id)
                }
            }
            _ => unreachable!(),
        }
    }
}
