//! Environment handle (`MDBX_env`).
//!
//! Wraps [`libmdbx::Database<E>`] and restores the C-level "env" terminology.

use crate::{DatabaseOptions, Result, Transaction, error::mdbx_result};
use libmdbx::{DatabaseKind, RO, RW};
use std::ops::Deref;
use std::path::Path;
use std::ffi::CString;

/// An environment handle, corresponding to `MDBX_env` in the C API.
///
/// Wraps [`libmdbx::Database<E>`]. All libmdbx-rs methods on `Database` are accessible
/// via [`Deref`].
pub struct Env<E: DatabaseKind>(pub(crate) libmdbx::Database<E>);

impl<E: DatabaseKind> Deref for Env<E> {
    type Target = libmdbx::Database<E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// --- Construction ---

impl<E: DatabaseKind> Env<E> {
    /// Open an environment at the given path with default options.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        libmdbx::Database::<E>::open(path).map(Env)
    }

    /// Open an environment at the given path with custom options.
    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: DatabaseOptions,
    ) -> Result<Self> {
        libmdbx::Database::<E>::open_with_options(path, options).map(Env)
    }

    /// Wrap an existing [`libmdbx::Database`] handle.
    pub fn from_inner(inner: libmdbx::Database<E>) -> Self {
        Env(inner)
    }

    /// Consume this wrapper and return the inner [`libmdbx::Database`].
    pub fn into_inner(self) -> libmdbx::Database<E> {
        self.0
    }
}

// --- Transactions (return mdbx4rs types) ---

impl<E: DatabaseKind> Env<E> {
    /// Create a read-only transaction.
    pub fn begin_ro_txn(&self) -> Result<Transaction<'_, RO, E>> {
        self.0.begin_ro_txn().map(Transaction)
    }

    /// Create a read-write transaction.
    /// Blocks while other write transactions are open.
    pub fn begin_rw_txn(&self) -> Result<Transaction<'_, RW, E>> {
        self.0.begin_rw_txn().map(Transaction)
    }
}

// --- Gap-filling: env copy ---

impl<E: DatabaseKind> Env<E> {
    /// Copy the environment to the specified path.
    ///
    /// Wraps `mdbx_env_copy()`. This can be used to make a backup.
    /// The destination directory must already exist and be writable.
    ///
    /// `flags` is a bitmask of `MDBX_CP_*` constants (0 for defaults,
    /// `MDBX_CP_COMPACT` = 1 for compaction).
    pub fn copy(&self, dest: impl AsRef<Path>, flags: u32) -> Result<()> {
        let path_str = dest.as_ref().to_str().ok_or(libmdbx::Error::Invalid)?;
        let path = CString::new(path_str).map_err(|_| libmdbx::Error::Invalid)?;
        unsafe {
            mdbx_result(ffi::mdbx_env_copy(
                self.0.ptr().0,
                path.as_ptr(),
                flags as ffi::MDBX_copy_flags_t,
            ))?;
        }
        Ok(())
    }

    /// Copy flags: no special options (default).
    pub const CP_DEFAULTS: u32 = 0;
    /// Copy flags: compact while copying.
    pub const CP_COMPACT: u32 = ffi::MDBX_CP_COMPACT as u32;
}

// --- Gap-filling: sync_poll ---

impl<E: DatabaseKind> Env<E> {
    /// Non-blocking sync poll.
    ///
    /// Checks sync thresholds and performs a flush if one is reached,
    /// but never blocks waiting for a write transaction.
    pub fn sync_poll(&self) -> Result<bool> {
        mdbx_result(unsafe { ffi::mdbx_env_sync_ex(self.0.ptr().0, false, true) })
    }

    /// Sync with full control over `force` and `nonblock` parameters.
    pub fn sync_ex(&self, force: bool, nonblock: bool) -> Result<bool> {
        mdbx_result(unsafe { ffi::mdbx_env_sync_ex(self.0.ptr().0, force, nonblock) })
    }
}

// --- Gap-filling: env introspection ---

impl<E: DatabaseKind> Env<E> {
    /// Get the maximum key size for this environment.
    pub fn max_key_size(&self) -> Result<usize> {
        let flags = self.get_flags()?;
        let size = unsafe {
            ffi::mdbx_env_get_maxkeysize_ex(self.0.ptr().0, flags as ffi::MDBX_db_flags_t)
        };
        if size < 0 {
            Err(libmdbx::Error::Invalid)
        } else {
            Ok(size as usize)
        }
    }

    /// Get the environment flags.
    pub fn get_flags(&self) -> Result<u32> {
        let mut flags: u32 = 0;
        unsafe {
            mdbx_result(ffi::mdbx_env_get_flags(self.0.ptr().0, &mut flags))?;
        }
        Ok(flags)
    }

    /// Get the maximum number of named databases (DBIs).
    pub fn max_dbs(&self) -> Result<u64> {
        let mut value: u64 = 0;
        unsafe {
            mdbx_result(ffi::mdbx_env_get_option(
                self.0.ptr().0,
                ffi::MDBX_opt_max_db,
                &mut value,
            ))?;
        }
        Ok(value)
    }

    /// Get the maximum number of reader slots.
    pub fn max_readers(&self) -> Result<u64> {
        let mut value: u64 = 0;
        unsafe {
            mdbx_result(ffi::mdbx_env_get_option(
                self.0.ptr().0,
                ffi::MDBX_opt_max_readers,
                &mut value,
            ))?;
        }
        Ok(value)
    }
}

// --- Gap-filling: database name enumeration ---

impl<E: DatabaseKind> Env<E> {
    /// Enumerate all named database (DBI) names in this environment.
    ///
    /// Opens a read-only transaction, scans the anonymous DBI for keys, and
    /// interprets them as UTF-8 database names. Requires that the environment
    /// was opened with `max_tables` > 1.
    pub fn database_names(&self) -> Result<Vec<String>> {
        let txn = self.begin_ro_txn()?;
        let anonymous_dbi = txn.open_dbi(None)?;
        let mut cursor = txn.cursor(&anonymous_dbi)?;
        let mut names = Vec::new();

        let mut entry = cursor.first::<Vec<u8>, Vec<u8>>()?;
        while let Some((key, _)) = entry {
            if let Ok(name) = String::from_utf8(key) {
                names.push(name);
            }
            entry = cursor.next::<Vec<u8>, Vec<u8>>()?;
        }
        Ok(names)
    }
}

impl<E: DatabaseKind> std::fmt::Debug for Env<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Env").finish()
    }
}
