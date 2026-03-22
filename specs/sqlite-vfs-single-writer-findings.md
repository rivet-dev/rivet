# SQLite VFS Single-Writer Optimization: Review Findings

Date: 2026-03-21
Status: Research notes from adversarial review of `sqlite-vfs-single-writer-optimization.md`

## Key Discovery: SQLITE_IOCAP_BATCH_ATOMIC

**This is the recommended approach.** It replaces the entire Phase 1 + Phase 2
design from the original spec with a simpler, safer, and equally fast solution.

`SQLITE_IOCAP_BATCH_ATOMIC` is an SQLite VFS capability flag that tells SQLite:
"this storage backend can atomically write multiple pages in one operation."
When declared, SQLite keeps the rollback journal **entirely in memory** and
brackets all page writes with file control opcodes that let the VFS batch them.

### How it works

1. VFS returns `SQLITE_IOCAP_BATCH_ATOMIC` from `xDeviceCharacteristics()`.
2. On transaction commit, SQLite calls `xFileControl(SQLITE_FCNTL_BEGIN_ATOMIC_WRITE)`.
3. SQLite calls `xWrite()` once per dirty page (same as today).
4. SQLite calls `xFileControl(SQLITE_FCNTL_COMMIT_ATOMIC_WRITE)`.
5. On rollback, SQLite calls `xFileControl(SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE)`.

The VFS buffers all xWrite calls between BEGIN and COMMIT, then flushes them
in a single `putBatch` on COMMIT. On ROLLBACK, the buffer is discarded.

### Why this is better than journal_mode=OFF + write-behind buffer

| Property | journal_mode=OFF + buffer | BATCH_ATOMIC |
|----------|--------------------------|--------------|
| Warm write RT | 1 putBatch | **1 putBatch** (same) |
| ROLLBACK support | Must implement ourselves | **SQLite manages it** |
| Transaction boundaries | Must detect via autocommit | **SQLite tells us** (BEGIN/COMMIT/ROLLBACK opcodes) |
| Flush timing | Must wire into Database class | **SQLite calls at right time** |
| Journal safety | None (journal_mode=OFF) | **In-memory journal** (graceful fallback) |
| Pager cache overflow | Corruption risk | **Falls back to disk journal** |
| Code complexity | ~100 lines + wiring | **~30 lines in xFileControl** |

### What's available in @rivetkit/sqlite

All required constants are exported from `@rivetkit/sqlite` (v0.1.1):

- `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE = 31`
- `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE = 32`
- `SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE = 33`
- `SQLITE_IOCAP_BATCH_ATOMIC = 0x00004000`

`SQLITE_ENABLE_BATCH_ATOMIC_WRITE` is compiled into the WASM binary.

A complete reference implementation exists at
`@rivetkit/sqlite/src/examples/IDBBatchAtomicVFS.js` (IndexedDB-backed).

### Current VFS state

```typescript
// vfs.ts:1192-1198 - currently returns "not implemented" / "no capabilities"
xFileControl(_fileId: number, _flags: number, _pArg: number): number {
    return VFS.SQLITE_NOTFOUND;
}
xDeviceCharacteristics(_fileId: number): number {
    return 0;
}
```

### Implementation plan

1. `xDeviceCharacteristics` returns `SQLITE_IOCAP_BATCH_ATOMIC`.
2. `xFileControl` handles three opcodes:
   - `BEGIN_ATOMIC_WRITE`: Set a flag, start buffering xWrite calls.
   - `COMMIT_ATOMIC_WRITE`: Flush all buffered writes via single `putBatch`.
   - `ROLLBACK_ATOMIC_WRITE`: Discard the buffer.
3. `xWrite` checks the flag. If in batch mode, buffer the write. If not, write
   directly (current behavior, for non-transactional writes).
4. Remove `PRAGMA journal_mode = OFF`. SQLite will use DELETE mode with an
   in-memory journal (no journal file I/O because BATCH_ATOMIC eliminates it).
