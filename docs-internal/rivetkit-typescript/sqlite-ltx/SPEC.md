# SQLite VFS v2 -- Canonical Specification

## 1. Overview

SQLite VFS v2 replaces the per-page KV storage layout (v1) with a sharded LTX + delta log architecture (Option D). SQLite runs inside the actor process (C1) with an in-memory page cache for zero-RTT warm reads. Writes land as small LTX delta blobs in a single round trip. Background engine-side compaction folds deltas into immutable shards. The actor-side VFS speaks a semantic `sqlite_*` protocol; it knows nothing about shards, deltas, or compaction.

Three layers: (1) actor-side VFS with write buffer, LRU page cache, and prefetch predictor; (2) runner-protocol v8 carrying six `sqlite_*` ops over WebSocket; (3) engine-side `sqlite-storage` crate owning storage layout, CAS-fenced commits, PIDX cache, and compaction.

The design is constrained by: SQLite in the actor process (C1), writes as primary optimization target (C2), cold reads pay RTTs (C3), no local disk (C4), single writer with fencing (C5), ~20 ms RTT (C6), schema-version dispatch (C7), and breaking API compatibility acceptable (C8).


## 2. Constraints

- **C1 -- Zero-RTT warm reads.** SQLite and its page cache run in the actor process. Warm reads hit RAM, not the network.
- **C2 -- Writes are the primary optimization target.** The VFS is designed first for write speed: atomic-commit envelopes, sharded storage, compression, delta log.
- **C3 -- Cold reads pay round trips.** Cache misses require an engine fetch. Sharding, prefetch, and preload mitigate but do not eliminate cold-read latency.
- **C4 -- No local disk.** All durable state lives in the actor's UDB subspace. Page caches and write buffers are ephemeral.
- **C5 -- Single writer with fencing.** The engine's runner-id check has a brief failover window. Generation-token CAS on every op defends against concurrent writers.
- **C6 -- ~20 ms RTT typical.** Every architectural decision that saves a round trip pays back proportionally.
- **C7 -- Schema-version dispatch.** v1 and v2 actors are routed by a dispatch system that probes the actor's UDB subspace prefix byte. This mechanism must be built (see section 8).
- **C8 -- Breaking API compatibility acceptable.** v1 stays v1. v2 is a new world. No migration, no v1 trait preservation.


## 3. Storage Layout

All keys live under the actor's UDB subspace with prefix byte `0x02`.

### 3.1 Key format

```
0x02/META                       -> DBHead (BARE-encoded, ~80 bytes)
0x02/SHARD/<shard_id_be32>      -> LZ4-compressed LTX blob for pages [shard_id*64 .. (shard_id+1)*64)
0x02/DELTA/<txid_be64>          -> LZ4-compressed LTX blob for pages dirtied by one committed tx
0x02/PIDX/delta/<pgno_be32>     -> txid_be64 (sparse: only pages in unmaterialized deltas)
0x02/STAGE/<stage_id_be64>/<chunk_idx_be16> -> raw staged chunk (slow-path only, temporary)
```

`shard_id = pgno / 64` -- computational, no lookup needed. PIDX entries are keyed by pgno because hot-path operations (reads, compaction) query by pgno, not by txid.

### 3.2 DBHead (META)

```rust
struct DBHead {
    schema_version:    u32,    // always 2
    generation:        u64,    // bumped on every takeover
    head_txid:         u64,    // last committed txid
    next_txid:         u64,    // monotonic counter, never reused
    materialized_txid: u64,    // largest txid fully compacted into SHARDs
    db_size_pages:     u32,    // SQLite "Commit" field
    page_size:         u32,    // 4096, immutable after creation
    shard_size:        u32,    // 64, immutable after creation
    creation_ts_ms:    i64,
}
```

Initial values for a new actor: `schema_version=2, generation=1, head_txid=0, next_txid=1, materialized_txid=0, db_size_pages=0, page_size=4096, shard_size=64, creation_ts_ms=now`.

### 3.3 LTX format

Both SHARD and DELTA values use LTX V3 framing with LZ4 block-compressed page bodies. The `litetx` Rust crate (v0.1.0) is **V1-only and cannot read/write V3 files** (page headers changed 4→6 bytes, LZ4 block format replaced frame format, varint page index added in V3). A custom V3 encoder/decoder must be written in-house: ~400-500 lines of Rust using `lz4_flex` for block compression. Rolling LTX checksums are dropped (set to zero, which V3 explicitly allows). UDB + SQLite provide byte fidelity.

### 3.4 Shard size

`S = 64` pages (~256 KiB raw, ~128 KiB compressed). Immutable after first run (persisted in META). Tunable empirically before launch.

### 3.5 FDB hard caps

UDB enforces FoundationDB-equivalent limits. Every storage operation must stay within these:

