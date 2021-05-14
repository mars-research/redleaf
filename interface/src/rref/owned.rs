use super::traits::RRefable;
use super::RRef;

use core::ops::Deref;

/// `Owned` ensures proper access to interior `RRef`.
///  - When `Owned` is non-empty, it marks interior `RRef` as owned
///     by a parent `RRef`.
///  - When `Owned` is empty (that is, when interior `RRef` is taken out),
///     the interior `RRef` is marked as owned by the current domain.
pub struct Owned<T>
where
    T: 'static + RRefable,
{
    pub(crate) rref: Option<RRef<T>>,
}

impl<T> Owned<T>
where
    T: 'static + RRefable,
{
    pub fn new_empty() -> Self {
        let mut owned = Self { rref: None };
        owned
    }

    pub fn new(rref: RRef<T>) -> Self {
        let mut owned = Self { rref: None };
        owned.replace(rref);
        owned
    }

    /// Pulls out interior `RRef`, and marks it as owned by the current domain.
    pub fn take(&mut self) -> Option<RRef<T>> {
        match self.rref.take() {
            None => None,
            Some(rref) => {
                unsafe {
                    rref.move_to_current();
                }
                Some(rref)
            }
        }
    }

    /// Inserts interior `RRef`, and marks it as owned by a parent `RRef`.
    /// Note: in the case that `Owned` already had an `RRef`, that `RRef`
    ///     is returned, and marked as owned by the current domain.
    pub fn replace(&mut self, rref: RRef<T>) -> Option<RRef<T>> {
        unsafe {
            rref.move_to(0);
        }

        match self.take() {
            None => self.rref.replace(rref),
            // we were holding another `RRef`. Since `take()` already
            // manages ownership, we can just return the other `RRef`
            Some(other) => {
                self.rref.replace(rref);
                Some(other)
            }
        }
    }
}

impl<T: RRefable> Deref for Owned<T>
where
    T: 'static + RRefable,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.rref.as_ref().unwrap()
    }
}
