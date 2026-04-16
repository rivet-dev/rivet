# SQLite VFS v2 -- Test Architecture Proposal

> **Status (2026-04-15):** Proposal. Supersedes the stale `test-architecture.md`. Incorporates the locked decisions: separate `SqliteProtocol` trait, standalone compaction crate, no Envoy dependency in tests, simplified coordinator topology (channel + local `HashMap<ActorId, JoinHandle>`).

---

## A. Trait boundaries

### A.1 `SqliteStore` -- the compaction module's UDB abstraction

The compaction module needs a minimal KV surface. It never sees UDB directly.

```rust
// engine/packages/sqlite-storage/src/store.rs

pub struct Mutation {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,  // Some = set, None = delete
}

#[async_trait]
pub trait SqliteStore: Send + Sync + 'static {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    async fn batch_get(&self, keys: &[Vec<u8>]) -> Result<Vec<Option<Vec<u8>>>>;
    async fn scan_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    async fn atomic_write(&self, mutations: Vec<Mutation>) -> Result<()>;
}
```

Four methods. This is the entire surface the compaction module and `SqliteEngine` need from the backing store. In production, `UdbStore` implements this against `universaldb::Database`. In tests, `MemoryStore` implements it against a `BTreeMap`.

### A.2 `SqliteProtocol` -- the actor-side VFS protocol trait

Defined in `protocol-and-vfs.md` section 3.1 and unchanged here:

```rust
// engine/packages/sqlite-storage/src/protocol.rs

#[async_trait]
pub trait SqliteProtocol: Send + Sync {
    async fn get_pages(&self, req: GetPagesRequest) -> Result<GetPagesResponse>;
    async fn commit(&self, req: CommitRequest) -> Result<CommitResponse>;
    async fn commit_stage(&self, req: CommitStageRequest) -> Result<CommitStageResponse>;
    async fn commit_finalize(&self, req: CommitFinalizeRequest) -> Result<CommitFinalizeResponse>;
}
```

### A.3 `SqliteEngine` -- the bridge

`SqliteEngine` is the concrete type that implements `SqliteProtocol` using a `SqliteStore`:

```rust
// engine/packages/sqlite-storage/src/engine.rs

pub struct SqliteEngine<S: SqliteStore> {
    store: Arc<S>,
    page_indices: scc::HashMap<String, DeltaPageIndex>,
    compaction_tx: mpsc::UnboundedSender<String>,
    metrics: SqliteStorageMetrics,
}

impl<S: SqliteStore> SqliteEngine<S> {
    pub fn new(store: Arc<S>) -> Self { ... }
}

#[async_trait]
impl<S: SqliteStore> SqliteProtocol for SqliteEngine<S> { ... }
```

In production, pegboard-envoy creates `SqliteEngine<UdbStore>`. In tests, the harness creates `SqliteEngine<MemoryStore>`. The same engine logic runs in both cases.

### A.4 Relationship diagram

```
Actor process (VFS)                        Engine process
--------------------------                 --------------------------
vfs_v2.rs                                  pegboard-envoy (prod glue)
  |                                           |
  v                                           v
SqliteProtocol    <-- trait boundary -->  SqliteEngine<S>
                                              |
                                              v
                                          SqliteStore  <-- trait boundary
                                              |
                              +---------------+---------------+
                              |                               |
                      UdbStore (prod)         MemoryStore (test)
```

---

## B. The in-memory test driver

### B.1 `MemoryStore`

```rust
// engine/packages/sqlite-storage/src/test_utils/memory_store.rs

pub struct MemoryStore {
    data: Arc<parking_lot::RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
    config: MemoryStoreConfig,
    op_log: Arc<parking_lot::Mutex<Vec<OpRecord>>>,
    op_count: AtomicU64,
}

pub struct MemoryStoreConfig {
    /// Base latency per operation in milliseconds.
    pub latency_ms: u64,
    /// Jitter range in milliseconds. Actual latency = latency_ms + rand(-jitter_ms, +jitter_ms).
    pub jitter_ms: u64,
    /// If set, return an error after this many operations.
    pub fail_after_ops: Option<u64>,
    /// If set, simulate fence mismatch after this many operations.
    pub fence_fail_after_ops: Option<u64>,
    /// If true, atomic_write applies only the first half of mutations to simulate partial writes.
    pub simulate_partial_write: bool,
}

impl Default for MemoryStoreConfig {
    fn default() -> Self {
        Self {
            latency_ms: 0,
            jitter_ms: 0,
            fail_after_ops: None,
            fence_fail_after_ops: None,
            simulate_partial_write: false,
        }
    }
}
```