5. Keep `PRAGMA locking_mode = EXCLUSIVE` (still correct for single-writer).
6. Change `PRAGMA synchronous = OFF` to `PRAGMA synchronous = NORMAL`. There is
   a confirmed bug where `synchronous=OFF` + `IOCAP_BATCH_ATOMIC` creates a
   corruption window (reported by Roy Hashimoto, fixed on SQLite trunk). Using
   NORMAL avoids this. With BATCH_ATOMIC, NORMAL does not add extra xSync calls
   because the batch write path bypasses the sync logic.
7. Add `PRAGMA temp_store = MEMORY` and `PRAGMA auto_vacuum = NONE`.

### Caveats

- **128-key putBatch limit**: If a transaction dirties >127 pages (508 KiB),
  the single putBatch will exceed the limit. Options: hard limit with error
  (recommended), or split into multiple putBatch calls with a committed-index
  protocol (future).
- **synchronous=OFF bug**: Verify whether the wa-sqlite WASM build includes the
  fix. If not, use `synchronous=NORMAL` (no performance penalty with BATCH_ATOMIC).

### Detailed round trip analysis

With BATCH_ATOMIC, SQLite manages the transaction lifecycle. The VFS buffers
xWrite calls between BEGIN_ATOMIC_WRITE and COMMIT_ATOMIC_WRITE, then flushes
all buffered pages + metadata in a single putBatch.

**Key behavior**: SQLite still calls xWrite once per dirty page. The VFS
buffers these in a `Map<string, Uint8Array>` instead of calling putBatch. On
COMMIT_ATOMIC_WRITE, the VFS issues one putBatch with all buffered entries.

**xRead during batch mode**: SQLite's pager cache (trusted due to
`locking_mode = EXCLUSIVE`) serves pages that were just written without calling
xRead. The VFS should still check the dirty buffer in xRead as a safety net,
but in practice SQLite won't call xRead for a page it just wrote.

#### Scenario breakdown

**Actor cold start (first query)**:
```
sqlite3.open_v2()          → xOpen → getBatch([meta:main]) = 1 RT (read file metadata)
PRAGMA locking_mode = ...  → no KV ops
First SQL statement:
  Page 1 (header)          → xRead → getBatch([chunk:main[0]]) = 1 RT
  ...additional pages      → xRead per page = 1 RT each
```
The cold start always pays at least 2 RT: 1 for file metadata + 1 for page 1.
Subsequent pages add 1 RT each unless batched in a single getBatch.

**Warm single-page UPDATE** (page in pager cache):
```
BEGIN_ATOMIC_WRITE         → set buffer flag (0 KV ops)
xWrite(page)               → buffer[chunk:main[N]] = data (0 KV ops)
COMMIT_ATOMIC_WRITE        → putBatch([chunk:main[N], meta:main]) = 1 RT
                                                          Total: 1 RT
```

**Warm multi-page UPDATE** (e.g., INSERT with index, 3 dirty pages):
```
BEGIN_ATOMIC_WRITE         → set buffer flag (0 KV ops)
xWrite(page A)             → buffer[chunk:main[A]] (0 KV ops)
xWrite(page B)             → buffer[chunk:main[B]] (0 KV ops)
xWrite(page C)             → buffer[chunk:main[C]] (0 KV ops)
COMMIT_ATOMIC_WRITE        → putBatch([chunk:A, chunk:B, chunk:C, meta:main]) = 1 RT
                                                          Total: 1 RT
```

**Cold single-page UPDATE** (page not in cache):
```
xRead(page)                → getBatch([chunk:main[N]]) = 1 RT
BEGIN_ATOMIC_WRITE         → (0 KV ops)
xWrite(page)               → buffer (0 KV ops)
COMMIT_ATOMIC_WRITE        → putBatch([chunk:main[N], meta:main]) = 1 RT
                                                          Total: 2 RT
```

