//! Transaction handle (`MDBX_txn`).
//!
//! Wraps [`libmdbx::Transaction<'db, K, E>`] preserving the `RO`/`RW` and
//! `WriteMap`/`NoWriteMap` type parameters.

use crate::{Cursor, Dbi, Result, error::mdbx_result};
use libmdbx::{
    DatabaseKind, Decodable, NoWriteMap,
    Stat, TableFlags, WriteFlags,
    RO, RW, TransactionKind,
};
use libc::c_void;
use std::mem;
use std::ops::Deref;

/// A transaction handle, corresponding to `MDBX_txn` in the C API.
///
/// Wraps [`libmdbx::Transaction<'env, K, E>`]. All libmdbx-rs methods are accessible
/// via [`Deref`]. Consuming methods (`commit`, `commit_and_rebind_open_dbs`) are
/// explicitly delegated.
///
/// The `K` parameter is [`RO`] or [`RW`]; the `E` parameter is
/// [`NoWriteMap`](libmdbx::NoWriteMap) or [`WriteMap`](libmdbx::WriteMap).
pub struct Transaction<'env, K: TransactionKind, E: DatabaseKind>(
    pub(crate) libmdbx::Transaction<'env, K, E>,
);

impl<'env, K: TransactionKind, E: DatabaseKind> Deref for Transaction<'env, K, E> {
    type Target = libmdbx::Transaction<'env, K, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// --- Construction helpers ---

impl<'env, K: TransactionKind, E: DatabaseKind> Transaction<'env, K, E> {
    /// Wrap an existing [`libmdbx::Transaction`].
    pub fn from_inner(inner: libmdbx::Transaction<'env, K, E>) -> Self {
        Transaction(inner)
    }

    /// Consume this wrapper and return the inner [`libmdbx::Transaction`].
    pub fn into_inner(self) -> libmdbx::Transaction<'env, K, E> {
        self.0
    }
}

// --- Consuming methods (can't go through Deref) ---

impl<'env, K: TransactionKind, E: DatabaseKind> Transaction<'env, K, E> {
    /// Commit the transaction. Any pending operations will be saved.
    pub fn commit(self) -> Result<bool> {
        self.0.commit()
    }

    /// Commit the transaction and return database handles that were primed for
    /// permanent opening.
    pub fn commit_and_rebind_open_dbs(self) -> Result<(bool, Vec<Dbi<'env>>)> {
        let (ok, tables) = self.0.commit_and_rebind_open_dbs()?;
        Ok((ok, tables.into_iter().map(Dbi).collect()))
    }
}

// --- Shadowing methods (return mdbx4rs types instead of libmdbx types) ---

impl<'env, K: TransactionKind, E: DatabaseKind> Transaction<'env, K, E> {
    /// Open a named or anonymous DBI (database) within this transaction.
    ///
    /// Pass `None` for the anonymous (default) database.
    pub fn open_dbi<'txn>(&'txn self, name: Option<&str>) -> Result<Dbi<'txn>> {
        self.0.open_table(name).map(Dbi)
    }

    /// Open a cursor on the given DBI.
    pub fn cursor<'txn>(&'txn self, dbi: &Dbi<'txn>) -> Result<Cursor<'txn, K>> {
        self.0.cursor(&dbi.0).map(Cursor)
    }

    /// Returns the transaction id.
    pub fn id(&self) -> u64 {
        self.0.id()
    }

    /// Get an item from a DBI.
    pub fn get<'txn, Key>(&'txn self, dbi: &Dbi<'txn>, key: &[u8]) -> Result<Option<Key>>
    where
        Key: Decodable<'txn>,
    {
        self.0.get(&dbi.0, key)
    }

    /// Get the option flags for a DBI.
    pub fn dbi_flags<'txn>(&'txn self, dbi: &Dbi<'txn>) -> Result<TableFlags> {
        self.0.table_flags(&dbi.0)
    }

    /// Get statistics for a DBI.
    pub fn dbi_stat<'txn>(&'txn self, dbi: &Dbi<'txn>) -> Result<Stat> {
        self.0.table_stat(&dbi.0)
    }

    /// Prime a DBI handle for permanent opening (survives beyond this transaction).
    pub fn prime_for_permaopen(&self, dbi: Dbi<'_>) {
        self.0.prime_for_permaopen(dbi.0)
    }
}