Constructors:

```rust
impl MemoryStore {
    /// Zero latency, no failure injection. For basic unit tests.
    pub fn new_fast() -> Self { ... }

    /// 20 ms latency, 5 ms jitter. Simulates C6 production RTT.
    pub fn new_with_latency() -> Self {
        Self::new(MemoryStoreConfig {
            latency_ms: 20,
            jitter_ms: 5,
            ..Default::default()
        })
    }

    /// Full configuration.
    pub fn new(config: MemoryStoreConfig) -> Self { ... }
}
```

### B.2 Artificial latency

Every `SqliteStore` method in the `MemoryStore` implementation calls `simulate_latency()` before executing:

```rust
async fn simulate_latency(&self) {
    if self.config.latency_ms == 0 && self.config.jitter_ms == 0 {
        return;
    }
    let jitter = if self.config.jitter_ms > 0 {
        let mut rng = rand::thread_rng();
        rng.gen_range(-(self.config.jitter_ms as i64)..=(self.config.jitter_ms as i64))
    } else {
        0
    };
    let delay = (self.config.latency_ms as i64 + jitter).max(0) as u64;
    tokio::time::sleep(Duration::from_millis(delay)).await;
}
```

This ensures latency-sensitive bugs (such as issuing sequential round trips where one batched call would suffice) show up in wall-clock timing of test runs.

### B.3 Operation log and assertions

```rust
#[derive(Debug, Clone)]
pub enum OpRecord {
    Get { key: Vec<u8> },
    BatchGet { keys: Vec<Vec<u8>> },
    ScanPrefix { prefix: Vec<u8> },
    AtomicWrite { mutation_count: usize },
}

impl MemoryStore {
    pub fn op_log(&self) -> Vec<OpRecord> { ... }
    pub fn op_count(&self) -> u64 { ... }
    pub fn clear_op_log(&self) { ... }

    /// Assert the op log contains at least one entry matching the predicate.
    pub fn assert_ops_contain(&self, pred: impl Fn(&OpRecord) -> bool) { ... }

    /// Assert the total number of operations equals `n`.
    pub fn assert_op_count(&self, n: u64) { ... }
}
```

### B.4 Snapshot and restore

```rust
impl MemoryStore {
    pub fn snapshot(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.data.read().clone()
    }

    pub fn restore(&self, snapshot: BTreeMap<Vec<u8>, Vec<u8>>) {
        *self.data.write() = snapshot;
    }
}
```

---

## C. The compaction module as a standalone crate

### C.1 Crate location and structure

```
engine/packages/sqlite-storage/
  Cargo.toml
  src/
    lib.rs                    -- pub mod declarations, re-exports
    store.rs                  -- SqliteStore trait + Mutation struct
    protocol.rs               -- SqliteProtocol trait, request/response types
    engine.rs                 -- SqliteEngine<S> implementing SqliteProtocol
    commit.rs                 -- commit handler (fast path + slow path)
    read.rs                   -- get_pages handler
    takeover.rs               -- takeover + recovery logic
    preload.rs                -- preload handler
    compaction/
      mod.rs                  -- coordinator + worker spawn
      worker.rs               -- compact_worker per-actor task
      shard.rs                -- compact_shard single-pass logic
    page_index.rs             -- DeltaPageIndex (persistent sparse + in-memory cache)
    keys.rs                   -- META, SHARD, DELTA, DELTAREF, PIDX key builders
    ltx.rs                    -- LTX encode/decode helpers (wraps litetx or hand-rolled)
    types.rs                  -- DbHead, DirtyPage, FetchedPage, shared structs
    metrics.rs                -- Prometheus metric definitions
    test_utils/
      mod.rs                  -- pub mod declarations
      memory_store.rs         -- MemoryStore
      helpers.rs              -- page_bytes(), setup_engine(), etc.
  tests/
    unit/                     -- #[test] for individual functions
    integration/              -- #[tokio::test] for full protocol round trips
    compaction/               -- #[tokio::test] for compaction-specific scenarios
    concurrency/              -- concurrent commit + compact + read tests
    failure/                  -- failure injection tests
    latency/                  -- RTT-assumption validation tests
  benches/
    v1_v2_comparison.rs       -- criterion benchmark comparing v1 and v2
```

