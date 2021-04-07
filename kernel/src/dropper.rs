use core::mem::transmute;
use hashbrown::HashMap;

use interface::rref::{traits::CustomCleanup, traits::TypeIdentifiable, RRef};
use interface::typeid::DropMap;
use interface;

/// GEN
lazy_static! {
    pub static ref DROPPER: Dropper = {
        Dropper::new(DropMap::new())
    };
}
/// END GEN

// Drops the pointer, assumes it is of type T
fn drop_t<T: CustomCleanup + TypeIdentifiable>(ptr: *mut u8) {
    // println!("DROPPING {}", core::any::type_name::<T>());
    unsafe {
        let ptr_t: *mut T = transmute(ptr);
        // recursively invoke further shared heap deallocation in the tree of rrefs
        (&mut *ptr_t).cleanup();
    }
}

pub struct Dropper {
    drop_map: DropMap,
}

impl Dropper {
    fn new(drop_map: DropMap) -> Self {
        Self { drop_map }
    }

    pub fn drop(&self, type_id: u64, ptr: *mut u8) -> bool {
        if let Some(drop_fn) = self.drop_map.get_drop(type_id) {
            unsafe {(drop_fn)(ptr)};
            true
        } else {
            println!("NO REGISTERED DROP FOR type hash {}", type_id);
            false
        }
    }

    pub fn has_type(&self, type_id: u64) -> bool {
        self.drop_map.get_drop(type_id).is_some()
    }
}
