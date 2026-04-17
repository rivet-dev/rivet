> **Stale design (2026-04-15):** Written before the decision to give v2 a separate `SqliteV2Protocol` trait (not shared with v1). The harness shape and 37 test cases mostly carry over; trait names and file paths need revision. See `protocol-and-vfs.md` §3 for current trait.

# SQLite VFS v2 — Test Architecture

Companion to [`walkthrough.md`](./walkthrough.md) and [`design-decisions.md`](./design-decisions.md). This document specifies how we test v2 (and, by the same structure, how we retroactively tighten v1 coverage).

> **Status (2026-04-15):** Design. No code has been written. Read §9 for the implementation checklist.

---

## 0. Guiding principles

Four principles shape everything below.

1. **Unit tests run in-process with no engine.** The production `SqliteKv` impl is `EnvoyKv`, which goes napi → websocket → engine → UDB. That is much too slow, much too stateful, and much too hard to coax into determinism for table-driven testing. We build a pure in-memory impl that the test binary owns end-to-end.

2. **v1 and v2 share one SQL-level conformance suite.** At the SQL layer the database must be indistinguishable: `CREATE TABLE`, `INSERT`, `SELECT`, `UPDATE`, `DELETE`, `VACUUM`-unsupported-error, transactions, schema changes — the same test suite runs against both VFS implementations via a shared trampoline. Layers below (orphan cleanup, materializer, fencing, preload shape) are v2-only and live in a separate v2 suite.

3. **Preload is first-class, both in the VFS and the harness.** Per Nathan's directive, the VFS must expose a `preload(keys, prefixes)` API the user can call. Our test harness uses the *same* API to seed deterministic initial state before every test. A test case that says "the actor KV starts in state S, preload the following keys on open" is the normal form for every v2 test.

4. **Failure injection is a first-class feature of the in-memory driver, not a separate mock.** The driver itself knows how to return errors after N ops, reject on generation mismatch, and drop the tail of a multi-put to simulate a partial write. Tests declare the injection plan up front and the driver enforces it inside its normal trait methods. No `vi.mock`-equivalent, no test-specific code paths inside `vfs_v2.rs`.

---

## 1. Scope and non-goals

**In scope for this doc:**
- The in-memory `SqliteKv` implementation (`MemoryKv`).
- A preload-aware test harness that both v1 and v2 consume.
- The shared SQL-level conformance suite, plus the v2-only invariants suite.
- How to extend `examples/sqlite-raw` to benchmark v2 without forking.
- A small but real e2e suite that runs the same scenarios through `EnvoyKv` against a local RocksDB engine.

**Explicitly out of scope:**
- Mutation-testing of `vfs_v2.rs` line by line. The coverage target here is correctness at the behavioral boundary, not line coverage.
- Benchmarking-for-benchmarking-sake. The bench harness is for v2 vs v1 comparison, not for unit-level performance tests.
- Testing the runner-protocol wire format for the new `kv_sqlite_*` ops. That belongs in `engine/packages/runner-protocol/` protocol conformance, tracked in [`design-decisions.md`](./design-decisions.md) §3.

---

## 2. Landscape today (what exists, what doesn't)

Before we design anything new, note what is already in the tree.

### 2.1 `SqliteKv` impls

There is exactly one production impl: `EnvoyKv` at `rivetkit-typescript/packages/rivetkit-napi/src/database.rs:37`. It wraps `EnvoyHandle` and delegates each method to a napi-exposed websocket round trip. **There is no in-tree in-memory impl** — neither a `MockKv` nor a `TestKv` nor a `#[cfg(test)]` helper inside `sqlite_kv.rs` or `vfs.rs`. This is the first gap we fill.

### 2.2 Existing Rust tests in `sqlite-native`

The only `#[cfg(test)]` in `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` covers metadata encoding helpers and `startup_preload_*` helpers. No end-to-end VFS test exists in Rust today — every behavior test runs through TypeScript driver tests that open a real engine. That works for v1 because v1 is simple, but it leaves the SqliteKv trait untested in isolation and the VFS callback layer untested in any form we can drive from Rust.

### 2.3 Existing TypeScript coverage

`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-db.ts` and `actor-db-stress.ts` cover SQL-level behavior against the full engine stack using:

- `dbActorRaw` / `dbActorDrizzle` fixtures at `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/actors/`.
- `setupDriverTest` in `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/utils.ts:14`.
- Tests invoke through `runActorDbTests(driverTestConfig)` which runs against the engine runtime.

These are integration tests, not unit tests. They require a running engine and cover "does the database work at all," not "did the write path take one round trip or six."

### 2.4 The engine preload primitive

`engine/packages/pegboard/src/actor_kv/preload.rs` defines `batch_preload` — a single UDB transaction that reads exact keys, scans prefixes with per-prefix and global byte budgets, and returns raw key-value pairs. This is exactly the primitive v2's `kv_sqlite_preload` op should wrap. The signature today accepts `PreloadPrefixRequest { prefix, max_bytes, partial }`, which maps cleanly onto the new op. Our test harness mimics the same shape so an in-memory preload call and a real preload call have the same observable behavior.

### 2.5 `examples/sqlite-raw`

The bench harness at `examples/sqlite-raw/scripts/bench-large-insert.ts` insert/verifies a large payload through `todoList.benchInsertPayload` and compares the end-to-end timing to `node:sqlite`. Its selector is `BENCH_MB`/`BENCH_ROWS`. The harness currently has no VFS selector — it runs whatever VFS `rivetkit/db` picks. We will add a `VFS_VERSION=v1|v2` env var and a way to emit a single BENCH_RESULTS row per (payload, vfs) pair so the existing `BENCH_RESULTS.md` table can grow new columns rather than forking into a v2 file.

---

## 3. The in-memory `SqliteKv` driver (`MemoryKv`)

### 3.1 Location and crate layout

New file `rivetkit-typescript/packages/sqlite-native/src/memory_kv.rs`, gated on neither `cfg(test)` nor a feature — it ships as a normal module behind `pub mod memory_kv;` in `lib.rs`. Reason: the same struct is used by three consumers: Rust unit tests inside `sqlite-native`, a small Rust-side bench binary, and (eventually) a `napi` wrapper that lets TypeScript tests run against the in-memory driver too.

