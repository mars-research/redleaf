pub trait DomC {
    fn no_arg(&self);
    fn one_arg(&self, x: usize) -> usize;
}