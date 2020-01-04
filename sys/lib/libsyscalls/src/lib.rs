#![no_std]
pub mod sysbdev;
pub mod syscalls;
pub mod errors;
pub mod time; 

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