> Rejected alternative: `rivetkit-typescript/packages/sqlite-native/src/test_kv.rs` gated on `#[cfg(test)]`. This works for the Rust-side tests but forces us to re-implement almost the same thing for the TS-side unit tests because `cfg(test)` doesn't leak across crate boundaries. One public module is simpler.

We also add a convenience feature `memory_kv` in `Cargo.toml` so downstream consumers who only want the VFS itself can opt out. Default features include `memory_kv`.

### 3.2 Data model

Internally a `BTreeMap<Vec<u8>, Vec<u8>>`, wrapped in a `tokio::sync::Mutex`. We use `tokio::sync::Mutex` (not `std::sync::Mutex`) because the trait methods are `async` and tests may hold the lock across an `await` when they do snapshot/restore. `BTreeMap` gives us ordered range scans for free — the materializer and orphan-cleanup paths both need them.

We use a plain `Mutex` rather than `scc::HashMap` because:
- The test driver serializes access on purpose so the test can reason about ordering.
- Range scans are the dominant shape and `scc::HashMap` cannot do them.
- Contention does not matter: tests run single-actor workloads.

This is consistent with the `CLAUDE.md` "Never use `Mutex<HashMap<...>>`" rule because (a) it is only in tests and a test helper crate module, and (b) it is a `BTreeMap`, not a `HashMap`. If we hit a case where two VFS threads inside one test race on the map, we add `async fn lock_guard()` and split reads from writes.

### 3.3 Core state struct

```rust
// rivetkit-typescript/packages/sqlite-native/src/memory_kv.rs

use async_trait::async_trait;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::sqlite_kv::{KvGetResult, SqliteKv, SqliteKvError};

/// In-memory SqliteKv impl. Deterministic, snapshot-friendly, supports
/// failure injection.
pub struct MemoryKv {
    inner: Arc<Mutex<MemoryKvInner>>,
}

struct MemoryKvInner {
    /// The actual KV store. Keys are ordered for range scans.
    kv: BTreeMap<Vec<u8>, Vec<u8>>,

    /// Current generation per actor_id. Checked by sqlite_commit/stage/
    /// materialize. Test cases can bump this to simulate a takeover.
    generations: BTreeMap<String, u64>,

    /// Head txid per actor_id. Checked by CAS in sqlite_commit/materialize.
    head_txids: BTreeMap<String, u64>,

    /// FIFO of operation records for assertions. Bounded to the last N.
    op_log: VecDeque<OpRecord>,
    op_log_capacity: usize,

    /// How many ops have been executed since construction or last reset.
    op_count: u64,

    /// Failure injection plan. None means "all ops succeed."
    failure_plan: Option<FailurePlan>,

    /// Snapshot stack (see §3.7).
    snapshot_stack: Vec<MemoryKvSnapshot>,
}

#[derive(Debug, Clone)]
pub struct OpRecord {
    pub op: OpKind,
    pub actor_id: String,
    pub details: OpDetails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    BatchGet,
    BatchPut,
    BatchDelete,
    DeleteRange,
    SqliteCommit,
    SqliteCommitStage,
    SqliteMaterialize,
    SqlitePreload,
    SqliteTakeover,
}

#[derive(Debug, Clone)]
pub enum OpDetails {
    KeyList(Vec<Vec<u8>>),
    KeyValuePairs(Vec<(Vec<u8>, Vec<u8>)>),
    Range { start: Vec<u8>, end: Vec<u8> },
    Commit { txid: u64, expected_head: u64, generation: u64, log_keys: usize },
    Materialize { pages: usize, delete_ranges: usize, new_mat_txid: u64 },
    Preload { exact: usize, prefix: usize, got: usize },
    Takeover { expected_gen: u64, new_gen: u64 },
}

#[derive(Debug, Clone)]
pub struct MemoryKvSnapshot {
    kv: BTreeMap<Vec<u8>, Vec<u8>>,
    generations: BTreeMap<String, u64>,
    head_txids: BTreeMap<String, u64>,
    op_count: u64,
}

#[derive(Debug, Clone)]
pub struct FailurePlan {
    /// Ops matching any of these predicates return the supplied error.
    pub injections: Vec<FailureInjection>,
}

#[derive(Debug, Clone)]
pub struct FailureInjection {
    /// After how many total ops does this injection arm?
    pub after_ops: u64,
    /// Which op kind should this fail on? `None` means any.
    pub on_op: Option<OpKind>,
    /// What error does the op return?
    pub error: FailureMode,
    /// How many times does this injection fire before disarming?
    pub fires: u32,
}

#[derive(Debug, Clone)]
pub enum FailureMode {
    /// Return SqliteKvError::new(msg).
    GenericError(String),
    /// Simulate a fencing mismatch. The op returns the documented error
    /// and leaves state untouched.
    FenceMismatch,
    /// Simulate a partial write: of M keys in the call, only the first
    /// `keys_written` are persisted; then return an error.
    PartialWrite { keys_written: usize },
    /// Simulate a partial commit-stage: only `frames_written` of the staged
    /// frames land; then return an error.
    PartialStage { frames_written: usize },
}
```

Notes on the shape:

- `op_log_capacity` is set by the constructor, default 256. Tests that want "the last 256 ops" for assertions get it for free; tests that want all ops set it to `usize::MAX`.
- `FailurePlan` is a list, not a single injection, because a test may want "succeed for 10 ops, then fence-mismatch once, then succeed." Each injection independently tracks `fires` and disarms itself.
- `Vec<OpRecord>` is deliberately not `scc::HashMap<u64, OpRecord>`. We want ordering and we have it.

### 3.4 Construction API