| Limit | FDB value | Impact on v2 |
|---|---|---|
| Value size | 100 KB | SHARDs (~128 KiB) and DELTAs (up to MiBs) exceed this. `UdbSqliteStore` must chunk values into 10 KB pieces (same `VALUE_CHUNK_SIZE = 10,000` pattern as existing `actor_kv`). |
| Key size | 10 KB | Our keys are < 50 bytes. No issue. |
| Transaction size | 10 MB | Fast-path commits with chunking overhead must stay under 10 MB. `MAX_DELTA_BYTES` is set to **8 MiB** (not 9) to leave headroom for chunking key overhead + PIDX + META within one tx. |
| Transaction time | 5 seconds | Compaction passes are ~5 ms. Commits are bounded by `MAX_DELTA_BYTES`. No issue. |

Value chunking is handled entirely inside `UdbSqliteStore`. The `SqliteStore` trait is chunk-unaware. The `SqliteEngine` writes and reads arbitrary-sized values; the production store impl splits them into 10 KB FDB-compatible pieces internally, same as `actor_kv/mod.rs:26-341`.

Per-SHARD chunk count: a 128 KiB compressed SHARD = ~14 internal FDB key-value pairs + 1 metadata entry = ~15 FDB operations per SHARD read/write. This is transparent to `SqliteEngine` but means "one `store.get(SHARD/K)`" fans out to ~15 FDB key reads under the hood. Still < 1 ms at FDB speeds.

### 3.6 Storage quota

SQLite v2 data has its own storage limit, **separate from the actor's general KV quota**. This prevents a large SQLite database from crowding out `c.kv.*` state (or vice versa).

- `sqlite_max_storage`: configurable per-actor, default 10 GiB. Tracked independently from the general KV `MAX_STORAGE_SIZE`.
- The `sqlite_commit` handler checks `sqlite_storage_used` before writing. If the commit would exceed the quota, it returns an error.
- `sqlite_storage_used` includes SHARDs + DELTAs + PIDX + META. Compaction does not change the quota usage significantly (it replaces DELTA bytes with SHARD bytes, roughly neutral).
- The quota is tracked in META or as a separate engine-side counter (implementation detail of `UdbSqliteStore`).


## 4. Envoy-Protocol Ops

Four ops added to the envoy-protocol schema. All carry fencing fields `(generation, expected_head_txid)`. Pages are sent **uncompressed** over the wire; the engine compresses/decompresses when talking to UDB.

`sqlite_takeover` and `sqlite_preload` are **NOT protocol ops**. They are handled automatically by pegboard-envoy as part of the actor lifecycle, before the actor process starts. Takeover (generation bump) and preload (warm page fetch) run engine-local against UDB (0 RTT). The results are included in the actor start message via the envoy protocol. The actor's VFS receives preloaded pages as initialization data — no additional round trips for cold start.

### 4.1 Common types

```bare
type SqliteGeneration u64
type SqliteTxid       u64
type SqlitePgno       u32
type SqliteStageId    u64

type SqlitePageBytes  data   # raw 4 KiB page, uncompressed on wire

type SqliteMeta struct {
    schema_version:    u32
    generation:        SqliteGeneration
    head_txid:         SqliteTxid
    materialized_txid: SqliteTxid
    db_size_pages:     u32
    page_size:         u32
    creation_ts_ms:    i64
    max_delta_bytes:   u64   # tells the actor the fast-path size threshold
}

type SqliteFenceMismatch struct {
    actual_meta: SqliteMeta
    reason:      str
}

type SqliteDirtyPage struct {
    pgno:  SqlitePgno
    bytes: SqlitePageBytes
}

type SqliteFetchedPage struct {
    pgno:  SqlitePgno
    bytes: optional<SqlitePageBytes>   # absent if pgno > db_size_pages
}

type SqlitePgnoRange struct {
    start: SqlitePgno
    end:   SqlitePgno   # exclusive
}
```

### 4.2 sqlite_takeover (internal, not a protocol op)

Handled automatically by pegboard-envoy before the actor starts. Not callable by the actor.

Engine-internal semantics:
- Create META if absent (new actor). Otherwise bump `generation` to `current + 1`.
- Scan for orphan `DELTA/` entries with `txid > head_txid`, delete them and their PIDX entries. Scan for orphan `STAGE/` entries, delete them.
- Fetch preload pages (page 1 + configured hints, up to `max_total_bytes = 1 MiB`).
- Schedule a compaction pass if `delta_count >= 32`.
- Include the resulting `(generation, meta, preloaded_pages)` in the actor start message.

### 4.3 sqlite_preload (internal, not a protocol op)

Handled as part of takeover above. Preload hints come from the actor's config (specified at actor creation time). The preloaded pages are included in the actor start message. The actor's VFS populates its page cache from this data on initialization — 0 RTTs.

Default: always preload page 1 (SQLite schema page). User can add specific pgnos and pgno ranges. `max_total_bytes = 1 MiB`.