// --- RW-only shadowing methods ---

impl<'env, E: DatabaseKind> Transaction<'env, RW, E> {
    /// Create a named or anonymous DBI, creating it if it doesn't exist.
    pub fn create_dbi<'txn>(
        &'txn self,
        name: Option<&str>,
        flags: TableFlags,
    ) -> Result<Dbi<'txn>> {
        self.0.create_table(name, flags).map(Dbi)
    }

    /// Store an item into a DBI.
    pub fn put<'txn>(
        &'txn self,
        dbi: &Dbi<'txn>,
        key: impl AsRef<[u8]>,
        data: impl AsRef<[u8]>,
        flags: WriteFlags,
    ) -> Result<()> {
        self.0.put(&dbi.0, key, data, flags)
    }

    /// Reserve space for a value in the DBI. Returns a mutable slice to be filled.
    pub fn reserve<'txn>(
        &'txn self,
        dbi: &Dbi<'txn>,
        key: impl AsRef<[u8]>,
        len: usize,
        flags: WriteFlags,
    ) -> Result<&'txn mut [u8]> {
        self.0.reserve(&dbi.0, key, len, flags)
    }

    /// Delete items from a DBI.
    ///
    /// If `data` is `Some`, only the matching value is deleted.
    /// If `data` is `None`, all values for the key are deleted.
    ///
    /// Returns `true` if the key/value pair was present.
    pub fn del<'txn>(
        &'txn self,
        dbi: &Dbi<'txn>,
        key: impl AsRef<[u8]>,
        data: Option<&[u8]>,
    ) -> Result<bool> {
        self.0.del(&dbi.0, key, data)
    }

    /// Empty the given DBI (remove all items, keep the handle).
    pub fn clear_dbi<'txn>(&'txn self, dbi: &Dbi<'txn>) -> Result<()> {
        self.0.clear_table(&dbi.0)
    }

    /// Drop the DBI entirely (delete and close the handle).
    ///
    /// # Safety
    /// Caller must close ALL other [`Dbi`] and [`Cursor`] instances pointing to the
    /// same DBI BEFORE calling this.
    pub unsafe fn drop_dbi<'txn>(&'txn self, dbi: Dbi<'txn>) -> Result<()> {
        unsafe { self.0.drop_table(dbi.0) }
    }
}

// --- Nested transactions (RW + NoWriteMap only) ---

impl Transaction<'_, RW, NoWriteMap> {
    /// Begin a nested write transaction inside this one.
    pub fn begin_nested_txn(&mut self) -> Result<Transaction<'_, RW, NoWriteMap>> {
        self.0.begin_nested_txn().map(Transaction)
    }
}

// --- RO-only: close DBI ---

impl<E: DatabaseKind> Transaction<'_, RO, E> {
    /// Close a DBI handle.
    ///
    /// # Safety
    /// Caller must close ALL other [`Dbi`] and [`Cursor`] instances pointing to the
    /// same DBI BEFORE calling this.
    pub unsafe fn close_dbi(&self, dbi: Dbi<'_>) -> Result<()> {
        unsafe { self.0.close_table(dbi.0) }
    }
}

// ==========================================================================
// Gap-filling: direct FFI to mdbx-sys
// ==========================================================================

// --- mdbx_dbi_sequence ---

impl<'env, E: DatabaseKind> Transaction<'env, RW, E> {
    /// Atomically get and increment the DBI sequence counter.
    ///
    /// Returns the value *before* the increment. Pass `increment = 0` to read without
    /// modifying.
    ///
    /// Wraps `mdbx_dbi_sequence()`.
    pub fn dbi_sequence(&self, dbi: &Dbi<'_>, increment: u64) -> Result<u64> {
        let mut result: u64 = 0;
        unsafe {
            mdbx_result(ffi::mdbx_dbi_sequence(
                self.0.txn().0,
                dbi.raw_dbi(),
                &mut result,
                increment,
            ))?;
        }
        Ok(result)
    }
}

