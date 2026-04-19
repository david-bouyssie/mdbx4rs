//! Cursor handle (`MDBX_cursor`).
//!
//! Wraps [`libmdbx::Cursor<'txn, K>`] preserving the `RO`/`RW` type parameter.

use crate::{Result, Transaction, error::mdbx_result};
use libmdbx::{DatabaseKind, Decodable, IntoIter, TransactionKind};
use std::ops::{Deref, DerefMut};

/// A cursor for navigating items within a [`Dbi`].
///
/// Wraps [`libmdbx::Cursor<'txn, K>`]. All libmdbx-rs methods (navigation, iteration,
/// put, del) are accessible via [`Deref`]/[`DerefMut`].
pub struct Cursor<'txn, K: TransactionKind>(pub(crate) libmdbx::Cursor<'txn, K>);

impl<'txn, K: TransactionKind> Deref for Cursor<'txn, K> {
    type Target = libmdbx::Cursor<'txn, K>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'txn, K: TransactionKind> DerefMut for Cursor<'txn, K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// --- Construction helpers ---

impl<'txn, K: TransactionKind> Cursor<'txn, K> {
    /// Wrap an existing [`libmdbx::Cursor`].
    pub fn from_inner(inner: libmdbx::Cursor<'txn, K>) -> Self {
        Cursor(inner)
    }

    /// Consume this wrapper and return the inner [`libmdbx::Cursor`].
    pub fn into_inner(self) -> libmdbx::Cursor<'txn, K> {
        self.0
    }
}

// --- Consuming methods (can't go through Deref) ---

impl<'txn, K: TransactionKind> Cursor<'txn, K> {
    /// Position at first key/data item and return an owned iterator.
    pub fn into_iter_start<Key, Value>(self) -> IntoIter<'txn, K, Key, Value>
    where
        Key: Decodable<'txn>,
        Value: Decodable<'txn>,
    {
        self.0.into_iter_start()
    }

    /// Position at the given key (or the first key >= it) and return an owned iterator.
    pub fn into_iter_from<Key, Value>(self, key: &[u8]) -> IntoIter<'txn, K, Key, Value>
    where
        Key: Decodable<'txn>,
        Value: Decodable<'txn>,
    {
        self.0.into_iter_from(key)
    }

    /// Position at the given key and return an owned iterator over its duplicates.
    pub fn into_iter_dup_of<Key, Value>(self, key: &[u8]) -> IntoIter<'txn, K, Key, Value>
    where
        Key: Decodable<'txn>,
        Value: Decodable<'txn>,
    {
        self.0.into_iter_dup_of(key)
    }
}

// ==========================================================================
// Gap-filling: direct FFI to mdbx-sys
// ==========================================================================

// --- mdbx_cursor_count ---

impl<K: TransactionKind> Cursor<'_, K> {
    /// Return the count of duplicates for the current key.
    ///
    /// Only valid on databases opened with `DUPSORT`.
    ///
    /// Wraps `mdbx_cursor_count()`.
    pub fn count(&self) -> Result<usize> {
        let mut count: usize = 0;
        unsafe {
            mdbx_result(ffi::mdbx_cursor_count(self.0.cursor().0, &mut count))?;
        }
        Ok(count)
    }
}

// --- mdbx_cursor_renew ---

impl<K: TransactionKind> Cursor<'_, K> {
    /// Renew a cursor, associating it with a new transaction.
    ///
    /// This allows reusing a cursor handle with a renewed read-only transaction,
    /// avoiding allocation overhead. The cursor must reference the same DBI as
    /// when it was created.
    ///
    /// Wraps `mdbx_cursor_renew()`.
    pub fn renew<E: DatabaseKind>(&mut self, txn: &Transaction<'_, K, E>) -> Result<()> {
        unsafe {
            mdbx_result(ffi::mdbx_cursor_renew(txn.0.txn().0, self.0.cursor().0))?;
        }
        Ok(())
    }
}

impl<K: TransactionKind> std::fmt::Debug for Cursor<'_, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor").finish()
    }
}
