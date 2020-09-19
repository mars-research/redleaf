pub unsafe auto trait RRefable {}
impl<T> !RRefable for *mut T {}
impl<T> !RRefable for *const T {}
impl<T> !RRefable for &T {}
impl<T> !RRefable for &mut T {}
impl<T> !RRefable for [T] {}

pub trait TypeIdentifiable {
    fn type_id() -> u64;
}

// TODO: replace this blanket implementation with generated type ids
impl<T> TypeIdentifiable for T {
    default fn type_id() -> u64 {
        let type_str = core::any::type_name::<T>();
        let mut hash = 5381u64;
        for byte in type_str.bytes() {
            hash = hash.wrapping_mul(33) ^ (byte as u64);
        }
        hash
    }
}

pub trait CustomCleanup: RRefable {
    fn cleanup(&mut self);
}

// blanket implementation, overriden by RRef, RRefArray, RRefDeque
impl<T: RRefable> CustomCleanup for T {
    default fn cleanup(&mut self) {
        // no-op by default
    }
}

// TODO: any other implementations?

impl<T: RRefable> CustomCleanup for Option<T> {
    fn cleanup(&mut self) {
        if let Some(val) = self {
            val.cleanup();
        }
    }
}

impl<T: RRefable, const N: usize> CustomCleanup for [T; N] {
    fn cleanup(&mut self) {
        for el in self.iter_mut() {
            el.cleanup();
        }
    }
}
