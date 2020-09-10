use hashbrown::HashMap;
use core::mem::transmute;
use crate::alloc::borrow::ToOwned;
use rref::type_hash;

use rref::{RRef, RRefArray, RRefDeque, traits::CustomCleanup};
use usr;

lazy_static! {
    pub static ref DROPPER: Dropper = {

        let mut drop_map = DropMap(HashMap::new());

        drop_map.add_type::<RRef<usize>>();
        drop_map.add_type::<usize>();
        drop_map.add_type::<[Option<RRef<[u8; 100]>>; 32]>();
        drop_map.add_type::<[u8; 100]>();

        Dropper::new(drop_map)
    };
}

fn drop_t<T: CustomCleanup>(ptr: *mut u8) {
    println!("DROPPING {}", core::any::type_name::<T>());
    unsafe {
        let ptr_t: *mut T = transmute(ptr);
        (&mut *ptr_t).cleanup();
    }
}

struct DropMap(HashMap<u64, fn (*mut u8) -> ()>);

impl DropMap {
    fn add_type<T: 'static + CustomCleanup>(&mut self) {
        let type_hash = type_hash::<T>();
        let type_erased_drop = drop_t::<T>;
        self.0.insert(type_hash, type_erased_drop);
    }

    fn get_drop(&self, type_hash: u64) -> Option<&fn (*mut u8) -> ()> {
        self.0.get(&type_hash)
    }

    fn print_types(&self) {
        // println!("--- start registered types ---");
        // for (type_name, _) in self.0.iter() {
        //     println!("{}", type_name);
        // }
        // println!("--- end registered types ---");
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

    pub fn drop(&self, type_hash: u64, ptr: *mut u8) {
        if let Some(drop_fn) = self.drop_map.get_drop(type_hash) {
            (drop_fn)(ptr);
        } else {
            println!("NO REGISTERED DROP FOR type hash {}", type_hash);
            self.drop_map.print_types();
        }
    }
}
