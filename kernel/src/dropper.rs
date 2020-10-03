use hashbrown::HashMap;
use core::mem::transmute;
use crate::alloc::borrow::ToOwned;

use rref::{RRef, RRefArray, RRefDeque, traits::CustomCleanup, traits::TypeIdentifiable};
use usr;

/// GEN
lazy_static! {
    pub static ref DROPPER: Dropper = {

        let mut drop_map = DropMap(HashMap::new());

        // dom a/b
        drop_map.add_type::<RRef<usize>>();
        drop_map.add_type::<usize>();
        drop_map.add_type::<[u8; 100]>();
        drop_map.add_type::<[Option<RRef<[u8; 100]>>; 32]>();

        // xv6fs
        drop_map.add_type::<[u8; usr::bdev::BSIZE]>();
        drop_map.add_type::<u8>();

        // benchnet
        drop_map.add_type::<[u8; 1514]>();
        drop_map.add_type::<[Option<RRef<[u8; 1514]>>; 32]>();
        drop_map.add_type::<[Option<RRef<[u8; 1514]>>; 512]>();
        drop_map.add_type::<[Option<RRef<usr::bdev::BlkReq>>; 128]>();
        drop_map.add_type::<[Option<RRef<usr::bdev::BlkReq>>; 1024]>();
        drop_map.add_type::<usr::bdev::BlkReq>();

        Dropper::new(drop_map)
    };
}
/// END GEN

// Drops the pointer, assumes it is of type T
fn drop_t<T: CustomCleanup + TypeIdentifiable>(ptr: *mut u8) {
    println!("DROPPING {}", core::any::type_name::<T>());
    unsafe {
        let ptr_t: *mut T = transmute(ptr);
        // recursively invoke further shared heap deallocation in the tree of rrefs
        (&mut *ptr_t).cleanup();
    }
}

struct DropMap(HashMap<u64, fn (*mut u8) -> ()>);

impl DropMap {
    fn add_type<T: 'static + CustomCleanup + TypeIdentifiable> (&mut self) {
        let type_id = T::type_id();
        let type_erased_drop = drop_t::<T>;
        self.0.insert(type_id, type_erased_drop);
    }

    fn get_drop(&self, type_id: u64) -> Option<&fn (*mut u8) -> ()> {
        self.0.get(&type_id)
    }
}

pub struct Dropper {
    drop_map: DropMap,
}

impl Dropper {
    fn new(drop_map: DropMap) -> Self {
        Self {
            drop_map
        }
    }

    pub fn drop(&self, type_id: u64, ptr: *mut u8) -> bool {
        if let Some(drop_fn) = self.drop_map.get_drop(type_id) {
            (drop_fn)(ptr);
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
