use rref::RRef;

pub trait Proxy {
    fn foo(&self) -> usize;
    fn new_value(&self, value: usize) -> RRef<usize>;
}
