use std::fmt::Debug;

pub mod topology;
use topology::Cpu;

/// A wrapper type to distinguish between arbitrary generated read or write operations
/// in the test harness.
#[derive(Debug, Clone)]
pub enum Operation<R: Sized + Clone + PartialEq + Debug, W: Sized + Clone + PartialEq + Debug> {
    ReadOperation(R),
    WriteOperation(W),
}

/// Pin a thread to a core
pub fn pin_thread(core_id: Cpu) {
    core_affinity::set_for_current(core_affinity::CoreId {
        id: core_id as usize,
    });
}
