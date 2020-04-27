/// RedLeaf block device interface
use alloc::boxed::Box;
use rref::RRef;

pub trait DomA {
    fn ping_pong(&self, buffer: RRef<[u8; 1024]>) -> RRef<[u8; 1024]>;
}