**Warm SELECT** (page in pager cache):
```
(pager cache hit)          → 0 KV ops
                                                          Total: 0 RT
```

**Cold SELECT** (page not in cache):
```
xRead(page)                → getBatch([chunk:main[N]]) = 1 RT
                                                          Total: 1 RT
```

**Warm UPDATE then SELECT** (same page):
```
UPDATE: 1 RT (putBatch)
SELECT: 0 RT (pager cache, trusted via EXCLUSIVE)
                                                          Total: 1 RT
```

**ROLLBACK**:
```
BEGIN_ATOMIC_WRITE         → set buffer flag (0 KV ops)
xWrite(page A)             → buffer (0 KV ops)
xWrite(page B)             → buffer (0 KV ops)
ROLLBACK_ATOMIC_WRITE      → discard buffer (0 KV ops)
                                                          Total: 0 RT
```

**Explicit multi-statement transaction**:
```
BEGIN;
  INSERT ...               → xWrite calls buffered (0 KV ops)
  INSERT ...               → xWrite calls buffered (0 KV ops)
  UPDATE ...               → xWrite calls buffered (0 KV ops)
COMMIT;
  COMMIT_ATOMIC_WRITE      → putBatch(all dirty pages + meta) = 1 RT
                                                          Total: 1 RT (writes)
```

#### Summary table

| Scenario | getBatch RT | putBatch RT | Total RT |
|----------|-------------|-------------|----------|
| Warm single-page UPDATE | 0 | 1 | **1** |
| Warm multi-page UPDATE (N pages) | 0 | 1 | **1** |
| Cold single-page UPDATE | 1 | 1 | **2** |
| Cold multi-page UPDATE (N pages) | 1+ | 1 | **2+** |
| Warm SELECT | 0 | 0 | **0** |
| Cold SELECT | 1 | 0 | **1** |
| Warm UPDATE + SELECT (same page) | 0 | 1 | **1** |
| ROLLBACK | 0 | 0 | **0** |
| Multi-stmt tx (warm, M stmts) | 0 | 1 | **1** |

### Large transaction handling (>127 dirty pages)

When COMMIT_ATOMIC_WRITE is called and the dirty buffer contains >127 pages
(+ 1 meta entry = 128 entries, exceeding the putBatch limit):

**Option A: Return SQLITE_IOERR (recommended)**

The VFS returns SQLITE_IOERR from COMMIT_ATOMIC_WRITE. SQLite then:
1. Calls ROLLBACK_ATOMIC_WRITE (VFS discards the buffer).
2. Falls back to standard journal mode for this transaction.
3. Writes original pages to a journal file via xWrite (to FILE_TAG_JOURNAL
   keys in KV). Multiple putBatch calls are needed.
4. Writes modified pages to the main file via xWrite. Multiple putBatch calls.
5. Deletes the journal file (commit point).

This fallback path costs 3+ RT but is correct (journal provides crash safety)
and rare (most actor transactions touch <127 pages). The VFS already has full
journal file support via FILE_TAG_JOURNAL routing in `#resolveFile()`.

Round trips for fallback:
```
Journal write:  ceil(N/127) putBatch calls  (original pages + journal meta)
Main write:     ceil(N/127) putBatch calls  (modified pages)
Journal delete: 1 deleteBatch call
Sync:           1 putBatch (if synchronous=NORMAL)
Total:          ~2*ceil(N/127) + 2 RT
```

For N=200 pages: ~6 RT. Slower but correct and very rare.

**Option B: Split putBatch (not recommended)**

Split the buffer into multiple putBatch calls. This is NOT atomic across
batches, so a crash between batches corrupts the database. Only viable with the
committed-index protocol (additional complexity).

**Option C: Hard error (simplest)**

