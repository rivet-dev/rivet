# SQLite BATCH_ATOMIC_WRITE: Internal Reference

This document covers the internals of SQLite's `SQLITE_IOCAP_BATCH_ATOMIC`
mechanism and its fallback behavior. It is intended as a reference for anyone
working on the KV-backed SQLite VFS.

## Overview

`SQLITE_IOCAP_BATCH_ATOMIC` is a VFS device characteristic flag that tells
SQLite the underlying storage can atomically write multiple pages in one
operation. When a VFS declares this capability, SQLite changes its commit
strategy to eliminate external journal file I/O.

The feature was introduced in SQLite 3.21.0 (2017-10-24) for Android's F2FS
filesystem. It is compiled into the wa-sqlite WASM binary used by
`@rivetkit/sqlite` via the `SQLITE_ENABLE_BATCH_ATOMIC_WRITE` compile flag.

## The protocol

Three `xFileControl` opcodes define the VFS contract:

| Opcode | Value | VFS action |
|--------|-------|------------|
| `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE` | 31 | Enter batch mode. Buffer subsequent xWrite calls. |
| `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` | 32 | Atomically persist all buffered writes. |
| `SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE` | 33 | Discard all buffered writes. |

### Batch eligibility

SQLite uses the batch-atomic path only when all of these conditions are met
(checked in `sqlite3PagerCommitPhaseOne()` in pager.c):

1. The transaction involves a single database (no super-journal / multi-db tx).
2. `xDeviceCharacteristics()` includes `SQLITE_IOCAP_BATCH_ATOMIC`.
3. `pPager->noSync` is false (i.e., `synchronous` is not OFF).
4. The journal is still in memory (hasn't been spilled to disk due to memory
   pressure).

If any condition fails, SQLite uses the standard journal-to-disk path.

**Important**: condition 3 means `PRAGMA synchronous = OFF` disables
BATCH_ATOMIC. The VFS must use `synchronous = NORMAL` or higher. With
BATCH_ATOMIC active, `synchronous = NORMAL` does not add extra xSync calls
because the batch write path bypasses the sync logic entirely.

### Commit sequence (happy path)

Inside `sqlite3PagerCommitPhaseOne()`:

```
1. pager_incr_changecounter()       Update the change counter in page 1 (in cache).
2. syncJournal()                    No-op: journal is in memory.
3. sqlite3PcacheDirtyList()         Collect all dirty pages into a linked list.
4. xFileControl(BEGIN_ATOMIC_WRITE) Tell VFS to start buffering.
5. pager_write_pagelist(pList)      Call xWrite() for each dirty page. VFS buffers.
6. xWrite (file extension)          If file grew, write trailing zeros. VFS buffers.
7. xFileControl(COMMIT_ATOMIC_WRITE) Tell VFS to persist everything atomically.
8. Continue normal cleanup.          Clear page cache dirty flags, etc.
```

The journal file is never created on disk/KV. It exists only in memory as a
backup of original page contents in case ROLLBACK is needed before commit.

### Commit sequence (failure + fallback)

If any step between BEGIN_ATOMIC_WRITE and COMMIT_ATOMIC_WRITE fails (including
COMMIT itself returning an error), SQLite executes a fallback:

```
1-4. Same as happy path.
5.   pager_write_pagelist()         xWrite calls → VFS buffers. (May succeed or fail.)
6.   (file extension write)         May succeed or fail.
7.   xFileControl(COMMIT_ATOMIC)    Returns SQLITE_IOERR (e.g., buffer too large).

     SQLite detects the error:

8.   xFileControl(ROLLBACK_ATOMIC)  VFS discards the buffer. Called as a "hint"
                                    (return value ignored).

9.   Error class check:
     - If (rc & 0xFF) == SQLITE_IOERR and rc != SQLITE_IOERR_NOMEM:
       → sqlite3JournalCreate()     Spill the in-memory journal to a real file.
                                    This calls xOpen(journal) + xWrite for each
                                    original page that was saved in memory.
       → bBatch = 0                 Disable batch mode for the rest of this commit.
     - Otherwise:
       → Hard failure. No retry. Error propagates to application.

10.  if bBatch == 0:
       pager_write_pagelist(pList)  Re-write all dirty pages via xWrite, this time
                                    NOT inside a BEGIN/COMMIT bracket. Each xWrite
                                    goes directly to storage (no buffering).

11.  Normal commit continues:
     - Extend file if needed.
     - xSync (database file).
     - Delete/truncate journal file (the commit point).
```

### Source code references

All line numbers refer to the SQLite 3.49.2 amalgamation (`sqlite3.c`).

- `sqlite3PagerCommitPhaseOne()`: ~line 63769. The main commit function.
- Batch eligibility check: ~line 63829-63832.
- BEGIN_ATOMIC_WRITE call: ~line 63921.
- COMMIT_ATOMIC_WRITE call: ~line 63929.
- ROLLBACK_ATOMIC_WRITE on failure: ~line 63935.
- IOERR retry decision: ~line 63940-63949.
- Journal spill via `sqlite3JournalCreate()`: ~line 63941.
- Non-batch page write retry: ~line 63953.

## The in-memory journal

When BATCH_ATOMIC is eligible, SQLite stores the rollback journal entirely in
memory using a `MemJournal` structure. This is controlled by
`jrnlBufferSize()` returning -1 when batch-atomic conditions are met (the -1
tells SQLite to use unbounded in-memory buffering).

