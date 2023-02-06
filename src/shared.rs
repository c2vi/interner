use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::pool::{Pool, PoolKindSealed};
use crate::{GlobalPool, PoolKind, Pooled};

#[cfg(feature = "fnv")]
use fnv::FnvBuildHasher as DefaultHasher;
#[cfg(not(feature = "fnv"))]
use std::collections::hash_map::RandomState as DefaultHasher;

/// A pooled string that belongs to a [`StringPool`].
pub type SharedString<S = DefaultHasher> = Pooled<SharedPool<String, S>, S>;
/// A pooled path that belongs to a [`PathPool`].
pub type SharedPath<S = DefaultHasher> = Pooled<SharedPool<PathBuf, S>, S>;
/// A pooled buffer that belongs to a [`BufferPool`].
pub type SharedBuffer<S = DefaultHasher> = Pooled<SharedPool<Vec<u8>, S>, S>;

/// A string interning pool that manages [`SharedString`]s.
///
/// Each [`StringPool`] has its own storage. When comparing [`SharedString`]s
/// from separate pools, the full string comparison function must be used.
pub type StringPool<S = DefaultHasher> = SharedPool<String, S>;
/// A path interning pool that manages [`SharedPath`]s.
///
/// Each [`PathPool`] has its own storage. When comparing [`SharedPath`]s
/// from separate pools, the full string comparison function must be used.
pub type PathPool<S = DefaultHasher> = SharedPool<PathBuf, S>;
/// A path interning pool that manages [`SharedBuffer`]s.
///
/// Each [`BufferPool`] has its own storage. When comparing [`SharedBuffer`]s
/// from separate pools, the full string comparison function must be used.
pub type BufferPool<S = DefaultHasher> = SharedPool<Vec<u8>, S>;

/// A shared pool of values that ensures only one copy of any given value exists
/// at any time.
///
/// To retrieve a [`Pooled`] value, use [`SharedPool::get()`] or
/// [`SharedPool::get_from_owned`], which are implemented for these types:
///
/// - [`String`]/[`&str`](str)
/// - [`PathBuf`]/[`&Path`](Path)
/// - [`Vec<u8>`]/`&[u8]`
#[derive(Debug)]
pub struct SharedPool<T, S = DefaultHasher>(Arc<Mutex<Pool<Self, S>>>)
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher;

impl<S> SharedPool<String, S>
where
    S: BuildHasher,
{
    /// Creates a new pool using the provided [`BuildHasher`] for hashing
    /// values.
    #[must_use]
    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(0, hasher)
    }

    /// Creates a new pool using the provided [`BuildHasher`] for hashing
    /// values. The pool will have enough capacity to allow inserting
    /// `initial_capacity` pooled entries without reallocation.
    #[must_use]
    pub fn with_capacity_and_hasher(initial_capacity: usize, hasher: S) -> Self {
        Self(Arc::new(Mutex::new(Pool::with_capacity_and_hasher(
            initial_capacity,
            hasher,
        ))))
    }

    /// Returns a copy of an existing [`SharedString`] if one is found.
    /// Otherwise, a new [`SharedString`] is created and returned.
    ///
    /// While any copies of the returned [`SharedString`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same string.
    #[must_use]
    pub fn get(&self, borrowed: &str) -> SharedString<S> {
        self.with_active_symbols(|symbols| symbols.get(Cow::Borrowed(borrowed), self))
    }

    /// Returns a copy of an existing [`SharedString`] if one is found.
    /// Otherwise, a new [`SharedString`] is created and returned.
    ///
    /// While any copies of the returned [`SharedString`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same string.
    #[must_use]
    pub fn get_from_owned(&self, owned: String) -> SharedString<S> {
        self.with_active_symbols(|symbols| symbols.get::<str>(Cow::Owned(owned), self))
    }
}

impl<S> SharedPool<PathBuf, S>
where
    S: BuildHasher,
{
    /// Returns a copy of an existing [`SharedPath`] if one is found. Otherwise,
    /// a new [`SharedPath`] is created and returned.
    ///
    /// While any copies of the returned [`SharedPath`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same path.
    #[must_use]
    pub fn get(&self, borrowed: &Path) -> SharedPath<S> {
        self.with_active_symbols(|symbols| symbols.get(Cow::Borrowed(borrowed), self))
    }

    /// Returns a copy of an existing [`SharedPath`] if one is found. Otherwise,
    /// a new [`SharedPath`] is created and returned.
    ///
    /// While any copies of the returned [`SharedPath`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same path.
    #[must_use]
    pub fn get_from_owned(&self, owned: PathBuf) -> SharedPath<S> {
        self.with_active_symbols(|symbols| symbols.get::<Path>(Cow::Owned(owned), self))
    }
}

impl<S> SharedPool<Vec<u8>, S>
where
    S: BuildHasher,
{
    /// Returns a copy of an existing [`SharedBuffer`] if one is found. Otherwise,
    /// a new [`SharedBuffer`] is created and returned.
    ///
    /// While any copies of the returned [`SharedBuffer`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same buffer.
    #[must_use]
    pub fn get(&self, borrowed: &[u8]) -> SharedBuffer<S> {
        self.with_active_symbols(|symbols| symbols.get(Cow::Borrowed(borrowed), self))
    }

    /// Returns a copy of an existing [`SharedBuffer`] if one is found. Otherwise,
    /// a new [`SharedBuffer`] is created and returned.
    ///
    /// While any copies of the returned [`SharedBuffer`] are still allocated,
    /// calling this function is guaranteed to return a copy of the same buffer.
    #[must_use]
    pub fn get_from_owned(&self, owned: Vec<u8>) -> SharedBuffer<S> {
        self.with_active_symbols(|symbols| symbols.get::<[u8]>(Cow::Owned(owned), self))
    }
}

impl<T, S> Clone for SharedPool<T, S>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, S> PoolKind<S> for SharedPool<T, S>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
}

impl<T, S> PoolKindSealed<S> for SharedPool<T, S>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
    type Stored = T;

    fn with_active_symbols<R>(&self, logic: impl FnOnce(&mut Pool<Self, S>) -> R) -> R {
        let mut symbols = self.0.lock().expect("poisoned");

        logic(&mut symbols)
    }
}

impl<T, S> PartialEq for SharedPool<T, S>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
    fn eq(&self, other: &SharedPool<T, S>) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T, S> PartialEq<GlobalPool<T>> for SharedPool<T, S>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
    fn eq(&self, _other: &GlobalPool<T>) -> bool {
        false
    }
}

impl<T, S> PartialEq<SharedPool<T, S>> for GlobalPool<T>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
    S: BuildHasher,
{
    fn eq(&self, _other: &SharedPool<T, S>) -> bool {
        false
    }
}

impl<T> Default for SharedPool<T, DefaultHasher>
where
    T: Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd,
{
    fn default() -> Self {
        Self(Arc::default())
    }
}