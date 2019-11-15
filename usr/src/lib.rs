#![no_std]
pub mod init;
pub mod capabilities;
pub mod syscalls; 

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
