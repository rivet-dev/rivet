# SQLite Deferred Commits -- Specification

## 1. Overview

Deferred commits let a SQLite transaction return to the caller as soon as it
has committed to the actor-local page state, before the engine has made it
durable. A background flusher ships committed pages to the engine in order and
publishes a monotonic **flush sequence** that callers can wait on. This is the
storage-layer half of Cloudflare Durable Object output gates: the runtime layer
decides *when* to wait; the storage layer guarantees *what* a wait means.

Today every SQLite commit in the native VFS blocks the SQLite worker thread
until the engine acknowledges the depot commit
(`SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` -> `commit_atomic_write` ->
`block_on_buffered_commit`). With the synchronous SQLite API that turns every
`INSERT` into one engine round trip on the JavaScript thread. Deferred commits
remove the round trip from the commit path and move it to a single flusher.

Deferred mode is **opt-in per actor**. The default stays `awaited`, which is
byte-for-byte today's behavior.

### 1.1 Goals

- `executeSync`/`transactionSync` writes return without an engine round trip.
- Read-your-own-writes is preserved across every connection to the same
  database, whether or not the writes are durable yet.
- A caller can wait until a specific point in commit history is durable, with
  snapshot semantics: later commits never extend an earlier wait.
- Commits reach the engine in order and each engine commit is atomic.
- A flush that cannot complete permanently breaks the database; every pending
  and future wait rejects, and the actor generation stops. Nothing that waited
  can observe a lost write as durable.
- The API surface is sufficient to implement Durable Object storage semantics
  (sync SQL, sync and async KV, implicit and explicit transactions, `sync()`,
  `allowUnconfirmed`) on top of it without further storage changes.

### 1.2 Non-goals

- Input gates, output gates, `blockConcurrencyWhile`, and actor reset live in
  the runtime that consumes this API. This spec does not implement them.
- Pipelining more than one engine commit at a time. The flusher keeps exactly
  one commit in flight. The sequence-number API does not change if pipelining
  is added later.
- Protocol or engine changes. The design uses the existing `sqlite_commit` and
  `sqlite_get_pages` requests and the existing head-txid and generation fences.
  There is no commit identity, so an indeterminate commit whose resend hits the
  head fence cannot be proven ours and breaks the database (section 3.5). A
  commit nonce in the protocol is the follow-up that removes that restart.
- WebAssembly and remote-SQLite backends. They reject `deferred` at actor
  context construction.
- An SQLite authorizer that rejects transaction-control statements in user SQL
  (workerd does this). Section 6.6 records the deviation.

## 2. Terminology

| Term | Meaning |
| --- | --- |
| **Local commit** | The VFS finished a SQLite transaction that changed at least one page or the database size. In deferred mode the change is in the overlay, not yet at the engine. Read-only transactions and no-op syncs are not local commits. |
| **Commit sequence** (`commit_seq`) | Monotonic `u64`, incremented once per local commit. Continues from the previous open of the same actor within the process (section 3.1). |
| **Flushed sequence** (`flushed_seq`) | The highest commit sequence whose changes the engine has acknowledged. Always `<= commit_seq`. In awaited mode it equals `commit_seq` at all times. |
| **Overlay** | Pinned, never-evicted map of page versions belonging to local commits that are not yet acknowledged. |
| **Batch** | The overlay snapshot sent to the engine in one `sqlite_commit` request. A batch covers every local commit with sequence `<= batch.seq`. |
| **Durable head** (`durable_head_txid`) | The engine txid the flusher has proven durable. A plain `u64` in deferred mode: the open path always yields one (`fetch_initial_pages` synthesizes `0` for a database that does not exist yet), and deferred mode is rejected at open if the peer omits it. Owned exclusively by the flusher and the open path. Never assigned from a read response. |
| **Broken** | Terminal state after a flush failure that exhausted retries or was unrecoverable. |

## 3. Semantics

### 3.1 Sequences and waits

- Every local commit, in either mode, increments `commit_seq` under the VFS
  state write lock before the VFS callback returns to SQLite. A commit that
  dirtied no page and did not change the database size does not increment it.
  In awaited mode the existing blocking commit path increments `commit_seq`
  and publishes the same value as `flushed_seq` after the engine acknowledges,
  so the sequence API is uniform across modes.
- On every open both counters start at `initial_commit_seq`; the seed never
  creates deferred work.
- `commit_seq` continues across close and reopen of the same actor within one
  process: `SqliteDb` remembers the last value and seeds the next open with it.
  A sequence captured before a sleep is therefore still meaningful after wake,
  and because close drains (section 3.6) it is always already durable.
  A failed drain still seeds a later open from the final local sequence; the
  lost generation cannot survive sleep with a waiter holding that sequence, so
  preserving monotonicity is preferable to reusing it.
- `wait_for_flush(seq)` resolves once `flushed_seq >= seq`, or rejects with the
  flush error once the database is broken. The error check happens before the
  sequence check, so a wait for an already-flushed sequence still rejects after
  a break.
- The output-gate primitive is `wait_for_flush` of a sequence captured
  synchronously at the call site. The TypeScript wrapper reads `commitSeq()`
  synchronously and passes an explicit sequence to native code; native code
  never defaults the target itself, because an async call may start polling
  after later commits.
- Waiting for `seq > commit_seq` rejects immediately with an invalid-argument
  error. Waiting for `0` resolves immediately.
- In awaited mode `wait_for_flush(seq)` resolves immediately for any valid
  `seq`.

### 3.2 Read-your-own-writes

All connections in the SQLite worker share one `VfsContext`. Page resolution
consults, in order:

1. `write_buffer.dirty` -- pages of the transaction SQLite is executing now.
2. The overlay -- pages of local commits not yet acknowledged.
3. `committed_page_cache` / `page_cache` -- evictable caches of durable pages.
4. The engine via `get_pages`.

A page any unacknowledged commit touched is always served from step 2, so
engine reads are only consulted for pages the in-flight batch did not touch.
Those pages are identical before and after the engine applies the batch, so
reads during an in-flight commit are consistent. The overlay is authoritative
in every read path: `has_readable_page`, the prefetch predictor's `to_fetch`
selection, and response insertion. Fetched pages, including prefetch and
overflow-expanded pages the engine adds to a response, are inserted into
caches only if, at insertion time under the state lock, the page number is
neither dirty nor in the overlay. The "synthesize an empty page 1 when the
database does not exist yet" path keeps its original awaited-mode gate,
`commit_total == 0`. Deferred mode uses `commit_seq == 0` instead, because a
superseded generation with a local commit must never be handed an empty
database before that commit is acknowledged.

### 3.3 Ordering and batching

- The flusher sends at most one `sqlite_commit` at a time.
- While a batch is in flight, further local commits merge into the overlay.
  When the acknowledgement arrives the flusher snapshots everything currently
  in the overlay into the next batch. Later bytes for the same page win. The
  engine therefore sees one atomic commit per batch, txids increase by exactly
  one per batch, and `flushed_seq` jumps to the highest local commit the batch
  covered.
- Ordering invariant: for local commits `a < b`, `b` is never durable unless
  `a` is durable. This follows from "one batch in flight, batches are prefixes
  of commit history".