### C.2 Dependency graph

`sqlite-storage` depends on:
- `tokio` (runtime, channels, time)
- `tracing` (structured logging)
- `scc` (concurrent HashMap for page index)
- `lz4_flex` or `lz4` (compression)
- `parking_lot` (RwLock for test utils)
- `rand` (jitter in test utils)
- `async-trait`
- `anyhow`
- `bytes`
- `prometheus` via `rivet-metrics` (metric types)

`sqlite-storage` does NOT depend on:
- `pegboard-envoy`
- `universaldb`
- `nats`
- `gas` / `gasoline`
- `rivet-guard-core`
- Any WebSocket crate
- `runner-protocol` (the types are defined locally in `protocol.rs`)

In production, `pegboard-envoy` imports `sqlite-storage` and provides `UdbStore`:

```rust
// engine/packages/pegboard-envoy/src/sqlite_bridge.rs
use sqlite_storage::{SqliteStore, SqliteEngine};
use universaldb::Database;

pub struct UdbStore { db: Arc<Database>, actor_subspace: Vec<u8> }

#[async_trait]
impl SqliteStore for UdbStore { ... }
```

This is the only file that bridges the two worlds. The test suite never touches it.

---

## D. Test structure

### D.1 Location

All tests live inside the `sqlite-storage` crate:
- `engine/packages/sqlite-storage/tests/` for integration tests
- `engine/packages/sqlite-storage/src/` inline `#[cfg(test)] mod tests` blocks for unit tests
- `engine/packages/sqlite-storage/benches/` for criterion benchmarks

Run with: `cargo test -p sqlite-storage`

### D.2 Test categories

#### Unit tests (inline in source files)

Individual function correctness. No async, no store, no engine.

- LTX encode/decode round trip
- Key encoding/decoding (META, SHARD, DELTA, PIDX)
- Page merge logic (latest-txid-wins)
- PIDX lookup and update
- DbHead serialization
- Shard ID computation (`pgno / 64`)
- Refcount arithmetic

#### Integration tests (`tests/integration/`)

Full round-trip through `SqliteEngine` with `MemoryStore`. Each test follows the pattern Nathan specified:

```rust
#[tokio::test]
async fn commit_and_read_back() {
    let store = MemoryStore::new_fast();
    let engine = SqliteEngine::new(Arc::new(store));

    let meta = engine.takeover(TakeoverRequest {
        actor_id: "actor-1".into(),
        expected_generation: 0,
    }).await.unwrap().unwrap_ok();

    engine.commit(CommitRequest {
        actor_id: "actor-1".into(),
        generation: meta.new_generation,
        expected_head_txid: meta.meta.head_txid,
        dirty_pages: vec![DirtyPage { pgno: 1, bytes: test_page(1) }],
        new_db_size_pages: 1,
    }).await.unwrap().unwrap_ok();

    let pages = engine.get_pages(GetPagesRequest {
        actor_id: "actor-1".into(),
        generation: meta.new_generation,
        pgnos: vec![1],
    }).await.unwrap().unwrap_ok();

    assert_eq!(pages.pages[0].bytes.as_deref(), Some(test_page(1).as_slice()));
}
```

Proposed integration tests:

- `commit_and_read_back` -- write pages, read them back
- `commit_multiple_pages` -- write 100 pages in one commit
- `commit_overwrites_previous` -- write page 1 twice, read gets latest
- `takeover_bumps_generation` -- second takeover increments generation
- `fence_mismatch_on_stale_generation` -- commit with old generation fails
- `fence_mismatch_on_stale_txid` -- commit with wrong head_txid fails
- `slow_path_commit_stage_finalize` -- stage chunks + finalize
- `slow_path_missing_stage` -- finalize with wrong stage_id fails
- `preload_returns_requested_pages` -- preload fetches pages in one call
- `preload_respects_max_bytes` -- preload truncates at byte budget
- `read_nonexistent_page_returns_none` -- pgno beyond db size
- `multiple_actors_isolated` -- two actors share a store, data is disjoint
- `commit_updates_db_size_pages` -- db_size_pages tracks correctly

#### Compaction tests (`tests/compaction/`)

Exercise the coordinator and worker against `MemoryStore`.

- `compaction_folds_deltas_into_shard` -- commit N deltas, run compaction, verify shard contains merged pages and deltas are deleted
- `compaction_preserves_latest_wins` -- two deltas overwrite the same page, compaction picks the latest
- `compaction_multi_shard_delta` -- a delta spanning 3 shards is consumed across 3 passes via refcounting
- `compaction_refcount_reaches_zero` -- delta deleted only when all shards have consumed their pages
- `compaction_idempotent` -- running compaction twice on an already-compacted actor is a no-op
- `compaction_concurrent_with_commit` -- commit fires during compaction, both succeed, data consistent
- `compaction_fence_mismatch_aborts` -- if generation changes mid-compaction, the worker exits
- `recovery_cleans_orphan_deltas` -- simulate crash after commit but before visibility, takeover cleans up
- `recovery_cleans_orphan_stages` -- simulate crash after commit_stage but before finalize
- `coordinator_deduplicates` -- sending actor_id twice only spawns one worker

#### Concurrency tests (`tests/concurrency/`)

- `concurrent_commits_serial_reads` -- 10 concurrent commits to different actors, reads return correct data
- `concurrent_commit_and_compaction` -- commit and compaction interleave, final state is consistent
- `concurrent_reads_during_compaction` -- reads always return correct page data even while compaction mutates storage layout

#### Failure injection tests (`tests/failure/`)

- `store_error_mid_commit` -- store returns error after N ops, commit fails cleanly, no partial state
- `partial_write_on_atomic_write` -- simulate_partial_write enabled, verify engine detects and fails the commit
- `store_error_during_compaction` -- compaction fails, next pass retries from consistent state
- `takeover_after_crash` -- snapshot state mid-commit, restore, takeover recovers

#### Quota tests (`tests/quota/`)

- `commit_within_quota` -- commit 100 pages, verify `sqlite_storage_used` tracked correctly, commit succeeds
- `commit_exceeds_quota` -- set `sqlite_max_storage` to 1 MiB, fill DB to near limit, verify next commit returns quota-exceeded error
- `quota_tracks_deltas_and_shards` -- write deltas, run compaction (folds delta bytes into shard bytes), verify quota stays roughly constant (delta bytes replaced by shard bytes)
- `quota_separate_from_kv` -- fill SQLite to 90% of its quota, verify general KV writes still succeed (independent limits)
- `quota_freed_on_truncate` -- write large DB, truncate, verify quota decreases
- `quota_accounts_for_pidx` -- write many small deltas creating many PIDX entries, verify PIDX bytes counted in quota
- `compaction_does_not_inflate_quota` -- large compaction pass replaces N deltas with 1 shard, verify quota does not grow

#### Latency tests (`tests/latency/`)

With `MemoryStore::new_with_latency()` (20 ms + 5 ms jitter):

- `small_commit_is_one_rtt` -- a 4-page commit takes approximately 1x latency (20 ms), not 2x or more
- `get_pages_is_one_rtt` -- reading 10 pages takes approximately 1x latency
- `cold_start_adds_zero_extra_rtts` -- VFS initializes from the startup data passed in the actor start message, with no protocol calls for takeover or preload
- `commit_does_not_block_on_compaction` -- commit returns in ~1 RTT even with compaction running

