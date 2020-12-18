/// RedLeaf block device interface
use alloc::boxed::Box;
use rref::{RRef, RRefDeque};

// #[interface]
pub trait DomA {
    fn ping_pong(&self, buffer: RRef<[u8; 1024]>) -> RRef<[u8; 1024]>;
    fn tx_submit_and_poll(&mut self,
        packets: RRefDeque<[u8; 100], 32>,
        reap_queue: RRefDeque<[u8; 100], 32>) -> (
            usize,
            RRefDeque<[u8; 100], 32>,
            RRefDeque<[u8; 100], 32>
        );
}