### 3.4 Truncation

`xTruncate` is a commit boundary of its own: SQLite may shrink the file after
`COMMIT_ATOMIC_WRITE` without another sync. A truncate that changes the size
outside an atomic write produces a local commit with an empty page set and the
new size. A shrink removes overlay pages above the new size. Pages above the
new size that are already in the in-flight batch are harmless: the engine
stores them, and the next batch's smaller `db_size_pages` truncates them
(`depot/src/conveyer/commit/apply.rs`, `collect_truncate_cleanup`).
Acknowledgement never moves a page above the current size into the committed
cache.

### 3.5 Failure handling

Each attempt to ship a batch ends in one of three classes:

| Class | Examples | Action |
| --- | --- | --- |
| **Indeterminate** | transport error, timeout, connection lost, any engine error response that is not a head fence mismatch | Resend the identical request (same pages, same size, same `expected_head_txid`) after backoff, until the retry deadline. |
| **Applied** | commit-ok response | Treat the batch as acknowledged. |
| **Fatal** | head fence mismatch; commit-ok with a head that is not `expected + 1`; retry deadline exceeded; flusher panic or cancellation; VFS already dead | Break the database. |

The head fence is the idempotency check. A resend after an indeterminate
attempt succeeds exactly when the earlier attempt never applied, because the
engine still sits at `expected`. If the earlier attempt did apply, or another
writer advanced the head, the resend fails the fence. Those two cases cannot
be told apart without a commit identity in the protocol: a foreign writer can
produce identical bytes on the batch's pages and also touch pages outside it,
and a size-only batch has no pages to compare at all. So the fence mismatch is
fatal, exactly as it is in awaited mode today. The cost is an actor restart on
a lost acknowledgement, which is rare (the engine applied the commit and the
reply was lost); the data is in fact durable and the next generation reopens
onto it. Every waiter rejects, which is the conservative and correct signal.

Breaking the database, in this order:

- The VFS is marked dead first (`VfsState.dead`, `fatal_error`). The worker
  handle's `check_fatal_error` runs before every enqueue and after every
  reply, so no statement can succeed at the `NativeDatabaseHandle` boundary
  after this point. (SQLite serves reads from its own page cache under
  `locking_mode = EXCLUSIVE`, so `VfsState.dead` alone would not fail them.)
- The terminal error is then stored under the progress lock and published;
  every waiter rejects with it. Because the fatal flag is already set, no
  waiter can observe the break while a new statement can still pass the
  pre-check.
- The database failure channel (section 6.1) resolves with the structured
  error. rivetkit-core's failure monitor reports it once through
  `report_sqlite_worker_fatal`, which calls `stop_actor`. That is the
  Cloudflare "shut down and restart the object" step.
- Overlay pages are discarded with the generation. The next generation reopens
  from the engine's durable head.

All of this is one idempotent operation, `break_database(err)`, used by every
fatal source: the flusher, an out-of-window read response (section 5.6), and
close. The acknowledgement path rechecks the terminal state under the lock
before publishing `flushed_seq`, so a break from another thread is never
overtaken by a late acknowledgement.

The retry deadline is enforced with `tokio::time::timeout_at` on every await
inside the flusher: the commit request, each staged-commit segment, and the
backoff sleep. A hung request cannot outlive the deadline.

### 3.6 Close

Closing a deferred-mode database follows one lifecycle owned by
`NativeDatabaseHandle::close`:

1. Stop admitting SQL (existing worker close request).
2. `NativeDatabaseHandle::close` sets the VFS `closing` flag first, which
	 releases byte backpressure waits (section 3.7). The hard page-count bound
	 still drains before merge while closing. The worker rejects queued SQL
	 and drops `NativeDatabase`. In deferred mode `Drop` never promotes
   `write_buffer` contents into the overlay: pages SQLite spilled for a
   transaction it has not committed must not become a local commit.
   `sqlite3_close_v2` rolls such a transaction back. Size-only truncates were
   already staged at their commit boundary (section 3.4), so nothing
   legitimate is left to stage. `Drop` discards the write buffer, notifies the
   flusher, and never blocks on the engine, which keeps it inside the worker's
   five-second close budget.
3. Request flusher drain (`flush_shutdown`), then `wait_for_flush(commit_seq)`
   bounded by `retry_deadline + retry_backoff_max`.
4. Join the flusher task, then unregister the VFS.

If the worker itself does not close within its five-second budget, close marks
the database broken with `Aborted("worker close timeout")`, aborts the flusher,
and returns the worker timeout without starting a drain. The still-running
worker could otherwise stage work after the drain target was captured.

Close returns the flush error if the database broke, and
`FlushError::Aborted("close deadline")` if the drain did not finish. The
flusher is terminated and joined before `close` returns; it exits through its
shutdown path after a successful drain and is aborted on failure or timeout.
No commit, probe, or verification request is issued by this `VfsContext` after
`close` returns, success or failure, and an attempt cut off by an abort is
logged as indeterminate. `SqliteDb::close` awaits the result; a drain timeout
or flush error during a sleep or stop is logged at error level as lost
unflushed data and completes the shutdown, it does not call `stop_actor`
again. The worker thread's own five-second close budget is untouched because
the drain runs after the worker has closed.

### 3.7 Backpressure

The overlay is bounded by `max_unflushed_bytes`. A local commit that pushes it
over the bound first publishes the commit and notifies the flusher, then blocks
the worker thread until the overlay shrinks below the bound or the database
breaks. This degrades to awaited behavior under sustained engine slowness
instead of growing memory without limit. The in-flight batch shares one `Arc`
page vector across retry attempts and makes only the protocol request's owned
copy, so transient memory is bounded by the overlay plus one shippable batch
and one request serialization copy.

Every formable batch must be shippable. `commit_buffered_pages` refuses more
than `MAX_COMMIT_DIRTY_PAGES` pages in one request, and the flusher cannot
split a coalesced batch at commit boundaries. Two rules keep the bound:
a transaction with more than `MAX_COMMIT_DIRTY_PAGES` dirty pages is rejected
with the same `SQLITE_IOERR` behavior as awaited mode **before** anything is
merged, so SQLite can roll it back. Otherwise, when the existing overlay plus
the transaction would exceed the hard page cap, staging arms the progress
receiver and drains before merging until the batch is formable or the database
breaks. Closing does not bypass this page-count drain. After the merge, staging
returns `SQLITE_OK` unconditionally unless the database is already dead; a
failed `COMMIT_ATOMIC_WRITE` would make SQLite discard pages the overlay
already holds. Byte backpressure waits only on flush progress, a break, or the
`closing` flag, never on a timer. Once `closing` is set that byte wait returns
immediately because the hard page cap is already guaranteed and the close
drain provides durability. No half-cap relationship between the byte and page
limits is required.

### 3.8 Mode interaction with existing features

- Explicit transactions (`db.transaction`, `transactionSync`) and actor state
  transactions (`JsActorStateTransaction`) commit through the same VFS and get
  the same deferral. `await c.saveState()` resolves at local commit in deferred
  mode; runtimes that need durability wait on `wait_for_flush`. Actor state
  persistence, connection state, queue, and alarm writes queue behind an open
  synchronous transaction handle (section 6.4) for at most one turn.
