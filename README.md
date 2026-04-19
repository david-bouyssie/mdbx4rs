# mdbx4rs

A Rust wrapper around [libmdbx-rs](https://github.com/vorot93/libmdbx-rs) that restores C-aligned naming, fills API gaps via direct FFI to `mdbx-sys`, and adds convenience types.

Inspired by [mdbx4s](https://github.com/david-bouyssie/mdbx4s), its Scala counterpart.

## Motivation

[libmdbx-rs](https://crates.io/crates/libmdbx) provides excellent Rust-idiomatic bindings for [libmdbx](https://gitflic.ru/project/erthink/libmdbx), but renames some core C API concepts (`MDBX_env` → `Database`, `MDBX_dbi` → `Table`) and leaves several useful C functions unexposed. **mdbx4rs** wraps libmdbx-rs to:

- **Restore C-aligned naming** — `Env`, `Dbi`, `Transaction`, `Cursor` match the C API and the [mdbx4s](https://github.com/david-bouyssie/mdbx4s) Scala library.
- **Fill API gaps** — expose `mdbx_dbi_sequence`, `mdbx_replace`, `mdbx_txn_reset/renew`, `mdbx_env_copy`, `mdbx_cursor_count/renew`, `mdbx_txn_commit_ex`, `mdbx_txn_info`, and more via direct FFI to `mdbx-sys`.
- **Add convenience types** — `TransactedDbi` bundles a transaction with a DBI handle for ergonomic single-database operations.
- **Preserve all libmdbx-rs strengths** — type-safe `RO`/`RW` transactions, `WriteMap`/`NoWriteMap` enforcement, zero-copy reads via `Decodable`, cursor iterators, geometry API, reserve, and permanent table handles all flow through transparently via `Deref`/`DerefMut`.

## Naming

| mdbx4rs | libmdbx-rs | C API (libmdbx) |
|---------|-----------|-----------------|
| `Env<E>` | `Database<E>` | `MDBX_env` |
| `Dbi<'txn>` | `Table<'txn>` | `MDBX_dbi` |
| `Transaction<'env, K, E>` | `Transaction<'db, K, E>` | `MDBX_txn` |
| `Cursor<'txn, K>` | `Cursor<'txn, K>` | `MDBX_cursor` |
| `TransactedDbi<'env, K, E>` | *(none)* | convenience wrapper |

## Quick Start

```toml
[dependencies]
mdbx4rs = "0.1.0"
```

```rust
use mdbx4rs::*;

fn main() -> Result<()> {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::<NoWriteMap>::open_with_options(
        dir.path(),
        DatabaseOptions {
            max_tables: Some(16),
            ..Default::default()
        },
    )?;

    // Write
    {
        let txn = env.begin_rw_txn()?;
        let dbi = txn.create_dbi(Some("my_store"), TableFlags::default())?;
        let td = TransactedDbi::new(&txn, dbi);

        td.upsert(b"hello", b"world")?;

        // Auto-increment sequence
        let id = td.sequence(1)?; // returns 0, increments to 1

        txn.commit()?;
    }

    // Read
    {
        let txn = env.begin_ro_txn()?;
        let dbi = txn.open_dbi(Some("my_store"))?;
        let val: Option<Vec<u8>> = txn.get(&dbi, b"hello")?;
        assert_eq!(val.as_deref(), Some(b"world".as_slice()));
    }

    Ok(())
}
```

## Gap-Filled APIs

Features added on top of libmdbx-rs, via direct FFI to `mdbx-sys`:

| Method | C function | Description |
|--------|-----------|-------------|
| `Transaction::dbi_sequence()` | `mdbx_dbi_sequence` | Atomic auto-increment counter per DBI |
| `Transaction::replace()` | `mdbx_replace` | Atomic get-old-and-put-new |
| `Transaction::reset()` / `renew()` | `mdbx_txn_reset/renew` | RO transaction reuse for streaming pipelines |
| `Transaction::commit_with_latency()` | `mdbx_txn_commit_ex` | Commit with performance timing |
| `Transaction::txn_info()` | `mdbx_txn_info` | Transaction space/reader diagnostics |
| `Cursor::count()` | `mdbx_cursor_count` | Duplicate count for DUPSORT databases |
| `Cursor::renew()` | `mdbx_cursor_renew` | Cursor reuse across renewed transactions |
| `Env::copy()` | `mdbx_env_copy` | Database backup/snapshot |
| `Env::sync_poll()` / `sync_ex()` | `mdbx_env_sync_ex` | Non-blocking and parameterized sync |
| `Env::max_key_size()` | `mdbx_env_get_maxkeysize_ex` | Query max key size |
| `Env::get_flags()` | `mdbx_env_get_flags` | Environment flag introspection |
| `Env::max_dbs()` / `max_readers()` | `mdbx_env_get_option` | Configuration introspection |
| `Env::database_names()` | *(composite)* | Enumerate all named DBIs |

## Architecture

```
┌─────────────────────────────────────────┐
│  Your application                       │
├─────────────────────────────────────────┤
│  mdbx4rs                                │
│  ├── Deref/DerefMut to libmdbx-rs      │
│  ├── C-aligned naming                  │
│  ├── Direct FFI to mdbx-sys for gaps   │
│  └── TransactedDbi convenience wrapper  │
├─────────────────────────────────────────┤
│  libmdbx-rs  (upstream, untouched)      │
├─────────────────────────────────────────┤
│  mdbx-sys    (C FFI bindings)           │
└─────────────────────────────────────────┘
```

## License

Apache-2.0