Return SQLITE_FULL or SQLITE_IOERR and fail the transaction entirely. The
application must restructure to use smaller transactions. This is the simplest
approach but least flexible.

**Recommendation**: Option A. Let SQLite's own fallback handle it. The journal
infrastructure already exists in the VFS. Large transactions are rare and the
3+ RT cost is acceptable for correctness.

### KV round trip verification tests

The existing `db-kv-stats` fixture (`fixtures/driver-test-suite/db-kv-stats.ts`)
provides instrumented KV with per-operation call counts and operation logs. The
existing test is diagnostic only (logs operations, asserts putBatch > 0). After
implementing BATCH_ATOMIC, replace with strict round trip assertions.

#### Test fixture changes needed

Add these actions to `db-kv-stats.ts`:

- `insertWithIndex`: INSERT into a table with an index (forces multi-page write).
- `rollbackTest`: BEGIN; INSERT ...; ROLLBACK; (verifies 0 KV writes).
- `multiStmtTx`: BEGIN; INSERT ...; INSERT ...; COMMIT; (verifies 1 putBatch).
- `bulkInsert(n)`: Insert N rows in a single transaction to test large tx behavior.
- `bulkInsertLarge()`: Insert enough rows in one tx to exceed 127 dirty pages
  (triggers journal fallback).

#### Test cases (`actor-db-kv-stats.ts`)

Each test calls `resetStats()`, performs an operation, then calls `getStats()`
and `getLog()` to assert exact KV call counts.

**Test 1: Warm single-page UPDATE = 1 putBatch, 0 getBatch**
```
resetStats()
increment()           // UPDATE counter SET count = count + 1
stats = getStats()
expect(stats.putBatchCalls).toBe(1)
expect(stats.getBatchCalls).toBe(0)
```

**Test 2: Warm SELECT after UPDATE = 0 KV ops**
```
increment()           // warm the page
resetStats()
getCount()            // SELECT (should hit pager cache)
stats = getStats()
expect(stats.getBatchCalls).toBe(0)
expect(stats.putBatchCalls).toBe(0)
```

**Test 3: Warm UPDATE + SELECT = 1 putBatch, 0 getBatch**
```
increment()           // warm
resetStats()
incrementAndRead()    // UPDATE + SELECT
stats = getStats()
expect(stats.putBatchCalls).toBe(1)
expect(stats.getBatchCalls).toBe(0)
```

**Test 4: Multi-page INSERT (with index) = 1 putBatch**
```
resetStats()
insertWithIndex()     // INSERT into table with index (touches 2-3 pages)
stats = getStats()
expect(stats.putBatchCalls).toBe(1)   // all pages in one batch
log = getLog()
// verify log shows single putBatch with multiple chunk keys
expect(log.filter(e => e.op === "putBatch")).toHaveLength(1)
expect(log[0].keys.filter(k => k.startsWith("chunk:"))).toHaveLength(...)
```

**Test 5: ROLLBACK = 0 putBatch, 0 getBatch**
```
resetStats()
rollbackTest()        // BEGIN; INSERT ...; ROLLBACK;
stats = getStats()
expect(stats.putBatchCalls).toBe(0)
expect(stats.getBatchCalls).toBe(0)
```

**Test 6: Multi-statement transaction = 1 putBatch**
```
resetStats()
multiStmtTx()        // BEGIN; INSERT; INSERT; COMMIT;
stats = getStats()
expect(stats.putBatchCalls).toBe(1)   // all writes batched to single commit
```

**Test 7: Verify no journal file operations**
```
resetStats()
increment()
log = getLog()
// No operations should touch journal or WAL file tags
for (const entry of log) {
  for (const key of entry.keys) {
    expect(key).not.toContain("journal")
    expect(key).not.toContain("wal")
  }
}
```

**Test 8: putBatch entry count within limits**
```
resetStats()
increment()
log = getLog()
const putBatchEntry = log.find(e => e.op === "putBatch")
expect(putBatchEntry.keys.length).toBeLessThanOrEqual(128)
```

