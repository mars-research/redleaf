#![no_std]
#![feature(const_fn)]
extern crate alloc;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut, Drop};
use core::cell::UnsafeCell;
use alloc::boxed::Box;
use spin::Mutex;

pub static rref_registry: RRefRegistry = RRefRegistry::new();

type DomainId = u64;
pub struct RRefRegistry {
    // type-erased list of pointers to shared heap memory along with the domains they belong to
    rrefs: Mutex<Vec<(DomainId, Box<dyn Send>)>>,
}

impl RRefRegistry {
    pub const fn new() -> RRefRegistry {
        RRefRegistry {
            rrefs: Mutex::new(Vec::new()),
        }
    }

    pub fn register_rref<T>(&self, reference: Box<T>, domain_id: DomainId) where T: 'static + Send {
        self.rrefs.lock().push((domain_id, reference as Box<dyn Send>));
    }

    pub fn drop_rrefs(&self, domain_id: DomainId) {
        self.rrefs.lock().retain(|(rref_domain_id, rref)| {
            if *rref_domain_id == domain_id {
                drop(rref);
                false
            } else {
                true
            }
        });
    }
}

// RRef (remote reference) has an unowned reference to an object on shared heap.
// Only one domain can hold an RRef at a single time, so therefore we can "safely" mutate it.
// A global table retains all RRefs. When a domain dies, all of its RRefs are dropped,
//   which gives us the guarantee that the unowned reference will be safe to dereference
//   as long as its domain is alive.
pub struct RRef<T> where T: 'static + Send {
    reference: UnsafeCell<T>,
    domain_id: DomainId
}

impl<T> RRef<T> where T: Send {
    pub fn new(value: T, domain_id: DomainId) -> RRef<T> {

        let cell = UnsafeCell::new(value);
        // TODO: think long and hard about whether this is correct
        let registry_reference = unsafe { Box::from_raw(cell.get()) };

        rref_registry.register_rref(registry_reference, domain_id);

        RRef {
            reference: cell,
            domain_id
        }
    }

    pub fn move_to(&mut self, new_domain_id: DomainId) {
        self.domain_id = new_domain_id;
        // TODO: update this domain change in the registry, or have the registry peek into RRef
    }
}

impl<T> Deref for RRef<T> where T: Send {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.reference.get() }
    }
}

impl<T> DerefMut for RRef<T> where T: Send {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.reference.get() }
    }
}