### 4.4 sqlite_get_pages

Hot read path. Returns the latest version of requested pages.

```bare
type SqliteGetPagesRequest struct {
    actor_id:   ActorId
    generation: SqliteGeneration
    pgnos:      list<SqlitePgno>
}

type SqliteGetPagesResponse union {
    SqliteGetPagesOk | SqliteFenceMismatch
}

type SqliteGetPagesOk struct {
    pages: list<SqliteFetchedPage>
    meta:  SqliteMeta
}
```

Engine semantics: for each pgno, check in-memory PIDX cache. If found, fetch from `DELTA/<txid>`. If not, fetch from `SHARD/<pgno/64>`. Batch all UDB reads into one operation. Decode LTX, extract requested pages, return uncompressed. Runs in one UDB snapshot for consistency.

Page 0 is invalid (SQLite uses 1-indexed page numbers). The engine omits it from the response or returns an error.

### 4.5 sqlite_commit (fast path)

Single-call commit when dirty buffer fits in one envelope.

```bare
type SqliteCommitRequest struct {
    actor_id:           ActorId
    generation:         SqliteGeneration
    expected_head_txid: SqliteTxid
    dirty_pages:        list<SqliteDirtyPage>
    new_db_size_pages:  u32
}

type SqliteCommitResponse union {
    SqliteCommitOk | SqliteFenceMismatch | SqliteCommitTooLarge
}

type SqliteCommitOk struct {
    new_head_txid: SqliteTxid
    meta:          SqliteMeta
}

type SqliteCommitTooLarge struct {
    actual_size_bytes: u64
    max_size_bytes:    u64
}
```

Engine semantics:
1. CAS-check `(generation, head_txid)` against META.
2. Encode dirty pages as one LTX delta (LZ4 internally).
3. If encoded size > `MAX_DELTA_BYTES`, return `SqliteCommitTooLarge`.
4. In one atomic UDB transaction: write `DELTA/<new_txid>`, write PIDX entries for each dirty pgno, update META (`head_txid = new_txid`, `next_txid = new_txid + 1`).
5. Update in-memory PIDX cache.
6. Send actor_id to the compaction coordinator channel (fire-and-forget).

The actor can pre-check whether to use the fast or slow path by comparing its raw dirty page count against `meta.max_delta_bytes / 4096`. This avoids wasting an RTT on `CommitTooLarge` in most cases.

### 4.6 sqlite_commit_stage (slow path, phase 1)

Streams chunks of dirty pages when the buffer exceeds the fast-path envelope.

```bare
type SqliteCommitStageRequest struct {
    actor_id:    ActorId
    generation:  SqliteGeneration
    stage_id:    SqliteStageId
    chunk_idx:   u16
    dirty_pages: list<SqliteDirtyPage>
    is_last:     bool
}

type SqliteCommitStageResponse union {
    SqliteCommitStageOk | SqliteFenceMismatch
}

type SqliteCommitStageOk struct {
    chunk_idx_committed: u16
}
```

Engine writes the chunk to `STAGE/<stage_id>/<chunk_idx>`. Stage entries are invisible to readers until `commit_finalize`.

The `stage_id` is a random u64 generated by the actor using a cryptographic RNG. Collision probability is ~1/2^64 and treated as a fatal error (actor restarts).

### 4.7 sqlite_commit_finalize (slow path, phase 2)

Atomically promotes all staged chunks into a real delta.

```bare
type SqliteCommitFinalizeRequest struct {
    actor_id:           ActorId
    generation:         SqliteGeneration
    expected_head_txid: SqliteTxid
    stage_id:           SqliteStageId
    new_db_size_pages:  u32
}

type SqliteCommitFinalizeResponse union {
    SqliteCommitFinalizeOk | SqliteFenceMismatch | SqliteStageNotFound
}

type SqliteCommitFinalizeOk struct {
    new_head_txid: SqliteTxid
    meta:          SqliteMeta
}

type SqliteStageNotFound struct {
    stage_id: SqliteStageId
}
```

Engine semantics: CAS-check, read all `STAGE/<stage_id>/*` entries, in one UDB transaction: assemble pages into a single `DELTA/<new_txid>`, write PIDX entries, delete `STAGE/<stage_id>/*`, update META.

### 4.8 Actor start message additions

The envoy-protocol actor start message is extended with SQLite startup data (for v2 actors only):

```bare
type SqliteStartupData struct {
    generation:      SqliteGeneration
    meta:            SqliteMeta
    preloaded_pages: list<SqliteFetchedPage>
}
```

This is populated by pegboard-envoy's internal takeover + preload (§4.2-4.3) before the actor starts. The actor's VFS reads this from the start message and populates its page cache. Zero additional RTTs.


## 5. Actor-Side VFS

### 5.1 File and trait

New file: `rivetkit-typescript/packages/sqlite-native/src/v2/vfs.rs` (scoped by module, not by name suffix).