- The commit counters that feed transaction round-trip metrics
  (`commit_total`, `record_commit`, `commit_atomic_count`) count engine
  acknowledgements, not local commits.
- Staged commits for oversized batches are unchanged, except that a retry of a
  staged batch re-begins the stage from scratch.
- Profiling, prefetch, page caches, and startup preload are unchanged.

## 4. Data structures

All new types live in `engine/packages/depot-client/src/vfs.rs` unless noted.
Names are normative; field layout is illustrative. `VfsState` derives
`Clone, Debug`, so everything stored in it must too.

```rust
/// Selected once per database open. Immutable afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommitMode {
    #[default]
    Awaited,
    Deferred,
}

#[derive(Clone, Debug)]
pub struct DeferredCommitConfig {
    /// Break the database after this long without acknowledging the current
    /// batch. Default 30 s. Bounds every await inside the flusher.
    pub retry_deadline: Duration,
    /// Exponential backoff bounds between attempts. Default 50 ms .. 2 s.
    pub retry_backoff_min: Duration,
    pub retry_backoff_max: Duration,
    /// Block local commits once the overlay exceeds this. Default 64 MiB.
    pub max_unflushed_bytes: usize,
}

// VfsConfig gains:
pub struct VfsConfig {
    // ...existing fields...
    pub commit_mode: CommitMode,
    pub deferred_commit: DeferredCommitConfig,
    /// Seeds commit_seq so sequences continue across reopen.
    pub initial_commit_seq: u64,
}
```

```rust
/// One page version in the overlay. `seq` is the local commit that wrote it.
#[derive(Clone, Debug)]
struct OverlayPage {
    bytes: Vec<u8>,
    seq: u64,
}

/// Committed-but-unacknowledged state. Lives inside `VfsState` so it shares
/// the existing state lock with `write_buffer`, `db_size_pages`, and
/// `committed_db_size_pages`.
#[derive(Clone, Debug, Default)]
struct Overlay {
    pages: BTreeMap<u32, OverlayPage>,
    bytes: usize,
    /// Highest local commit sequence.
    commit_seq: u64,
    /// `db_size_pages` as of `commit_seq`.
    db_size_pages: u32,
    /// The batch currently at the engine, if any.
    in_flight: Option<InFlightBatch>,
}

#[derive(Clone, Debug)]
struct InFlightBatch {
    /// Covers every local commit with sequence <= seq.
    seq: u64,
    /// `durable_head_txid` when the batch was formed; sent as the head fence
    /// and reused verbatim by retries.
    expected_head_txid: u64,
    db_size_pages: u32,
    /// Pages included, with the bytes that were sent, so acknowledgement can
    /// move exactly these versions into the committed cache and a resend is
    /// byte-identical.
    pages: Arc<Vec<protocol::SqliteDirtyPage>>,
    started_at: tokio::time::Instant,
    attempts: u32,
}
```

`VfsState` gains `durable_head_txid: u64`, set at open from the initial
pages (`fetch_initial_pages` synthesizes `0` for a database that does not exist
yet). Deferred mode is rejected at open if the peer reports no head, so an
unfenced commit is never sent: the engine skips the fence entirely when it is
`None`, and a lost acknowledgement plus resend would then apply a batch twice.
In deferred mode the existing `head_txid` field is written only at open and by
the flusher's acknowledgement path; `get_pages` responses validate against it
but never assign it. In awaited mode behavior is unchanged.

```rust
/// Progress state, guarded by its own mutex, with a watch channel used only
/// as a change notification. Storing the value under the lock rather than in
/// the channel means a waiter that subscribes late still sees the latest
/// value, and the terminal error is checked before the sequence.
#[derive(Clone, Debug, Default)]
pub struct FlushProgress {
    pub flushed_seq: u64,
    pub error: Option<FlushError>,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum FlushError {
    #[error("sqlite flush retry deadline exceeded after {attempts} attempts: {last_error}")]
    RetryDeadlineExceeded { attempts: u32, last_error: String },
    #[error("sqlite durable head diverged: expected {expected}, engine has {actual:?}")]
    HeadDiverged { expected: u64, actual: Option<u64> },
    #[error("sqlite flusher aborted: {0}")]
    Aborted(String),
    #[error("sqlite flush sequence {requested} is ahead of commit sequence {current}")]
    InvalidSequence { requested: u64, current: u64 },
}

/// Owned by `VfsContext`. Acyclic: the flusher task holds an
/// `Arc<VfsContext>`; the context holds only the task's `JoinHandle`, which
/// does not keep the task alive. `flush_shutdown` plus the supervision guard
/// guarantee the task exits, so the context is always released.
struct FlushController {
    progress: Mutex<FlushProgress>,
    progress_changed: watch::Sender<u64>,   // bumped on every publish
    _progress_rx: watch::Receiver<u64>,     // permanent receiver so sends never fail
    wake: Notify,
    shutdown: AtomicBool,
    /// Set by `NativeDatabaseHandle::close` before the worker is closed.
    closing: AtomicBool,
    task: Mutex<Option<JoinHandle<()>>>,
}
```

`SqliteVfs.ctx` becomes `Arc<VfsContext>`; SQLite's `pAppData` and
`VfsFile.ctx` hold `Arc::as_ptr` raw pointers exactly as they hold the `Box`
pointer today. The flusher is spawned in
`register_with_transport_and_initial_pages` when `commit_mode == Deferred`.

### 4.1 Result metadata

`depot_client_types::ExecuteResult` gains:

- `readonly: Option<bool>` from `sqlite3_stmt_readonly`, read after prepare and
  before finalize. `Some` for the local backend, `None` for the remote backend
  whose protocol carries no such bit. `sqlite3_stmt_readonly` reports `true`
  for `BEGIN`, `COMMIT`, `SAVEPOINT`, `RELEASE`, and `ROLLBACK`; the empty
  statement is `Some(true)`.
- `commit_seq: Option<u64>`: the sequence of the local commit this statement
  produced, if it produced one (the worker compares `commit_seq` before and
  after the statement on its single connection).

`depot_client_types::QueryResult` (multi-statement `exec`) gains
`readonly: Option<bool>`, the conjunction over every prepared statement.

## 5. Algorithms

### 5.1 Local commit

The two commit entry points keep their existing guards and share one staging
function:

- `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` stages only when `in_atomic_write` is set
  and the dirty set or size changed.
- `xSync` stages only outside atomic mode, when the dirty set or size changed.
  It remains a no-op during an atomic write.
- `xTruncate` outside atomic mode with a size change stages a size-only commit
  **only when the write buffer is empty** (the existing guard). With dirty
  pages present the truncate is part of an in-progress transaction: it only
  updates `db_size_pages`, and the shrink's overlay eviction happens when that
  transaction reaches its commit boundary. Evicting earlier would lose a
  committed page if the transaction rolled back.
- `SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE` is unchanged: it discards
  `write_buffer.dirty` and restores the saved size. It never touches the
  overlay.