```rust
impl MemoryKv {
    /// Construct an empty in-memory KV with default settings.
    pub fn new() -> Arc<Self> { /* ... */ }

    /// Construct a KV pre-populated with these entries.
    pub fn with_entries(entries: Vec<(Vec<u8>, Vec<u8>)>) -> Arc<Self> { /* ... */ }

    /// Adjust the op-log capacity (default 256).
    pub fn with_op_log_capacity(self: Arc<Self>, cap: usize) -> Arc<Self> { /* ... */ }

    /// Install a failure plan. Replaces any existing plan.
    pub fn install_failure_plan(&self, plan: FailurePlan) { /* ... */ }

    /// Remove any installed failure plan.
    pub fn clear_failure_plan(&self) { /* ... */ }

    /// Snapshot current state, push onto the snapshot stack.
    pub async fn snapshot(&self) { /* ... */ }

    /// Pop the top snapshot. Restores KV, generations, head txids, op_count.
    pub async fn restore(&self) { /* ... */ }

    /// Read the op count.
    pub async fn op_count(&self) -> u64 { /* ... */ }

    /// Read the last N ops (bounded by capacity).
    pub async fn recent_ops(&self, n: usize) -> Vec<OpRecord> { /* ... */ }

    /// Return a full dump of the KV state (for assertions and diffs).
    pub async fn dump(&self) -> BTreeMap<Vec<u8>, Vec<u8>> { /* ... */ }

    /// Diff the current state against a snapshot.
    pub async fn diff(&self, base: &BTreeMap<Vec<u8>, Vec<u8>>) -> KvDiff { /* ... */ }

    /// Reach into the generation/head fences directly. Tests that want
    /// to simulate "an older runner is still alive" bump these manually.
    pub async fn set_generation(&self, actor_id: &str, gen: u64) { /* ... */ }
    pub async fn set_head_txid(&self, actor_id: &str, txid: u64) { /* ... */ }
}

pub struct KvDiff {
    pub added: Vec<(Vec<u8>, Vec<u8>)>,
    pub modified: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>, // key, before, after
    pub removed: Vec<(Vec<u8>, Vec<u8>)>,
}
```

`Arc<Self>` is the return type for `new` because `SqliteKv` needs `Arc<dyn SqliteKv>` to hand to `KvVfs::register`. All mutating methods take `&self` (not `&mut self`) so we can share the `Arc` between the VFS thread and the test thread.

### 3.5 Trait surface

`MemoryKv` implements the full `SqliteKv` trait including the new v2 methods. Here's the signature for the v2 additions we will add to the trait alongside the existing ones:

```rust
// Added to sqlite_kv.rs as part of the v2 implementation, not part of
// memory_kv.rs. Listed here for clarity.

#[async_trait]
pub trait SqliteKv: Send + Sync {
    // ... existing batch_get / batch_put / batch_delete / delete_range ...

    /// Commit a transaction in one UDB round trip (fast path). Does the
    /// CAS on (generation, head_txid) and atomically applies log_writes
    /// and meta_write.
    async fn sqlite_commit(
        &self,
        actor_id: &str,
        op: KvSqliteCommitOp,
    ) -> Result<(), KvSqliteError>;

    /// Stage frames for a large transaction. Non-atomic with respect to
    /// other stage calls. CASes only on generation (not head_txid).
    async fn sqlite_commit_stage(
        &self,
        actor_id: &str,
        op: KvSqliteCommitStageOp,
    ) -> Result<(), KvSqliteError>;

    /// Advance materialized_txid, atomically (page_writes + range_deletes
    /// + meta_write in one transaction).
    async fn sqlite_materialize(
        &self,
        actor_id: &str,
        op: KvSqliteMaterializeOp,
    ) -> Result<(), KvSqliteError>;

    /// One-shot preload: exact keys + prefix scans + optional byte budget.
    /// Returns all entries in insertion-stable order.
    async fn sqlite_preload(
        &self,
        actor_id: &str,
        op: KvSqlitePreloadOp,
    ) -> Result<KvSqlitePreloadResult, KvSqliteError>;

    /// CAS the generation forward. Used on startup.
    async fn sqlite_takeover(
        &self,
        actor_id: &str,
        op: KvSqliteTakeoverOp,
    ) -> Result<(), KvSqliteError>;
}

/// Distinct error type for the v2 ops so fencing failures are visible
/// as a distinct variant instead of an opaque string.
#[derive(Debug)]
pub enum KvSqliteError {
    /// Generation or head_txid CAS mismatch. Carries the current values
    /// for debugging.
    FenceMismatch { current_generation: u64, current_head_txid: u64 },
    /// Exceeded per-op envelope (value too large, too many keys, etc.)
    EnvelopeExceeded(String),
    /// Unknown / transport error.
    Other(SqliteKvError),
}

pub struct KvSqliteCommitOp {
    pub generation: u64,
    pub expected_head_txid: u64,
    pub log_writes: Vec<(Vec<u8>, Vec<u8>)>,
    pub meta_write: Vec<u8>,
    pub range_deletes: Vec<(Vec<u8>, Vec<u8>)>,
}

pub struct KvSqliteCommitStageOp {
    pub generation: u64,
    pub txid: u64,
    pub log_writes: Vec<(Vec<u8>, Vec<u8>)>,
    /// True on the first stage call for this txid. Triggers an eager
    /// range_delete of LOG/<txid>/* to clear orphans.
    pub wipe_txid_first: bool,
}

pub struct KvSqliteMaterializeOp {
    pub generation: u64,
    pub expected_head_txid: u64,
    pub page_writes: Vec<(Vec<u8>, Vec<u8>)>,
    pub range_deletes: Vec<(Vec<u8>, Vec<u8>)>,
    pub meta_write: Vec<u8>,
}

pub struct KvSqlitePreloadOp {
    pub get_keys: Vec<Vec<u8>>,
    pub prefix_scans: Vec<(Vec<u8>, Vec<u8>)>,  // (start, end) inclusive
    pub max_total_bytes: u64,
}

pub struct KvSqlitePreloadResult {
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
    pub requested_get_keys: Vec<Vec<u8>>,
    pub requested_prefix_scans: Vec<(Vec<u8>, Vec<u8>)>,
}

pub struct KvSqliteTakeoverOp {
    pub expected_generation: u64,
    pub new_generation: u64,
}
```

The `KvSqliteError` enum is new and exists specifically so fencing failures surface as a distinct type that the VFS can branch on.

### 3.6 Failure injection semantics

Inside each trait method, `MemoryKv` runs the same prelude:

```rust
async fn sqlite_commit(
    &self,
    actor_id: &str,
    op: KvSqliteCommitOp,
) -> Result<(), KvSqliteError> {
    let mut guard = self.inner.lock().await;
    guard.op_count += 1;
    guard.record_op(OpKind::SqliteCommit, actor_id, /* details */);

    if let Some(failure) = guard.consume_failure(OpKind::SqliteCommit) {
        match failure {
            FailureMode::GenericError(msg) => {
                return Err(KvSqliteError::Other(SqliteKvError::new(msg)));
            }
            FailureMode::FenceMismatch => {
                return Err(KvSqliteError::FenceMismatch {
                    current_generation: guard.current_gen(actor_id),
                    current_head_txid: guard.current_head(actor_id),
                });
            }
            FailureMode::PartialWrite { keys_written } => {
                // Apply the first `keys_written` from log_writes, then err.
                for (k, v) in op.log_writes.iter().take(keys_written) {
                    guard.kv.insert(k.clone(), v.clone());
                }
                return Err(KvSqliteError::Other(SqliteKvError::new(
                    "simulated partial write",
                )));
            }
            FailureMode::PartialStage { .. } => {
                // Not applicable for sqlite_commit; ignore or fail loudly.
                return Err(KvSqliteError::Other(SqliteKvError::new(
                    "wrong failure mode",
                )));
            }
        }
    }

    // ... normal CAS + apply ...
}
```

The key property: **a `PartialWrite` simulation is the only path that mutates the in-memory KV and then returns an error**. Everything else is atomic — either the whole op applies or the whole op bails. This matches real UDB behavior: the only way to observe a half-applied commit is if the engine-side transaction commits some rows and then the runtime goes away before acknowledging. `PartialWrite` lets us reproduce that without nondeterminism.

### 3.7 Snapshot and restore

Snapshot pushes a `MemoryKvSnapshot` onto an internal stack. Restore pops the top one and replaces `kv`, `generations`, `head_txids`, and `op_count`. Op log and failure plan are *not* snapshotted — they are test-configuration, not test-state.

Tests typically use snapshots in one of two patterns:

```rust
// Pattern 1: assert that an op is pure.
kv.snapshot().await;
let err = kv.sqlite_commit(aid, bad_op).await.unwrap_err();
assert!(matches!(err, KvSqliteError::FenceMismatch { .. }));
let diff = kv.diff(&base).await;
assert!(diff.is_empty()); // nothing mutated
kv.restore().await;

// Pattern 2: rollback after a successful op for table-driven tests.
kv.snapshot().await;
for case in cases {
    kv.snapshot().await;
    run_case(&kv, case).await;
    kv.restore().await;
}
kv.restore().await;
```

### 3.8 Determinism and ordering

`BTreeMap` iteration is deterministic by key. The only source of nondeterminism inside `MemoryKv` would be timestamps, and we do not store any. The op log is ordered strictly by call order. The snapshot stack is LIFO.

If a test needs "time" (for example to assert that `DBHead.creation_ts_ms` advances), the test passes a fake clock into the VFS, not into `MemoryKv`. `MemoryKv` never touches a clock.

---

## 4. The preload-aware test harness

### 4.1 Goal and shape

We want test cases to look like this:

```rust
#[tokio::test]
async fn fast_path_single_op_round_trip_count() -> Result<()> {
    VfsV2Harness::builder()
        .actor_id("act-1")
        .initial_kv(&[
            (meta_key(), encode_initial_meta(head_txid: 0, db_size: 1)),
            (page_key(1), page_1_bytes()),
        ])
        .preload_keys(&[meta_key(), page_key(1)])
        .build()
        .await?
        .run(|actor| async move {
            actor.sql("CREATE TABLE t (x INT)").await?;
            actor.sql("INSERT INTO t VALUES (1)").await?;
            Ok(())
        })
        .await?
        .assert_op_count("sqlite_commit", 1)
        .assert_op_count("sqlite_commit_stage", 0)
        .assert_op_count("batch_put", 0)
        .ok()
}
```

This is the shape for every test: build a harness, seed initial KV, declare preload hints, run some SQL inside an `actor` closure, and assert on the op log and the final KV state.

### 4.2 The `VfsV2Harness` struct

New file `rivetkit-typescript/packages/sqlite-native/src/test_harness.rs`. Also shipped as a public module, not `cfg(test)`, so a Rust-side bench binary can construct one too.

```rust
pub struct VfsV2HarnessBuilder {
    actor_id: String,
    initial_kv: Vec<(Vec<u8>, Vec<u8>)>,
    preload_keys: Vec<Vec<u8>>,
    preload_prefixes: Vec<(Vec<u8>, Vec<u8>)>,
    failure_plan: Option<FailurePlan>,
    vfs_kind: VfsKind,
    /// When set, the VFS is opened at this generation. Defaults to 1.
    starting_generation: u64,
}

pub enum VfsKind {
    V1,
    V2,
}

pub struct VfsV2Harness {
    kv: Arc<MemoryKv>,
    vfs: KvVfs,
    db: NativeDatabase,
    actor_id: String,
    rt: Handle,
}

pub struct HarnessRun {
    harness: VfsV2Harness,
    ran_sql: bool,
}

impl VfsV2HarnessBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn actor_id(mut self, id: impl Into<String>) -> Self { /* ... */ }
    pub fn initial_kv(mut self, entries: &[(Vec<u8>, Vec<u8>)]) -> Self { /* ... */ }
    pub fn preload_keys(mut self, keys: &[Vec<u8>]) -> Self { /* ... */ }
    pub fn preload_prefixes(mut self, ranges: &[(Vec<u8>, Vec<u8>)]) -> Self { /* ... */ }
    pub fn failure_plan(mut self, plan: FailurePlan) -> Self { /* ... */ }
    pub fn vfs_kind(mut self, kind: VfsKind) -> Self { /* ... */ }
    pub fn starting_generation(mut self, g: u64) -> Self { /* ... */ }
    pub async fn build(self) -> Result<VfsV2Harness> { /* ... */ }
}

impl VfsV2Harness {
    pub fn kv(&self) -> &Arc<MemoryKv> { &self.kv }

    /// Run arbitrary SQL inside the actor "closure." The closure receives
    /// a thin ActorHandle wrapper around the NativeDatabase so it can
    /// .sql() and .query() without touching raw sqlite3 pointers.
    pub async fn run<F, Fut>(self, f: F) -> Result<HarnessRun>
    where
        F: FnOnce(ActorHandle<'_>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    { /* ... */ }

    /// Crash the actor and re-open it. Returns a new harness where the
    /// KV state is preserved but the VFS, dirty buffer, in-memory caches,
    /// and dirty_pgnos_in_log are all fresh.
    pub async fn crash_and_reopen(self) -> Result<VfsV2Harness> { /* ... */ }
}

impl HarnessRun {
    pub fn assert_op_count(self, op: &str, expected: u64) -> Self { /* ... */ }
    pub fn assert_total_ops_under(self, limit: u64) -> Self { /* ... */ }
    pub fn assert_key_equals(self, key: &[u8], value: &[u8]) -> Self { /* ... */ }
    pub fn assert_key_absent(self, key: &[u8]) -> Self { /* ... */ }
    pub fn assert_no_orphan_log_frames(self) -> Self { /* ... */ }
    pub fn assert_materializer_lag_bounded(self, max: u64) -> Self { /* ... */ }
    pub fn into_harness(self) -> VfsV2Harness { self.harness }
    pub fn ok(self) -> Result<()> { Ok(()) }
}
```