**Test 9: Large transaction (>127 pages) falls back to journal**

This test verifies that transactions exceeding the 128-entry putBatch limit
still complete correctly by falling back to SQLite's standard journal mode.

The test inserts enough data in a single transaction to dirty >127 pages. With
4 KiB pages, this requires writing ~508 KiB of data. A table with a TEXT
column and an index will dirty roughly 1 page per ~3.5 KiB of row data (data
page fills, B-tree splits, index pages). Inserting ~200 rows of ~2 KiB each
in a single BEGIN/COMMIT should exceed 127 dirty pages.
```
resetStats()
bulkInsertLarge()     // BEGIN; INSERT x200 large rows; COMMIT;
stats = getStats()
log = getLog()

// BATCH_ATOMIC COMMIT failed, so SQLite fell back to journal mode.
// Verify journal file operations appeared in the log.
const journalOps = log.filter(e =>
  e.keys.some(k => k.includes("journal"))
)
expect(journalOps.length).toBeGreaterThan(0)

// Verify multiple putBatch calls (journal write + main write batches).
expect(stats.putBatchCalls).toBeGreaterThan(1)

// Verify all putBatch calls stayed within the 128-entry limit.
for (const entry of log.filter(e => e.op === "putBatch")) {
  expect(entry.keys.length).toBeLessThanOrEqual(128)
}
```

**Test 10: Large transaction data integrity after journal fallback**

Verifies that data written via the journal fallback path is correct and the
database is not corrupted.
```
bulkInsertLarge()     // INSERT ~200 large rows
count = getRowCount() // SELECT COUNT(*)
expect(count).toBe(200)

// Verify database integrity
integrityResult = runIntegrityCheck()  // PRAGMA integrity_check
expect(integrityResult).toBe("ok")
```

**Test 11: Large transaction survives actor restart**

Verifies that data written via journal fallback is durable across actor
restarts (the journal was fully committed and deleted before the actor
responds).
```
bulkInsertLarge()
// Force actor restart (destroy + recreate)
destroyAndRecreate()
count = getRowCount()
expect(count).toBe(200)

integrityResult = runIntegrityCheck()
expect(integrityResult).toBe("ok")
```

These tests serve as regression guards. If BATCH_ATOMIC stops working (e.g.,
wa-sqlite update removes the flag, or a PRAGMA change disables it), the round
trip counts will spike and tests will fail immediately. If the journal fallback
is broken, the large transaction tests will catch it.

---

## Adversarial Review Summary

Three independent adversarial reviews identified issues ranked by severity.
Most of these are resolved by the BATCH_ATOMIC approach.

### Critical Issues (resolved by BATCH_ATOMIC)

**1. Multi-page transaction atomicity gap**

With `journal_mode = OFF`, each `xWrite` triggers a separate `putBatch`. Crash
between writes = corruption. BATCH_ATOMIC solves this: all writes are buffered
and flushed in one putBatch on COMMIT_ATOMIC_WRITE.

**2. Flush semantics (per-statement vs per-transaction)**

The original spec flushes per-statement, breaking multi-statement transactions.
BATCH_ATOMIC solves this: SQLite calls BEGIN/COMMIT at the correct transaction
boundaries. No application-level detection needed.

**3. putBatch failure during flush**

With BATCH_ATOMIC: if putBatch fails on COMMIT_ATOMIC_WRITE, return
SQLITE_IOERR. SQLite's in-memory journal still has the original pages. SQLite
can attempt ROLLBACK_ATOMIC_WRITE (discard buffer) and the pager falls back to
its pre-transaction state. This is strictly safer than journal_mode=OFF where
there is no recovery path.

### High Issues

**4. KV batch limits (128 keys / 976 KiB)**

Still applies. Transactions exceeding 127 dirty pages need either a hard limit
or a multi-batch protocol. Start with hard limit.