```
fn stage_local_commit(ctx) -> Result<(), CommitBufferError>:
    if ctx.commit_mode == Awaited:
        return existing blocking path (unchanged)
    seq = with state.write():
        if state.dead: return Err(dead)
        if write_buffer.dirty.is_empty() && state.db_size_pages == overlay.db_size_pages:
            write_buffer.in_atomic_write = false            // no-op: no sequence, but leave atomic mode
            return Ok(())
        if write_buffer.dirty.len() > MAX_COMMIT_DIRTY_PAGES:
            return Err(too large)                           // SQLite rolls back; overlay untouched
        while overlay.pages.len() + write_buffer.dirty.len() > MAX_COMMIT_DIRTY_PAGES:
            drop(state); wait_for_flush_progress_or_break() // also while closing; receiver armed first
            reacquire state
        seq = overlay.commit_seq + 1
        for (pgno, bytes) in write_buffer.dirty.drain():
            overlay.replace(pgno, OverlayPage { bytes, seq })  // updates overlay.bytes
        if state.db_size_pages < overlay.db_size_pages:
            overlay.remove_pages_above(state.db_size_pages)    // shrink
        overlay.db_size_pages = state.db_size_pages
        overlay.commit_seq = seq
        state.committed_db_size_pages = state.db_size_pages    // locally committed
        write_buffer.in_atomic_write = false
        seq
    ctx.flush.wake.notify_one()
    apply_backpressure(ctx, seq)                                // 5.7
    Ok(())
```

`committed_db_size_pages` keeps its meaning of "size SQLite believes is
committed" for `io_close`, `Drop`, and `truncate_main_file`; the durable size
is tracked by the flusher through `InFlightBatch.db_size_pages`.

### 5.2 Page resolution and cache insertion

`resolve_pages` inserts one lookup between the dirty check and the cache
check: `overlay.pages.get(&pgno)`. When a `get_pages` response is applied,
every returned page (requested, prefetched, or overflow-expanded) is inserted
only if, under the state write lock, `pgno` is neither in `write_buffer.dirty`
nor in `overlay.pages`. Overlay pages are inserted into the committed cache only
by the acknowledgement path.

### 5.3 Flusher loop

```
async fn flusher_loop(ctx: Arc<VfsContext>):
    guard = on drop (panic, cancel, unexpected return): if not terminal, break(Aborted)
    loop:
        batch = with state.write():
            if overlay.commit_seq == flushed_seq:
                if ctx.flush.shutdown: return
                None
            else:
                b = InFlightBatch { seq: overlay.commit_seq,
                                    expected_head_txid: state.durable_head_txid,   // u64
                                    db_size_pages: overlay.db_size_pages,
                                    pages: snapshot of overlay.pages, started_at: now, attempts: 0 }
                overlay.in_flight = Some(b.clone()); Some(b)
        if batch is None: ctx.flush.wake.notified().await; continue
        match ship(ctx, batch).await:
            Acked(head) =>
                with state.write():
                    if terminal error already set (another thread broke the database): return
                    state.durable_head_txid = head
                    state.head_txid = Some(head)
                    for page in batch.pages:
                        if let Some(op) = overlay.pages.get(page.pgno) && op.seq <= batch.seq:
                            overlay.pages.remove(page.pgno)          // updates overlay.bytes
                            if page.pgno <= state.db_size_pages:
                                state.cache_committed_page(page.pgno, page.bytes)
                                // cache_committed_page is a no-op in some cache modes; a stale
                                // engine version must not remain readable, so also evict the
                                // page from page_cache / protected_page_cache in that case
                    overlay.in_flight = None
                    commit_total += 1; metrics.record_commit()
                publish(flushed_seq = batch.seq)
            Broken(err) =>
                break_database(err); return

fn break_database(err):                       // idempotent; every fatal source calls this
    ctx.mark_fatal(err.to_string())           // VFS dead + fatal_error FIRST (section 3.5)
    first = with ctx.flush.progress.lock(): if error.is_none() { error = Some(err); true } else { false }
    ctx.flush.progress_changed.send_replace(next)
    if first: ctx.failure_tx.send(DatabaseFailure::Flush(err))   // failure channel, 6.1, once
```

Pages with `seq > batch.seq` were rewritten while the batch was in flight and
stay in the overlay for the next batch. That is the whole coalescing rule.

### 5.4 Ship with retries

```
async fn ship(ctx, batch) -> Acked(u64) | Broken(FlushError):
    deadline = batch.started_at + retry_deadline
    backoff = retry_backoff_min
    last_error = None
    target = batch.expected_head_txid + 1
    loop:
        batch.attempts += 1
        request = SqliteCommitRequest { dirty_pages: batch.pages, db_size_pages: batch.db_size_pages,
                                        expected_head_txid: Some(batch.expected_head_txid), .. }
        result = timeout_at(deadline, commit_buffered_pages(transport, request)).await
        match result:
            Ok(Ok(SqliteCommitOk { head_txid })) =>
                match head_txid:
                    None => return Acked(target)                 // ok is definitive for this request
                    Some(h) if h == target => return Acked(h)
                    Some(h) => return Broken(HeadDiverged { expected: target, actual: Some(h) })
            Ok(Err(e)) if is_head_fence_mismatch(e) =>
                return Broken(HeadDiverged { expected: target, actual: e.actual_head })
            Ok(Err(e)) => last_error = e                         // structured {group, code, message} kept
            Err(elapsed) => return Broken(RetryDeadlineExceeded { attempts, last_error })
        if now >= deadline: return Broken(RetryDeadlineExceeded { .. })
        timeout_at(deadline, sleep(backoff)).await or return Broken(RetryDeadlineExceeded { .. })
        backoff = min(backoff * 2, retry_backoff_max)
```

`CommitBufferError` keeps the engine's `group` and `code` alongside the
message so the classification above and the final error carry structure. A
retry of a staged (oversized) batch re-begins the stage from scratch. The
envoy reports a stale generation as an unstructured internal error with the
message `actor does not exist`, the same message a never-created database
returns; it is Indeterminate here and the deadline breaks the database.

### 5.5 Idempotency of resends

A resend is byte-identical to the first attempt and carries the same fence.
The engine applies it only if its head still equals `expected`, which is true
exactly when no earlier attempt applied. There is therefore no double apply
and no need to probe. The single unrecoverable case, an applied commit whose
reply was lost, surfaces as a fence mismatch on the resend and is fatal by
section 3.5.

### 5.6 Reads while deferred work exists

`resolve_pages` builds `get_pages` requests with
`expected_head_txid: state.head_txid` today. In deferred mode, whenever
`overlay.commit_seq > flushed_seq` (pending or in flight), the request sends
`expected_head_txid: None` and records `durable_at_request =
durable_head_txid` under the state lock. When the response is handled, again
under the state lock, the returned `head_txid` (when present) must satisfy

```
durable_at_request <= head <= durable_at_response + (in_flight.is_some() ? 1 : 0)
```

A read served before an acknowledgement but processed after it, or served
after a later batch applied, both fall inside this window; anything outside
it is a foreign writer and calls `break_database`. The response never assigns
`head_txid` or `durable_head_txid` in deferred mode. When no deferred work
exists at request time the existing fence is sent unchanged, and a mismatch
is fatal as today.

