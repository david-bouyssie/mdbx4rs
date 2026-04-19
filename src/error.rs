//! Error helpers.
//!
//! Re-exports the `mdbx_result` helper from libmdbx's internals, and provides
//! any additional error utilities needed by mdbx4rs.

use libc::c_int;

/// Convert a raw libmdbx C return code into a `Result<bool>`.
///
/// - `MDBX_SUCCESS` (0) → `Ok(false)`
/// - `MDBX_RESULT_TRUE` (1) → `Ok(true)`
/// - anything else → `Err(Error::from_err_code(rc))`
pub fn mdbx_result(err_code: c_int) -> crate::Result<bool> {
    match err_code {
        ffi::MDBX_SUCCESS => Ok(false),
        ffi::MDBX_RESULT_TRUE => Ok(true),
        other => Err(libmdbx::Error::from_err_code(other)),
    }
}
