pub unsafe auto trait RRefable {}
impl<T> !RRefable for *mut T {}
impl<T> !RRefable for *const T {}
impl<T> !RRefable for &T {}
impl<T> !RRefable for &mut T {}
impl<T> !RRefable for [T] {}

pub trait CustomCleanup: RRefable {
    fn cleanup(&mut self);
}

// blanket implementation, overriden by RRef, RRefArray, RRefDeque
impl<T: RRefable> CustomCleanup for T {
    default fn cleanup(&mut self) {
        // no-op by default
        // println!("CustomCleanup::{}::cleanup()", core::any::type_name_of_val(self));
    }
}

// TODO: any other implementations?

impl<T: RRefable> CustomCleanup for Option<T> {
    fn cleanup(&mut self) {
        // println!("CustomCleanup::{}::cleanup()", core::any::type_name_of_val(self));
        if let Some(val) = self {
            // println!("is some, calling cleanup on {}", core::any::type_name_of_val(val));
            val.cleanup();
        }
    }
}

impl<T: RRefable, const N: usize> CustomCleanup for [T; N] {
    fn cleanup(&mut self) {
        // println!("CustomCleanup::{}::cleanup()", core::any::type_name_of_val(self));
        for el in self.iter_mut() {
            el.cleanup();
        }
    }
}