### 5.7 Backpressure

```
fn apply_backpressure(ctx, seq):
    rx = ctx.flush.progress_changed.subscribe()              // arm BEFORE the first check
    loop:
        if ctx.flush.closing.load(): return Ok(())           // staged already; drain covers it
        (bytes, error) = with state.read(): (overlay.bytes, ctx.flush.progress.lock().error.clone())
        if error.is_some(): return Ok(())                    // already dead; SQLite sees it on the next call
        if bytes <= max_unflushed_bytes: return Ok(())
        ctx.runtime.block_on(rx.changed())                   // wakes on any publish, break, or closing
```

Arming the receiver before the first check closes the lost-notification
window: a publish that lands between the check and the wait is still observed.
`close` and `break_database` both bump `progress_changed`. The flusher was
already notified in `stage_local_commit` before this loop, so it is never
asleep with work pending while a commit waits here. The commit callback has
already merged the pages, so this function never returns an error (section
3.7).

### 5.8 Wait for flush

```
async fn wait_for_flush(ctx, seq) -> Result<(), FlushError>:
    if seq > commit_seq(): return Err(InvalidSequence { requested: seq, current: commit_seq() })
    loop:
        rx = ctx.flush.progress_changed.subscribe()          // arm before checking
        p = ctx.flush.progress.lock().clone()
        if let Some(e) = p.error: return Err(e)
        if p.flushed_seq >= seq: return Ok(())
        rx.changed().await
```

## 6. API surface

### 6.1 Rust: `depot-client`

```rust
// vfs.rs
pub enum CommitMode { Awaited, Deferred }
pub struct DeferredCommitConfig { .. }
pub struct FlushProgress { pub flushed_seq: u64, pub error: Option<FlushError> }
pub enum FlushError { .. }

/// Why the database can no longer serve SQL. Delivered once on the failure channel.
pub enum DatabaseFailure {
    Closed,
    WorkerStopped,
    Flush(FlushError),
}

impl SqliteVfs {                       // NativeVfsHandle = Arc<SqliteVfs>
    pub fn commit_mode(&self) -> CommitMode;
    pub fn commit_seq(&self) -> u64;
    pub fn flushed_seq(&self) -> u64;
    pub fn flush_error(&self) -> Option<FlushError>;
    pub async fn wait_for_flush(&self, seq: u64) -> Result<(), FlushError>;
    pub async fn drain_and_shutdown_flusher(&self, timeout: Duration) -> Result<(), FlushError>;
}

// database.rs
pub async fn open_database_from_transport(
    transport, actor_id, generation, rt_handle, metrics,
    commit_mode: CommitMode,            // new
    initial_commit_seq: u64,            // new
) -> Result<NativeDatabaseHandle>;

impl NativeDatabaseHandle {
    pub fn commit_seq(&self) -> u64;
    pub fn flushed_seq(&self) -> u64;
    pub fn flush_error(&self) -> Option<FlushError>;
    pub async fn wait_for_flush(&self, seq: u64) -> Result<(), FlushError>;
    /// Resolves once with the structured reason: worker thread death or a
    /// broken flusher, whichever comes first. Replaces the bool-returning
    /// `wait_for_worker_failure`.
    pub async fn wait_for_failure(&self) -> DatabaseFailure; // Closed is clean and is not reported
    /// Section 3.6 lifecycle.
    pub async fn close(&self) -> Result<()>;
}

// depot-client-types
pub struct ExecuteResult { .., pub readonly: Option<bool>, pub commit_seq: Option<u64> }
pub struct QueryResult   { .., pub readonly: Option<bool> }

// worker.rs: every command reply carries the connection's autocommit state
// after the command, on success and on error, so the coordinator can detect
// an SQLite-initiated rollback (section 6.2).
pub struct SqliteWorkerReply<T> { pub result: Result<T>, pub post_autocommit: bool }
```

Callers of `open_database_from_transport` outside rivetkit-core
(`depot-client-embedded`, rivetkit-core `tests/metrics.rs`) pass
`CommitMode::Awaited` and `0`.

`SqliteVfsMetrics` gains default no-op hooks: `set_overlay_pages(u64)`,
`record_flush_batch(pages, bytes)`, `observe_flush_latency(ns)`,
`record_flush_retry(class: &'static str)`, `record_flush_broken()`.
`SqliteVfsMetricsSnapshot` is unchanged; sequence state is read through the
accessors above.

`depot-client` adds `thiserror` as a dependency. `tokio` already provides
`watch`, `Notify`, and `time`.

### 6.2 Rust: `rivetkit-core`

```rust
// actor/config.rs
pub enum SqliteCommitMode { Awaited, Deferred }        // maps 1:1 to depot_client::CommitMode
pub struct ActorConfig      { .., pub sqlite_commit_mode: SqliteCommitMode }
pub struct ActorConfigInput { .., pub sqlite_commit_mode: Option<SqliteCommitMode> }

// actor/sqlite/mod.rs
impl SqliteDb {
    pub fn new_with_remote_sqlite(handle, actor_id, actor_key, generation, enabled, remote_sqlite,
                                  commit_mode: SqliteCommitMode) -> Result<Self>;
    pub fn commit_mode(&self) -> SqliteCommitMode;
    pub fn commit_seq(&self) -> u64;                 // 0 before first open; remembered across reopen
    pub fn flushed_seq(&self) -> u64;
    pub fn flush_error(&self) -> Option<String>;
    pub async fn wait_for_flush(&self, seq: u64) -> Result<()>;
}
```

- `select_sqlite_backend` rejects `Deferred` with `RemoteEnvoy` or a build
  without `sqlite-local` using `SqliteRuntimeError::DeferredCommitsUnsupported`
  (code `deferred_commits_unsupported`). This runs at `ActorContext::build`,
  before any lazy open.
- `SqliteDb` stores the mode and the last `commit_seq` independently of the
  native handle. `open()` passes both to `open_database_from_transport`.
  `close_backend` records the handle's final `commit_seq` after close returns,
  on both success and failure.
