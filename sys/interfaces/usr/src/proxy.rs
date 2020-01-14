use rref::RRef;

pub trait Proxy {
    fn foo(&self) -> usize;
    fn new_value(&self, value: [u8; 512]) -> RRef<[u8; 512]>;
}