```rust
#[async_trait]
pub trait SqliteProtocol: Send + Sync {
    async fn get_pages(&self, req: GetPagesRequest) -> Result<GetPagesResponse>;
    async fn commit(&self, req: CommitRequest) -> Result<CommitResponse>;
    async fn commit_stage(&self, req: CommitStageRequest) -> Result<CommitStageResponse>;
    async fn commit_finalize(&self, req: CommitFinalizeRequest) -> Result<CommitFinalizeResponse>;
}
```

Four ops. Takeover and preload are not protocol ops (handled by pegboard-envoy before the actor starts).

Two impls:
- `envoy::Protocol` -- production, over WebSocket via napi bindings on `EnvoyHandle`.
- `memory::Protocol` -- tests, wraps `SqliteEngine<MemoryStore>` in-process.

### 5.2 Per-connection state

```rust
pub struct VfsV2Context {
    actor_id:  String,
    runtime:   tokio::runtime::Handle,
    protocol:  Arc<dyn SqliteV2Protocol>,
    state:     parking_lot::RwLock<VfsV2State>,
}

struct VfsV2State {
    generation:    u64,
    head_txid:     u64,
    db_size_pages: u32,
    max_delta_bytes: u64,
    page_cache:    moka::sync::Cache<u32, Bytes>,  // default 50,000 pages
    write_buffer:  WriteBuffer,
    predictor:     PrefetchPredictor,
}

struct WriteBuffer {
    in_atomic_write: bool,
    saved_db_size:   u32,
    dirty:           BTreeMap<u32, Bytes>,
}
```

### 5.3 Cold start

**Zero additional RTTs.** Pegboard-envoy handles takeover + preload internally (engine-local, 0 RTT) before starting the actor. The actor start message includes `SqliteStartupData` with the generation, meta, and preloaded pages.

The VFS initializes from the startup data:
```rust
pub fn init(protocol: Arc<dyn SqliteProtocol>, startup: SqliteStartupData) -> Self {
    let mut cache = PageCache::new(config.cache_capacity);
    for page in startup.preloaded_pages {
        if let Some(bytes) = page.bytes {
            cache.insert(page.pgno, bytes);
        }
    }
    // generation and meta from startup data — no protocol calls needed
}
```

### 5.4 Three-layer read path (xRead)

1. **Write buffer** -- current open atomic-write window. Checked first as a safety net.
2. **Page cache** -- `moka::sync::Cache<u32, Bytes>`. LRU eviction, configurable capacity.
3. **Engine fetch** -- `sqlite_get_pages` with prefetch predictions. Populate cache from response.

The prefetch predictor (Markov + stride, ported from mvSQLite, Apache-2.0, attribution required) generates up to `prefetch_depth` (default 16) predicted pgnos per miss. Only pages not already in cache are included in the request. Max prefetch response size: `max_prefetch_bytes = 256 KiB`.

### 5.5 Write path (xWrite + atomic write window)

`xWrite` buffers pages into `write_buffer.dirty`. No engine communication.

- `BEGIN_ATOMIC_WRITE`: set `in_atomic_write = true`, save `db_size_pages`, clear dirty buffer.
- `COMMIT_ATOMIC_WRITE`:
  1. If raw dirty size <= `max_delta_bytes`: try fast-path `sqlite_commit`.
  2. If `CommitTooLarge`: fall back to slow path (`commit_stage` x N + `commit_finalize`).
  3. On success: update `head_txid`, promote dirty pages into page cache.
- `ROLLBACK_ATOMIC_WRITE`: clear dirty buffer, restore `db_size_pages`. Purely local, nothing sent to engine.

### 5.6 Writes outside atomic-write window

SQLite may write outside an atomic-write window during recovery replays, schema changes that overflow the pager cache, or journal-mode fallback. These writes are buffered in `dirty`. The next `xSync` call commits them as a **single delta containing all pending pages** (not one delta per page). `xSync` is a no-op only when the dirty buffer is empty; when there are pending non-atomic writes, it flushes them via `sqlite_commit`.

### 5.7 Other VFS callbacks

- `xLock / xUnlock / xCheckReservedLock`: no-ops. Single-writer enforced by fencing.
- `xFileSize`: returns `db_size_pages * PAGE_SIZE`.
- `xTruncate`: shrinks `db_size_pages`. Engine learns on next commit.
- `xSync`: flush pending non-atomic writes (see 5.6). No-op if dirty buffer is empty.
- `xDeviceCharacteristics`: returns `SQLITE_IOCAP_BATCH_ATOMIC`.
- `xSectorSize`: returns 4096.
- `xClose`: drops local state. No engine "close" op.
- `xOpen` for temp DB files: returns `SQLITE_IOERR` (VACUUM unsupported).

### 5.8 Pragmas

