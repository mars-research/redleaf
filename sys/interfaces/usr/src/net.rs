/// RedLeaf network interface
use alloc::boxed::Box;
use rref::{RRef, RRefDeque};
// TODO: remove once Ixgbe transitions to RRefDeque
use alloc::{vec::Vec, collections::VecDeque};

pub trait Net {
    fn submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> usize;

    fn submit_and_poll_rref(
        &mut self,
        packets: RRefDeque<[u8; 1512], 32>,
        collect: RRefDeque<[u8; 1512], 32>,
        tx: bool) -> (
            usize,
            RRefDeque<[u8; 1512], 32>,
            RRefDeque<[u8; 1512], 32>
        );
}
