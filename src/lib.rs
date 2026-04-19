//! # mdbx4rs
//!
//! A wrapper around [libmdbx](https://docs.rs/libmdbx) that restores C-aligned naming,
//! fills API gaps via direct FFI to `mdbx-sys`, and adds convenience types.
//!
//! ## Naming
//!
//! | mdbx4rs | libmdbx-rs | C API (`libmdbx`) |
//! |---------|-----------|-------------------|
//! | [`Env`] | `Database` | `MDBX_env` |
//! | [`Dbi`] | `Table` | `MDBX_dbi` |
//! | [`Transaction`] | `Transaction` | `MDBX_txn` |
//! | [`Cursor`] | `Cursor` | `MDBX_cursor` |
//! | [`TransactedDbi`] | *(none)* | convenience wrapper |
//!
//! All libmdbx-rs features (type-safe `RO`/`RW`, `WriteMap`, `Decodable`, cursor iterators,
//! geometry, reserve, permanent handles) are accessible transparently via `Deref`/`DerefMut`.

#![allow(clippy::type_complexity)]

mod cursor;
mod dbi;
mod env;
mod error;
mod transacted_dbi;
mod transaction;

// Re-export our wrapper types as the primary API.
pub use crate::{
    cursor::Cursor,
    dbi::Dbi,
    env::Env,
    transacted_dbi::TransactedDbi,
    transaction::Transaction,
};

// Re-export libmdbx-rs types that users need but we don't wrap.
pub use libmdbx::{
    DatabaseKind, NoWriteMap, WriteMap,
    Decodable,
    Stat, Info, PageSize,
    DatabaseOptions, Mode, ReadWriteOptions, SyncMode,
    TableFlags, WriteFlags,
    IntoIter, Iter, IterDup,
};
pub use libmdbx::{RO, RW, TransactionKind};

// Re-export the error types.
pub use libmdbx::Error;
pub type Result<T> = libmdbx::Result<T>;

// Re-export gap-filling types from our transaction module.
pub use crate::transaction::{CommitLatency, TxnInfo};

// --- Internal helpers ---

/// Convert `WriteFlags` to the C enum type expected by `mdbx-sys`.
/// On Unix this is `u32`; on Windows it's `i32`.
#[cfg(not(windows))]
#[inline(always)]
pub(crate) fn flags_to_c(flags: WriteFlags) -> ffi::MDBX_put_flags_t {
    flags.bits()
}

#[cfg(windows)]
#[inline(always)]
pub(crate) fn flags_to_c(flags: WriteFlags) -> ffi::MDBX_put_flags_t {
    flags.bits() as i32
}
