use crate::capabilities::Capability; 

pub trait Syscall {
    fn sys_print(&self, s: &str);
    fn sys_yield(&self);
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Capability;
}