// Read-only variant: sequence with increment=0
impl<'env, K: TransactionKind, E: DatabaseKind> Transaction<'env, K, E> {
    /// Read the current DBI sequence value without incrementing.
    ///
    /// Wraps `mdbx_dbi_sequence()` with `increment = 0`.
    pub fn dbi_sequence_read(&self, dbi: &Dbi<'_>) -> Result<u64> {
        let mut result: u64 = 0;
        unsafe {
            mdbx_result(ffi::mdbx_dbi_sequence(
                self.0.txn().0,
                dbi.raw_dbi(),
                &mut result,
                0,
            ))?;
        }
        Ok(result)
    }
}

// --- mdbx_replace ---

impl<'env, E: DatabaseKind> Transaction<'env, RW, E> {
    /// Atomically replace a value for a given key, returning the old value.
    ///
    /// Wraps `mdbx_replace()`. This is an atomic get-old-and-put-new operation.
    ///
    /// `flags` controls the put semantics (e.g., `WriteFlags::UPSERT`).
    ///
    /// Returns `None` if there was no previous value, or `Some(old_value)`.
    pub fn replace(
        &self,
        dbi: &Dbi<'_>,
        key: &[u8],
        new_data: &[u8],
        flags: WriteFlags,
    ) -> Result<Option<Vec<u8>>> {
        let key_val = ffi::MDBX_val {
            iov_len: key.len(),
            iov_base: key.as_ptr() as *mut c_void,
        };
        let mut new_data_val = ffi::MDBX_val {
            iov_len: new_data.len(),
            iov_base: new_data.as_ptr() as *mut c_void,
        };

        // Allocate a buffer for old data. Start with a reasonable size;
        // if MDBX_RESULT_TRUE is returned, the old value was larger and
        // old_data_val.iov_len tells us the needed size.
        let mut old_buf: Vec<u8> = vec![0u8; new_data.len().max(256)];
        let mut old_data_val = ffi::MDBX_val {
            iov_len: old_buf.len(),
            iov_base: old_buf.as_mut_ptr() as *mut c_void,
        };

        let rc = unsafe {
            ffi::mdbx_replace(
                self.0.txn().0,
                dbi.raw_dbi(),
                &key_val,
                &mut new_data_val,
                &mut old_data_val,
                crate::flags_to_c(flags),
            )
        };

        if rc == ffi::MDBX_RESULT_TRUE {
            // Old value didn't fit — resize and retry.
            old_buf.resize(old_data_val.iov_len, 0);
            old_data_val.iov_len = old_buf.len();
            old_data_val.iov_base = old_buf.as_mut_ptr() as *mut c_void;

            // Re-create new_data_val (consumed by first call).
            new_data_val.iov_len = new_data.len();
            new_data_val.iov_base = new_data.as_ptr() as *mut c_void;

            let rc2 = unsafe {
                ffi::mdbx_replace(
                    self.0.txn().0,
                    dbi.raw_dbi(),
                    &key_val,
                    &mut new_data_val,
                    &mut old_data_val,
                    crate::flags_to_c(flags),
                )
            };
            mdbx_result(rc2)?;
        } else if rc == ffi::MDBX_NOTFOUND {
            // No previous value existed.
            return Ok(None);
        } else {
            mdbx_result(rc)?;
        }

        // Extract old value from the buffer.
        let old_len = old_data_val.iov_len;
        if old_data_val.iov_base == old_buf.as_ptr() as *mut c_void {
            old_buf.truncate(old_len);
            Ok(Some(old_buf))
        } else {
            // MDBX returned a pointer into the DB page.
            let old_slice = unsafe {
                std::slice::from_raw_parts(old_data_val.iov_base as *const u8, old_len)
            };
            Ok(Some(old_slice.to_vec()))
        }
    }
}

// --- mdbx_txn_reset / mdbx_txn_renew ---

