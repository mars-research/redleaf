use crate::capabilities::Capability; 

#[derive(Copy, Clone)]
pub struct Syscall {
    pub sys_print: fn(s: &str),
    pub sys_yield: fn(),
    pub sys_create_thread: fn(name: &str, func: extern fn()) -> Capability,
}

