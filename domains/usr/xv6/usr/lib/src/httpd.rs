#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate arrayvec;
extern crate malloc;
use crate::{eprintln, println};
use alloc::boxed::Box;
use core::panic::PanicInfo;
use interface::error::Result;
use interface::rv6::Rv6;
use interface::usrnet::UsrNet;
use syscalls::{Heap, Syscall};

#[macro_use]
use redhttpd::usrnet::Httpd;

pub fn main(rv6: Box<dyn Rv6>, args: &str) {
    println!("Starting rv6 httpd with args: {}", args);

    main_loop(rv6).unwrap();
}

fn main_loop(rv6: Box<dyn Rv6>) -> Result<()> {
    let usrnet = rv6.get_usrnet()?;

    let mut httpd = Httpd::new();

    loop {
        UsrNet::poll(&*usrnet, false);
        httpd.handle(&*usrnet);
        UsrNet::poll(&*usrnet, true);
    }

    Ok(())
}
