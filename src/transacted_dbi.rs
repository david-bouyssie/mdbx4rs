//! Convenience wrapper: transaction + DBI bundle.
//!
//! [`TransactedDbi`] mirrors the `TransactedDatabase` pattern from mdbx4s, providing
//! a shorthand for operations on a single DBI within a known transaction.

use crate::{Cursor, Dbi, Result};
use libmdbx::{
    DatabaseKind, Decodable, Stat, TableFlags, TransactionKind, WriteFlags, RW,
};

/// A convenience handle that bundles a [`Transaction`](crate::Transaction) reference
/// with a [`Dbi`] handle, so that DBI operations don't require passing both every time.
///
/// This mirrors the `TransactedDatabase` pattern from mdbx4s (Scala).
pub struct TransactedDbi<'env, K: TransactionKind, E: DatabaseKind> {
    txn: &'env crate::Transaction<'env, K, E>,
    dbi: Dbi<'env>,
}

impl<'env, K: TransactionKind, E: DatabaseKind> TransactedDbi<'env, K, E> {
    /// Create a new `TransactedDbi` from a transaction and a DBI handle.
    pub fn new(txn: &'env crate::Transaction<'env, K, E>, dbi: Dbi<'env>) -> Self {
        Self { txn, dbi }
    }

    /// Access the underlying transaction.
    pub fn txn(&self) -> &crate::Transaction<'env, K, E> {
        self.txn
    }

    /// Access the underlying DBI handle.
    pub fn dbi(&self) -> &Dbi<'env> {
        &self.dbi
    }

    /// Consume this wrapper and return the DBI handle.
    pub fn into_dbi(self) -> Dbi<'env> {
        self.dbi
    }

    // --- Read operations (available for both RO and RW) ---

    /// Get an item by key.
    pub fn get<Key>(&self, key: &[u8]) -> Result<Option<Key>>
    where
        Key: Decodable<'env>,
    {
        self.txn.get(&self.dbi, key)
    }

    /// Open a cursor on this DBI.
    pub fn cursor(&self) -> Result<Cursor<'env, K>> {
        self.txn.cursor(&self.dbi)
    }

    /// Get DBI statistics.
    pub fn stat(&self) -> Result<Stat> {
        self.txn.dbi_stat(&self.dbi)
    }

    /// Get DBI flags.
    pub fn flags(&self) -> Result<TableFlags> {
        self.txn.dbi_flags(&self.dbi)
    }
}

// --- Write operations (RW only) ---

impl<'env, E: DatabaseKind> TransactedDbi<'env, RW, E> {
    /// Store an item.
    pub fn put(
        &self,
        key: impl AsRef<[u8]>,
        data: impl AsRef<[u8]>,
        flags: WriteFlags,
    ) -> Result<()> {
        self.txn.put(&self.dbi, key, data, flags)
    }

    /// Store an item with default upsert semantics.
    pub fn upsert(&self, key: impl AsRef<[u8]>, data: impl AsRef<[u8]>) -> Result<()> {
        self.txn.put(&self.dbi, key, data, WriteFlags::UPSERT)
    }

    /// Delete an item by key (all values if DUPSORT).
    pub fn del(&self, key: impl AsRef<[u8]>) -> Result<bool> {
        self.txn.del(&self.dbi, key, None)
    }

    /// Delete a specific key/value pair.
    pub fn del_exact(&self, key: impl AsRef<[u8]>, data: &[u8]) -> Result<bool> {
        self.txn.del(&self.dbi, key, Some(data))
    }

    /// Empty this DBI (remove all items).
    pub fn clear(&self) -> Result<()> {
        self.txn.clear_dbi(&self.dbi)
    }

    /// Reserve space for a value. Returns a mutable slice to fill.
    pub fn reserve(
        &self,
        key: impl AsRef<[u8]>,
        len: usize,
        flags: WriteFlags,
    ) -> Result<&'env mut [u8]> {
        self.txn.reserve(&self.dbi, key, len, flags)
    }

    /// Atomically replace a value, returning the old value.
    pub fn replace(
        &self,
        key: &[u8],
        new_data: &[u8],
        flags: WriteFlags,
    ) -> Result<Option<Vec<u8>>> {
        self.txn.replace(&self.dbi, key, new_data, flags)
    }

    /// Atomically get and increment the DBI sequence counter.
    pub fn sequence(&self, increment: u64) -> Result<u64> {
        self.txn.dbi_sequence(&self.dbi, increment)
    }
}

// --- Sequence read (available for any K) ---

impl<'env, K: TransactionKind, E: DatabaseKind> TransactedDbi<'env, K, E> {
    /// Read the current DBI sequence value without incrementing.
    pub fn sequence_read(&self) -> Result<u64> {
        self.txn.dbi_sequence_read(&self.dbi)
    }
}

impl<K: TransactionKind, E: DatabaseKind> std::fmt::Debug for TransactedDbi<'_, K, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactedDbi").finish()
    }
}