- `start_worker_failure_monitor` awaits `wait_for_failure()` and reports the
  structured reason (`sqlite worker thread stopped unexpectedly` or the flush
  error's message) through `report_sqlite_worker_fatal` exactly once.
- New error codes in `error.rs`: `flush_failed`, `invalid_argument`,
  `deferred_commits_unsupported`, `transaction_active`. Invalid or ahead-of-
  current flush sequences use `invalid_argument`; terminal durability failures
  use `flush_failed`.

Transaction coordinator (`actor/sqlite/tx.rs`):

- Every core entry point that can wait for the coordinator gate takes a
  `CallMode { Async, SyncBlocking }`. NAPI's `*_sync` methods pass
  `SyncBlocking`; everything else passes `Async`. Core cannot infer this from
  the call, because today both NAPI paths call the same async core methods.
- Every transaction records a `TransactionOrigin`: `Bridge` for leases begun
  from NAPI (sync or async, including state transactions) or `Internal` for
  Rust-owned leases (state persistence). NAPI passes the origin. The origin is
  recorded as *pending* under the coordinator mutex **before** the gate is
  acquired and `BEGIN` runs, and moves to *active* afterwards, so there is no
  window in which a Bridge lease holds or is about to hold the gate without
  being visible.
- A `SyncBlocking` operation that would wait for the gate while a `Bridge`
  lease is pending or active fails immediately with `transaction_active`. The
  JavaScript thread owns such a lease and would block itself. Waiting for an
  `Internal` lease is unchanged, and `Async` callers always wait.
- Every worker reply carries `post_autocommit`. After any statement runs
  through a lease, on success or error, the coordinator reads it. If SQLite
  rolled the transaction back on its own (`SQLITE_FULL`, `SQLITE_IOERR`,
  `ON CONFLICT ROLLBACK`, a user `COMMIT` or `ROLLBACK` in SQL), the lease
  becomes terminal `RolledBack`: further statements and `commit` fail with
  `transaction_closed`, and `rollback` is a no-op. Later statements can no
  longer autocommit individually behind the caller's back. For a
  transaction-scoped multi-statement `exec`, the worker checks autocommit
  after each statement and stops executing the rest as soon as it turns on,
  returning `transaction_closed` for the remainder.
- `finish_transaction(commit = true)` returns `Option<u64>`: the local commit
  sequence the `COMMIT` produced, or `None` when nothing was written.

### 6.3 NAPI: `@rivetkit/rivetkit-napi`

```ts
export declare class JsNativeDatabase {
  // existing members unchanged
  commitSeq(): number
  flushedSeq(): number
  waitForFlush(seq: number): Promise<void>          // seq is required; TS supplies the default
  flushError(): string | null
  supportsSyncMetadata(): boolean
}
export declare class JsSqliteTransaction {
  // existing members, plus:
  commit(): Promise<number | null>                  // was Promise<void>
  commitSync(): number | null                       // was void
}
export interface NativeExecuteResult { columns; rows; changes; lastInsertRowId; readonly?: boolean; commitSeq?: number }
export interface QueryResult        { columns; rows; readonly?: boolean }
export interface JsActorConfig      { ..; sqliteCommitMode?: string }   // validated to "awaited" | "deferred" in From<JsActorConfig>
```

Sequences are `u64` in Rust and `number` in JS, converted with `as f64`; the
JS side validates a non-negative safe integer before calling native.
`index.d.ts` is regenerated by `napi build`, never edited by hand.

### 6.4 TypeScript: `rivetkit`

```ts
// common/database/config.ts
export type SqliteCommitMode = "awaited" | "deferred";

export interface DatabaseFactoryConfig {
  // existing: onMigrate, warnOnManualTransactions, profiling
  commitMode?: SqliteCommitMode;             // default "awaited"
}

export interface DatabaseProvider<DB> {
  // existing: sqliteProfiling, createClient, ...
  sqliteCommitMode?: SqliteCommitMode;       // read by the native runtime when building actor config
}

export interface SqliteExecuteResult {
  columns; rows; changes; lastInsertRowId?;
  readonly?: boolean;                        // present on the native backend
  commitSeq?: number;                        // present when the statement produced a local commit
}

export interface SqliteDatabase {
  // existing members unchanged
  commitSeq?(): number;
  flushedSeq?(): number;
  waitForFlush?(seq: number): Promise<void>;
  flushError?(): string | null;
  supportsSyncMetadata?(): boolean;
}
export interface SqliteTransactionDatabase {
  execSync(sql: string, callback?): { readonly?: boolean };
  commit(): Promise<number | null>;
  ..
}
export type SynchronousSqliteTransactionDatabase = SqliteTransactionDatabase & { commitSync(): number | null; .. };

// RawAccess: optional, like executeSync? and nativeMetrics?
type RawAccess = {
  // existing members unchanged
  commitSeq?(): number;
  flushedSeq?(): number;
  waitForFlush?(seq?: number): Promise<void>;
  flushError?(): string | null;
};

// SynchronousRawAccess (Node.js native): required
interface SynchronousRawAccess extends RawAccess {
  // existing: executeSync, transactionSync
  commitSeq(): number;
  flushedSeq(): number;
  /** Resolves when `seq` (default: commitSeq() read synchronously now) is durable. Rejects once the database is broken. */
  waitForFlush(seq?: number): Promise<void>;
  flushError(): string | null;
  /** Like executeSync but returns metadata: readonly (required here), changes, lastInsertRowId, commitSeq. */
  executeSyncRaw(query: string, ...args: unknown[]): SqliteExecuteResult & { readonly: boolean };
  /** Handle-form synchronous transaction that stays open across event-loop turns. */
  beginTransactionSync(options?: Omit<SqliteTransactionOptions, "experimental">): SynchronousTransactionHandle;
}

interface SynchronousTransactionHandle {
  executeSync<TRow>(query: string, ...args: unknown[]): TRow[];
  executeSyncRaw(query: string, ...args: unknown[]): SqliteExecuteResult & { readonly: boolean };
  execSync(sql: string, callback?): { readonly?: boolean }; // multi-statement
  commitSeq(): number;
  flushedSeq(): number;
  waitForFlush(seq?: number): Promise<void>;
  flushError(): string | null;
  commitSync(): number | null;                       // local commit sequence, null when nothing was written
  rollbackSync(): void;
  readonly isOpen: boolean;
}
```

Rules enforced by the `db()` client (and mirrored in `db/drizzle.ts`, which
keeps its own copy of the guard):

- While a `beginTransactionSync` handle is open, the **synchronous** members
  of the base client throw: `executeSync`, `executeSyncRaw`, `execSync`,
  `transactionSync`, `beginTransactionSync`. The JavaScript thread would
  otherwise block on the Rust lease it holds (the Rust side now also fails
  fast, section 6.2, so this is belt and braces). Asynchronous members
  (`execute`, `exec`, `transaction`) are allowed and queue behind the lease.
  `commitSeq`, `flushedSeq`, `waitForFlush`, and `flushError` are always allowed.
- Transaction-scoped clients (the `tx` passed to `transaction()` and
  `transactionSync()`, including its synchronous callback access) delegate
  `commitSeq`/`flushedSeq`/`waitForFlush`/`flushError` to the base client.
- `commitSync`/`rollbackSync` close the handle. `isOpen` turns false on either,
  and on `transaction_closed`/`transaction_expired` errors surfaced by a
  statement.
- `executeSyncRaw` checks `supportsSyncMetadata()` before executing and rejects
  remote or WebAssembly backends without running the statement. On runtimes
  without native SQLite, `commitSeq()` returns `0`, `flushedSeq()` returns `0`,
  `waitForFlush()` resolves immediately, `flushError()` returns `null`, and
  `executeSyncRaw`/`beginTransactionSync` throw the existing "only available in
  the Node.js native runtime" error.
- `db({ commitMode: "deferred" })` on a WebAssembly or remote-SQLite actor
  fails at actor start with `deferred_commits_unsupported`. The WebAssembly
  runtime mirrors the config field (`rivetkit-wasm/src/lib.rs`) so it rejects
  rather than silently running awaited.

Runtime interface (`registry/runtime.ts`, `napi-runtime.ts`, `wasm-runtime.ts`)
gains `actorSqlCommitSeq`, `actorSqlFlushedSeq`, `actorSqlWaitForFlush`,
`actorSqlFlushError`, `actorSqlSupportsSyncMetadata`,
`RuntimeSqlExecuteResult.readonly?`/`commitSeq?`, `RuntimeSqlExecResult.readonly?`,
transaction `commit` returning the sequence, and `sqliteCommitMode` on the actor
config handed to the native factory, mirroring how `remoteSqlite` flows today.
The `lastInsertRowIdColumnName` shortcut in `native-database.ts` synthesizes
`readonly: true`.

### 6.5 Sufficiency for Durable Object storage

| Durable Object capability | Provided by |
| --- | --- |
| `sql.exec` synchronous cursor, multi-statement | `executeSyncRaw` / `execSync` on the open handle; `readonly` on both results |
| `sql.databaseSize` | `PRAGMA page_count` and `page_size` via `executeSync` |
| Sync KV `get/put/delete/list` | `executeSync` over a KV table |
| Async KV, `deleteAll`, `list` options | JS wrappers over the same, resolving after local commit |
| Write coalescing between awaits | The consumer opens a `beginTransactionSync` handle before the **first statement of a turn**, routes every statement through it, and commits it on the next event-loop turn. `commitSync()` returns the sequence to gate on, or `null` for a read-only turn. `readonly` decides whether the turn holds a confirmed write (`allowUnconfirmed` bookkeeping), not whether to open the handle: by the time `readonly` is known the statement has run, so a first write outside a handle would already be its own local commit. |
| Explicit `transactionSync()` | Nested inside the open handle with `SAVEPOINT`/`RELEASE`/`ROLLBACK TO` statements issued through the handle (workerd does the same). |
| Explicit async `transaction()` | Not nested: the consumer first awaits its pending implicit commit, then opens a new handle at the root and holds it across the closure's awaits (its input-gate critical section blocks other events), with nested transactions as savepoints. The consumer sets a long `timeout`; Cloudflare has none. |
| Output gate before an outbound send | `waitForFlush(seq)` where `seq` comes from the pending implicit commit's `commitSync()` (committed first if still open), or `commitSeq()` when no handle is open |
| `sync()` | `waitForFlush()` after committing any open implicit transaction |
| `allowUnconfirmed` | Do not chain that commit's sequence into the gate; the local commit still happens and ordering still covers it |
| Gate broken -> discard outputs, reset object | `waitForFlush` rejects and `flushError()` is set; the actor generation stops via the failure channel. Commit-time errors (`transaction_closed`, constraint failures at `COMMIT`) leave storage healthy; the consumer breaks its own gate and resets the object itself. |
| Streaming bodies / queued sends | `commitSeq()` now, `waitForFlush(seq)` later |
| Shutdown | The runtime commits or rolls back its open implicit handle before `close()`; a handle still open at close is rolled back by the lease and its writes are lost, which the never-sent response makes unobservable |

### 6.6 Known deviations from Cloudflare

- No SQLite authorizer. A user `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `ATTACH`, or
  restricted `PRAGMA` in SQL text is not rejected. The autocommit check in
  section 6.2 bounds the damage of a user `COMMIT`/`ROLLBACK` to "the rest of
  the turn fails", and the consumer may screen statements with its own
  matcher. A real authorizer is a follow-up.
- `rowsRead`/`rowsWritten` cursor counters are not provided; rows are fully
  materialized.

## 7. Configuration plumbing

```
db({ commitMode: "deferred" })
  -> DatabaseProvider.sqliteCommitMode
  -> native runtime actor config (registry/native.ts, next to remoteSqlite)
  -> JsActorConfig.sqliteCommitMode (NAPI actor_factory.rs; wasm mirror in rivetkit-wasm/src/lib.rs)
  -> ActorConfigInput.sqlite_commit_mode -> ActorConfig.sqlite_commit_mode
  -> SqliteDb::new_with_remote_sqlite(.., commit_mode)      // registry/mod.rs, rejects unsupported backends
  -> open_database_from_transport(.., commit_mode, initial_commit_seq)
  -> VfsConfig.commit_mode / initial_commit_seq
```

Result metadata flows back the other way:

```
sqlite3_stmt_readonly / commit_seq delta (query.rs, worker.rs)
  -> depot_client_types::ExecuteResult / QueryResult
  -> rivetkit-core (remote conversion sets None)
  -> NAPI NativeExecuteResult.readonly? / commitSeq?, QueryResult.readonly?
  -> RuntimeSqlExecuteResult / RuntimeSqlExecResult
  -> SqliteExecuteResult (native-database.ts) -> executeSyncRaw
```

`DeferredCommitConfig` values come from `SqliteOptimizationFlags`
(`RIVETKIT_SQLITE_OPT_FLUSH_RETRY_DEADLINE_MS`,
`RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MIN_MS`,
`RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MAX_MS`,
`RIVETKIT_SQLITE_OPT_MAX_UNFLUSHED_BYTES`) parsed with the existing
`from_env_reader` helpers and mapped in `VfsConfig::from_optimization_flags`.
They are runner-wide tuning, not per-actor API. Tests construct
`DeferredCommitConfig` directly with small real durations; nothing uses
`tokio::time::pause`.

## 8. Invariants

1. `flushed_seq <= commit_seq` always; both are monotonic per process for a
   given actor.
2. In awaited mode `flushed_seq == commit_seq` at every observable point.
3. A page in the overlay is never evicted and is always the newest local
   version of that page outside `write_buffer.dirty`. No read response ever
   overwrites it.
4. At most one `InFlightBatch` exists, and its `seq` equals `commit_seq` at
   the moment it was formed.
5. `durable_head_txid` is assigned only at open and by the acknowledgement
   path, every acknowledgement advances it by exactly one, and every commit
   request carries `Some(durable_head_txid)` as its fence.
6. If `wait_for_flush(s)` resolved `Ok`, the engine's durable state equals the
   local commit history up to at least `s`, and reopening the database from
   the engine observes all of it.
7. After a break, no later `wait_for_flush` resolves `Ok` (including for
   already-flushed sequences), no later SQL statement succeeds, and the actor
   generation is stopped exactly once.
8. Engine contents after any sequence of operations, including a break, equal
   the local commit history up to some sequence `f` with
   `flushed_seq <= f <= commit_seq`, never a mix.
9. Every await inside the flusher is bounded by the retry deadline, and the
   flusher exiting for any reason other than a clean drain breaks the
   database.
10. Every batch the flusher can form is within `MAX_COMMIT_DIRTY_PAGES`.
11. No engine request is issued by a `VfsContext` after its `close` returned.

## 9. Testing

### 9.1 Deterministic VFS tests (`engine/packages/depot-client/tests/inline/vfs.rs`)

The harness already provides `DirectEngineHarness`, `DirectTransportHooks`
(`fail_next_commit`, `fail_next_commit_after_apply` for a lost ack,
`hang_next_commit`, `pause_next_commit`/`DirectCommitPause` for hold/release,
`commit_requests()` capture), `DirectStorage::read_branch_head` for the engine
txid, and `open_db_on_engine` for reopening. Add a repeat count to the fail
hooks ("fail the next N attempts") and a hook that returns a response with a
wrong or missing head. Tests, each ending with the durability oracle in 9.2:

- `deferred_commit_returns_before_engine_ack`
- `read_only_transaction_does_not_advance_commit_seq`
- `wait_for_flush_snapshot_ignores_later_commits`
- `wait_for_flush_rejects_future_sequence` and `wait_for_flush_zero_is_immediate`
- `wait_for_flush_rejects_after_break_even_for_flushed_sequence`
- `read_your_writes_while_batch_in_flight` (also invalidates the page cache
  mid-flight and asserts overlay bytes win)
- `overflow_expanded_read_does_not_overwrite_overlay`
- `read_between_staging_and_in_flight_omits_head_fence`
- `stale_unfenced_read_does_not_regress_durable_head` (read delayed across an
  acknowledgement; next batch still expects the right head)
- `read_window_accepts_response_served_before_ack_processed_after` and
  `read_window_rejects_foreign_head`
- `prefetch_and_has_readable_page_skip_overlay_pages`
- `empty_page_synthesis_disabled_after_local_commit`
- `oversized_transaction_is_rejected_before_merge` and
  `closing_drains_page_cap_before_merging_another_commit`;
  `oversized_sql_transaction_rolls_back_and_connection_remains_usable`;
  `awaited_mode_ignores_deferred_byte_limit_with_small_pages`
- `no_requests_after_close_returns` (abort an in-flight attempt at close and
  assert the transport sees nothing afterwards)
- `worker_timeout_aborts_flusher_before_paused_io_resumes`
- `commits_during_in_flight_batch_coalesce_into_one_request` (request count,
  newest bytes, txid chain)
- `lost_ack_resend_hits_fence_and_breaks_database` (engine applied, reply
  dropped; the oracle shows the data durable while every waiter rejected)
- `transient_error_resend_is_byte_identical_and_applies_once` (request capture
  proves one applied commit and the same fence on both attempts)
- `foreign_writer_at_expected_plus_one_breaks_database`
- `commit_ok_with_wrong_head_breaks_database`; `commit_ok_without_head_is_accepted`
- `transient_errors_retry_then_succeed`
- `retry_deadline_breaks_database_and_rejects_waiters`
- `hung_commit_is_bounded_by_deadline`
- `late_ack_after_break_does_not_publish_progress`
- `truncate_with_dirty_pages_defers_staging_to_commit_boundary`
- `drop_with_open_transaction_stages_nothing`
- `backpressure_returns_when_closing`
- `awaited_mode_advances_sequences_on_ack`
- `flusher_panic_breaks_database` (test-only hook)
- `size_only_truncate_is_a_local_commit`; `shrink_evicts_overlay_pages_above_size`;
  `shrink_while_expansion_in_flight`
- `xsync_during_atomic_write_does_not_stage`; `rollback_atomic_write_leaves_overlay`
- `backpressure_blocks_commit_until_flush_progress` (flusher asleep, no batch
  in flight when the bound is hit)
- `close_drains_pending_flushes`; `close_returns_flush_error_when_broken`;
  `drop_without_close_stages_only_and_is_short`
- `commit_seq_continues_across_reopen`
- `awaited_mode_is_unchanged` (sequence equality, request count parity with
  the existing tests)
- `execute_result_reports_readonly_and_commit_seq`

### 9.2 Durability oracle

A helper reopens a fresh `NativeDatabase` against the same in-process engine
and compares table contents with the expected prefix of local commits.
Invariants 6 and 8 are asserted by every deferred-mode test, including the
lost-acknowledgement path (data durable, waiters rejected).

### 9.3 Seeded randomized interleaving

One test drives a seeded RNG over: write transaction, read, size-only
truncate, `wait_for_flush`, hold/release/fail/lose-ack/wrong-head on the
transport, page-cache invalidation, and close/reopen, against an in-memory
model of `{ local: Vec<Commit>, acked_prefix }`. Small real durations; the
seed is printed on failure; runs a fixed iteration count in CI
(`cargo test -p rivet-depot-client deferred_random`).

### 9.4 rivetkit-core

- `SqliteDb` in deferred mode: `wait_for_flush` maps errors, unsupported
  backends reject at context construction, a broken flusher calls `stop_actor`
  exactly once with the flush error message, `commit_seq` survives close and
  reopen.
- Coordinator: a `SyncBlocking` operation against a pending or active
  `Bridge` lease fails fast with `transaction_active` (including the window
  between gate acquisition and `BEGIN`); an `Internal` lease still waits; a
  statement that triggers SQLite auto-rollback makes the lease terminal on
  both the success and error paths; `exec("COMMIT; INSERT ...")` inside a
  lease stops before the `INSERT`; `commit` returns the sequence or `None`.

### 9.5 TypeScript driver tests

Add a `dbActorDeferred` fixture (`db({ commitMode: "deferred" })`) registered
in `fixtures/driver-test-suite/registry-static.ts`, with actions for
`executeSync` writes, `waitForFlush`, `commitSeq`/`flushedSeq`, an implicit
transaction via `beginTransactionSync` committed on `setImmediate`, and
`executeSyncRaw().readonly`. A separate sleep fixture with a short
`sleepTimeout` follows `sleep-db.ts`. Tests in
`tests/driver/actor-db-deferred.test.ts` over the SQLite matrix:

- writes are visible to a following read before `waitForFlush`
- `waitForFlush` resolves and a fresh actor instance sees the rows
- `commitSeq` advances per write, not per read, and `flushedSeq` catches up
- `waitForFlush()` captures the sequence synchronously (a write issued right
  after the call is not waited for)
- `readonly` is `true` for `SELECT`, `false` for `INSERT` and DDL
- handle-form transaction: `commitSync()` returns the sequence, a read-only
  turn returns `null`, synchronous base-client calls throw while it is open,
  asynchronous ones queue, and the gate-then-send ordering holds
- native/remote and wasm/remote matrix cells assert
  `deferred_commits_unsupported` at actor start
- sleep variant: write, `waitForFlush`, trigger sleep, wake, read back;
  `commitSeq` after wake is greater than before sleep

### 9.6 Docs

- `docs/content/docs/sqlite.mdx`: a "Deferred commits" section under the
  synchronous-operations section explaining `commitMode`, `waitForFlush`,
  `beginTransactionSync`, and the durability contract in user terms.
- `docs-internal/engine/sqlite-vfs.md`: a "Deferred commits" rules block
  linking here.

## 10. Follow-ups (out of scope)

- Pipelined batches with predicted head txids once the engine orders
  per-actor commits or accepts a client commit id.
- A client commit nonce recorded by the engine and echoed on `get_pages`, so
  a lost acknowledgement can be recovered by a probe instead of a restart.
- A structured generation-fence error code at the envoy boundary, so a stale
  generation breaks immediately instead of at the retry deadline.
- An SQLite authorizer for transaction-control and restricted statements.
- Worker-thread isolation of synchronous SQLite.
- Runtime-level input and output gates, `blockConcurrencyWhile`, and actor
  reset semantics in the Durable Object compatibility layer.