**5. xTruncate + dirty buffer interaction**

Still applies in batch mode. xTruncate during a batch must purge buffered writes
beyond the truncation point. But this is simpler now: the buffer is only active
between BEGIN_ATOMIC_WRITE and COMMIT/ROLLBACK, so the scope is well-defined.

**6. ROLLBACK broken with journal_mode=OFF**

Fully resolved by BATCH_ATOMIC. SQLite keeps an in-memory journal and
ROLLBACK_ATOMIC_WRITE discards the buffer. Standard ROLLBACK works correctly.

### Medium Issues

**7. No rollback/detection plan**: Still relevant. Add PRAGMA quick_check on
startup.

**8. Hibernation flush failure**: Still relevant. Log as error.

**9. Validation plan**: Must pass all driver FS tests. Test ROLLBACK behavior.

---

## Industry Research: How Others Handle KV-Backed SQLite

### Cloudflare D1 / Durable Objects

- SQLite runs on **local disk** with a custom VFS intercepting WAL writes.
- **WAL mode exclusively**. The VFS captures WAL frames and replicates them to
  5 followers across data centers (3-of-5 quorum for durability).
- Storage Relay Service (SRS) batches WAL changes for up to 10 seconds / 16 MB
  before uploading to cold storage.
- "Output gate" pattern: blocks external communication until durability is
  confirmed by quorum.
- No per-page KV round trips. Reads are local disk speed (microseconds).

### Litestream (Fly.io)

- **Not a custom VFS** (originally). Runs as a background process intercepting
  WAL file changes.
- **WAL mode required**. Litestream holds a read transaction to prevent
  auto-checkpoint, then copies new WAL frames to "shadow WAL" files.
- `SYNCHRONOUS=NORMAL` recommended (fsync only at checkpoint, not per-tx).
- Newer version has a writable VFS that batches dirty pages into LTX files
  (similar to our putBatch approach).
- **Key lesson**: Dirty page batching + atomic upload is the right pattern for
  remote-storage-backed SQLite.

### rqlite

- SQLite on local disk with **WAL mode** and **`SYNCHRONOUS=OFF`**.
- Durability comes from Raft log (BoltDB), not SQLite's own durability.
- Statement-based replication (ships SQL, not pages).
- **Queued writes**: Batches multiple write requests into a single Raft entry.
  ~15x throughput improvement.
- Periodic fsync strategy tied to Raft snapshots. SQLite file may be
  inconsistent after OS crash; rebuilt from Raft log.

### Turso / libSQL

- Fork of SQLite with **virtual WAL methods** (hooks for custom WAL backends).
- **WAL mode exclusively** for replication.
- "Bottomless" VFS replicates WAL frames to S3 in batches (1000 frames or 10s).
- Embedded replicas for local reads.

### LiteFS (Fly.io)

- **FUSE-based filesystem** intercepting file operations.
- Supports both rollback journal and WAL mode, converts both to unified LTX
  format.
- Detects transaction boundaries by watching journal lifecycle (create/delete)
  or WAL commit frames.
- Rolling checksums detect split-brain.

### wa-sqlite IDBBatchAtomicVFS (most relevant)

- **Uses SQLITE_IOCAP_BATCH_ATOMIC** to eliminate external journal entirely.
- IndexedDB transactions provide atomicity natively.
- SQLite keeps journal in memory, calls BEGIN/COMMIT/ROLLBACK file controls.
- VFS buffers writes and commits atomically using IndexedDB transaction.
- **This is exactly our architecture** (KV store instead of IndexedDB).

### Key Industry Patterns

