//! Module implementing various iterators
//! needed by an [`Index`] hash table.
//!
//! [`Index`]: struct.Index.html

use super::Bucket;

use core::cell::{Ref, RefMut};

/// Iterator over the entries of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and returns
/// immutable references to the key-value pairs contained in `Bucket:Some(_)` options.
///
/// [`Index`]: struct.Index.html
pub struct Iter<'a, K, V> {
    buckets: &'a [Bucket<K, V>],
    buckets_len: usize,
    counter: usize,
}

impl<K, V> Iter<'_, K, V> {
    /// Creates a new iterator over the buckets in the
    /// slice passed as an immutable reference.
    pub fn new(buckets: &[Bucket<K, V>]) -> Iter<K, V> {
        Iter {
            buckets,
            buckets_len: buckets.len(),
            counter: 0,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = Ref<'a, (K, V)>; // immutable reference from a RefCell

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter < self.buckets_len {
            match &self.buckets[self.counter] {
                Bucket::Some(pair) => {
                    // returns borrowed pair
                    self.counter += 1;
                    Some(pair.borrow())
                }

                Bucket::None => {
                    // ignores empty bucket
                    self.counter += 1;
                    self.next()
                }
            }
        } else {
            // end of iterator
            None
        }
    }
}

/// Mutable iterator over the entries of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and returns
/// mutable references to the key-value pairs contained in `Bucket:Some(_)` options.
///
/// [`Index`]: struct.Index.html
pub struct IterMut<'a, K, V> {
    buckets: &'a [Bucket<K, V>],
    buckets_len: usize,
    counter: usize,
}

impl<K, V> IterMut<'_, K, V> {
    /// Creates a new iterator over the buckets in the
    /// slice passed as an immutable reference. The interior mutability
    /// of the `RefCell`s is taken advantage of inside the `next` method.
    pub fn new(buckets: &[Bucket<K, V>]) -> IterMut<K, V> {
        IterMut {
            buckets,
            buckets_len: buckets.len(),
            counter: 0,
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = RefMut<'a, (K, V)>; // mutable reference from a RefCell

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter < self.buckets_len {
            match &self.buckets[self.counter] {
                Bucket::Some(pair) => {
                    // returns mutably borrowed pair
                    self.counter += 1;
                    Some(pair.borrow_mut())
                }

                Bucket::None => {
                    // ignoring empty bucket
                    self.counter += 1;
                    self.next()
                }
            }
        } else {
            // end of iterator
            None
        }
    }
}

/// Iterator over the keys of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and returns
/// immutable references to the keys contained in `Bucket:Some(_)` options.
///
/// [`Index`]: struct.Index.html
pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<K, V> Keys<'_, K, V> {
    /// Creates a new iterator over the keys in the
    /// slice passed as an immutable reference.
    pub fn new(buckets: &[Bucket<K, V>]) -> Keys<K, V> {
        Keys {
            inner: Iter::new(buckets),
        }
    }
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = Ref<'a, K>; // immutable reference from a RefCell

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pair) = self.inner.next() {
            // returns borrowed key
            Some(Ref::map(pair, |t| &t.0))
        } else {
            // end of iterator
            None
        }
    }
}

/// Iterator over the values of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and returns
/// immutable references to the values contained in `Bucket:Some(_)` options.
///
/// [`Index`]: struct.Index.html
pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<K, V> Values<'_, K, V> {
    /// Creates a new iterator over the values in the
    /// slice passed as an immutable reference.
    pub fn new(buckets: &[Bucket<K, V>]) -> Values<K, V> {
        Values {
            inner: Iter::new(buckets),
        }
    }
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = Ref<'a, V>; // immutable reference from a RefCell

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pair) = self.inner.next() {
            // returns borrowed value
            Some(Ref::map(pair, |t| &t.1))
        } else {
            // end of iterator
            None
        }
    }
}

/// Mutable iterator over the values of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and returns
/// mutable references to the values contained in `Bucket:Some(_)` options
/// before returning them as simple key-value pairs.
///
/// [`Index`]: struct.Index.html
pub struct ValuesMut<'a, K, V> {
    inner: IterMut<'a, K, V>,
}

impl<K, V> ValuesMut<'_, K, V> {
    /// Creates a new iterator over the values in the
    /// slice passed as an immutable reference. The interior mutability
    /// of the `RefCell`s is taken advantage of inside the `next` method.
    pub fn new(buckets: &[Bucket<K, V>]) -> ValuesMut<K, V> {
        ValuesMut {
            inner: IterMut::new(buckets),
        }
    }
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = RefMut<'a, V>; // mutable reference from a RefCell

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pair) = self.inner.next() {
            // returns mutably borrowed value
            Some(RefMut::map(pair, |t| &mut t.1))
        } else {
            // end of iterator
            None
        }
    }
}

/// Iterator taking ownership of the entries of an [`Index`] hash table.
///
/// The iterator ignores `Bucket::None` options and moves entries
/// out of their `Bucket:Some(_)` options and `RefCell`s.
///
/// The `Drain` also updates the `len` field of the [`Index`] as it moves
/// out it's content.
///
/// [`Index`]: struct.Index.html
pub struct Drain<'a, K, V> {
    buckets: &'a mut [Bucket<K, V>],
    buckets_len: usize,
    index_len: &'a mut usize,
    counter: usize,
}

impl<K, V> Drain<'_, K, V> {
    /// Creates a new iterator over the values in the
    /// slice passed as a mutable reference (since it will be moving out the entries
    /// and replacing them with empty buckets). It also takes a mutable reference to the
    /// `len` field of the associated [`Index`] since it needs to update it when removing
    /// entries.
    ///
    /// [`Index`]: struct.Index.html
    pub fn new<'a>(buckets: &'a mut [Bucket<K, V>], index_len: &'a mut usize) -> Drain<'a, K, V> {
        let buckets_len = buckets.len();
        Drain {
            buckets,
            buckets_len,
            index_len,
            counter: 0,
        }
    }
}

impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V); // moved out key-value pair

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter < self.buckets_len {
            match &self.buckets[self.counter] {
                Bucket::Some(_) => {
                    // returns moved out entry
                    let removed = core::mem::replace(&mut self.buckets[self.counter], Bucket::None); // replacing with empty bucket
                    let removed = removed.unwrap(); // safe because we know from match that it's a Some option
                    self.counter += 1;
                    *self.index_len -= 1; // updating len field of index
                    Some(removed.into_inner()) // moving pair out of the RefCell
                }

                Bucket::None => {
                    // ignores empty bucket
                    self.counter += 1;
                    self.next()
                }
            }
        } else {
            // end of iterator
            None
        }
    }
}
