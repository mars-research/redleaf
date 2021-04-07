// This file is for the dummy typeid trait.
// This allows `cargo expands` to run without proper typeid generation.
// Changing this file will not change the generated typeid.

pub trait TypeIdentifiable {
    fn type_id() -> u64 { 123_456_789 }
}

impl<T> TypeIdentifiable for T { }