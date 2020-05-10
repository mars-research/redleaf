use rref::RRef;

pub trait DomC {
    fn no_arg(&self);
    fn one_arg(&self, x: usize) -> Result<usize, i64>;
    fn one_rref(&self, x: RRef<usize>) -> RRef<usize>;
}