Same as v1: `journal_mode=DELETE, synchronous=NORMAL, page_size=4096, locking_mode=EXCLUSIVE, auto_vacuum=NONE, temp_store=MEMORY`.


## 6. Engine-Side Subsystem

### 6.1 Standalone crate

`engine/packages/sqlite-storage/` -- contains `SqliteStore` trait, `SqliteEngine<S>`, compaction, PIDX, LTX helpers, metrics. No dependency on pegboard-envoy, universaldb, nats, WebSocket, or envoy-protocol.

### 6.2 SqliteStore trait

Deliberately simple. No transaction closure, no generic bounds, no boxed futures. Four methods.

```rust
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

Object-safe. No `StoreTx` sub-trait. The CAS fencing (generation + head_txid checked via the META mutation inside `atomic_write`) handles read-then-write atomicity externally: callers read first, validate the CAS fields, then call `atomic_write` with all mutations including the META update. If someone else committed between the read and write, the CAS catches it.

Production impl (`UdbStore`): `atomic_write` wraps `db.run(|tx| async { for m in mutations { tx.set/tx.clear } })`. Handles FDB value chunking (§3.5) internally.
Test impl (`MemoryStore`): `atomic_write` locks a `BTreeMap`, applies mutations, unlocks. Configurable artificial latency + jitter for C6 simulation.

Production impl: `UdbSqliteStore` in `engine/packages/pegboard-envoy/src/sqlite_bridge.rs`.
Test impl: `MemorySqliteStore` in `engine/packages/sqlite-storage/src/test_utils/memory_store.rs`.

### 6.3 SqliteEngine

```rust
pub struct SqliteEngine<S: SqliteStore> {
    store: Arc<S>,
    page_indices: scc::HashMap<String, DeltaPageIndex>,
    compaction_tx: mpsc::UnboundedSender<String>,  // actor_id channel
    metrics: SqliteStorageMetrics,
}
```

Implements all six protocol ops. Owns the per-actor in-memory PIDX cache (`scc::HashMap<u32, u64>` -- pgno to txid, loaded lazily from `PIDX/delta/*` on first access via prefix scan).

### 6.4 Commit handler

Receives dirty pages, CAS-checks `(generation, head_txid)`, encodes as LTX delta, writes `DELTA/<new_txid>` + PIDX entries + META in one `SqliteStore::transact`. After successful commit, sends actor_id to compaction channel.

### 6.5 Page reader (sqlite_get_pages)

For each requested pgno:
1. Check in-memory PIDX cache (nanoseconds). If found: key is `DELTA/<txid>`.
2. If not: key is `SHARD/<pgno/64>`.
3. Batch all keys into one `batch_get`.
4. LTX-decode each blob, extract requested pages, return uncompressed.

One UDB read operation total.


## 7. Compaction

### 7.1 Coordinator

One long-lived tokio task per engine process.

```rust
struct CompactionCoordinator {
    rx: mpsc::UnboundedReceiver<String>,         // actor_id
    workers: HashMap<String, JoinHandle<()>>,    // actor_id -> running worker
}
```

No `DeltaStats` map, no `scc::HashSet<Id> in_flight`, no `antiox` (TypeScript-only).

Pseudocode for the coordinator loop:

```rust
loop {
    tokio::select! {
        Some(actor_id) = rx.recv() => {
            // Deduplicate: skip if a worker is already running.
            if let Entry::Vacant(e) = workers.entry(actor_id.clone()) {
                let handle = tokio::spawn(compact_worker(
                    store.clone(), actor_id.clone()
                ));
                e.insert(handle);
            }
        }
        // Reap completed workers.
        _ = reap_interval.tick() => {
            workers.retain(|_, handle| !handle.is_finished());
        }
    }
}
```

### 7.2 Worker

Per-actor tokio task, spawned on demand. Reads delta state from UDB (PIDX scan), decides whether to compact (delta count >= `N_count` threshold). Runs bounded compaction passes (up to `shards_per_batch` shards per invocation). Exits when caught up.

### 7.3 One compaction pass = one shard

For target `shard_id = K`:

1. CAS-check generation against META.
2. PIDX range scan for pgnos in `[K*64, (K+1)*64)`. Group by txid.
3. Read old SHARD + relevant DELTAs in one `batch_get`.
4. LTX decode all. Merge latest-txid-wins per pgno.
5. LTX encode merged shard.
6. Atomic `SqliteStore::transact`: write new SHARD, delete consumed PIDX entries, delete DELTAs whose pages are all consumed (refcount-checked via PIDX scan across all shards, or tracked per-delta with a page count), advance `materialized_txid`.

Cost per pass: ~5 ms wall-clock, ~700 us CPU, bounded byte transfer (~256 KiB shard + delta slices).

### 7.4 Delta lifecycle

A delta spanning multiple shards (e.g., 3 shards) is consumed across 3 passes. The delta is deleted only when no PIDX entries reference it. The compaction pass for shard K deletes only the PIDX entries for pgnos in `[K*64, (K+1)*64)`. After all shards have consumed their pages from a delta, no PIDX entries reference that txid, and a scan confirms deletion is safe.

### 7.5 Idle compaction

A periodic task (every 5 s) scans for actors with >= 8 lingering deltas and no recent commits, enqueues them to the coordinator.

### 7.6 Crash recovery

- Crash before `transact` commit: no-op, previous state intact.
- Crash after commit: consistent state, next pass continues from new META.
- Recovery on takeover: scan for `DELTA/` with txid > `head_txid` (orphans from failed commits), delete them. Scan for orphan `STAGE/` entries, delete them. Scan for PIDX entries referencing nonexistent deltas, delete them.
- All recovery operations are idempotent.


## 8. Schema-Version Dispatch

Must be built. The dispatch decision is made **in the actor process** (because VFS registration happens in the actor process, not the engine). The engine cannot reach in and tell the actor which VFS to register.

The actor knows its schema version from its **creation-time config**, not from probing UDB:

- When an actor is created, it is assigned either v1 or v2 based on the engine's current default (configurable via engine flag for gradual rollout).
- The version is part of the actor's metadata, communicated to the actor during the WebSocket handshake or as part of the actor startup payload.
- The actor branches at VFS registration time: v1 actors register `vfs.rs` + `SqliteKv` + `EnvoyKv`. v2 actors register `vfs_v2.rs` + `SqliteV2Protocol` + `EnvoyV2`.
- v1 actors use the general KV API (prefix `0x08` in UDB). v2 actors use the `sqlite_*` API (prefix `0x02` in UDB). The two never share keys.
- Existing v1 actors stay v1 forever. New actors after the flag flip are v2. No runtime probing, no migration.


## 9. Config Management

### Immutable after first run (persisted in META)

- `page_size` (4096)
- `shard_size` (64)

On subsequent startups, the engine reads these from META and refuses to start if the engine config specifies different values.

### Mutable at any time (read from engine config on each startup)

- `cache_capacity_pages` (default 50,000)
- `prefetch_depth` (default 16)
- `max_prefetch_bytes` (default 256 KiB)
- `max_pages_per_stage` (default 4,000)
- `N_count` compaction threshold (default 64)
- `B_soft` delta byte threshold (default 16 MiB)
- `B_hard` back-pressure threshold (default 200 MiB)
- `T_idle` idle timer (default 5 s)
- `shards_per_batch` fairness budget (default 8)
- Compaction worker pool size (default `max(2, num_cpus / 2)`)
- Preload hints


## 10. Failure Modes

| Failure | Behavior |
|---|---|
| Fence mismatch on any op | Actor marks itself dead, refuses all subsequent ops, exits. Rivet restarts clean. |
| Network error (engine unreachable) | Retry once with backoff. If still failing, return `SQLITE_IOERR`. |
| `CommitTooLarge` | Actor falls back to slow path (commit_stage + commit_finalize). |
| Crash mid-commit (before transact) | No-op. No partial state. |
| Crash mid-compaction | No-op. Next pass retries from unchanged META. |
| Orphan deltas after crash | Cleaned up on next `sqlite_takeover`. |
| Orphan stages after crash | Cleaned up on next `sqlite_takeover`. |
| Writes outside atomic window | Buffered and flushed on next `xSync` as a single-page delta. |
| `B_hard` back-pressure exceeded | Engine refuses new commits until compaction drains below threshold. Actor receives a retryable error. |
| Preload fails (non-fence) | Actor retries preload. Generation is already bumped, no need to re-takeover. |
| VACUUM attempted | Returns `SQLITE_IOERR`. Unsupported in v2. |


## 11. Logging and Metrics

### Tracing

All logging via `tracing` macros. Structured fields, lowercase messages per CLAUDE.md conventions. Never `println!` or `eprintln!`.

### Prometheus metrics

Engine-side (in `sqlite-storage/src/metrics.rs`):

| Metric | Type | Description |
|---|---|---|
| `sqlite_v2_commit_duration_seconds` | HistogramVec (label: path=fast/slow) | Commit latency |
| `sqlite_v2_commit_pages` | HistogramVec (label: path) | Dirty pages per commit |
| `sqlite_v2_commit_total` | IntCounter | Total commits |
| `sqlite_v2_get_pages_duration_seconds` | Histogram | get_pages latency |
| `sqlite_v2_get_pages_count` | Histogram | Pages per get_pages call |
| `sqlite_v2_pidx_hit_total` | IntCounter | Pages served from delta via PIDX |
| `sqlite_v2_pidx_miss_total` | IntCounter | Pages served from shard |
| `sqlite_v2_compaction_pass_duration_seconds` | Histogram | Single compaction pass latency |
| `sqlite_v2_compaction_pass_total` | IntCounter | Total compaction passes |
| `sqlite_v2_compaction_pages_folded_total` | IntCounter | Pages folded delta to shard |
| `sqlite_v2_compaction_deltas_deleted_total` | IntCounter | Fully consumed deltas deleted |
| `sqlite_v2_delta_count` | IntGauge | Current unfolded deltas |
| `sqlite_v2_compaction_lag_seconds` | Histogram | Time from commit to compaction |
| `sqlite_v2_takeover_duration_seconds` | Histogram | Takeover latency |
| `sqlite_v2_recovery_orphans_cleaned_total` | IntCounter | Orphans cleaned during recovery |
| `sqlite_v2_fence_mismatch_total` | IntCounter | Fence mismatch errors |

Actor-side VFS metrics (extending existing `VfsMetrics` pattern):

- `cache_hit_total` / `cache_miss_total`
- `prefetch_hit_total` / `prefetch_miss_total`
- `commit_count`, `commit_pages_total`, `commit_duration_us`
- `read_duration_us` (existing, kept)

Use `rivet_metrics` patterns from existing engine code (lazy_static, REGISTRY, BUCKETS).


## 12. Testing Architecture

### 12.1 Standalone crate

`engine/packages/sqlite-storage/` is testable without pegboard-envoy. Tests import it directly and provide `MemorySqliteStore`.

### 12.2 MemorySqliteStore

```rust
pub struct MemorySqliteStore {
    data: Arc<parking_lot::RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
    config: MemoryStoreConfig,
    op_log: Arc<parking_lot::Mutex<Vec<OpRecord>>>,
    op_count: AtomicU64,
}
```

Constructors:
- `new_fast()` -- zero latency, no failure injection.
- `new_with_latency()` -- 20 ms latency, 5 ms jitter (simulates C6).
- `new(config)` -- full configuration: `latency_ms`, `jitter_ms`, `fail_after_ops`, `simulate_partial_write`.

Features: operation log for assertions (`assert_ops_contain`, `assert_op_count`), snapshot/restore for crash simulation.

### 12.3 Test categories

- **Unit tests** (inline `#[cfg(test)]`): LTX encode/decode, key builders, page merge, shard_id computation, DbHead serialization.
- **Integration tests** (`tests/integration/`): full protocol round-trips through `SqliteEngine<MemorySqliteStore>`. Commit-and-read-back, multi-page, overwrites, preload, fencing, slow path.
- **Compaction tests** (`tests/compaction/`): delta folding, latest-wins, multi-shard delta consumption, idempotency, concurrent commit+compaction, fence mismatch abort, orphan cleanup.
- **Concurrency tests** (`tests/concurrency/`): concurrent commits to different actors, interleaved commit+compaction+read.
- **Failure injection tests** (`tests/failure/`): store errors mid-commit, partial writes, crash recovery via snapshot/restore.
- **Latency tests** (`tests/latency/`): with `new_with_latency()`, verify small commit is 1 RTT, get_pages is 1 RTT, takeover+preload is 2 RTTs.

### 12.4 Benchmark harness

`engine/packages/sqlite-storage/benches/v1_v2_comparison.rs` using Criterion. Workloads: insert 1 MiB, insert 10 MiB, hot-row update x100, cold read 100 pages, mixed read/write. Produces a comparison table with RTT counts derived from `store.op_count()`.


## 13. Out of Scope

- v1 to v2 migration. v1 stays v1 forever.
- Rolling LTX checksum maintenance.
- `journal_mode=MEMORY` or `synchronous=OFF`.
- VACUUM support.
- Engine-hosted SQLite (Model A).
- Any changes to the general KV API.
- Streaming ops for very large reads.
- General-purpose CAS op (fencing is baked into every op).


## 14. Tuning Parameters

| Parameter | Default | Immutable | Measurement plan |
|---|---|---|---|
| `shard_size` (S) | 64 pages | Yes (after creation) | Sweep {16, 32, 64, 128, 256}. Measure cold-read latency vs write throughput. |
| `page_size` | 4096 | Yes (after creation) | Keep at SQLite default unless benchmarks strongly motivate change. |
| `cache_capacity_pages` | 50,000 (~200 MiB) | No | Sweep {5k, 10k, 25k, 50k, 100k}. Measure cache hit rate vs memory pressure. |
| `prefetch_depth` | 16 | No | Sweep {4, 8, 16, 32, 64}. Measure prefetch hit rate and overfetch ratio. |
| `max_prefetch_bytes` | 256 KiB | No | Cap per get_pages response. Adjust if deserialization becomes a bottleneck. |
| `max_pages_per_stage` | 4,000 | No | Constrained by ~8 MiB raw envelope (10 MB FDB tx limit minus chunking overhead). Sweep {1k, 2k, 4k, 8k}. |
| `N_count` | 64 deltas | No | Sweep {16, 32, 64, 128, 256}. Trade compaction CPU vs cold-read penalty. |
| `B_soft` | 16 MiB | No | Measure storage amplification at different thresholds. |
| `B_hard` | 200 MiB | No | Sweep {50, 100, 200, 500} MiB. Measure write stall frequency. |
| `T_idle` | 5 s | No | Probably fine at 5 s. Lower = more CPU on lightly-loaded actors. |
| `shards_per_batch` | 8 | No | Sweep via load test. Trade per-actor speed vs fairness. |
| `worker_pool_size` | max(2, num_cpus / 2) | No | Measure compaction lag under sustained write pressure. |
| `preload max_total_bytes` | 1 MiB | No | Measure cold-start latency vs bandwidth waste. |
| `MAX_DELTA_BYTES` | ~8 MiB | No | Constrained by 10 MB FDB tx limit minus chunking overhead (~14 bytes/10KB chunk). Benchmark actual tx latency. |

Tuning plan: ship with defaults, instrument all metrics from day 1, run `examples/sqlite-raw` benchmark (extended with v2 mode) at realistic RTT, sweep each parameter independently, set production defaults from results.


## 15. Implementation Checklist

Ordered by dependency. Create files in this order.

### Engine-side crate: `engine/packages/sqlite-storage/`

1. `Cargo.toml` -- crate manifest. Add to workspace `[members]` and `[workspace.dependencies]`.
2. `src/lib.rs` -- module root, public re-exports.
3. `src/types.rs` -- `DbHead`, `DirtyPage`, `FetchedPage`, type aliases.
4. `src/keys.rs` -- key builders for META, SHARD, DELTA, PIDX, STAGE.
5. `src/store.rs` -- `SqliteStore` and `StoreTx` traits.
6. `src/ltx.rs` -- LTX encode/decode (hand-written, ~200 lines).
7. `src/page_index.rs` -- `DeltaPageIndex` (`scc::HashMap<u32, u64>`).
8. `src/protocol.rs` -- `SqliteV2Protocol` trait, request/response types.
9. `src/metrics.rs` -- all Prometheus metrics.
10. `src/engine.rs` -- `SqliteEngine<S>` struct and constructor.
11. `src/takeover.rs` -- takeover + recovery handler.
12. `src/read.rs` -- get_pages handler.
13. `src/commit.rs` -- commit + commit_stage + commit_finalize handlers.
14. `src/preload.rs` -- preload handler.
15. `src/compaction/mod.rs` -- coordinator (mpsc channel + HashMap).
16. `src/compaction/worker.rs` -- compact_worker per-actor task.
17. `src/compaction/shard.rs` -- compact_shard single-pass logic.
18. `src/test_utils/mod.rs` -- test utility module root.
19. `src/test_utils/memory_store.rs` -- `MemorySqliteStore`.
20. `src/test_utils/helpers.rs` -- `test_page()`, `setup_engine()`, assertion helpers.

### Tests and benchmarks

21. `tests/integration/` -- basic, fencing, slow path tests.
22. `tests/compaction/` -- fold, latest-wins, multi-shard, recovery, coordinator.
23. `tests/concurrency/` -- concurrent commit/compact/read.
24. `tests/failure/` -- store errors, partial writes, crash recovery.
25. `tests/latency/` -- RTT assumption validation.
26. `benches/v1_v2_comparison.rs` -- Criterion benchmark harness.

### Envoy-protocol additions

27. Add `sqlite_*` request/response types to `engine/sdks/schemas/envoy-protocol/` (verify current schema version and bump accordingly).
28. Update envoy-protocol versioning and bridging as needed.

### Envoy-client glue (actor-side Rust)

29. `engine/sdks/rust/envoy-client/` -- add 6 new methods: `sqlite_takeover`, `sqlite_get_pages`, `sqlite_commit`, `sqlite_commit_stage`, `sqlite_commit_finalize`, `sqlite_preload`. These wrap the envoy-protocol serialization/deserialization.
30. `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` -- napi bindings exposing the 6 methods to the VFS.

### Pegboard-envoy integration

31. `engine/packages/pegboard-envoy/src/sqlite_bridge.rs` -- `UdbSqliteStore` impl wrapping universaldb.
32. New dispatch arms in `ws_to_tunnel_task.rs` for `sqlite_*` ops, routing to `SqliteEngine<UdbSqliteStore>`.
33. Spawn `CompactionCoordinator` task at envoy startup (alongside existing tunnel/ping tasks).

### Actor-side dispatch

34. Actor startup payload or WebSocket handshake carries the schema version (v1 or v2), set at actor creation time.
35. VFS registration in `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` branches on the version flag.

### Actor-side VFS

34. `rivetkit-typescript/packages/sqlite-native/src/vfs_v2.rs` -- new VFS implementation.
35. `EnvoyV2` impl in `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` -- napi bindings.
36. v1/v2 branch in actor startup code (VFS registration).
