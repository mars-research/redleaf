//! Parse the memcached binary format
//!
//! Based on the description provided here:
//! https://github.com/memcached/memcached/wiki/BinaryProtocolRevamped
//!
//! and a bit of `tcpdump -i lo udp port 11211 -vv -X`

#![allow(unused)] // For now

use alloc::vec::Vec;
use core::cell::Ref;

use arrayvec::ArrayVec;
use crate::{KVKey, KVVal};

pub mod serialize;

// Let's separate the vocabularies of client and server to make
// reasoning about lifetimes easier. Here ClientValue<'req> will
// contain references to the original request, whereas
// ServerValue<'kv> will refer to the Index.

/// Data format description for a parsed packet from the client
#[derive(Debug)]
pub enum ClientValue<'req> {
    Get(u32, &'req [u8]),
    Set(u32, &'req [u8], u32, &'req [u8]),
}

/// Data format description for a packet to be sent out
pub enum ServerValue<'kv> {
    // (seq, key, val_ref)
    Value(u32, KVKey, Ref<'kv, KVVal>),
    Stored(u32),
    NotStored(u32),
    NoReply,
}

/// A decoder error
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DecodeError {
    InvalidOpCode,
    UnexpectedEof,
}