| Pattern | Used by | Applicability |
|---------|---------|---------------|
| Local disk + WAL interception | CF D1/DO, Litestream, LiteFS | Not applicable (no local disk) |
| External durability, SQLite sync=OFF | rqlite | Applicable but risky without journal |
| Virtual WAL hooks | Turso/libSQL | Requires SQLite fork |
| **BATCH_ATOMIC + in-memory journal** | **wa-sqlite IDBBatchAtomicVFS** | **Directly applicable** |
| FUSE passthrough | LiteFS | Not applicable (no filesystem) |

The industry consensus for non-filesystem SQLite backends that need atomic
multi-page writes is SQLITE_IOCAP_BATCH_ATOMIC. It's the mechanism SQLite
provides specifically for this use case.

---

## Additional PRAGMAs for Single-Writer

### Recommended set (with BATCH_ATOMIC)

```sql
PRAGMA locking_mode = EXCLUSIVE;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA auto_vacuum = NONE;
```

Note: `journal_mode` is left as default (DELETE). SQLite will use an in-memory
journal when BATCH_ATOMIC is active. No journal file I/O occurs.

### PRAGMAs removed vs original spec

- `journal_mode = OFF`: Removed. Let SQLite manage the journal (in-memory with
  BATCH_ATOMIC). This restores ROLLBACK support and crash safety.
- `synchronous = OFF`: Changed to NORMAL. Avoids confirmed bug with
  BATCH_ATOMIC. No performance impact because BATCH_ATOMIC bypasses sync.

### Optional

**`cache_size = -N`** (e.g., `-4096` for 4 MiB): Larger pager cache reduces
cold reads. Worth tuning per workload.

---

## Round Trip Analysis (Updated)

| Approach | Warm write RT | Cold write RT | ROLLBACK works | Crash safe |
|----------|--------------|---------------|----------------|------------|
| DELETE journal over KV | 3+ | 4+ | Yes | Yes |
| journal_mode=OFF (no buffer) | 1 per page | 2 per page | **No** | **No** |
| journal_mode=OFF + buffer | 1 | 2 | Must implement | Yes (within limit) |
| **BATCH_ATOMIC** | **1** | **2** | **Yes (native)** | **Yes (within limit)** |
| WAL over KV | 2-4 | 3-5 | Yes | Yes |

BATCH_ATOMIC achieves the same 1 RT warm writes as journal_mode=OFF + buffer,
but with native ROLLBACK support, SQLite-managed transaction boundaries, and
graceful fallback for edge cases.

---

## Sources

- [SQLite Atomic Commit](https://sqlite.org/atomiccommit.html)
- [SQLite Batch Atomic Write Tech Note](https://www3.sqlite.org/cgi/src/technote/714f6cbbf78c8a1351cbd48af2b438f7f824b336)
- [SQLite File Control Constants](https://sqlite.org/c3ref/c_fcntl_begin_atomic_write.html)
- [wa-sqlite IDBBatchAtomicVFS](https://github.com/rhashimoto/wa-sqlite/blob/master/src/examples/IDBBatchAtomicVFS.js)
- [wa-sqlite BATCH_ATOMIC Discussion](https://github.com/rhashimoto/wa-sqlite/discussions/78)
- [Cloudflare: Zero-latency SQLite in Durable Objects](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- [Cloudflare: D1 Read Replication](https://blog.cloudflare.com/d1-read-replication-beta/)
- [How Litestream Works](https://litestream.io/how-it-works/)
- [Litestream Writable VFS (Fly.io)](https://fly.io/blog/litestream-writable-vfs/)
- [rqlite Design](https://rqlite.io/docs/design/)
- [rqlite Performance](https://rqlite.io/docs/guides/performance/)
- [libSQL Engineering Deep Dive](https://compileralchemy.substack.com/p/libsql-diving-into-a-database-engineering)
- [LiteFS Architecture](https://github.com/superfly/litefs/blob/main/docs/ARCHITECTURE.md)
- [F2FS Atomic Writes + SQLite](https://www3.sqlite.org/cgi/src/technote/714f6cbbf78c8a1351cbd48af2b438f7f824b336)
