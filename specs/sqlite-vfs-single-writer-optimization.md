# SQLite VFS Single-Writer Optimization Spec

Date: 2026-03-21
Package: `@rivetkit/sqlite-vfs`, `rivetkit/db`
Status: Draft (revised after third adversarial review - SQLite contract validation)

Related docs:
- `docs/internal/sqlite-batch-atomic-write.md` (BATCH_ATOMIC internals + fallback)
- `specs/sqlite-vfs-single-writer-findings.md` (adversarial review + industry research)
- `specs/sqlite-vfs-adversarial-review.md` (three rounds: 32 issues, 12 real, validated)

## Problem

A simple `UPDATE + SELECT` on a KV-backed SQLite actor takes ~1.3 seconds end
to end. Benchmarking shows 247ms for a state increment (pure network) vs
1325ms for a SQLite increment. The ~1.1 second overhead comes from the VFS
making 5-7 sequential KV round trips per write transaction.

With the default `DELETE` journal mode, a single UPDATE triggers:

| Step                          | VFS call          | KV op       |
|-------------------------------|-------------------|-------------|
| Check journal exists          | xAccess           | 1 get       |
| Open journal file             | xOpen             | 1 get       |
| Write rollback page           | xWrite (journal)  | 1 putBatch  |
| Write updated page            | xWrite (main)     | 1 putBatch  |
| Sync main file                | xSync             | 1 put       |
| Delete journal (meta + data)  | xDelete           | 2 ops       |
| **Total**                     |                   | **7**       |

A subsequent SELECT adds 1 more round trip if the pager cache doesn't retain
the page across transactions.

## Solution: SQLITE_IOCAP_BATCH_ATOMIC

Instead of disabling the journal (`journal_mode = OFF`) and building a custom
write-behind buffer, use SQLite's built-in `SQLITE_IOCAP_BATCH_ATOMIC`
mechanism. This tells SQLite the VFS can atomically write multiple pages in one
operation. SQLite responds by keeping the rollback journal in memory and giving
the VFS explicit transaction boundary signals.

This approach was identified through adversarial review (see findings doc) and
industry research. It is the same pattern used by:
- Android F2FS (first implementation, SQLite 3.21.0)
- wa-sqlite `IDBBatchAtomicVFS` (IndexedDB backend)
- Gazette `store_sqlite` (RocksDB backend)
- gRPSQLite (gRPC remote store)

All required constants and the compile flag (`SQLITE_ENABLE_BATCH_ATOMIC_WRITE`)
are already present in `@rivetkit/sqlite` v0.1.1.

### How it works

SQLite uses three `xFileControl` opcodes to bracket atomic writes:

1. `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE` (31): SQLite tells the VFS to start
   buffering. The VFS sets a flag and allocates a dirty page buffer.

2. SQLite calls `xWrite()` once per dirty page. The VFS stores each write in
   the buffer instead of calling `putBatch`.

3. `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` (32): SQLite tells the VFS to persist
   everything atomically. The VFS calls `putBatch` with all buffered pages +
   metadata in a single KV round trip. Batch mode ends regardless of success
   or failure (per the SQLite xFileControl contract).

4. On commit failure: `SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE` (33): Called by
   SQLite as a best-effort hint after COMMIT_ATOMIC_WRITE fails. The VFS
   should already have exited batch mode in step 3. This is a defensive
   no-op.

**Note**: These opcodes are part of the **commit** path inside
`sqlite3PagerCommitPhaseOne()`. A user-issued SQL `ROLLBACK` (e.g.,
`BEGIN; INSERT; ROLLBACK;`) never enters batch mode. The pager restores
pages from its in-memory journal without calling any xFileControl opcodes.

### PRAGMAs

Set immediately after `sqlite3.open_v2()` in `SqliteVfs.open()`:

```sql
PRAGMA page_size = 4096;
PRAGMA journal_mode = DELETE;
PRAGMA locking_mode = EXCLUSIVE;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA auto_vacuum = NONE;
```

