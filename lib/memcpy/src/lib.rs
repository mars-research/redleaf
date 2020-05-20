// https://doc.redox-os.org/kernel/src/kernel/externs.rs.html
#![no_std]
// #![no_builtins]

#[cfg(feaure = "musl")]
mod musl;
#[cfg(not(feaure = "musl"))]
mod redox;