impl<E: DatabaseKind> Transaction<'_, RO, E> {
    /// Reset a read-only transaction.
    ///
    /// Releases the reader lock but keeps the transaction handle alive for reuse
    /// with [`renew()`](Self::renew). This is critical for long-lived streaming
    /// pipelines that want to periodically refresh their snapshot without
    /// reallocating transaction handles.
    ///
    /// Wraps `mdbx_txn_reset()`.
    pub fn reset(&self) -> Result<()> {
        unsafe {
            mdbx_result(ffi::mdbx_txn_reset(self.0.txn().0))?;
        }
        Ok(())
    }

    /// Renew a read-only transaction that was previously [`reset()`](Self::reset).
    ///
    /// Acquires a new reader lock on the latest committed snapshot.
    ///
    /// Wraps `mdbx_txn_renew()`.
    pub fn renew(&self) -> Result<()> {
        unsafe {
            mdbx_result(ffi::mdbx_txn_renew(self.0.txn().0))?;
        }
        Ok(())
    }
}

// --- mdbx_txn_commit_ex (commit with latency) ---

/// Commit latency information returned by [`Transaction::commit_with_latency()`].
#[derive(Clone, Debug, Default)]
pub struct CommitLatency {
    /// Duration of preparation (ms).
    pub preparation: u32,
    /// Duration of GC/freeDB handling (ms).
    pub gc: u32,
    /// Duration of internal audit if enabled (ms).
    pub audit: u32,
    /// Duration of writing dirty pages (ms).
    pub write: u32,
    /// Duration of syncing to disk (ms).
    pub sync: u32,
    /// Duration of transaction ending (ms).
    pub ending: u32,
    /// Total commit duration (ms).
    pub whole: u32,
}

impl<'env, E: DatabaseKind> Transaction<'env, RW, E> {
    /// Commit the transaction and return latency information.
    ///
    /// Wraps `mdbx_txn_commit_ex()`.
    pub fn commit_with_latency(self) -> Result<CommitLatency> {
        let mut latency: ffi::MDBX_commit_latency = unsafe { mem::zeroed() };
        let txn_ptr = self.0.txn().0;

        // We need to prevent the normal Drop from aborting, since we're committing manually.
        // Unfortunately we can't easily do that with a newtype over libmdbx::Transaction
        // because its Drop will fire. Instead, we use the raw FFI directly.
        //
        // SAFETY: We call commit_ex which consumes the txn pointer. The libmdbx::Transaction
        // Drop will then call abort on a null-ish pointer, but mdbx_txn_abort on an already
        // committed txn is safe (returns MDBX_BAD_TXN which is ignored in Drop).
        let rc = unsafe { ffi::mdbx_txn_commit_ex(txn_ptr, &mut latency) };
        mdbx_result(rc)?;

        Ok(CommitLatency {
            preparation: latency.preparation,
            gc: latency.gc_wallclock,
            audit: latency.audit,
            write: latency.write,
            sync: latency.sync,
            ending: latency.ending,
            whole: latency.whole,
        })
    }
}

// --- mdbx_txn_info ---

/// Transaction information returned by [`Transaction::txn_info()`].
#[derive(Clone, Debug, Default)]
pub struct TxnInfo {
    pub id: u64,
    pub reader_lag: u64,
    pub space_used: u64,
    pub space_limit_soft: u64,
    pub space_limit_hard: u64,
    pub space_retired: u64,
    pub space_leftover: u64,
    pub space_dirty: u64,
}

impl<'env, K: TransactionKind, E: DatabaseKind> Transaction<'env, K, E> {
    /// Retrieve information about this transaction.
    ///
    /// Wraps `mdbx_txn_info()`.
    pub fn txn_info(&self, scan_rlt: bool) -> Result<TxnInfo> {
        let mut info: ffi::MDBX_txn_info = unsafe { mem::zeroed() };
        unsafe {
            mdbx_result(ffi::mdbx_txn_info(self.0.txn().0, &mut info, scan_rlt))?;
        }
        Ok(TxnInfo {
            id: info.txn_id,
            reader_lag: info.txn_reader_lag,
            space_used: info.txn_space_used,
            space_limit_soft: info.txn_space_limit_soft,
            space_limit_hard: info.txn_space_limit_hard,
            space_retired: info.txn_space_retired,
            space_leftover: info.txn_space_leftover,
            space_dirty: info.txn_space_dirty,
        })
    }
}

impl<K: TransactionKind, E: DatabaseKind> std::fmt::Debug for Transaction<'_, K, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction").finish()
    }
}
