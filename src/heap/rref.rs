#![feature(const_fn)]
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut, Drop};
use alloc::boxed::Box;
use spin::Mutex;
use crate::heap::alloc::move_object;
use core::alloc::Layout;

pub static rref_registry: RRefRegistry = RRefRegistry::new();

pub struct RRefRegistry {
    // type-erased list of pointers to shared heap memory objects
    rrefs: Mutex<Vec<Box<dyn HasDomainId + Send>>>,
}

impl RRefRegistry {
    pub const fn new() -> RRefRegistry {
        RRefRegistry {
            rrefs: Mutex::new(Vec::new()),
        }
    }

    pub fn register_rref<T>(&self, object: Box<SharedHeapObject<T>>) where T: 'static + Send {
        self.rrefs.lock().push(object as Box<dyn HasDomainId + Send>);
    }

    pub fn unregister_rref<T>(&self, pointer: *mut SharedHeapObject<T>) where T: 'static + Send {

        self.rrefs.lock().retain(|obj: &Box<dyn HasDomainId + Send>| {
            let x = pointer as usize;
            let y = obj.as_ref() as *const (dyn HasDomainId + Send) as *const () as usize;

            x != y
        })
    }

    pub fn drop_rrefs(&self, domain_id: DomainId) {
        self.rrefs.lock().retain(|object| {
            if object.get_domain_id() == domain_id {
                drop(object);
                false
            } else {
                true
            }
        });
    }
}

// Shared heap allocated value. Returned by an allocation syscall
// like *mut SharedHeapObject<T>
pub struct SharedHeapObject<T> where T: 'static + Send {
    pub domain_id: DomainId,
    pub value: T,
}

impl<T> Drop for SharedHeapObject<T> where T: Send {
    fn drop(&mut self) {
//        println!("DROPPING SHARED HEAP OBJECT; VALUE []\n");
    }
}

impl<T> HasDomainId for SharedHeapObject<T> where T: Send {
    fn get_domain_id(&self) -> DomainId {
        self.domain_id
    }
}

type DomainId = u64;
pub trait HasDomainId {
    fn get_domain_id(&self) -> DomainId;
}

// RRef (remote reference) has an unowned reference to an object on shared heap.
// Only one domain can hold an RRef at a single time, so therefore we can "safely" mutate it.
// A global table retains all RRefs. When a domain dies, all of its shard heap objects are dropped,
//   which gives us the guarantee that the unowned reference will be safe to dereference as
//   long as its domain is alive.
pub struct RRef<T> where T: 'static + Send {
    reference: *mut SharedHeapObject<T>
}

impl<T> RRef<T> where T: Send {
    pub fn new(object: Box<SharedHeapObject<T>>) -> RRef<T> {
        // `object` is a pointer to a value on the shared heap.

        let ptr: *mut SharedHeapObject<T> = Box::into_raw(object);

        // we get another reference to the pointer, which we will give to the registry.
        // the registry will clean it up when the domain dies
        rref_registry.register_rref(unsafe { Box::from_raw(ptr) });

        RRef {
            reference: ptr
        }
    }

    pub fn move_to(&mut self, new_domain_id: DomainId) {
        // TODO: race here
        unsafe {
            move_object((*self.reference).domain_id, new_domain_id, self.reference as *mut u8, Layout::new::<T>());
            (*self.reference).domain_id = new_domain_id
        };

    }
}

impl<T> Drop for RRef<T> where T: Send {
    fn drop(&mut self) {
        rref_registry.unregister_rref(self.reference);
    }
}

impl<T> Deref for RRef<T> where T: Send {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.reference).value }
    }
}

impl<T> DerefMut for RRef<T> where T: Send {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.reference).value }
    }
}
