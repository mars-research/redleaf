#![no_std]
#![feature(
    allocator_api,
)]

#[macro_use]
extern crate alloc;

pub mod init;
pub mod capabilities;
pub mod syscalls; 

mod ls;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