These tests measure wall-clock time with a tolerance band (e.g., 15--80 ms for a 1-RTT operation) and assert the design's RTT assumptions hold.

---

## E. How it maps to the Envoy protocol

In production, the mapping is thin glue code in pegboard-envoy:

```
ws_to_tunnel_task.rs receives SqliteCommitRequest over WebSocket
  -> deserializes using runner-protocol v8
  -> calls sqlite_engine.commit(request)
  -> serializes SqliteCommitResponse
  -> sends over WebSocket
```

In tests, the test calls `sqlite_engine.commit(request)` directly. Same function, same types, different `SqliteStore` implementation. The Envoy protocol adds serialization/framing but zero logic.

The `SqliteProtocol` trait methods map 1:1 to functions on `SqliteEngine`:

| Protocol op | Engine method |
|---|---|
| `sqlite_get_pages` | `engine.get_pages()` |
| `sqlite_commit` | `engine.commit()` |
| `sqlite_commit_stage` | `engine.commit_stage()` |
| `sqlite_commit_finalize` | `engine.commit_finalize()` |

`takeover` and `preload` are not protocol ops. They are handled internally by pegboard-envoy before the actor starts, and the results are passed to the actor via the start message.

The envoy-protocol schema defines the wire types. `SqliteEngine` uses its own internal types. The pegboard-envoy glue converts between them. Tests bypass the conversion entirely.

---

## F. Metrics

All metrics use `rivet_metrics::{REGISTRY, BUCKETS, prometheus::*}` and `lazy_static!`, following the pattern in `engine/packages/pegboard/src/actor_kv/metrics.rs`.

### F.1 Engine-side metrics (`sqlite-storage/src/metrics.rs`)

```rust
lazy_static::lazy_static! {
    // Commit path
    pub static ref SQLITE_COMMIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        "sqlite_v2_commit_duration_seconds",
        "Duration of sqlite v2 commit operations.",
        &["path"],  // "fast" or "slow"
        BUCKETS.to_vec(),
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMMIT_PAGES: HistogramVec = register_histogram_vec_with_registry!(
        "sqlite_v2_commit_pages",
        "Number of dirty pages per commit.",
        &["path"],
        vec![1.0, 4.0, 16.0, 64.0, 256.0, 1024.0, 4096.0],
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMMIT_TOTAL: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_commit_total",
        "Total number of sqlite v2 commits.",
        *REGISTRY
    ).unwrap();

    // Read path
    pub static ref SQLITE_GET_PAGES_DURATION: Histogram = register_histogram_with_registry!(
        "sqlite_v2_get_pages_duration_seconds",
        "Duration of sqlite v2 get_pages operations.",
        BUCKETS.to_vec(),
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_GET_PAGES_COUNT: Histogram = register_histogram_with_registry!(
        "sqlite_v2_get_pages_count",
        "Number of pages requested per get_pages call.",
        vec![1.0, 4.0, 16.0, 64.0, 256.0],
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_PIDX_HIT_TOTAL: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_pidx_hit_total",
        "Pages served from delta via PIDX lookup.",
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_PIDX_MISS_TOTAL: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_pidx_miss_total",
        "Pages served from shard (no PIDX entry).",
        *REGISTRY
    ).unwrap();

    // Compaction
    pub static ref SQLITE_COMPACTION_PASS_DURATION: Histogram = register_histogram_with_registry!(
        "sqlite_v2_compaction_pass_duration_seconds",
        "Duration of a single compaction pass (one shard).",
        BUCKETS.to_vec(),
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMPACTION_PASS_TOTAL: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_compaction_pass_total",
        "Total compaction passes executed.",
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMPACTION_PAGES_FOLDED: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_compaction_pages_folded_total",
        "Total pages folded from deltas into shards.",
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMPACTION_DELTAS_DELETED: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_compaction_deltas_deleted_total",
        "Total delta entries fully consumed and deleted.",
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_DELTA_COUNT: IntGauge = register_int_gauge_with_registry!(
        "sqlite_v2_delta_count",
        "Current number of unfolded deltas across all actors.",
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_COMPACTION_LAG_SECONDS: Histogram = register_histogram_with_registry!(
        "sqlite_v2_compaction_lag_seconds",
        "Time between commit and compaction of that commit's deltas.",
        BUCKETS.to_vec(),
        *REGISTRY
    ).unwrap();

    // Takeover
    pub static ref SQLITE_TAKEOVER_DURATION: Histogram = register_histogram_with_registry!(
        "sqlite_v2_takeover_duration_seconds",
        "Duration of sqlite v2 takeover operations.",
        BUCKETS.to_vec(),
        *REGISTRY
    ).unwrap();

    pub static ref SQLITE_RECOVERY_ORPHANS_CLEANED: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_recovery_orphans_cleaned_total",
        "Total orphan deltas or stages cleaned during recovery.",
        *REGISTRY
    ).unwrap();

    // Fence
    pub static ref SQLITE_FENCE_MISMATCH_TOTAL: IntCounter = register_int_counter_with_registry!(
        "sqlite_v2_fence_mismatch_total",
        "Total fence mismatch errors returned.",
        *REGISTRY
    ).unwrap();
}
```