### 4.3 The preload wiring

When the harness builds a VFS, it calls `sqlite_preload(get_keys, prefix_scans)` on the `MemoryKv` *before* the VFS enters the first SQLite call. The result is then pushed into the VFS's in-memory startup-preload buffer via the same mechanism `KvVfs::register` uses today (it already accepts `startup_preload: StartupPreloadEntries`). For v2, the VFS registration entry point gains an `explicit_preload_hints: PreloadHints` parameter so the user of the VFS (the harness, the production runtime wrapper, or a benchmark) can all declare the same hints.

Important: **the same code path is used in production.** The harness does not take a shortcut. The production actor runtime will call `sqlite_preload` on startup with whatever hints came from the actor config (see [`walkthrough.md`](./walkthrough.md) Chapter 7, "Preload hints"). The test just uses a hardcoded plan instead of reading one from actor metadata.

### 4.4 Crash-and-reopen

The `VfsV2Harness::crash_and_reopen` method is the key mechanism for testing recovery. Concretely:

1. Close the `NativeDatabase`. Drop the `KvVfs`. Do NOT touch `MemoryKv`.
2. Build a new `KvVfs` with the same `MemoryKv`, a new generation number (bumped by one), and the same preload plan.
3. Return a new `VfsV2Harness` bound to the new VFS and DB.

Everything in-memory gets a fresh slate (page cache, `dirty_pgnos_in_log`, dirty buffer, retry counters). The KV state is *exactly* what survived the crash. This is the closest analog we can get to a real process restart while staying inside one test binary.

### 4.5 Failure-injection integration

The harness accepts a `FailurePlan` at build time and installs it onto `MemoryKv` before opening the VFS. Tests that want to inject failures mid-run call `harness.kv().install_failure_plan(plan)` during the closure — this is a runtime operation on the `MemoryKv` and doesn't require rebuilding anything.

A common pattern is "succeed during preload, then fail on the next commit":

```rust
let harness = VfsV2Harness::builder().build().await?;
// Preload already happened inside build().
harness.kv().install_failure_plan(FailurePlan {
    injections: vec![FailureInjection {
        after_ops: 0, // relative to install time
        on_op: Some(OpKind::SqliteCommit),
        error: FailureMode::FenceMismatch,
        fires: 1,
    }],
});
let err = harness.run(|actor| async move {
    actor.sql("INSERT INTO t VALUES (1)").await
}).await.unwrap_err();
```

`after_ops` is relative to the op_count at installation time, not relative to the driver's total count. This makes failure plans composable and avoids the "which ops does preload use?" counting headache.

---

## 5. Test cases we want to exist

The suite is organized into three tiers.

### Tier A — shared SQL-level conformance (v1 and v2 both pass)

These tests run against the VFS via the harness and cover correctness at the SQL boundary. They pass against v1 today (if we wire them up) and must pass against v2.

1. **A1** — Open an empty DB, create one table, insert one row, read it back. Basic smoke test.
2. **A2** — Create a table, insert 100 rows in 100 separate transactions, read them all back. Validates write+read path for small transactions.
3. **A3** — Create a table, insert 100 rows in one transaction. Validates batch-atomic commit.
4. **A4** — Insert a row with a 1 MiB TEXT payload, read it back. Validates large-page handling.
5. **A5** — Insert 1,000 rows each with a 10 KiB payload in one transaction, read them back. This is the case that pushed v1 into the journal-fallback path. v2 should take the fast path; v1 falls back. Both must produce the same rows.
6. **A6** — Insert 10,000 rows in one transaction (slow path territory on v2). Validates multi-stage commit and reads after.
7. **A7** — Schema change: `ALTER TABLE ... ADD COLUMN`. Validates schema cookie bump and page 1 update.
8. **A8** — Transaction rollback via explicit `ROLLBACK`. Validates rollback semantics.
9. **A9** — `SELECT COUNT(*)` on a 1,000-row table. Validates aggregate reads across many pages.
10. **A10** — `SELECT ... WHERE ...` with an index. Validates random-access B-tree page reads.

### Tier B — v2-only invariants

These exercise the machinery v1 doesn't have. They run only against v2.

