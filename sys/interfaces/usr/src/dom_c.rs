use rref::RRef;

pub trait DomC {
    fn no_arg(&self);
    fn one_arg(&self, x: usize) -> usize;
    fn one_rref(&self, x: RRef<usize>) -> RRef<usize>;
}