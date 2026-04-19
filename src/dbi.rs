//! Database handle (`MDBX_dbi`).
//!
//! In the C API, a DBI ("database index") represents a named or anonymous sub-database
//! inside an environment. libmdbx-rs calls this `Table`; we restore the C-level name.

use std::ops::Deref;

/// A handle to an individual database (DBI) within an [`Env`](crate::Env).
///
/// Wraps [`libmdbx::Table<'txn>`]. All libmdbx-rs methods on `Table` are accessible
/// via [`Deref`].
///
/// In the C API this corresponds to an `MDBX_dbi` handle. libmdbx-rs calls it `Table`.
#[derive(Debug)]
pub struct Dbi<'txn>(pub(crate) libmdbx::Table<'txn>);

impl<'txn> Deref for Dbi<'txn> {
    type Target = libmdbx::Table<'txn>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'txn> Dbi<'txn> {
    /// Wrap an existing [`libmdbx::Table`] handle.
    pub fn from_inner(inner: libmdbx::Table<'txn>) -> Self {
        Dbi(inner)
    }

    /// Consume this wrapper and return the inner [`libmdbx::Table`].
    pub fn into_inner(self) -> libmdbx::Table<'txn> {
        self.0
    }

    /// Returns the raw DBI integer handle.
    pub fn raw_dbi(&self) -> ffi::MDBX_dbi {
        self.0.dbi()
    }
}