**`page_size = 4096`**: Enforces the page size that the VFS assumes throughout.
CHUNK_SIZE in kv.ts is hardcoded to 4096 so that one SQLite page maps to
exactly one KV value. If page_size differs from CHUNK_SIZE, xWrite calls would
span multiple chunks or leave partial chunks, breaking the 1:1 mapping. This
PRAGMA must come before any table creation. For existing databases, page_size
is stored in the header and this PRAGMA is a no-op (matching the existing
value).

**`journal_mode = DELETE`**: Explicitly sets the default rollback journal mode.
This is normally redundant (DELETE is SQLite's default), but WAL mode persists
in the database header across reopens. If a database ever entered WAL mode
(e.g., user code ran `PRAGMA journal_mode=WAL`), reopening without this PRAGMA
would keep WAL mode, which interacts with BATCH_ATOMIC differently. Setting
DELETE explicitly is cheap insurance. With BATCH_ATOMIC active, the journal
stays in memory (no journal file I/O). It only materializes to a real file if
the atomic commit fails and SQLite falls back (see "Large transaction
handling" below).

**`locking_mode = EXCLUSIVE`**: Each actor is single-writer. SQLite acquires
the lock once and never releases it. The pager cache is trusted across
transactions, so a SELECT after an UPDATE typically reads from cache (0 KV
ops). Note: SQLite's LRU pager cache can evict pages under memory pressure, so
"0 KV ops for warm SELECT" is the expected case for typical small-actor
workloads, not a hard guarantee.

**`synchronous = NORMAL`**: Required for BATCH_ATOMIC eligibility. SQLite
checks `pPager->noSync` as a batch eligibility condition. `synchronous = OFF`
sets `noSync = 1`, which **disables** BATCH_ATOMIC. `NORMAL` does not add
extra xSync calls because the batch write path bypasses sync logic entirely.
There is also a confirmed bug with `synchronous = OFF` + BATCH_ATOMIC
(reported by Roy Hashimoto, wa-sqlite author). Using NORMAL avoids it.

**`temp_store = MEMORY`**: Keeps temp tables and sort spills in memory instead
of creating temp files through the VFS. Eliminates xOpen/xWrite/xRead/xClose
calls for complex queries.

**`auto_vacuum = NONE`**: Disables page reorganization on DELETE. Without this,
deleting rows triggers page moves (extra xWrite + xRead calls). Acceptable
tradeoff: database file never shrinks, but actor databases are small and
short-lived. Note: `auto_vacuum` is persistent in the database header. For
existing databases with tables, this PRAGMA is silently ignored by SQLite
(auto_vacuum can only be changed on empty databases or via VACUUM). Since the
current VFS code does not set auto_vacuum, existing databases use whatever
default SQLite was compiled with (typically NONE). This is acceptable for
actor databases.

### VFS changes

**Async registration**: Add `"xFileControl"` to `SQLITE_ASYNC_METHODS`
(vfs.ts:85-95). COMMIT_ATOMIC_WRITE calls `putBatch` which is async. Without
async registration, the wa-sqlite relay calls xFileControl synchronously and
does not `await` the returned Promise. The return value would be a Promise
object (truthy, non-zero), which SQLite interprets as an error. The wa-sqlite
framework already supports async xFileControl (it's in `VFS_METHODS`); it just
needs to be declared in `SQLITE_ASYNC_METHODS`.

**`xDeviceCharacteristics`**: Return `SQLITE_IOCAP_BATCH_ATOMIC` (0x4000).
Return `SQLITE_NOTFOUND` for all unhandled xFileControl opcodes (current
behavior, no change needed).

**Batch mode scope**: `batchMode`, the dirty buffer, and `savedFileSize` are
properties on `OpenFile`, NOT on `SqliteSystem` or `Database`. SQLite sends
xFileControl to the main database file handle only. During journal fallback,
xWrite calls go to the journal file via a different fileId. If batchMode were
global, journal writes during fallback would be incorrectly buffered.

**`xFileControl`**: Handle three opcodes. Per the SQLite xFileControl contract,
COMMIT_ATOMIC_WRITE ends batch mode regardless of success or failure. The VFS
must not depend on ROLLBACK_ATOMIC_WRITE being called for cleanup.

```
BEGIN_ATOMIC_WRITE (31):
  Save file.size to file.savedFileSize.
  Set file.batchMode = true.
  Set file.metaDirty = false.
  Allocate file.dirtyBuffer: Map<string, Uint8Array>.
  Return SQLITE_OK.

COMMIT_ATOMIC_WRITE (32):
  If dirtyBuffer.size > 127:
    Clear dirtyBuffer, restore file.size, set metaDirty = false.
    Set file.batchMode = false.
    Return SQLITE_IOERR (triggers journal fallback, see below).
  Construct entries: [...dirtyBuffer entries, [file.metaKey, encodeFileMeta(file.size)]].
  Call putBatch(entries).
  If putBatch fails:
    Clear dirtyBuffer, restore file.size, set metaDirty = false.
    Set file.batchMode = false.
    Return SQLITE_IOERR.
  Clear dirtyBuffer, set file.batchMode = false.
  Return SQLITE_OK.

ROLLBACK_ATOMIC_WRITE (33):
  If not file.batchMode: return SQLITE_OK.  (Already cleaned up by COMMIT.)
  Discard dirtyBuffer.
  Restore file.size from file.savedFileSize.
  Set file.metaDirty = false.
  Set file.batchMode = false.
  Return SQLITE_OK.
```

The buffer limit check is `dirtyBuffer.size > 127` (page entries only). At
commit time, the metadata entry is appended: 127 pages + 1 metadata = 128 keys,
which is the KV putBatch maximum. This also guarantees the payload stays under
the 976 KiB putBatch limit: 127 * 4096 bytes + key overhead + metadata is
approximately 525 KiB, well under 976 KiB.

**`xWrite`**: If `file.batchMode` is true, add `[chunkKey, data]` to the dirty
buffer instead of calling `putBatch`. Also update `file.size` in memory if the
write extends the file (so `xFileSize` returns the correct value during the
transaction). Since CHUNK_SIZE = page_size = 4096 (enforced by the page_size
PRAGMA), all SQLite xWrite calls are complete chunk replacements. No
partial-chunk merging is needed. If `file.batchMode` is false, write directly
to KV (current behavior, used during journal fallback).

**`xRead`**: If `file.batchMode` is true, check the dirty buffer first. Return
buffered data if present, otherwise call `getBatch`. In practice, SQLite's
pager cache (trusted via EXCLUSIVE mode) handles most cases. The buffer check
is defense-in-depth. Note: the SQLite FCNTL documentation states that between
BEGIN_ATOMIC_WRITE and COMMIT/ROLLBACK, the only operations on the file
descriptor are xWrite and xFileControl(SIZE_HINT). xRead is not called during
the batch window, so this check is purely defensive.

**`xTruncate`**: No batch-mode handling needed. The SQLite FCNTL documentation
states that between BEGIN_ATOMIC_WRITE and COMMIT/ROLLBACK, the only
operations on the file descriptor are xWrite and xFileControl(SIZE_HINT).
xTruncate is never called during the batch window. If stale KV chunks need
cleanup after VACUUM or explicit file shrink, that is a post-commit concern
outside the BATCH_ATOMIC path.

### Expected round trips

| Scenario | getBatch | putBatch | Total |
|----------|----------|----------|-------|
| Warm single-page UPDATE | 0 | 1 | **1** |
| Warm multi-page UPDATE (N pages, N ≤ 127) | 0 | 1 | **1** |
| Cold single-page UPDATE | 1 | 1 | **2** |
| Cold multi-page UPDATE | 1+ | 1 | **2+** |
| Warm SELECT (page cached) | 0 | 0 | **0** |
| Cold SELECT | 1 | 0 | **1** |
| Warm UPDATE + SELECT (same page) | 0 | 1 | **1** |
| SQL ROLLBACK (warm) | 0 | 0 | **0** |
| SQL ROLLBACK (cold pages read) | 0+ | 0 | **0+** |
| Multi-stmt BEGIN/COMMIT (warm) | 0 | 1 | **1** |

### Large transaction handling (>127 dirty pages)

When the dirty buffer exceeds 127 page entries, the VFS returns `SQLITE_IOERR`
from `COMMIT_ATOMIC_WRITE`. SQLite handles the fallback internally:

1. The VFS exits batch mode during COMMIT_ATOMIC_WRITE (per the SQLite
   contract: batch mode ends regardless of success or failure). The dirty
   buffer is cleared and `file.size` is restored. KV is untouched. SQLite
   then calls ROLLBACK_ATOMIC_WRITE as a hint, which is a no-op since batch
   mode was already exited.

2. SQLite spills the in-memory journal to a real journal file. This calls
   `xOpen` for the journal file (routed to `FILE_TAG_JOURNAL` keys in KV via
   `#resolveFile()`), then `xWrite` for each original page. These writes go
   directly to KV (not buffered, since batch mode was exited).

3. SQLite re-writes all dirty pages to the main database file via `xWrite`.
   Again, direct to KV (not buffered).

4. SQLite syncs the database file, then deletes the journal file. The journal
   deletion is the commit point. If the actor crashes before the journal is
   deleted, SQLite detects the "hot journal" on next open and rolls back
   automatically.

This fallback is implemented in `sqlite3PagerCommitPhaseOne()` in pager.c.
See `docs/internal/sqlite-batch-atomic-write.md` for the exact code flow and
source code references.

**Important**: The fallback ONLY triggers for `SQLITE_IOERR` family errors.
Other error codes (e.g., `SQLITE_FULL`) cause a hard failure that propagates
to the application. Always return `SQLITE_IOERR` (not SQLITE_FULL or other
codes) when the buffer exceeds the limit.

**Round trips for fallback (N = 200 pages)**:

During fallback, `batchMode` is false and each `xWrite` call goes directly to
KV as a separate `putBatch` call (current behavior). Each `putBatch` contains
1-2 entries (chunk + optional metadata). This means:

```
Journal writes:  N putBatch calls  (one per original page)    ~200
Main writes:     N putBatch calls  (one per dirty page)       ~200
Journal delete:  1 deleteBatch call                           1
Sync:            1 put (metadata)                             1
Total:           ~2N + 2 RT                                   ~402
```

This is significantly slower than the 1 RT batch path. But the fallback is
correct and rare. Most actor transactions touch far fewer than 127 pages. For
the uncommon case where large transactions are needed, the cost is acceptable
because it maintains full crash recovery guarantees via the standard SQLite
journal protocol.

## Safety argument

1. **Atomicity**: A single `putBatch` call persists all dirty pages atomically.
   KV `putBatch` is all-or-nothing. This is strictly stronger than the DELETE
   journal approach where journal-write and main-write are separate KV calls
   with a crash window between them.

2. **putBatch atomicity is a hard requirement**: The batch path crash recovery
   depends entirely on `putBatch` being all-or-nothing. If the KV backend has
   partial failure modes (some keys written, others not), the safety argument
   collapses. In production, the KV layer is backed by FoundationDB which
   provides this guarantee. Test and dev KV drivers must also provide atomic
   putBatch semantics for the batch path to be safe.

3. **Crash recovery (batch path)**: If the actor crashes before `putBatch`,
   nothing is written. If it crashes after, everything is committed. No partial
   state is possible.

4. **Crash recovery (journal fallback)**: If the actor crashes during the
   journal fallback path, the journal file persists in KV. On next open,
   SQLite detects the hot journal and rolls back automatically. This is
   standard SQLite crash recovery.

5. **SQL ROLLBACK**: Works correctly. A user-issued `BEGIN; ...; ROLLBACK;`
   never enters batch mode (BATCH_ATOMIC is part of the commit path, not the
   transaction lifecycle). The pager restores pages from its in-memory journal.
   Zero KV write operations. KV reads may occur if statements inside the
   transaction touched cold pages.

   **COMMIT failure rollback**: If COMMIT_ATOMIC_WRITE fails (buffer too large
   or putBatch error), the VFS exits batch mode, clears the dirty buffer, and
   restores `file.size`. SQLite then falls back to the journal path. See
   "Large transaction handling" above.

6. **No concurrent access**: Single-writer with `locking_mode = EXCLUSIVE`.
   No locking overhead.

## Observability

### Actor metrics

Each actor instance collects in-memory metrics via `ActorMetrics`
(`src/actor/metrics.ts`). Metrics are **not persisted**. They reset when the
actor sleeps and are collected fresh on each wake cycle. This keeps the system
simple and avoids KV overhead for metrics storage.

Exposed via `GET /inspector/metrics` (inspector token auth required). Returns
JSON:

```json
{
  "kv_operations": {
    "type": "labeled_timing",
    "help": "KV round trips by operation type",
    "values": {
      "get": { "count": 2, "totalMs": 512.34, "keys": 2 },
      "putBatch": { "count": 1, "totalMs": 248.10, "keys": 2 }
    }
  },
  "sql_statements": {
    "type": "labeled_counter",
    "help": "SQL statements executed by type",
    "values": { "select": 5, "insert": 0, "update": 3, "delete": 0, "other": 1 }
  },
  "sql_duration_ms": {
    "type": "counter",
    "help": "Total SQL execution time in milliseconds",
    "value": 1823.5
  },
  "action_calls": { "type": "counter", "help": "Total action invocations", "value": 8 },
  "action_errors": { "type": "counter", "help": "Total action errors", "value": 0 },
  "action_duration_ms": { "type": "counter", "help": "Total action execution time in milliseconds", "value": 2100.3 },
  "connections_opened": { "type": "counter", "help": "Total WebSocket connections opened", "value": 0 },
  "connections_closed": { "type": "counter", "help": "Total WebSocket connections closed", "value": 0 }
}
```

Metric types:

- **counter**: Monotonically increasing value.
- **gauge**: Point-in-time value (can go up or down).
- **labeled_counter**: Counter with string labels.
- **labeled_timing**: Counter with calls, keys, and duration per label.

### Inspector UI integration

Metrics are shown in the inspector dashboard under the **Metadata** tab in an
**Advanced** foldout section. The foldout is collapsed by default to avoid
cluttering the primary view. It displays the raw metrics JSON in a readable
format. This is an internal debugging tool and is not part of the public API.

## KV round trip verification tests

The `db-kv-stats` fixture (`fixtures/driver-test-suite/db-kv-stats.ts`)
provides an instrumented KV store with per-operation call counts and an
operation log (including decoded key names like `chunk:main[0]`,
`meta:main`, `chunk:journal[0]`). Tests call `resetStats()` before each
operation and `getStats()` / `getLog()` after to assert exact KV call counts.

### Test fixture actions needed

Add to `db-kv-stats.ts`:

- `insertWithIndex`: INSERT into a table with an index (multi-page write).
- `rollbackTest`: `BEGIN; INSERT ...; ROLLBACK;`
- `multiStmtTx`: `BEGIN; INSERT ...; INSERT ...; COMMIT;`
- `bulkInsertLarge`: Insert ~200 large rows in one transaction to exceed 127
  dirty pages and trigger the journal fallback.
- `getRowCount`: `SELECT COUNT(*)` on the bulk table.
- `runIntegrityCheck`: `PRAGMA integrity_check`.

### Test cases

**Test 1: Warm single-page UPDATE**
```
resetStats → increment → getStats
assert putBatchCalls == 1
assert getBatchCalls == 0
```

**Test 2: Warm SELECT (pager cache hit)**
```
increment (warm cache) → resetStats → getCount → getStats
assert getBatchCalls == 0
assert putBatchCalls == 0
```

**Test 3: Warm UPDATE + SELECT**
```
increment (warm) → resetStats → incrementAndRead → getStats
assert putBatchCalls == 1
assert getBatchCalls == 0
```

**Test 4: Multi-page INSERT (with index)**
```
resetStats → insertWithIndex → getStats, getLog
assert putBatchCalls == 1
assert log has single putBatch with multiple chunk keys
```

**Test 5: SQL ROLLBACK produces no writes**

A user-issued ROLLBACK never enters BATCH_ATOMIC mode (that protocol is part
of the commit path). The pager restores from its in-memory journal. No KV
writes occur, but KV reads may happen if the INSERT touched cold pages.

```
resetStats → rollbackTest → getStats
assert putBatchCalls == 0
(getBatchCalls may be > 0 if cold pages were read during the INSERT)
```

**Test 6: Multi-statement transaction**
```
resetStats → multiStmtTx → getStats
assert putBatchCalls == 1
```

**Test 7: No journal/WAL file operations (BATCH_ATOMIC verification)**

This test serves as the primary verification that BATCH_ATOMIC is active. If
xDeviceCharacteristics does not return the flag, or if any BATCH_ATOMIC
eligibility condition fails (e.g., synchronous=OFF), SQLite silently falls back
to journal mode for every transaction. There is no error or warning. This test
catches that by asserting no journal operations occurred during a normal write.

```
resetStats → increment → getLog
assert no log entry keys contain "journal" or "wal"
```

**Test 8: putBatch entries within limit**
```
resetStats → increment → getLog
assert putBatch entry key count <= 128
```

**Test 9: Large transaction (>127 pages) falls back to journal**
```
resetStats → bulkInsertLarge → getStats, getLog
assert journalOps = log entries with keys containing "journal"
assert journalOps.length > 0  (journal fallback activated)
assert putBatchCalls > 1  (multiple batches: journal + main)
assert every putBatch entry has <= 128 keys  (respects KV limit)
```

**Test 10: Large transaction data integrity**
```
bulkInsertLarge → getRowCount
assert count == 200
runIntegrityCheck
assert result == "ok"
```

**Test 11: Large transaction survives actor restart**
```
bulkInsertLarge → destroy + recreate actor → getRowCount
assert count == 200
runIntegrityCheck
assert result == "ok"
```

## Documentation updates

When this lands, update the following:

- **`website/src/content/docs/actors/limits.mdx`**: Document that SQLite actors
  use BATCH_ATOMIC with KV-layer atomicity. Note the 127-page (508 KiB) soft
  limit per transaction before journal fallback. Document that crash recovery
  is handled by KV putBatch atomicity (batch path) or SQLite hot journal
  rollback (fallback path).
- **`website/src/metadata/skill-base-rivetkit.md`**: Update the SQLite VFS
  section to mention the BATCH_ATOMIC optimization.
- **`website/src/content/docs/actors/debugging.mdx`**: Document the
  `/inspector/metrics` endpoint.

## Files changed

### VFS (BATCH_ATOMIC implementation)

- `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`:
  - `SQLITE_ASYNC_METHODS`: Add `"xFileControl"` so putBatch is awaited.
  - `OpenFile` interface: Add `batchMode`, `dirtyBuffer`, `savedFileSize`
    fields.
  - `xDeviceCharacteristics()`: Return `SQLITE_IOCAP_BATCH_ATOMIC`.
  - `xFileControl()`: Handle BEGIN/COMMIT/ROLLBACK_ATOMIC_WRITE. COMMIT
    tears down batch mode on both success and failure per SQLite contract.
  - `xWrite()`: Buffer writes when in batch mode. Update file.size in memory.
  - `xRead()`: Check dirty buffer when in batch mode (defense-in-depth).
  - `SqliteVfs.open()`: Update PRAGMAs (add page_size=4096, change
    journal_mode=OFF to DELETE, change synchronous=OFF to NORMAL, add
    temp_store=MEMORY and auto_vacuum=NONE).

### Metrics (already implemented)

- `rivetkit-typescript/packages/rivetkit/src/actor/metrics.ts`: ActorMetrics
  class with KV, SQL, and action tracking.
- `rivetkit-typescript/packages/rivetkit/src/db/shared.ts`: Instrumented
  createActorKvStore with ActorMetrics.
- `rivetkit-typescript/packages/rivetkit/src/db/config.ts`: metrics on
  DatabaseProviderContext.
- `rivetkit-typescript/packages/rivetkit/src/db/mod.ts`: Pass metrics through,
  track SQL statement type and duration.
- `rivetkit-typescript/packages/rivetkit/src/db/drizzle/mod.ts`: Pass metrics
  through.
- `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts`: Store
  ActorMetrics, track action calls/errors/duration.
- `rivetkit-typescript/packages/rivetkit/src/actor/router.ts`: Inspector
  `/inspector/metrics` endpoint returning JSON.

### Tests

- `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/db-kv-stats.ts`:
  Add `insertWithIndex`, `rollbackTest`, `multiStmtTx`, `bulkInsertLarge`,
  `getRowCount`, `runIntegrityCheck` actions.
- `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-db-kv-stats.ts`:
  Replace diagnostic test with strict round trip assertion tests (Tests 1-11).
