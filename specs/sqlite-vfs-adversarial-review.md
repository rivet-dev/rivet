# SQLite VFS Single-Writer Optimization: Adversarial Review

Date: 2026-03-21
Spec: `specs/sqlite-vfs-single-writer-optimization.md`

26 issues were raised by three adversarial reviewers (correctness, completeness,
implementability). Each issue was then validated against the actual source code.
A third-round SQLite contract review added 6 more issues (27-32).

## Verdict Summary

| # | Issue | Severity | Verdict | Action |
|---|-------|----------|---------|--------|
| 1 | Copy-paste bug in BEGIN_ATOMIC_WRITE | CRITICAL | **BULLSHIT** | None. Reviewer misread the pseudocode. |
| 2 | xFileControl must be async | CRITICAL | **REAL** | Add `xFileControl` to `SQLITE_ASYNC_METHODS`. |
| 3 | Journal fallback RT wrong by ~100x | CRITICAL | **PARTIAL** | Spec must clarify fallback batching strategy. |
| 4 | Existing DBs have journal_mode=OFF in header | CRITICAL | **BULLSHIT** | None. journal_mode=OFF is session-level, not persistent. |
| 5 | Buffer limit off-by-one | HIGH | **REAL** | Fix check to `buffer.size > 127` (pages only). |
| 6 | ROLLBACK_ATOMIC must restore file.size | HIGH | **REAL** | Spec must require saving/restoring file.size. |
| 7 | Metadata tracking unspecified | HIGH | **PARTIAL** | Real gap is file.size (Issue 6). Metadata construction is obvious. |
| 8 | xTruncate underspecified | HIGH | ~~PARTIAL~~ **BULLSHIT** | Superseded by Issue 29: xTruncate cannot be called during batch mode. |
| 9 | Partial-chunk writes need async reads | HIGH | **BULLSHIT** | CHUNK_SIZE = page_size = 4096. All writes are full chunks. |
| 10 | Batch mode scope ambiguous | MEDIUM | **REAL** | Spec must state batchMode is per-OpenFile. |
| 11 | xSync during batch mode | MEDIUM | **PARTIAL** | SQLite won't call xSync during batch mode. Non-issue in practice. |
| 12 | xRead buffer check is load-bearing | MEDIUM | **BULLSHIT** | EXCLUSIVE mode pager cache handles page 1. Buffer check is defense-in-depth. |
| 13 | putBatch atomicity is a hard dependency | MEDIUM | **REAL** | Spec should explicitly state this as a requirement. Production KV (FDB) provides it. |
| 14 | Error state after putBatch failure | MEDIUM | ~~REAL~~ **WRONG** | Superseded by Issue 28: COMMIT must tear down batch mode itself, not defer to ROLLBACK_ATOMIC. |
| 15 | 976 KiB payload limit not checked | MEDIUM | **PARTIAL** | Math works out (512 KiB < 976 KiB). Spec should document why. |
| 16 | No BATCH_ATOMIC verification test | MEDIUM | **REAL** | Test 7 covers this but spec should call out the risk explicitly. |
| 17 | Empty transaction unspecified | LOW | **BULLSHIT** | No dirty pages = no BATCH_ATOMIC calls. Standard SQLite. |
| 18 | Page 1 always dirty | LOW | **BULLSHIT** | Doesn't affect RT count (all pages go in one putBatch). |
| 19 | No mechanism to decline batch mode | LOW | **BULLSHIT** | COMMIT failure triggers fallback naturally. No separate mechanism needed. |
| 20 | Test 9 is vacuous | LOW | **PARTIAL** | Weak alone but Tests 10+11 complete the verification. |
| 21 | No cold start test | LOW | **PARTIAL** | Test 11 covers restart. Explicit cold-start RT test would be nice. |
| 22 | PRAGMA execution order | LOW | **BULLSHIT** | These four PRAGMAs have no order-dependent interactions. |
| 23 | Dirty buffer key format | LOW | **BULLSHIT** | Defined by existing KV key system. |
| 24 | Other xDeviceCharacteristics flags | LOW | **PARTIAL** | BATCH_ATOMIC is the only flag that matters for KV backend. |
| 25 | xFileControl return SQLITE_NOTFOUND | LOW | **BULLSHIT** | Already correct in current code. |
| 26 | xDelete during batch mode | LOW | **BULLSHIT** | Never called during batch mode. |
| 27 | Spec conflates SQL ROLLBACK with ROLLBACK_ATOMIC_WRITE | HIGH | **REAL** | Rewrite ROLLBACK references. Fix Test 5. |
| 28 | COMMIT_ATOMIC_WRITE failure must tear down batch mode | HIGH | **REAL** | COMMIT exits batch mode on failure per SQLite contract. Supersedes Issue 14. |
| 29 | Batch-mode xTruncate is dead code | MEDIUM | **REAL** | Remove pendingDeletes design. SQLite only calls xWrite + SIZE_HINT during batch. |
| 30 | auto_vacuum=NONE is persistent, not a runtime switch | MEDIUM | **REAL** | Document limitation for existing DBs. |
| 31 | No explicit journal_mode=DELETE guard | MEDIUM | **REAL** | Add PRAGMA or assertion. WAL persists in DB header. |
| 32 | page_size=4096 assumed but not enforced | MEDIUM | **REAL** | Add PRAGMA page_size=4096 or runtime assertion. |

