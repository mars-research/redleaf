/// RedLeaf network interface
use alloc::boxed::Box;
use rref::{RRef, RRefDeque};
// TODO: remove once Ixgbe transitions to RRefDeque
use alloc::{vec::Vec, collections::VecDeque};

pub trait Net {
    fn submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> usize;
}
