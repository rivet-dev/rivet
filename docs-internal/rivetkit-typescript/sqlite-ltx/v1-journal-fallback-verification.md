# V1 Journal Fallback Verification

**Verdict: CONFIRMED (with correction)**

The v1 VFS always uses the journal-mode write path. However the mechanism is not "SQLite tries atomic, gets SQLITE_IOERR, falls back to journal." It is simpler: the batch-atomic path is never attempted because `SQLITE_ENABLE_BATCH_ATOMIC_WRITE` is not defined at compile time.

## Evidence

### 1. Compile-time guard disables atomic batch path entirely

`libsqlite3-sys 0.30` with `bundled` does not define `SQLITE_ENABLE_BATCH_ATOMIC_WRITE`. In the SQLite amalgamation (`sqlite3.c`), all batch-atomic logic is inside `#ifdef SQLITE_ENABLE_BATCH_ATOMIC_WRITE` (line 63696). Without the define, `bBatch` is hardcoded to 0 (line 63610), and SQLite never calls `BEGIN_ATOMIC_WRITE`, `COMMIT_ATOMIC_WRITE`, or `ROLLBACK_ATOMIC_WRITE`.

### 2. VFS atomic handlers are dead code

The VFS at `vfs.rs:1011-1091` handles `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE`, `COMMIT_ATOMIC_WRITE`, and `ROLLBACK_ATOMIC_WRITE`, and reports `SQLITE_IOCAP_BATCH_ATOMIC` from `device_characteristics` (line 1102). None of this code is ever invoked because the SQLite pager skips the batch path at compile time.

### 3. Every write goes through the unbatched kv_put path

With `batch_mode` always false, `kv_io_write` (line 645) falls through to the non-batch path (line 709+), issuing one `kv_put` per `xWrite` call. Journal file writes use the same path because `resolve_file_tag` maps `-journal` to `FILE_TAG_JOURNAL` (line 256), and `kv_vfs_open` opens journal files with the same KV-backed `kv_io_write`.

### 4. The 287-put count matches the journal path

For 1 MiB inserted into a fresh database with 4 KiB chunks: 256 new data pages + journal header pages + metadata updates = ~287 puts. This is consistent with the journal path. If atomic writes had worked, it would be 1-2 puts total.

### 5. SQLite does have journal fallback (for future reference)

If `SQLITE_ENABLE_BATCH_ATOMIC_WRITE` were defined and `COMMIT_ATOMIC_WRITE` returned `SQLITE_IOERR`, SQLite would: (a) call `ROLLBACK_ATOMIC_WRITE` (line 63712), (b) create the journal via `sqlite3JournalCreate` (line 63717), (c) set `bBatch = 0` (line 63722), and (d) retry through `pager_write_pagelist` on the journal path (line 63729). This is the "future versions" retry the tech-note alludes to, and it exists in current SQLite source.

## Conclusion

The v1 VFS dirty-page overflow logic at `vfs.rs:1028-1033` and the `SQLITE_IOCAP_BATCH_ATOMIC` capability flag are inert. All writes always go through the journal path because the compile-time flag is missing. Enabling `SQLITE_ENABLE_BATCH_ATOMIC_WRITE` in the `libsqlite3-sys` build would activate the batch path and the overflow fallback.

Closes task #12.
