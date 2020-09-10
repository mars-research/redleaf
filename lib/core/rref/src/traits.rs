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