The in-memory journal holds the **original** page contents (before
modification). If a ROLLBACK occurs, SQLite reads these original pages back
from the MemJournal to restore the database to its pre-transaction state.

During the fallback (step 9 above), `sqlite3JournalCreate()` converts the
MemJournal into a real file:
1. Opens the journal file via `xOpen` with `SQLITE_OPEN_MAIN_JOURNAL` flags.
2. Writes all buffered journal chunks from the MemJournal to the real file via
   `xWrite`.
3. Replaces the MemJournal I/O methods with the real file I/O methods.

After this, SQLite proceeds with the normal DELETE-mode journal commit: write
dirty pages to the main database file, sync, then delete the journal file.

## Error code requirements

The fallback ONLY triggers for `SQLITE_IOERR` family errors. Specifically, the
check in the source is:

```c
if( (rc&0xFF)==SQLITE_IOERR && rc!=SQLITE_IOERR_NOMEM ){
    // retry with journal
}
```

This means:
- `SQLITE_IOERR` (10): triggers retry.
- `SQLITE_IOERR_WRITE` (778): triggers retry.
- `SQLITE_IOERR_FSYNC` (1034): triggers retry.
- Any other `SQLITE_IOERR_*` variant: triggers retry (except NOMEM).
- `SQLITE_IOERR_NOMEM` (3082): hard failure, no retry.
- `SQLITE_FULL` (13): hard failure, no retry.
- `SQLITE_NOMEM` (7): hard failure, no retry.
- Any non-IOERR error: hard failure, no retry.

**For our VFS**: when the dirty buffer exceeds 128 entries, return
`SQLITE_IOERR` from `COMMIT_ATOMIC_WRITE` to trigger the retry path.

## Documentation status

The BATCH_ATOMIC feature has fragmented documentation:

- **Official FCNTL docs** (`sqlite.org/c3ref/c_fcntl_begin_atomic_write.html`):
  Documents the three opcodes and their contracts. Does not document the
  fallback behavior.

- **Tech note** (2017, `sqlite3.org/cgi/src/technote/714f6cbb...`):
  Design document written before the feature was fully implemented. States:
  "a failed batch-atomic transaction is a hard failure which is not retried.
  Future versions of SQLite might retry a failed batch-atomic transaction as a
  normal transaction." The retry was implemented in a later version but the
  tech note was never updated.

- **Source code**: The only authoritative documentation of the fallback
  behavior is the source code in `pager.c` (`sqlite3PagerCommitPhaseOne()`).

- **Atomic commit docs** (`sqlite.org/atomiccommit.html`): Describes the
  general journal commit protocol but does not mention BATCH_ATOMIC.

## The synchronous=OFF bug

Roy Hashimoto (wa-sqlite author) reported a corruption window when combining
`PRAGMA synchronous = OFF` with `SQLITE_IOCAP_BATCH_ATOMIC`. The issue is that
`synchronous = OFF` sets `pPager->noSync = 1`, which is one of the batch
eligibility conditions (condition 3 above). When noSync is true, SQLite skips
the batch-atomic path and falls through to the normal commit path, but the
journal is still in memory (because the VFS declared BATCH_ATOMIC). This can
lead to a state where dirty pages are written without journal protection.

Richard Hipp (SQLite author) fixed this on the SQLite trunk. The fix ensures
the journal is spilled to disk when noSync is true and batch-atomic is not
used. However, the fix may not be present in all wa-sqlite builds.

**Recommendation**: Use `PRAGMA synchronous = NORMAL` to avoid this issue
entirely. With BATCH_ATOMIC active, NORMAL adds zero overhead because the
batch write path bypasses all sync operations.

## Industry usage

| Implementation | Backend | Approach |
|----------------|---------|----------|
| Android F2FS | Filesystem | `F2FS_IOC_START_ATOMIC_WRITE` / `F2FS_IOC_COMMIT_ATOMIC_WRITE` ioctls. Kernel buffers writes and commits atomically. First production use of BATCH_ATOMIC. |
| wa-sqlite IDBBatchAtomicVFS | IndexedDB | Declares BATCH_ATOMIC. BEGIN/COMMIT are no-ops because IDB transactions are inherently atomic. Actual commit happens at IDB transaction boundary. |
| Gazette store_sqlite | RocksDB | Uses BATCH_ATOMIC to collect dirty pages, commits via RocksDB WriteBatch. |
| gRPSQLite | gRPC remote store | Uses BATCH_ATOMIC with `journal_mode=MEMORY`. Sends all dirty pages in a single `AtomicWriteBatch` gRPC call at commit. |

## References

- SQLite FCNTL constants: https://sqlite.org/c3ref/c_fcntl_begin_atomic_write.html
- SQLite atomic commit: https://sqlite.org/atomiccommit.html
- Batch atomic write tech note: https://www3.sqlite.org/cgi/src/technote/714f6cbbf78c8a1351cbd48af2b438f7f824b336
- wa-sqlite IDBBatchAtomicVFS source: `@rivetkit/sqlite/src/examples/IDBBatchAtomicVFS.js`
- wa-sqlite BATCH_ATOMIC discussion: https://github.com/rhashimoto/wa-sqlite/discussions/78
- F2FS atomic writes: https://www.kernel.org/doc/html/latest/filesystems/f2fs.html
- SQLite forum on synchronous=OFF + BATCH_ATOMIC: https://sqlite.org/forum/forumpost/7ceaee2262e52377