### F.2 Actor-side VFS metrics (existing `VfsMetrics` pattern extended)

The actor-side VFS already has `VfsMetrics` in `vfs.rs`. For v2, add:

- `cache_hit_total` / `cache_miss_total` -- page cache hit rate
- `prefetch_hit_total` / `prefetch_miss_total` -- pages from prefetch that were actually used
- `commit_count` -- total commits issued
- `commit_pages_total` -- total dirty pages committed
- `commit_duration_us` -- commit latency
- `read_duration_us` -- xRead latency (already exists, keep it)

---

## G. How the bench works

### G.1 Benchmark harness

Located at `engine/packages/sqlite-storage/benches/v1_v2_comparison.rs`, using criterion.

```rust
fn bench_insert_1mib(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("insert_1mib");

    // v2 with latency
    group.bench_function("v2_20ms_rtt", |b| {
        b.to_async(&rt).iter(|| async {
            let store = MemoryStore::new_with_latency();
            let engine = SqliteEngine::new(Arc::new(store));
            // takeover + commit 256 pages (1 MiB)
            run_insert_workload(&engine, "actor-1", 256).await;
        });
    });

    // v2 without latency
    group.bench_function("v2_0ms_rtt", |b| {
        b.to_async(&rt).iter(|| async {
            let store = MemoryStore::new_fast();
            let engine = SqliteEngine::new(Arc::new(store));
            run_insert_workload(&engine, "actor-1", 256).await;
        });
    });

    group.finish();
}
```

For the v1 comparison, the bench creates a `MemoryKv` (a new in-memory implementation of the existing `SqliteKv` trait, or reuses the existing v1 test infrastructure if one exists by then) and opens a real SQLite database through the v1 VFS, running the same SQL workload.

### G.2 Workloads

Each workload runs the same SQL statements against both implementations:

1. **insert_1mib** -- `INSERT` 256 rows of 4 KiB each into a single table
2. **insert_10mib** -- `INSERT` 2560 rows of 4 KiB each
3. **hot_row_update_100x** -- `UPDATE` the same 4 rows 100 times
4. **cold_read_100_pages** -- populate 100 pages, drop cache, `SELECT *`
5. **mixed_read_write** -- 80% reads, 20% writes, 1000 operations

### G.3 Output

The benchmark produces a comparison table:

```
Workload              v1 @ 20ms    v2 @ 20ms    Speedup    RTTs (v1)    RTTs (v2)
insert_1mib           5700 ms      60 ms        95x        287          3
insert_10mib          65000 ms     100 ms       650x       ~2000        5
hot_row_update_100x   4000 ms      2000 ms      2x         ~200         ~100
cold_read_100_pages   2000 ms      40 ms        50x        100          2
mixed_read_write      ...          ...          ...        ...          ...
```

RTT counts are derived from `store.op_count()` on the `MemoryStore`.

---

## Implementation checklist

Files to create, in dependency order:

1. `engine/packages/sqlite-storage/Cargo.toml` -- crate manifest. Add `sqlite-storage` to workspace `[members]` in root `Cargo.toml` and add workspace dependencies for `tokio`, `tracing`, `scc`, `lz4_flex`, `async-trait`, `anyhow`, `bytes`, `parking_lot`, `rand`, `criterion`.
2. `engine/packages/sqlite-storage/src/lib.rs` -- module root, public re-exports.
3. `engine/packages/sqlite-storage/src/types.rs` -- `DbHead`, `DirtyPage`, `FetchedPage`, generation/txid/pgno type aliases.
4. `engine/packages/sqlite-storage/src/keys.rs` -- key builders for META, SHARD, DELTA, DELTAREF, PIDX.
5. `engine/packages/sqlite-storage/src/store.rs` -- `SqliteStore` trait + `Mutation` struct.
6. `engine/packages/sqlite-storage/src/ltx.rs` -- LTX encode/decode (start with raw concatenation, add LZ4 after).
7. `engine/packages/sqlite-storage/src/page_index.rs` -- `DeltaPageIndex`.
8. `engine/packages/sqlite-storage/src/protocol.rs` -- `SqliteProtocol` trait, request/response enums.
9. `engine/packages/sqlite-storage/src/metrics.rs` -- all Prometheus metrics from section F.
10. `engine/packages/sqlite-storage/src/engine.rs` -- `SqliteEngine<S>` struct and constructor.
11. `engine/packages/sqlite-storage/src/takeover.rs` -- takeover + recovery handler.
12. `engine/packages/sqlite-storage/src/read.rs` -- get_pages handler.
13. `engine/packages/sqlite-storage/src/commit.rs` -- commit + commit_stage + commit_finalize handlers.
14. `engine/packages/sqlite-storage/src/preload.rs` -- preload handler.
15. `engine/packages/sqlite-storage/src/compaction/mod.rs` -- coordinator (mpsc channel + `HashMap<String, JoinHandle>`).
16. `engine/packages/sqlite-storage/src/compaction/worker.rs` -- compact_worker per-actor task.
17. `engine/packages/sqlite-storage/src/compaction/shard.rs` -- compact_shard single-pass logic.
18. `engine/packages/sqlite-storage/src/test_utils/mod.rs` -- test utility module root.
19. `engine/packages/sqlite-storage/src/test_utils/memory_store.rs` -- `MemoryStore` with latency, failure injection, op log.
20. `engine/packages/sqlite-storage/src/test_utils/helpers.rs` -- `test_page()`, `setup_engine()`, assertion helpers.
21. `engine/packages/sqlite-storage/tests/integration/mod.rs` -- integration test module.
22. `engine/packages/sqlite-storage/tests/integration/basic.rs` -- commit_and_read_back, multiple pages, overwrites, preload.
23. `engine/packages/sqlite-storage/tests/integration/fencing.rs` -- generation mismatch, txid mismatch, takeover sequences.
24. `engine/packages/sqlite-storage/tests/integration/slow_path.rs` -- commit_stage + commit_finalize tests.
25. `engine/packages/sqlite-storage/tests/compaction/mod.rs` -- compaction test module.
26. `engine/packages/sqlite-storage/tests/compaction/basic.rs` -- fold, latest-wins, multi-shard, refcount, idempotent.
27. `engine/packages/sqlite-storage/tests/compaction/recovery.rs` -- orphan cleanup, stage cleanup.
28. `engine/packages/sqlite-storage/tests/compaction/coordinator.rs` -- deduplication, worker lifecycle.
29. `engine/packages/sqlite-storage/tests/concurrency/mod.rs` -- concurrent commit/compact/read tests.
30. `engine/packages/sqlite-storage/tests/failure/mod.rs` -- store errors, partial writes, crash recovery.
31. `engine/packages/sqlite-storage/tests/latency/mod.rs` -- RTT-assumption validation tests.
32. `engine/packages/sqlite-storage/benches/v1_v2_comparison.rs` -- criterion benchmark harness.
33. `engine/packages/pegboard-envoy/src/sqlite_bridge.rs` -- `UdbStore` production implementation (created later, during integration).
