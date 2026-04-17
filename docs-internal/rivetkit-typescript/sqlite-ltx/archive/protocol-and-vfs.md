# SQLite VFS v2 — Protocol and VFS Design

> **Read [`constraints.md`](./constraints.md) first.** This document derives from the C0–C8 constraint set. It describes the wire protocol between the actor and the engine, the actor-side VFS that consumes the protocol, and the engine-side compaction subsystem that maintains the storage layout. If a constraint changes, this design has to be re-evaluated.
>
> Companion documents: [`compaction-design.md`](./compaction-design.md), [`key-decisions.md`](./key-decisions.md).
>
> **Status (2026-04-15):** Draft. Sections 1–4 complete. Under review.

---

## 1. Overview

v2 is a complete fork of the SQLite-on-KV path. v1 actors keep using the existing general KV API (`kv_get`, `kv_put`, `kv_delete`, `kv_list`) with their per-page key layout. v2 actors use a **brand new, SQLite-specific runner-protocol op family** (`sqlite_*`) that talks to a **brand new engine-side subsystem** (no shared code with the existing `actor_kv` module).

Dispatch between the two happens at the engine schema-version flag (per C7). v1 actors and v2 actors never share keys: v1 uses prefix `0x08` inside the actor's UDB subspace, v2 uses a disjoint prefix (proposed `0x10`). The general KV namespace (used by `c.kv.*` actor state) is unchanged and remains available to v2 actors alongside the SQLite path.

**The architecture has three layers:**

```
┌─────────────────────────────────────────────────────────────────┐
│ Actor process                                                   │
│ ┌─────────────┐    ┌──────────────────────────────────────────┐ │
│ │   SQLite    │ ←→ │  vfs_v2.rs                                │ │
│ │   engine    │    │  - LRU page cache (~50k pages)            │ │
│ │             │    │  - Write buffer (current open tx)         │ │
│ │             │    │  - Prefetch predictor                     │ │
│ └─────────────┘    │  - Calls sqlite_* ops via SqliteV2Protocol│ │
│                    └──────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │ runner-protocol (new schema v8)
                              │ ~20 ms RTT (per C6)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Engine process                                                  │
│ ┌──────────────────────────────────────────────────────────────┐│
│ │  actor_sqlite/ subsystem (new, NOT actor_kv)                 ││
│ │  ┌──────────┐ ┌──────────┐ ┌────────────┐ ┌────────────┐    ││
│ │  │ commit.rs│ │ read.rs  │ │ compactor  │ │ takeover.rs│    ││
│ │  └──────────┘ └──────────┘ └────────────┘ └────────────┘    ││
│ │  - LTX encode/decode (litetx crate)                          ││
│ │  - Page index per actor (in-memory + persistent backing)     ││
│ │  - Background compaction scheduler                           ││
│ │  - Generation token CAS validation                           ││
│ └──────────────────────────────────────────────────────────────┘│
│                              │                                  │
│                              ▼                                  │
│ ┌──────────────────────────────────────────────────────────────┐│
│ │  UDB (postgres or rocksdb driver)                            ││
│ │  Actor subspace, prefix 0x10:                                ││
│ │    META, SHARD/<id>, DELTA/<txid>, PIDX/...                  ││
│ └──────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

The actor-side VFS knows nothing about shards, deltas, compaction, or the page index. It speaks a small, semantic API: *"give me these pages"*, *"commit these dirty pages"*. The engine-side subsystem owns the storage layout, the compaction logic, and the generation fencing.

---

## 2. The runner-protocol additions

A new runner-protocol schema version (proposed: **v8**) is bumped exclusively to add the SQLite-specific op family. Per the `CLAUDE.md` rule about not mutating published `*.bare` files, this is a fresh `engine/sdks/schemas/runner-protocol/v8.bare` that adds new types and request/response unions while leaving v7 fully intact.

### 2.1 Common types

```bare
# v8.bare additions

type SqliteGeneration u64
type SqliteTxid       u64
type SqlitePgno       u32
type SqliteShardId    u32
type SqliteStageId    u64                   # client-allocated, opaque

# Opaque page bytes — uncompressed when sent over the wire.
# The engine compresses on the way to UDB; the actor-side VFS sees raw pages.
type SqlitePageBytes  data

# Carried in every response so the actor can detect external state changes.
type SqliteMeta struct {
    schema_version:    u32                   # always 2 for v2
    generation:        SqliteGeneration
    head_txid:         SqliteTxid
    materialized_txid: SqliteTxid            # advanced by compaction
    db_size_pages:     u32
    page_size:         u32                   # 4096
    creation_ts_ms:    i64
}

# Standard fence-mismatch shape returned by every op when CAS fails.
type SqliteFenceMismatch struct {
    actual_meta: SqliteMeta
    reason:      str                          # human-readable, for logs
}
```

### 2.2 Op surface

There are six ops total. Four are on the hot path (`takeover`, `get_pages`, `commit`, `preload`). Two are slow-path companions for commits that exceed the single-op envelope (`commit_stage`, `commit_finalize`). All six carry a `generation` field for fencing.

#### Op 1 — `sqlite_takeover`

Called once on actor cold start. Bumps the generation, fences out any previous actor process, and returns the current state. This is the equivalent of "claim the lease" in a distributed system.

```bare
type SqliteTakeoverRequest struct {
    actor_id:            ActorId
    expected_generation: SqliteGeneration   # 0 for first claim ever
}