**Totals: 12 REAL, 7 PARTIAL, 12 BULLSHIT, 1 WRONG (superseded)**

---

## REAL issues (must fix in spec)

### 2. xFileControl must be registered as async

`xFileControl` is not in `SQLITE_ASYNC_METHODS` (vfs.ts:85-95). The wa-sqlite
async relay uses `hasAsyncMethod()` at VFS registration time to decide which
callbacks get an async trampoline. If `xFileControl` is not listed, the WASM
bridge calls it synchronously and does NOT await the returned Promise.

COMMIT_ATOMIC_WRITE calls `putBatch` which is async. Without async registration,
`putBatch` fires-and-forget, SQLite sees a non-zero return value (the Promise
object coerced to a number), and interprets it as an error.

**Fix**: Add `"xFileControl"` to `SQLITE_ASYNC_METHODS`. The wa-sqlite framework
already supports async xFileControl (it's in `VFS_METHODS`); it just needs to be
declared. The IDBBatchAtomicVFS doesn't need it async because IDB transactions
are queued synchronously, but our KV VFS needs it for `putBatch`.

### 5. Buffer limit off-by-one

The COMMIT pseudocode checks `buffer entries > 128`. The buffer contains only
page chunks (from xWrite). Metadata is added separately at commit time:
`putBatch(buffer entries + metadata)`. If the buffer has 128 page entries, adding
1 metadata = 129 keys, exceeding the 128-key KV limit.

The narrative section says "127 pages + 1 metadata" which is correct, but the
pseudocode contradicts it.

**Fix**: Change the check to `buffer.size > 127` (page entries only), so
127 pages + 1 metadata = 128 keys fits exactly.

### 6. ROLLBACK must restore file.size

During batch mode, xWrite must update `file.size` in memory (otherwise
`xFileSize` returns stale values). On ROLLBACK_ATOMIC_WRITE, the buffer is
discarded but `file.size` remains at the extended value. Subsequent operations
see a file size that doesn't match KV state.

**Fix**: Save `file.size` at BEGIN_ATOMIC_WRITE time. Restore it on
ROLLBACK_ATOMIC_WRITE. (On COMMIT, the persisted metadata catches up to the
in-memory value, so no restore needed.)

### 10. Batch mode scope must be per-file

The spec defines `batchMode` and the dirty buffer without specifying their
scope. SQLite sends xFileControl to the main database file handle only. During
journal fallback, xWrite calls go to the journal file via a different fileId.
If batchMode is global, journal writes during fallback would be incorrectly
buffered.

**Fix**: Explicitly state that `batchMode` and the dirty buffer are properties
on `OpenFile`, not on the `SqliteSystem` or `Database`.

### 13. putBatch atomicity is a hard requirement

The safety argument states "KV `putBatch` is all-or-nothing" but this is
asserted, not validated. The entire batch path crash recovery depends on this.
In production (FoundationDB), this holds. In test/dev drivers, it may not.

**Fix**: Add a note in the safety argument that putBatch atomicity is a hard
requirement for the batch path to be safe, and that the KV driver must guarantee
all-or-nothing semantics.

### 14. ~~Buffer state after putBatch failure~~ (SUPERSEDED by Issue 28)

Original recommendation was to leave the buffer intact on putBatch failure and
let ROLLBACK_ATOMIC_WRITE clean up. This is **wrong per the SQLite contract**.
The SQLite xFileControl documentation states: "Regardless of whether or not the
commit succeeded, the batch write mode is ended by this file control." See
Issue 28 for the correct approach: COMMIT_ATOMIC_WRITE must tear down batch
mode itself on failure.

### 16. No verification that BATCH_ATOMIC is active

If xDeviceCharacteristics doesn't return the flag (implementation bug), SQLite
silently falls back to journal mode for every transaction. No error, no warning.
Test 7 ("no journal file operations") would catch this, but the spec should
explicitly call out the risk.

**Fix**: Add a note that Test 7 serves as the BATCH_ATOMIC activation check.
Consider also adding a one-time log at open time confirming BATCH_ATOMIC is
active (e.g., "VFS: BATCH_ATOMIC enabled").

## PARTIAL issues (clarify in spec)

### 3. Journal fallback round trips

The spec claims ~6 RT for N=200 pages during fallback. With the current xWrite
implementation, each xWrite call is a separate putBatch (1 RT each). So the
naive fallback is ~400 RT, not ~6.

The `ceil(N/127)` formula only works if the implementation adds write-batching
in the non-batch xWrite path (e.g., accumulate writes and flush at xSync). The
spec doesn't describe this.

**Status**: The fallback is explicitly described as rare (most transactions touch
fewer than 127 pages). The spec should either (a) acknowledge the fallback is
~N*2 RT with current xWrite, or (b) describe an xSync-triggered flush mechanism
for the fallback path.

### 7. Metadata tracking

The spec says "putBatch(buffer entries + metadata)" without specifying when
metadata is constructed. This is an obvious implementation detail (construct at
commit time from current file.size). The real gap is file.size tracking, covered
by Issue 6.

### 8. ~~xTruncate in batch mode~~ (SUPERSEDED by Issue 29)

Originally flagged as a gap in deletion tracking. Reclassified as BULLSHIT
because the SQLite FCNTL documentation states that between BEGIN_ATOMIC_WRITE
and COMMIT/ROLLBACK, the only operations on the file descriptor are xWrite and
xFileControl(SIZE_HINT). xTruncate is never called during batch mode. See
Issue 29.

### 15. 976 KiB payload limit

The entry count check (127 pages) implicitly enforces the payload limit:
127 * 4096 + overhead = ~525 KiB, well under 976 KiB. The spec should document
this arithmetic so future readers don't have to derive it.

### 20. Test 9 is weak alone

Test 9 only checks that journal operations appeared and putBatchCalls > 1. It
doesn't verify efficiency of the fallback. But Tests 10+11 verify data integrity
and restart survival, completing the verification.

### 21. No explicit cold-start RT test

Test 11 covers restart survival. A dedicated test for cold-start round trip
counts (first-ever actor open + migration + first write) would be useful but
isn't critical.

### 24. Other xDeviceCharacteristics flags

`SQLITE_IOCAP_ATOMIC`, `SQLITE_IOCAP_SAFE_APPEND`, etc. could provide minor
optimizations. But BATCH_ATOMIC is the only flag that fundamentally changes the
commit path. Low priority.

## BULLSHIT issues (no action needed)

### 1. Copy-paste bug in BEGIN_ATOMIC_WRITE

Reviewer misread the pseudocode. The actual spec text for BEGIN is:
```
BEGIN_ATOMIC_WRITE (31):
  Set batchMode = true.
  Allocate dirty buffer: Map<string, Uint8Array>.
  Return SQLITE_OK.
```
No "clear buffer" or "set batchMode = false" appears. Those lines are in the
COMMIT and ROLLBACK handlers where they belong.

### 4. Existing databases have journal_mode=OFF in header

`journal_mode=OFF` is session-level in SQLite, NOT persistent. Only WAL mode
modifies the database header (bytes 18-19 set to 0x02). All rollback modes
(DELETE, TRUNCATE, PERSIST, OFF) use header bytes 0x01 and must be set via
PRAGMA on every open. Removing the PRAGMA means existing databases open in
DELETE mode (the default), which is exactly what the spec wants.

Additionally, BATCH_ATOMIC eligibility does NOT check journal_mode. The four
conditions are: (1) single database, (2) xDeviceCharacteristics includes
BATCH_ATOMIC, (3) synchronous is not OFF, (4) journal in memory.

### 9. Partial-chunk writes need async KV reads

CHUNK_SIZE = page_size = 4096 (intentionally aligned, documented in kv.ts:17).
SQLite's page-level I/O always writes complete pages. Every xWrite during batch
mode is a complete chunk replacement. No partial-chunk merging needed.

### 11. xSync during batch mode

SQLite does not call xSync between BEGIN_ATOMIC_WRITE and COMMIT_ATOMIC_WRITE.
The batch path bypasses sync logic entirely. During fallback, batchMode is
already false (ROLLBACK_ATOMIC exited it) before xSync is called. xSync during
batchMode=true is not a real scenario.

### 12. xRead buffer check is load-bearing for page 1

With `locking_mode = EXCLUSIVE`, the pager cache is trusted across transactions.
Page 1's change counter is updated in-cache via `pager_incr_changecounter()` and
read back from cache, not via xRead. The buffer check is defense-in-depth, not
a correctness requirement.

### 17-19, 22-23, 25-26

These are either standard SQLite behavior that doesn't need specification,
already handled by existing code, or implementation details that are obvious
from the existing VFS code. See verdict table above.

---

## Third-round issues (SQLite contract review)

### 27. Spec conflates SQL ROLLBACK with ROLLBACK_ATOMIC_WRITE

The BATCH_ATOMIC protocol lives inside `sqlite3PagerCommitPhaseOne()`. The
sequence is: user issues COMMIT (or autocommit) -> pager calls
BEGIN_ATOMIC_WRITE -> xWrite per dirty page -> COMMIT_ATOMIC_WRITE. A
user-issued `BEGIN; INSERT; ROLLBACK;` never enters the commit path at all.
The pager restores pages from its in-memory journal. No xFileControl calls
happen.

The spec conflates these in four places:

- Line 66: "On ROLLBACK: SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE" presented as a
  general rollback mechanism, when it only occurs during commit failure.
- Line 272 (safety argument): "The VFS discards the dirty buffer" during
  ROLLBACK. But a user-issued ROLLBACK never enters batch mode, so there is
  no dirty buffer to discard.
- Line 196 (RT table): "ROLLBACK = 0 RT" is true but not because of
  BATCH_ATOMIC. It's because the pager cache rollback is in-memory.
- Test 5 (line 383): `rollbackTest` (`BEGIN; INSERT; ROLLBACK;`) asserting
  `getBatchCalls == 0` is too strong. The INSERT inside the transaction may
  require cold-page reads via xRead -> getBatch. The correct assertion is
  `putBatchCalls == 0` (no writes persisted), but getBatch may be non-zero.

**Fix**: Rewrite all four locations to distinguish between:
1. SQL ROLLBACK: pager restores from in-memory journal. No batch mode involved.
   No KV writes (putBatch == 0). KV reads are possible for cold pages.
2. ROLLBACK_ATOMIC_WRITE: only called during commit failure fallback. Discards
   the dirty buffer and restores file.size.

Test 5 should assert `putBatchCalls == 0` only, not `getBatchCalls == 0`.

### 28. COMMIT_ATOMIC_WRITE failure must tear down batch mode

The SQLite xFileControl documentation states: "Regardless of whether or not
the commit succeeded, the batch write mode is ended by this file control."

The spec (line 138, 142) says the opposite: "Do NOT clear the buffer here.
ROLLBACK_ATOMIC_WRITE will clean up." and "Leave buffer intact."

While current pager.c does issue ROLLBACK_ATOMIC as a best-effort hint after
COMMIT failure, the FCNTL contract says COMMIT itself ends batch mode. Relying
on ROLLBACK_ATOMIC for cleanup is coding against an implementation detail of
pager.c, not the documented VFS contract.

**Fix**: Rewrite COMMIT_ATOMIC_WRITE pseudocode:

```
COMMIT_ATOMIC_WRITE (32):
  If dirtyBuffer.size > 127:
    Clear dirtyBuffer, restore file.size, set batchMode = false.
    Return SQLITE_IOERR.
  Construct entries: [...dirtyBuffer entries, metadata].
  Call putBatch(entries).
  If putBatch fails:
    Clear dirtyBuffer, restore file.size, set batchMode = false.
    Return SQLITE_IOERR.
  Clear dirtyBuffer, set batchMode = false.
  Return SQLITE_OK.
```

ROLLBACK_ATOMIC_WRITE becomes a no-op if batch mode is already exited (which
is the expected case). Keep it as a defensive fallback that checks batchMode
before acting.

Supersedes Issue 14.

### 29. Batch-mode xTruncate design is dead code

The SQLite FCNTL documentation for BEGIN_ATOMIC_WRITE states: "Between the
BEGIN and COMMIT or ROLLBACK calls, the only operations on that file descriptor
will be xWrite calls and possibly xFileControl(SQLITE_FCNTL_SIZE_HINT) calls."

xTruncate is never called during the batch window. The entire `pendingDeletes`
/ tombstone design (spec lines 173-183) is solving a path SQLite does not
take. If stale KV chunks need cleanup after VACUUM or file shrink, that should
be modeled as post-commit cleanup, not as part of the atomic batch.

**Fix**: Remove the batch-mode xTruncate section entirely. Drop
`pendingDeletes` from the OpenFile fields. If KV chunk cleanup after truncation
is needed, design it as a separate post-commit mechanism.

Supersedes Issue 8.

### 30. auto_vacuum=NONE is persistent, not a runtime switch

`auto_vacuum` is stored in the database header (bytes 52-55). Unlike
`journal_mode` (session-level for rollback modes) and `synchronous`
(session-level), `auto_vacuum` persists. SQLite only allows changing
auto_vacuum mode on empty databases. For existing databases with tables,
`PRAGMA auto_vacuum = NONE` is silently ignored.

This means: if an actor database was ever opened by a version of the code that
did not set auto_vacuum (allowing SQLite's default, which varies by build),
the PRAGMA on re-open does nothing. The database keeps its original mode.

**Fix**: Document in the spec that `auto_vacuum = NONE` only takes effect for
newly created databases. For existing databases, the mode is whatever was set
at creation time. Since the current code does NOT set auto_vacuum, existing
DBs use whatever default SQLite was compiled with (typically NONE, but not
guaranteed). This is acceptable for actor databases that are small and
short-lived, but should be documented as a known limitation.

### 31. No explicit journal_mode=DELETE guard

The spec says "No journal_mode PRAGMA" and relies on SQLite's default (DELETE).
However, WAL mode persists in the database header (bytes 18-19 set to 0x02).
If a database ever enters WAL mode (e.g., user code runs
`PRAGMA journal_mode=WAL`), reopening without an explicit journal_mode PRAGMA
would keep WAL mode, which interacts with BATCH_ATOMIC differently than
expected.

Current code sets `journal_mode=OFF` (session-level, does not persist). The
new code removes this PRAGMA entirely. DELETE is the default for non-WAL
databases, so this is safe as long as no database ever enters WAL mode.

**Fix**: Add `PRAGMA journal_mode = DELETE` to the PRAGMA list. This is cheap
insurance: it's a no-op for databases already in DELETE mode, and it forces
WAL databases back to DELETE mode. Alternatively, assert at open time that the
journal mode is not WAL.

### 32. page_size=4096 assumed but not enforced

The spec assumes `page_size == CHUNK_SIZE == 4096` throughout: "all SQLite
xWrite calls are complete chunk replacements" (line 163), the buffer limit
math (127 * 4096), and the payload limit proof (525 KiB). kv.ts:27 explicitly
warns: "If page_size is ever changed via PRAGMA, CHUNK_SIZE must be updated."

But nothing in the VFS prevents a different page_size. If user code runs
`PRAGMA page_size = 8192` before creating tables, writes would span 2 chunks
per page, breaking the 1:1 mapping assumption.

**Fix**: Add `PRAGMA page_size = 4096` to the PRAGMA list, or add a runtime
assertion after open that verifies `PRAGMA page_size` returns 4096. The PRAGMA
approach is preferred because it's declarative and takes effect before any
tables are created.