11. **B1** — `sqlite_commit` fast path: single 4-page commit. Assert exactly one `sqlite_commit` op and zero `sqlite_commit_stage` ops.
12. **B2** — `sqlite_commit_stage` slow path: 10,000-page commit. Assert `N > 0` stage ops followed by exactly one `sqlite_commit`. Assert that `materialized_txid` does not advance before `sqlite_commit` returns.
13. **B3** — Orphan cleanup on startup: seed the KV with `LOG/100/0..5` where `head_txid = 99`. Crash-and-reopen. Assert those keys are gone after startup.
14. **B4** — Orphan cleanup is idempotent: seed orphans, crash, install a failure plan that fails the cleanup `delete_range` on the first attempt, crash again, assert the orphans are still cleaned up.
15. **B5** — Generation fencing: open a harness, note its generation, manually bump `MemoryKv::set_generation`. The next commit from the harness must fail with `FenceMismatch` and leave KV state unchanged.
16. **B6** — `sqlite_takeover` on startup always bumps generation by exactly one, even if the previous shutdown was clean.
17. **B7** — Read path layer 1 (page cache): prefetch a page, then `xRead` it, assert zero KV ops after the warmup.
18. **B8** — Read path layer 2 (dirty buffer): begin a transaction, dirty page 10, read page 10 inside the same transaction (bypassing SQLite's pager — harness needs a way to force this), assert the bytes match the dirty value and zero KV ops fire.
19. **B9** — Read path layer 3 (unmaterialized log): commit a transaction that dirties page 10, do not let the materializer run, read page 10. Assert one KV get on the LOG frame and zero on `PAGE/10`.
20. **B10** — Read path layer 3 → layer 4 fallback: same as B9, but inject a race: the materializer fires just before the read. Assert the read retries against fresh state and still returns the correct bytes.
21. **B11** — Read path layer 4 (materialized): run the materializer to completion, read a random page. Assert exactly one `batch_get` on `PAGE/<pgno>`.
22. **B12** — Prefetch predictor on sequential reads: sequentially read pages 5, 8, 11, 14, 17, 20. Assert the first call fetches 1 page, subsequent calls fetch N pages in one shot (N ≥ 2).
23. **B13** — Materializer basic: write 3 tiny transactions. Trigger materializer. Assert one `sqlite_materialize` op with 3 page_writes and range_deletes for `LOG/` and `LOGIDX/`.
24. **B14** — Materializer latest-wins merge: write page 10 in txid 5, txid 6, txid 7 (different bytes each time). Materialize. Assert `PAGE/10` contains the bytes from txid 7 only. Assert the materializer issued exactly one page_write for page 10, not three.
25. **B15** — Materializer race with reader: halfway through `sqlite_materialize`, simulate a reader issuing a layer-3 read. Assert the reader gets the correct bytes (either the pre-materialize LOG frame or the post-materialize PAGE, depending on which side of the atomic boundary the read lands).
26. **B16** — Materializer race with writer: halfway through `sqlite_materialize`, simulate a writer issuing `sqlite_commit_stage`. Assert the stage succeeds (because it CASes only generation, not `materialized_txid`) and the materializer completes successfully.
27. **B17** — Preload hits warm cache: preload `PAGE/1..10`. Read pages 1–10. Assert zero KV ops during reads.
28. **B18** — Preload ignores missing keys: preload a key that doesn't exist. Assert the preload op returns no error and the miss falls through to normal `batch_get` on first access.
29. **B19** — Cold-start round trip count = 1: start a 10,000-page database from state. Assert that on open, exactly one `sqlite_preload` op fires and zero other ops fire before the first SQL statement.
30. **B20** — Preload with a byte budget: preload a prefix that exceeds the budget. Assert the preload returns fewer entries than the full prefix but the VFS degrades gracefully (falls back to `batch_get` for the missed pages without error).
31. **B21** — Partial-write recovery on commit-stage: install a `PartialStage { frames_written: 3 }` injection on the 5th frame. Trigger a 10-frame commit. Assert the commit returns error, the LOG has only the first 3 frames of the failing txid, and then crash-and-reopen cleans the partial. Re-run the commit and it succeeds.
32. **B22** — Lock page skip: write enough data to straddle the SQLite lock page (page 262,145 at 4 KiB). Assert the LTX encoder skips it and reads around it return zeros.
33. **B23** — `db_size_pages` truncation: shrink the database via `VACUUM`-like semantics (manually truncate file). Assert `DBHead.db_size_pages` shrinks and `PAGE/` entries past the new size are garbage-collected on the next materialize pass.
34. **B24** — Empty DB on first open: build a harness with no initial KV. Assert that open bootstraps META with `head_txid = 0, materialized_txid = 0, generation = 1`, and that the SQLite header page is synthesized.

### Tier C — chaos and invariants under churn

These are less about specific behavior and more about "does the system stay consistent when we hammer it."

35. **C1** — Randomized insert+commit+materialize+crash loop for 60 seconds with a fixed PRNG seed. At the end, open the DB cleanly and read every row. Assert the row count and checksum matches a parallel "oracle" `MemoryKv` that applied the same successful operations.
36. **C2** — Same as C1 but with a fence-mismatch injection every 10 ops. Assert: no durable state is lost unless a commit's error was surfaced to the test.
37. **C3** — Materializer lag bound: run 1,000 small commits without letting the materializer advance. Assert the materializer eventually catches up and `LOG/` is bounded in size within the configured back-pressure threshold.

That's 37 tests. 10 in Tier A, 24 in Tier B, 3 in Tier C. Tier A also runs against v1 via the same harness (see §6).

---

## 6. Integration with existing infrastructure

### 6.1 v1 and v2 share the harness

The `VfsV2Harness` is named v2 in the code but the `vfs_kind: VfsKind` field lets it build v1 VFSes too. Tier A tests are parameterized over `VfsKind::V1` and `VfsKind::V2`:

```rust
#[tokio::test]
async fn tier_a_smoke() -> Result<()> {
    for kind in [VfsKind::V1, VfsKind::V2] {
        tier_a_smoke_body(kind).await?;
    }
    Ok(())
}
```

For v1, `MemoryKv` implements the v1-shape methods (`batch_get`, `batch_put`, `batch_delete`, `delete_range`) and ignores the v2 methods. For v2, all methods are implemented. The VFS uses whatever it needs — v1 never calls `sqlite_commit`, and v2 never falls back to `batch_put` for a committed transaction.

This gives us a tight reciprocal: every v1 bug we retroactively fix gets a harness-level regression test, and every v2 test case we write is also a v1 regression test if we tag it `#[tier_a]`.

### 6.2 Extending `examples/sqlite-raw`

We extend the existing bench harness in three small ways, no fork.

1. **`VFS_VERSION` env var** in `examples/sqlite-raw/scripts/bench-large-insert.ts`. When set to `v2`, the bench constructs an actor that opens its database with the v2 VFS. When set to `v1` or unset, it opens with v1. This is a one-line change in the RivetKit registry setup — `db({ vfsVersion: env.VFS_VERSION })` — and pre-existing actors still work because v1 is the default.
2. **Single-row output format** — the bench currently logs a pretty table. We teach it to also emit one line of JSON to stdout suffixed with a `BENCH_RESULT:` prefix. The CI driver parses those lines and updates `BENCH_RESULTS.md` with a v2 column next to the existing v1 numbers. Per `design-decisions.md` §5, v1 numbers are preserved as a baseline.
3. **Actor-side VFS telemetry** — we add a new action `benchGetVfsTelemetry` that returns the actor's `VfsMetrics` struct (already exists at `vfs.rs:167` for v1) plus a new `Vfs2Metrics` struct for v2 (number of fast-path commits, number of slow-path commits, number of materializer passes, current `head_txid - materialized_txid` lag). The bench script calls this after the insert and prints the values. This satisfies the existing `CLAUDE.md` directive about using VFS telemetry around benchmark work.

### 6.3 The existing driver test suite

`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-db.ts` and `actor-db-stress.ts` already cover SQL-level behavior against the full engine stack. We treat these as our "Tier D" — the final level above the harness, running against the real engine. They do not change as part of v2. When v2 ships behind a flag, we add one new driver-test-suite config variant that runs the whole existing DB test suite against v2:

```ts
// New file: rivetkit-typescript/packages/rivetkit/tests/fixtures/driver-test-suite/vfs-v2-config.ts
export const driverVfsV2Config: DriverTestConfig = {
    ...baseDriverConfig,
    vfsVersion: "v2",
};
```

And in `driver-engine.test.ts`:

```ts
describe("driver engine (v1 VFS)", () => runDriverTests(driverVfsV1Config));
describe("driver engine (v2 VFS)", () => runDriverTests(driverVfsV2Config));
```

`runDriverTests` already walks the shared suite. The variant matrix expands by one.

### 6.4 Relationship to existing `.agent/research/sqlite/`

The research file at `.agent/research/sqlite/sqlite-vfs-ltx-redesign.md` captured the initial redesign brainstorm. The test architecture here is the operational output of that research. If the research file and this document disagree, this document wins.

---

## 7. End-to-end verification path

Unit tests against `MemoryKv` are the bulk of our coverage. But they can't prove that the new `EnvoyKv` napi methods are correct, that the new runner-protocol ops are wired on the wire format, or that the new `actor_kv::sqlite::*` engine handlers commit to UDB atomically.

For that we need a small e2e suite that runs the same Tier A workloads through the real stack. The plan:

### 7.1 Shape

New file: `rivetkit-typescript/packages/rivetkit/tests/vfs-v2-e2e.test.ts`.

Each test spins up a local RocksDB engine (via `scripts/run/engine-rocksdb.sh`), creates an actor configured for v2, runs a workload, and asserts the result. Tests use `setupDriverTest` from the existing driver-test-suite utilities so we inherit the engine-lifecycle plumbing.

### 7.2 Cases to run

- **E1** — single-row insert + read (smoke test for `kv_sqlite_commit` and `kv_sqlite_preload`).
- **E2** — 5,000-row single-transaction insert (smoke test for `kv_sqlite_commit_stage` multi-phase).
- **E3** — insert + crash (actor `destroy()`) + recreate + read (smoke test for `kv_sqlite_takeover` and orphan cleanup).
- **E4** — insert + materializer pass + read (smoke test for `kv_sqlite_materialize`).
- **E5** — `SELECT *` from a 10,000-row table (smoke test for preload hints and prefetch). Assert the actor-side `Vfs2Metrics.preload_entries_count` matches the expected preload.
- **E6** — repeat E1 with `RUST_LOG=rivet_pegboard::actor_kv::sqlite=debug` and assert the per-op traces appear. This doubles as a tripwire for anyone who accidentally removes the tracing.

### 7.3 Running

```bash
./scripts/run/engine-rocksdb.sh >/tmp/rivet-engine-e2e.log 2>&1 &
pnpm --filter rivetkit test vfs-v2-e2e
```

The tests do NOT run in CI by default (they depend on the engine binary). They run:

- Manually by the engineer doing v2 work.
- On the v2 feature branch nightly via a new `test-vfs-v2-e2e` workflow that builds the engine and runs this test file.

Once v2 ships and the flag flips, these tests become default and move into the regular CI run.

### 7.4 Cross-check: the bench harness as an e2e

Because §6.2 wires `examples/sqlite-raw` to optionally use v2, running `VFS_VERSION=v2 pnpm --dir examples/sqlite-raw bench:large-insert` is *also* an e2e test. We explicitly call this out so the team doesn't build two parallel e2e rigs. The bench harness has the advantage of producing numbers for `BENCH_RESULTS.md` as a side effect, so it's the preferred ad-hoc manual verification tool.

---

## 8. Open questions

None of these block implementation, but they should get answered early in the v2 work.

1. **napi wrapper for `MemoryKv`?** TypeScript-side unit tests don't need it today (everything we test is at the SQL level via `actor-db.ts`). We skip this until someone has a concrete test that benefits. Documenting the option is enough.
2. **Harness for VFS callback-level unit tests?** The Rust harness as specced drives SQL, not raw VFS callbacks. If a test needs to inject an `xRead` directly (say, to force the layer-3 retry path), we add a `VfsV2Harness::raw_vfs()` accessor that returns a type that wraps the lower-level machinery. Defer until needed.
3. **Checksum-based workload oracle for Tier C.** C1 calls for a parallel oracle that shadows the successful operations. Define this as a simple `HashMap<(table, pk), Vec<u8>>` with explicit row-by-row comparison. It does not need to be a full SQLite — just enough to assert row-level equivalence.
4. **Running Tier A against `file-system` driver instead of `MemoryKv`.** Currently we build one in-memory impl. If a Tier A test fails *only* on v2, we can't tell whether it's a VFS bug or a `MemoryKv` bug. One mitigation: optionally run Tier A against `file-system`-backed SQLite too. Punt until we see a real disagreement in practice.

---

## 9. Implementation checklist

In order. Each item is one small commit.

### Phase 1 — ship `MemoryKv` against the v1 trait (no v2 methods yet)

1. Create `rivetkit-typescript/packages/sqlite-native/src/memory_kv.rs` implementing `MemoryKv` against the *existing* `SqliteKv` trait (v1 methods only). Include the snapshot/restore, op log, and a minimal `FailurePlan` with only `GenericError` and `PartialWrite`.
2. Add `pub mod memory_kv;` to `rivetkit-typescript/packages/sqlite-native/src/lib.rs`.
3. Add `#[cfg(test)]` unit tests inside `memory_kv.rs` that verify it behaves as a correct KV store (get / put / delete / delete_range / snapshot / restore).
4. Add `anyhow`, `tokio` (with `sync` feature), and `futures-util` to `rivetkit-typescript/packages/sqlite-native/Cargo.toml` as workspace deps.

### Phase 2 — ship `VfsV2Harness` against v1 only

5. Create `rivetkit-typescript/packages/sqlite-native/src/test_harness.rs` with the builder, the harness struct, and the `HarnessRun` assertion type. `VfsKind::V2` returns an error for now.
6. Add `pub mod test_harness;` to `lib.rs`.
7. Port Tier A tests 1–10 into `rivetkit-typescript/packages/sqlite-native/tests/tier_a.rs`. Run against `VfsKind::V1`. Confirm they pass. This is the baseline — if they pass now, v2 must not regress them.

### Phase 3 — add the v2 trait methods

8. Modify `rivetkit-typescript/packages/sqlite-native/src/sqlite_kv.rs`:
   - Add the `KvSqliteError`, `KvSqliteCommitOp`, `KvSqliteCommitStageOp`, `KvSqliteMaterializeOp`, `KvSqlitePreloadOp`, `KvSqlitePreloadResult`, `KvSqliteTakeoverOp` types.
   - Add `sqlite_commit`, `sqlite_commit_stage`, `sqlite_materialize`, `sqlite_preload`, `sqlite_takeover` methods to the trait with default impls that return `NotImplemented`. This preserves v1 binary compatibility.
9. Extend `MemoryKv` to implement all five new methods against the `BTreeMap` backing store, including the CAS checks, orphan range-delete, and generation bump.
10. Extend `FailurePlan` with `FenceMismatch` and `PartialStage` variants.
11. Extend the op log with the new `OpKind` variants.

### Phase 4 — ship v2 VFS

12. Create `rivetkit-typescript/packages/sqlite-native/src/vfs_v2.rs` per [`design-decisions.md`](./design-decisions.md) §3. This is a separate PR from the test work; document the cross-PR dependency.
13. `VfsKind::V2` in `test_harness.rs` now constructs a `vfs_v2::KvVfsV2`.
14. Port the Tier A tests to also run against `VfsKind::V2`. Assert both variants pass.

### Phase 5 — v2-only tests

15. Create `rivetkit-typescript/packages/sqlite-native/tests/tier_b.rs` with Tier B tests 11–34.
16. Create `rivetkit-typescript/packages/sqlite-native/tests/tier_c.rs` with Tier C tests 35–37 (uses `rand` with a fixed seed).

### Phase 6 — EnvoyKv delegation

17. Modify `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` `EnvoyKv` impl to implement the v2 methods. Each delegates to a new napi method on `EnvoyHandle` (which in turn speaks the new runner-protocol ops). This is the production path — out of scope for the test architecture doc, but the test changes depend on this existing.

### Phase 7 — bench harness extension

18. Modify `examples/sqlite-raw/src/index.ts` to take `vfsVersion` from an actor config field and plumb it through `db({ vfsVersion })`. Add a `benchGetVfsTelemetry` action.
19. Modify `examples/sqlite-raw/scripts/bench-large-insert.ts` to honor `VFS_VERSION`, emit `BENCH_RESULT:` JSON lines, and print both v1 and v2 telemetry summaries.
20. Modify `examples/sqlite-raw/BENCH_RESULTS.md` to add v2 columns next to the existing v1 columns. Do not remove the v1 rows.

### Phase 8 — driver-test-suite variant

21. Create `rivetkit-typescript/packages/rivetkit/tests/fixtures/driver-test-suite/vfs-v2-config.ts` as a new driver test config that flags v2.
22. Modify `rivetkit-typescript/packages/rivetkit/tests/driver-engine.test.ts` to call `runDriverTests` with both configs.

### Phase 9 — e2e tests

23. Create `rivetkit-typescript/packages/rivetkit/tests/vfs-v2-e2e.test.ts` with E1–E6.
24. Add a new GitHub Actions workflow `test-vfs-v2-e2e.yml` that starts the RocksDB engine in the background, runs the test, and uploads logs as an artifact. This file is separate from the regular `test.yml` because the dependency on the engine binary makes it slower and flakier.

### Phase 10 — docs and tidying

25. Update `docs-internal/rivetkit-typescript/sqlite-ltx/design-decisions.md` §3 to mark the testing checklist items done as they land.
26. Update `website/src/content/docs/actors/limits.mdx` with any v2 limits that surface (per `CLAUDE.md` docs-sync directive).
27. Update `CLAUDE.md` (root or rivetkit-typescript) with one-liner pointers to `memory_kv.rs` and `test_harness.rs` so future agents find them.

### Files to create

- `rivetkit-typescript/packages/sqlite-native/src/memory_kv.rs` (Phase 1)
- `rivetkit-typescript/packages/sqlite-native/src/test_harness.rs` (Phase 2)
- `rivetkit-typescript/packages/sqlite-native/src/vfs_v2.rs` (Phase 4, outside this doc's scope but depended on)
- `rivetkit-typescript/packages/sqlite-native/tests/tier_a.rs` (Phase 2)
- `rivetkit-typescript/packages/sqlite-native/tests/tier_b.rs` (Phase 5)
- `rivetkit-typescript/packages/sqlite-native/tests/tier_c.rs` (Phase 5)
- `rivetkit-typescript/packages/rivetkit/tests/fixtures/driver-test-suite/vfs-v2-config.ts` (Phase 8)
- `rivetkit-typescript/packages/rivetkit/tests/vfs-v2-e2e.test.ts` (Phase 9)
- `.github/workflows/test-vfs-v2-e2e.yml` (Phase 9)

### Files to modify

- `rivetkit-typescript/packages/sqlite-native/src/lib.rs` — add `pub mod memory_kv;` and `pub mod test_harness;`. (Phases 1 and 2)
- `rivetkit-typescript/packages/sqlite-native/src/sqlite_kv.rs` — add v2 trait methods, `KvSqliteError` enum, op structs. (Phase 3)
- `rivetkit-typescript/packages/sqlite-native/Cargo.toml` — add `anyhow`, `tokio` sync, `futures-util`, `rand` dev-dep. (Phases 1 and 5)
- `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` — implement v2 trait methods on `EnvoyKv`. (Phase 6)
- `examples/sqlite-raw/src/index.ts` — expose `vfsVersion` and telemetry action. (Phase 7)
- `examples/sqlite-raw/scripts/bench-large-insert.ts` — honor `VFS_VERSION`, emit BENCH_RESULT JSON. (Phase 7)
- `examples/sqlite-raw/BENCH_RESULTS.md` — add v2 columns. (Phase 7)
- `rivetkit-typescript/packages/rivetkit/tests/driver-engine.test.ts` — run driver suite against both VFS variants. (Phase 8)
- `docs-internal/rivetkit-typescript/sqlite-ltx/design-decisions.md` — tick checklist items. (Phase 10)
- `website/src/content/docs/actors/limits.mdx` — note any new v2 limits. (Phase 10)
- `CLAUDE.md` — one-liner pointers. (Phase 10)