type SqliteTakeoverResponse union {
    SqliteTakeoverOk
    | SqliteFenceMismatch
}

type SqliteTakeoverOk struct {
    new_generation: SqliteGeneration         # actor uses this in all subsequent ops
    meta:           SqliteMeta
}
```

If `expected_generation` is 0 (first-ever claim) the engine creates the initial META and DBHead and returns `new_generation = 1`. If `expected_generation` matches the current generation, the engine bumps it by 1 and returns the new value. If `expected_generation` is non-zero and doesn't match, the takeover fails — the new actor must read the current generation (via a fresh takeover with `expected_generation = 0`) and try again.

The takeover op is also the engine's signal to clean up orphan deltas from the previous actor's failed commits. See section 4 (compaction) for details.

#### Op 2 — `sqlite_get_pages`

The hot read path. Fetches the latest version of one or more pages. The engine internally checks the page index, fetches from delta or shard as appropriate, and returns the bytes.

```bare
type SqliteGetPagesRequest struct {
    actor_id:    ActorId
    generation:  SqliteGeneration
    pgnos:       list<SqlitePgno>            # batched: target page + prefetch hints
}

type SqliteGetPagesResponse union {
    SqliteGetPagesOk
    | SqliteFenceMismatch
}

type SqliteGetPagesOk struct {
    pages: list<SqliteFetchedPage>           # parallel with request order
    meta:  SqliteMeta                        # for staleness checks
}

type SqliteFetchedPage struct {
    pgno:   SqlitePgno
    bytes:  optional<SqlitePageBytes>        # absent if pgno > db_size_pages (zero-fill)
}
```

The engine handler runs in one UDB transaction so the response is a self-consistent snapshot. The actor populates its LRU cache from the response and serves subsequent reads from cache until eviction.

#### Op 3 — `sqlite_commit` (fast path)

The single-call commit. Used when the entire dirty buffer fits in one envelope (< ~9 MiB compressed LTX after framing). This is the dominant case for typical OLTP workloads.

```bare
type SqliteCommitRequest struct {
    actor_id:           ActorId
    generation:         SqliteGeneration
    expected_head_txid: SqliteTxid           # CAS check
    dirty_pages:        list<SqliteDirtyPage>
    new_db_size_pages:  u32                  # SQLite's "Commit" field
}

type SqliteDirtyPage struct {
    pgno:  SqlitePgno
    bytes: SqlitePageBytes
}

type SqliteCommitResponse union {
    SqliteCommitOk
    | SqliteFenceMismatch
    | SqliteCommitTooLarge
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

The engine handler:
1. CAS-checks `(generation, head_txid)` against META.
2. Encodes `dirty_pages` as one LTX delta frame (LZ4 internally).
3. Checks the resulting frame size against `MAX_DELTA_BYTES`. If too large, returns `SqliteCommitTooLarge` and the actor falls back to the slow path.
4. Writes `DELTA/<new_txid>` and the new META atomically in one UDB tx.
5. Updates the in-memory page index for the affected pgnos.
6. Optionally enqueues a compaction trigger if the delta count threshold is exceeded.
7. Returns `new_head_txid` and the updated META.

#### Op 4 — `sqlite_commit_stage` (slow path, phase 1)

Used when the dirty buffer exceeds the single-op envelope. The actor allocates a `stage_id` (a random u64) and streams chunks of dirty pages under that stage id. Each chunk is one UDB tx. The pages are not yet visible to readers because they're stored under a future txid.

```bare
type SqliteCommitStageRequest struct {
    actor_id:    ActorId
    generation:  SqliteGeneration
    stage_id:    SqliteStageId               # opaque to engine, scoped by actor
    chunk_idx:   u16                         # ordering within the stage
    dirty_pages: list<SqliteDirtyPage>
    is_last:     bool                        # set on the final chunk
}

type SqliteCommitStageResponse union {
    SqliteCommitStageOk
    | SqliteFenceMismatch
}

type SqliteCommitStageOk struct {
    chunk_idx_committed: u16
}
```

The engine writes the chunk to a temporary key like `STAGE/<stage_id>/<chunk_idx>` (under a separate prefix from `DELTA/`) and CAS-checks generation. Stage entries are invisible to readers until the matching `commit_finalize` lands.

If the actor crashes before `commit_finalize`, the stage entries become orphans and are cleaned up by the next compaction pass or by recovery on takeover.

#### Op 5 — `sqlite_commit_finalize` (slow path, phase 2)

Atomically promotes all the staged chunks for one `stage_id` into a real delta, advances `head_txid`, and returns the new META. This is the single small operation that flips the visibility bit.

```bare
type SqliteCommitFinalizeRequest struct {
    actor_id:           ActorId
    generation:         SqliteGeneration
    expected_head_txid: SqliteTxid
    stage_id:           SqliteStageId
    new_db_size_pages:  u32
}

type SqliteCommitFinalizeResponse union {
    SqliteCommitFinalizeOk
    | SqliteFenceMismatch
    | SqliteStageNotFound
}

type SqliteCommitFinalizeOk struct {
    new_head_txid: SqliteTxid
    meta:          SqliteMeta
}

type SqliteStageNotFound struct {
    stage_id: SqliteStageId
}
```

The engine:
1. CAS-checks `(generation, head_txid)`.
2. Reads all `STAGE/<stage_id>/*` entries.
3. In one UDB tx: rename them from `STAGE/<stage_id>/*` to `DELTA/<new_txid>/<chunk_idx>`, write the new META advancing `head_txid` to `new_txid`.
4. Updates the page index.
5. Returns `new_head_txid` and META.

The slow path is rarely exercised in practice — only when a single SQLite transaction dirties more pages than fit in one ~9 MiB compressed LTX frame, which is roughly 4,500–5,000 raw pages.

#### Op 6 — `sqlite_preload`

Cold-start optimization. Bundles "fetch META + a list of warm pages + a few page ranges" into one round trip. Used by the actor immediately after `sqlite_takeover` on cold boot.

```bare
type SqlitePreloadRequest struct {
    actor_id:        ActorId
    generation:      SqliteGeneration
    page_hints:      list<SqlitePgno>        # specific pages
    range_hints:     list<SqlitePgnoRange>   # contiguous ranges
    max_total_bytes: u64                     # safety bound for the response size
}

type SqlitePgnoRange struct {
    start: SqlitePgno
    end:   SqlitePgno                        # exclusive
}

type SqlitePreloadResponse union {
    SqlitePreloadOk
    | SqliteFenceMismatch
}

type SqlitePreloadOk struct {
    meta:  SqliteMeta
    pages: list<SqliteFetchedPage>
}
```

The engine handler runs in one UDB tx and uses the existing `actor_kv::preload::batch_preload` primitive (or its equivalent in the new subsystem) to fetch META + the page set in a single round trip.

### 2.3 Errors and fencing

Every response union includes `SqliteFenceMismatch` as a variant. The engine returns it whenever the CAS fails on `(generation, head_txid)`. Receiving a fence mismatch is the actor's signal that it is no longer the authoritative writer — the right response is to log the event, drop in-memory state, and exit (Rivet will restart it clean).

There is no retry on fence mismatch. The actor process is dead the moment the engine says its generation is stale. This is the same pattern as a leader losing its lease in any distributed system.

### 2.4 What the protocol does NOT include

These are intentionally absent:

- **No "raw KV" ops on the SQLite path.** The actor cannot send `kv_get(SHARD/0)` directly. All access goes through semantic ops. This is the boundary that lets the engine change the storage layout freely.
- **No streaming op for very large reads.** `sqlite_get_pages` returns one self-contained batch. If the actor needs more pages, it issues another op. We can revisit if a workload needs streaming.
- **No transaction-state RPCs.** Multi-statement SQL transactions still happen entirely inside the actor's local SQLite. The protocol doesn't model BEGIN/COMMIT/ROLLBACK because the VFS doesn't see them at the granularity SQLite uses internally — only the page-write boundary, which is what `sqlite_commit` represents.
- **No "give me a shard" op.** The shard layout is internal to the engine. The actor never sees it.
- **No general-purpose CAS op.** Every op has fencing baked in via `(generation, head_txid)` fields. We don't expose a generic CAS primitive.

---

## 3. The actor-side VFS

The actor-side VFS lives in a new file: `rivetkit-typescript/packages/sqlite-native/src/vfs_v2.rs`. It implements the SQLite VFS C ABI exactly like v1, but its callbacks delegate to a new trait `SqliteV2Protocol` instead of the existing `SqliteKv`. The two implementations coexist — v1 actors keep using `vfs.rs` + `SqliteKv` + `EnvoyKv`, v2 actors use `vfs_v2.rs` + `SqliteV2Protocol` + `EnvoyV2`.

### 3.1 Per-connection state

```rust
// vfs_v2.rs (sketch)

pub struct VfsV2Context {
    actor_id:  String,
    runtime:   tokio::runtime::Handle,
    protocol:  Arc<dyn SqliteV2Protocol>,

    state: parking_lot::RwLock<VfsV2State>,
}

struct VfsV2State {
    // Authoritative state mirrored from the engine.
    generation:    SqliteGeneration,
    head_txid:     SqliteTxid,
    db_size_pages: u32,

    // In-memory caches.
    page_cache:   PageCache,                  // LRU, default 50k pages = 200 MiB
    write_buffer: WriteBuffer,                // current open atomic-write window

    // Read-side optimization.
    predictor: PrefetchPredictor,             // mvSQLite-ported Markov+stride
    metrics:   VfsV2Metrics,
}

struct PageCache {
    inner:           moka::sync::Cache<SqlitePgno, Bytes>,
    capacity_pages:  usize,
}

struct WriteBuffer {
    in_atomic_write:  bool,
    saved_db_size:    u32,                    // for ROLLBACK_ATOMIC_WRITE
    dirty:            BTreeMap<SqlitePgno, Bytes>,
}

#[async_trait]
pub trait SqliteV2Protocol: Send + Sync {
    async fn takeover(&self, req: SqliteTakeoverRequest) -> Result<SqliteTakeoverResponse>;
    async fn get_pages(&self, req: SqliteGetPagesRequest) -> Result<SqliteGetPagesResponse>;
    async fn commit(&self, req: SqliteCommitRequest) -> Result<SqliteCommitResponse>;
    async fn commit_stage(&self, req: SqliteCommitStageRequest) -> Result<SqliteCommitStageResponse>;
    async fn commit_finalize(&self, req: SqliteCommitFinalizeRequest) -> Result<SqliteCommitFinalizeResponse>;
    async fn preload(&self, req: SqlitePreloadRequest) -> Result<SqlitePreloadResponse>;
}
```

Concrete impls:
- `EnvoyV2` in `rivetkit-typescript/packages/rivetkit-napi/src/database.rs` — production impl that delegates to napi methods on `EnvoyHandle`, which in turn talks to the engine over WebSocket.
- `MemoryV2` in `rivetkit-typescript/packages/sqlite-native/src/memory_v2.rs` (or the test crate) — in-process implementation that runs the entire engine subsystem against an in-memory backing store, for unit tests.

The two share no code with the v1 trait `SqliteKv`. Migration to v2 is by-construction since dispatch happens at the engine schema-version flag at registration time.

### 3.2 Initialization

When the SQLite connection opens, the VFS is registered and immediately runs:

```rust
pub fn open_v2(actor_id: String, protocol: Arc<dyn SqliteV2Protocol>) -> Result<Self> {
    let runtime = tokio::runtime::Handle::current();

    // 1. Takeover: claim the actor's SQLite namespace, bump generation.
    let takeover = runtime.block_on(protocol.takeover(SqliteTakeoverRequest {
        actor_id: actor_id.clone(),
        expected_generation: 0,  // we don't know the current value yet
    }))?;
    let (generation, mut meta) = match takeover {
        SqliteTakeoverResponse::SqliteTakeoverOk(ok) => (ok.new_generation, ok.meta),
        SqliteTakeoverResponse::SqliteFenceMismatch(_) => {
            // Another actor process holds the lease — we lost the race.
            return Err(VfsError::FenceMismatchOnTakeover);
        }
    };

    // 2. Preload: fetch META + warm pages in one RTT.
    //    Hints come from the actor's startup config (e.g., "first 1000 pages").
    let preload = runtime.block_on(protocol.preload(SqlitePreloadRequest {
        actor_id: actor_id.clone(),
        generation,
        page_hints:      preload_hints.exact_pages,
        range_hints:     preload_hints.ranges,
        max_total_bytes: preload_hints.max_bytes,
    }))?;
    let preload_ok = match preload {
        SqlitePreloadResponse::SqlitePreloadOk(ok) => ok,
        SqlitePreloadResponse::SqliteFenceMismatch(_) => {
            return Err(VfsError::FenceMismatchOnPreload);
        }
    };
    meta = preload_ok.meta;

    // 3. Populate the page cache with the preloaded pages.
    let mut page_cache = PageCache::new(config.cache_capacity_pages);
    for page in preload_ok.pages {
        if let Some(bytes) = page.bytes {
            page_cache.insert(page.pgno, bytes);
        }
    }

    Ok(Self {
        actor_id,
        runtime,
        protocol,
        state: parking_lot::RwLock::new(VfsV2State {
            generation,
            head_txid: meta.head_txid,
            db_size_pages: meta.db_size_pages,
            page_cache,
            write_buffer: WriteBuffer::default(),
            predictor: PrefetchPredictor::new(),
            metrics: VfsV2Metrics::default(),
        }),
    })
}
```

Total cost of cold start: **2 round trips** (takeover + preload). At 20 ms RTT that's 40 ms before the first SQL query can run — acceptable for actor cold-start.

### 3.3 Read path: `xRead`

```rust
unsafe extern "C" fn x_read_v2(
    p_file: *mut sqlite3_file,
    buf: *mut c_void,
    n: c_int,
    offset: sqlite3_int64,
) -> c_int {
    vfs_catch_unwind!(SQLITE_IOERR, {
        let file = get_file(p_file);
        let ctx = &*file.ctx;
        let pgno = (offset / PAGE_SIZE as i64) as SqlitePgno;

        // Layer 1: write buffer (current open atomic-write window).
        // SQLite's pager usually intercepts this before reaching the VFS,
        // but we keep the check as a safety net.
        {
            let state = ctx.state.read();
            if let Some(bytes) = state.write_buffer.dirty.get(&pgno) {
                copy_bytes_to_buf(buf, n, bytes);
                return SQLITE_OK;
            }
        }

        // Layer 2: page cache.
        {
            let state = ctx.state.read();
            if let Some(bytes) = state.page_cache.inner.get(&pgno) {
                copy_bytes_to_buf(buf, n, &bytes);
                state.metrics.read_cache_hit.fetch_add(1, Ordering::Relaxed);
                return SQLITE_OK;
            }
        }

        // Layer 3: ask the engine.
        // Build a batched fetch with prefetch predictions.
        let to_fetch = {
            let mut state = ctx.state.write();
            state.predictor.record(pgno);
            let predictions = state.predictor.multi_predict(pgno, PREFETCH_DEPTH);
            let mut v = Vec::with_capacity(1 + predictions.len());
            v.push(pgno);
            for p in predictions {
                if !state.page_cache.inner.contains_key(&p) {
                    v.push(p);
                }
            }
            v
        };

        let generation = ctx.state.read().generation;
        let response = ctx.runtime.block_on(ctx.protocol.get_pages(
            SqliteGetPagesRequest {
                actor_id: ctx.actor_id.clone(),
                generation,
                pgnos: to_fetch.clone(),
            },
        )).map_err(|_| SQLITE_IOERR)?;

        let pages = match response {
            SqliteGetPagesResponse::SqliteGetPagesOk(ok) => ok.pages,
            SqliteGetPagesResponse::SqliteFenceMismatch(_) => {
                // We've lost ownership. Refuse all further ops.
                ctx.mark_dead();
                return SQLITE_IOERR_FENCE_MISMATCH;
            }
        };

        // Populate cache and return the requested page.
        let mut state = ctx.state.write();
        let mut found_target: Option<Bytes> = None;
        for fetched in pages {
            match fetched.bytes {
                Some(bytes) => {
                    state.page_cache.inner.insert(fetched.pgno, bytes.clone());
                    if fetched.pgno == pgno {
                        found_target = Some(bytes);
                    }
                }
                None => {
                    // pgno > db_size_pages — return zero-filled page per SQLite semantics.
                    if fetched.pgno == pgno {
                        let zeros = Bytes::from(vec![0u8; n as usize]);
                        found_target = Some(zeros);
                    }
                }
            }
        }
        state.metrics.read_cache_miss.fetch_add(1, Ordering::Relaxed);
        drop(state);

        match found_target {
            Some(bytes) => {
                copy_bytes_to_buf(buf, n, &bytes);
                SQLITE_OK
            }
            None => SQLITE_IOERR_SHORT_READ,
        }
    })
}
```

Key properties:
- **Three lookup layers** (down from four in the original v2 sketch): write buffer, page cache, engine fetch. The `dirty_pgnos_in_log` map is gone — the engine handles delta/shard lookup transparently.
- **Each engine fetch is one round trip** that pulls the target page plus prefetch predictions in one batched response.
- **Fence mismatch is fatal**: the actor marks itself dead and refuses further ops. Rivet restarts it clean.
- **No knowledge of shards or deltas anywhere in the VFS code**.

### 3.4 Write path: `xWrite` and the atomic-write window

`xWrite` itself just buffers:

```rust
unsafe extern "C" fn x_write_v2(
    p_file: *mut sqlite3_file,
    buf: *const c_void,
    n: c_int,
    offset: sqlite3_int64,
) -> c_int {
    vfs_catch_unwind!(SQLITE_IOERR, {
        let file = get_file(p_file);
        let ctx = &*file.ctx;
        let pgno = (offset / PAGE_SIZE as i64) as SqlitePgno;
        let bytes = Bytes::copy_from_slice(slice_from_raw(buf, n as usize));

        let mut state = ctx.state.write();
        if !state.write_buffer.in_atomic_write {
            // Outside an atomic-write window. SQLite is doing direct page writes,
            // probably because it's mid-recovery or running outside our preferred
            // mode. We still buffer — the next sync will commit a single-page tx.
            // (This path is rare with IOCAP_BATCH_ATOMIC.)
        }
        state.write_buffer.dirty.insert(pgno, bytes);

        // Track the high-water mark for db_size_pages.
        let new_size = ((offset + n as i64) / PAGE_SIZE as i64) as u32;
        if new_size > state.db_size_pages {
            state.db_size_pages = new_size;
        }
        SQLITE_OK
    })
}
```

`BEGIN_ATOMIC_WRITE` opens the window:

```rust
SQLITE_FCNTL_BEGIN_ATOMIC_WRITE => {
    let mut state = ctx.state.write();
    state.write_buffer.in_atomic_write = true;
    state.write_buffer.saved_db_size = state.db_size_pages;
    state.write_buffer.dirty.clear();
    SQLITE_OK
}
```

`COMMIT_ATOMIC_WRITE` is where the work happens:

```rust
SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
    let (dirty, generation, head_txid, new_db_size) = {
        let mut state = ctx.state.write();
        let dirty = std::mem::take(&mut state.write_buffer.dirty);
        let new_db_size = state.db_size_pages;
        let generation = state.generation;
        let head_txid = state.head_txid;
        state.write_buffer.in_atomic_write = false;
        (dirty, generation, head_txid, new_db_size)
    };

    let dirty_pages: Vec<SqliteDirtyPage> = dirty.iter()
        .map(|(pgno, bytes)| SqliteDirtyPage {
            pgno: *pgno,
            bytes: bytes.clone(),
        })
        .collect();

    // Try the fast path first.
    let fast_response = ctx.runtime.block_on(ctx.protocol.commit(
        SqliteCommitRequest {
            actor_id:           ctx.actor_id.clone(),
            generation,
            expected_head_txid: head_txid,
            dirty_pages:        dirty_pages.clone(),
            new_db_size_pages:  new_db_size,
        },
    )).map_err(|_| SQLITE_IOERR)?;

    let new_head_txid = match fast_response {
        SqliteCommitResponse::SqliteCommitOk(ok) => ok.new_head_txid,

        SqliteCommitResponse::SqliteCommitTooLarge(_) => {
            // Fall through to slow path.
            let stage_id = generate_stage_id();
            let chunks = split_into_chunks(&dirty_pages, MAX_PAGES_PER_STAGE);
            for (idx, chunk) in chunks.iter().enumerate() {
                let response = ctx.runtime.block_on(ctx.protocol.commit_stage(
                    SqliteCommitStageRequest {
                        actor_id:    ctx.actor_id.clone(),
                        generation,
                        stage_id,
                        chunk_idx:   idx as u16,
                        dirty_pages: chunk.to_vec(),
                        is_last:     idx == chunks.len() - 1,
                    },
                )).map_err(|_| SQLITE_IOERR)?;
                if let SqliteCommitStageResponse::SqliteFenceMismatch(_) = response {
                    ctx.mark_dead();
                    return SQLITE_IOERR_FENCE_MISMATCH;
                }
            }
            let finalize = ctx.runtime.block_on(ctx.protocol.commit_finalize(
                SqliteCommitFinalizeRequest {
                    actor_id:           ctx.actor_id.clone(),
                    generation,
                    expected_head_txid: head_txid,
                    stage_id,
                    new_db_size_pages:  new_db_size,
                },
            )).map_err(|_| SQLITE_IOERR)?;
            match finalize {
                SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) => ok.new_head_txid,
                SqliteCommitFinalizeResponse::SqliteFenceMismatch(_) => {
                    ctx.mark_dead();
                    return SQLITE_IOERR_FENCE_MISMATCH;
                }
                SqliteCommitFinalizeResponse::SqliteStageNotFound(_) => {
                    return SQLITE_IOERR;
                }
            }
        }

        SqliteCommitResponse::SqliteFenceMismatch(_) => {
            ctx.mark_dead();
            return SQLITE_IOERR_FENCE_MISMATCH;
        }
    };

    // Update local state.
    let mut state = ctx.state.write();
    state.head_txid = new_head_txid;
    // Promote dirty pages directly into the cache so subsequent reads are 0 RTT.
    for (pgno, bytes) in dirty {
        state.page_cache.inner.insert(pgno, bytes);
    }
    state.metrics.commit_count.fetch_add(1, Ordering::Relaxed);
    SQLITE_OK
}
```

`ROLLBACK_ATOMIC_WRITE` is the simplest:

```rust
SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
    let mut state = ctx.state.write();
    state.write_buffer.dirty.clear();
    state.write_buffer.in_atomic_write = false;
    state.db_size_pages = state.write_buffer.saved_db_size;
    SQLITE_OK
}
```

Note that **ROLLBACK is purely a local operation** — nothing has been sent to the engine yet because writes only happen at COMMIT. This eliminates an entire class of race conditions present in earlier designs where rollback had to coordinate with a partial commit on the engine side.

### 3.5 Other VFS callbacks

- **`xLock` / `xUnlock` / `xCheckReservedLock`**: no-ops, same as v1. Single-writer is enforced by the engine via fencing tokens.
- **`xFileSize`**: reads `state.db_size_pages * PAGE_SIZE`. No KV access needed.
- **`xTruncate`**: shrinks `state.db_size_pages`. Engine learns about the new size on the next commit (it's part of `SqliteCommitRequest`).
- **`xSync`**: no-op (the commit path already handles durability via the engine).
- **`xDeviceCharacteristics`**: returns `SQLITE_IOCAP_BATCH_ATOMIC`, same as v1. This is what gets SQLite to use the atomic-write window in the first place.
- **`xSectorSize`**: returns 4096.
- **`xClose`**: drops the local state. The engine doesn't care — there's no "close session" op because the generation token is the only thing that matters.

### 3.6 Configuration knobs

The actor declares VFS configuration at registration:

```rust
pub struct VfsV2Config {
    pub cache_capacity_pages: usize,           // default 50_000 (200 MiB)
    pub prefetch_depth:       usize,           // default 16
    pub max_pages_per_stage:  usize,           // default 4_000 (slow-path chunk size)
    pub preload_hints:        PreloadHints,
}

pub struct PreloadHints {
    pub exact_pages: Vec<SqlitePgno>,
    pub ranges:      Vec<SqlitePgnoRange>,
    pub max_bytes:   u64,
}
```

Defaults are tuned for typical interactive actors. Analytical actors (the workload analyses suggest) want higher cache capacity and higher prefetch depth.

### 3.7 Failure handling

| Failure | VFS response |
|---|---|
| Fence mismatch on any op | Mark actor dead, refuse all subsequent ops with `SQLITE_IOERR_FENCE_MISMATCH`, exit on next user query. Rivet restarts. |
| Network error (engine unreachable) | Retry the op once with backoff. If still failing, surface `SQLITE_IOERR`. SQLite's normal error handling kicks in. |
| Commit too large (slow-path threshold exceeded mid-stage) | Should not happen — the actor sizes the chunks ahead of time. If it does, it's a bug. |
| Engine returns malformed response | `SQLITE_IOERR_CORRUPT_FS`, log and fail the actor. |
| Page cache exhaustion | Normal LRU eviction, no special handling. |
| Predictor produces invalid pgnos | Filter and ignore — predictor is best-effort. |

The actor never tries to recover from a fence mismatch in-process. The semantics is "your generation is dead, your view of the world is potentially stale, the only safe action is to die and let a fresh process start over."

---

## 4. Engine-side compaction subsystem

> **Full design:** [`compaction-design.md`](./compaction-design.md). Section below is the summary; consult the linked doc for storage layout details, the full pseudocode of a compaction pass, the page-index implementation, scheduler internals, recovery semantics, and the open-questions list.

The compaction subsystem is the engine-side counterpart to the actor-side VFS. It owns the storage layout (shards + deltas + a sparse page index), folds delta entries into shards in the background, and never touches a network in its hot loop. Its design is byte-level only — no SQLite linking, no SQL parsing, no page-format awareness. Pages are 4 KiB opaque blobs merged by latest-txid-wins.

### 4.1 Storage layout

All keys live under the actor's UDB subspace, prefixed with the v2 schema byte `0x02`:

```
v2/META                      → DBHead { generation, head_txid, materialized_txid,
                                        db_size_pages, next_txid, ... }
v2/SHARD/<shard_id_be32>     → LZ4-compressed LTX blob holding pages
                                [shard_id*64 .. (shard_id+1)*64)
v2/DELTA/<txid_be64>         → LZ4-compressed LTX blob holding pages dirtied by
                                one committed transaction
v2/DELTAREF/<txid_be64>      → i64 remaining-unfolded-pages refcount
v2/PIDX/delta/<pgno_be32>    → txid_be64 — sparse "freshest copy of pgno is in
                                DELTA/<this txid>" index
```

`shard_id = pgno / 64` is computational; no key needed for shard discovery. Working default `S = 64` pages per shard (~256 KiB raw, ~128 KiB compressed) — tunable.

### 4.2 Trigger policy

A pass fires for an actor when any of the following becomes true:

1. **Delta count threshold** — `N_count = 64` unfolded deltas (bounds the page-index scan size).
2. **Delta byte threshold** — `B_soft = 16 MiB` aggregate compressed delta bytes.
3. **Idle timer** — ≥ 8 deltas present and no writes for `T_idle = 5 s`.
4. **Hard back-pressure** — aggregate > `B_hard = 200 MiB`. Engine refuses new commits until drained. Last-resort safety valve.
5. **Startup recovery** — ≥ 32 deltas present at takeover triggers an immediate pass.

All thresholds are per-actor configurable. The trigger path is event-driven, not polling: every `sqlite_commit` handler updates a per-actor `DeltaStats` (`scc::HashMap<ActorId, Arc<DeltaStats>>`, ~100 ns per commit) and pushes to a scheduler queue when a threshold trips. Idle triggers come from a once-per-second background scan. Polling is never used and there is zero wasted work for idle actors.

The commit path **never blocks on compaction**. Compaction runs after the commit's UDB tx returns success. Only the hard-back-pressure rule blocks writers, and only when the 200 MiB cap is blown.

### 4.3 The page index

The engine has to answer "for page P, what's the latest version — in a delta or in shard `P/64`?" without scanning every delta. The strategy is a **persistent sparse index with an in-memory cache**:

- **Persistent form**: `v2/PIDX/delta/<pgno_be32> → txid_be64` is the source of truth. One key per *currently-unfolded* page (sparse — fully-materialized pages have no PIDX entry).
- **In-memory form**: `scc::HashMap<Pgno, Txid>` per actor, lazy-loaded on the first `sqlite_get_pages` call after takeover via a single `kv_list` prefix scan over `v2/PIDX/delta/`. Mirrors the persistent state.
- **Updates**: every commit and every compaction pass updates both the persistent and the in-memory copies inside the same UDB tx that writes the delta or shard.
- **Sparse cost**: a typical actor has tens to low hundreds of unfolded pages, so the in-memory map is ~1–10 KiB per actor. Across 10,000 actors per host that's ~100 MiB — affordable.
- **Restart**: the persistent state is canonical. On engine restart, the in-memory cache is rebuilt from the persistent form on first access — no recovery dance needed.

A read for page P:
1. Check `PIDX/delta/<P>` (in-memory cache). If present, fetch `DELTA/<txid>`, LTX-decode, extract P.
2. Else fetch `SHARD/<P/64>`, LTX-decode, extract P.
Both paths are one UDB-internal fetch — no actor-visible RTT difference.

### 4.4 The compaction step

**Unit of work: one shard per pass.** A delta spanning 80 shards becomes 80 bounded passes, not one mega-tx. This keeps each tx well under the 5 s UDB timeout and provides natural fairness checkpointing.

End-to-end pseudocode of a pass for `shard_id = K`:

```rust
db.run(|tx| async move {
    // 1. CAS-check the actor is still ours.
    let head = read_meta(&tx, actor_id).await?;
    if head.generation != expected_generation { return Err(FenceMismatch); }

    // 2. Find delta txids that touch any pgno in this shard's range.
    //    Use the in-memory PIDX cache filtered by [K*64, (K+1)*64).
    let touching_deltas: Vec<Txid> = pidx_cache
        .range(K*64 .. (K+1)*64)
        .map(|(_, txid)| *txid)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if touching_deltas.is_empty() { return Ok(()); }

    // 3. Read shard + relevant deltas in one batch_get.
    let shard_bytes = tx.get(shard_key(K)).await?;
    let delta_bytes = batch_get(&tx, touching_deltas.iter().map(delta_key)).await?;

    // 4. Decode all of it (litetx).
    let mut pages: HashMap<Pgno, (Txid, Bytes)> = decode_shard(shard_bytes, K);
    for (txid, blob) in delta_bytes {
        for (pgno, page_bytes) in decode_delta(blob) {
            // Latest-wins: only insert if newer.
            if pages.get(&pgno).map_or(true, |(t, _)| *t < txid) {
                pages.insert(pgno, (txid, page_bytes));
            }
        }
    }

    // 5. Encode the merged shard.
    let new_shard_bytes = encode_shard(K, pages.iter().map(|(p, (_, b))| (*p, b)));

    // 6. Atomic UDB tx: write new shard, decrement DELTAREF for each consumed delta,
    //    delete fully-consumed deltas, clear consumed PIDX entries, advance materialized_txid.
    tx.set(shard_key(K), new_shard_bytes);
    for (pgno, _) in pages_consumed_from_deltas {
        tx.delete(pidx_key(pgno));
    }
    for txid in touching_deltas {
        let new_refcount = tx.atomic_op(deltaref_key(txid), -pages_from_this_delta, Add);
        if new_refcount == 0 {
            tx.delete(delta_key(txid));
            tx.delete(deltaref_key(txid));
        }
    }
    write_meta(&tx, actor_id, advance_materialized_txid(head)).await?;
    Ok(())
}).await
```

Cost per pass: ~5 ms wall-clock (dominated by the UDB tx commit), ~700 µs CPU (LZ4 + merge), bounded byte transfer (~256 KiB shard + ~30 KiB of relevant delta slices).

**Crash safety**: a crash before `tx.commit()` is a no-op — the partial work is discarded by UDB. A crash after commit leaves consistent persistent state. The next compaction pass starts from the new META and continues. Recovery is idempotent at pass granularity.

### 4.5 Concurrency with writers

The actor commits new deltas at the same time compaction is folding old deltas into shards. Three races to handle:

1. **Commit lands during compaction**: the commit's UDB tx writes a new `DELTA/<new_txid>` and updates META. Compaction's UDB tx CAS-checks `(generation, materialized_txid)`. If the commit's META update interleaves before compaction commits, compaction sees the new META on its CAS and either retries (taking the new delta into account on the next pass) or proceeds (it's still operating on the older shard contents which haven't been touched). Both are correct.

2. **Compaction lands during a read**: the actor's `sqlite_get_pages` op runs in one UDB tx. If compaction commits between the actor's read of META and its read of the page bytes, the actor's tx sees a snapshot — either pre-compaction (page is in delta, fetch delta) or post-compaction (page is in shard, fetch shard). Both return the same bytes because compaction is byte-preserving. UDB's snapshot isolation does the work.

3. **Failover during compaction**: a new actor calls `sqlite_takeover`, generation bumps. The old compaction's CAS fails on the next pass; it discards its state and exits. The new actor's takeover triggers a fresh recovery pass.

The fencing CAS in the compaction tx is what makes all three races safe without locking.

### 4.6 Scheduling

A per-host `CompactionScheduler` runs compaction passes across actors. Implementation:

- `tokio::task::JoinSet` of background workers, sized at `max(2, num_cpus / 2)`.
- `antiox::sync::mpsc` queue of `(ActorId, TriggerReason)` events.
- `scc::HashSet<ActorId> in_flight` to serialize per-actor work (C5 — one pass per actor at a time).
- `shards_per_batch = 8` fairness budget — a single actor can compact at most 8 shards before yielding back to the queue, preventing noisy actors from starving others.
- Idle-scan task fires every 1 s to enqueue idle-triggered compactions.

The scheduler is shared across all actors on a host. Workers don't block on commits; commits don't block on workers. The only synchronization is the per-actor `in_flight` flag.

### 4.7 Recovery on takeover

`sqlite_takeover` runs a fast recovery scan:

1. Bump generation in META (CAS).
2. List `DELTA/` entries with txid > head.head_txid → orphan Phase-1 stages from a previous actor's failed commit. Delete them (and their `DELTAREF/` and `STAGE/` entries).
3. List `DELTAREF/` entries — anything with no matching `DELTA/<txid>` is a leaked refcount tracker. Delete it.
4. Trigger an immediate compaction pass if `delta_count >= N_recovery`.

All recovery operations are idempotent. A crash during recovery is a no-op for the next attempt.

### 4.8 Performance characteristics

At a 1000 commits/sec workload with ~10 dirty pages per commit and a working set spread across ~100 shards:

- **CPU per actor**: ~30% of one core for compaction. ~22 hot actors per core. ~350 actors per 16-core host before compaction CPU becomes the bottleneck.
- **Storage amplification**: ~1.3× steady-state (delta tier stays small, shards reflect committed state).
- **Wall-clock per pass**: ~5 ms.
- **Compared to actor-side materializer**: ~8× saved per pass because the network is not in the loop (160 ms of actor-side network work compresses to ~20 ms of engine-side CPU + UDB tx work).

### 4.9 Open questions

Documented in `compaction-design.md` §10 — the load-bearing ones are:
- Actual UDB tx latency for a 128 KiB shard write across the postgres and rocksdb drivers.
- Whether `MutationType::Add` re-read semantics work the way the refcount mechanism assumes.
- `litetx` crate feature audit (does it support sparse page sets, our LZ4 settings, etc.).
- Tuning `S = 64` and the threshold constants empirically.

These are decidable with measurement and don't change the architecture.

---

## 5. What's not in this document

- **Detailed engine module structure** (`actor_sqlite/mod.rs`, `commit.rs`, `read.rs`, etc.) — task #14, separate sketch.
- **In-memory test driver design** (`MemoryV2`) — task #15. Will run the entire engine subsystem against an in-memory backing store so unit tests can exercise the protocol without UDB.
- **Integration with existing engine infra** — fairness, metrics namespace, tracing, what existing engine modules to reuse vs. replace.
- **Migration tooling** — there is none. v1 actors stay v1. v2 actors are new. C8.
- **Recompiled workload analyses at 20 ms RTT** — the current `workload-*.md` files were computed at 2.9 ms RTT and need a recompute pass. Separate task.

---

## 6. Update log

- **2026-04-15** — Initial draft. Sections 1–3 (overview, protocol, VFS) complete. Section 4 (compaction) being designed by sub-agent.
