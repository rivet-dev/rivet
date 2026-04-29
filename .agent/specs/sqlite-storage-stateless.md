# Stateless SQLite Storage + Out-of-Process Compactor

## Goals

1. **Actor-side (pegboard-envoy) must be stateless.** No `open` / `close` ops. Every request self-describes its fence and runs against the current KV state. An actor must be safely takeoverable at any time without coordination with the previous host. In-memory state on pegboard-envoy is allowed only as a perf cache — never as the source of truth, never as a correctness fence, never as something that must survive across requests.
2. **Compactor (compaction + background work) lives in a separate process and is allowed to be stateful.** Compactor pods can hold per-actor in-memory state (in-flight tracking, plan caches, lease tables, etc.) if it helps. The actor-side is the constraint; the compactor-side is free to be stateful when it serves throughput, dedup, or latency.
3. **Must support the same SQL workload as vanilla SQLite, within UDB's transaction constraints.** Large transactions, large dirty-page sets, multi-MB blobs all need to work. The protocol cannot impose a tighter cap than UDB itself does. Where UDB has hard limits (per-tx size, per-tx age), the design must explicitly handle them — either by streaming/staging at the wire level, splitting transactions, or surfacing a clear error that the client knows how to recover from.
4. **Minimize hot-path latency.** Drop everything that isn't required for correctness on `get_pages` and `commit`.
5. **No extra reads/writes for defensive checks in release.** Trust the surrounding system contracts (pegboard exclusivity, the lost-timeout + ping protocol, UDB tx isolation) and design the hot path for perf. Defensive checks for "this should never happen" invariant violations belong behind `#[cfg(debug_assertions)]` so they fire loudly during development and CI but cost zero RTTs, zero KV ops, and zero comparisons in release builds. Do not add belt-and-suspenders fences that duplicate work the surrounding system is already responsible for.

## Non-goals

- Changing the on-disk KV layout (META / DELTA / PIDX / SHARD prefixes stay).
- Changing the LTX file format.
- Adding JetStream or any other durable message bus. Compaction triggers go through UPS (`engine/packages/universalpubsub/`) which is core-NATS-compatible — plain pub/sub, queue groups, no durability layer.
- Cross-process distributed locking. Concurrency safety stays fence-based.
- Eliminating in-memory state on the compactor side. State is fine there if it helps.

## Current architecture (relevant pieces only)

Pegboard-envoy holds a process-wide `SqliteEngine` (`engine/packages/sqlite-storage/src/engine.rs:17`) with three caches:

- `open_dbs: HashMap<String, OpenDb>` — per-actor `generation` for fast-fail.
- `page_indices: HashMap<String, DeltaPageIndex>` — PIDX cache.
- `pending_stages: HashMap<(String, u64), PendingStage>` — multi-chunk commit state machine (`next_chunk_idx`, `saw_last_chunk`, sticky `error_message`).

Plus an in-process `CompactionCoordinator` task that consumes from `mpsc<String>` and spawns per-actor compaction workers.

Wire protocol today:

```
open(actor_id, preload_pgnos)         → {generation, meta, preloaded_pages}
get_pages(generation, pgnos)
commit(generation, head_txid, dirty)  // fast path
commit_stage_begin(generation)        → {txid}
commit_stage(generation, txid, chunk_idx, bytes, is_last)
commit_finalize(generation, txid, ...)
close(actor_id, generation)
```

`open()` does three jobs: cache warm, orphan recovery (`build_recovery_plan`), and compaction trigger when delta count ≥ 32. Only recovery is real work; the rest is setup overhead.

## Proposed architecture

Single crate `engine/packages/sqlite-storage/` exposes two modules:

- **`pump`** — the hot path. Active component. Used by pegboard-envoy for actor reads and writes. Exports `ActorDb` (the per-actor handle), `commit`, `get_pages`, META/PIDX/DELTA/SHARD layout.
- **`compactor`** — the background service. Used standalone (registered in `engine/run_config.rs`). Owns lease handling, compaction algorithm, UPS subscriber loop.

Plus `takeover.rs` (top-level) for the takeover-tx helper called from pegboard.

```
┌─ pegboard-envoy (process per host) ──────────────────────┐
│  Conn (per WS connection):                               │──▶ UDB ◀──┐
│   └─ scc::HashMap<actor_id, Arc<ActorDb>>                │           │
│       (lazily upserted on first request,                 │           │
│        removed on command_stop_actor or WS close)        │           │
│                                                          │           │
│  ActorDb (per actor, exported from sqlite-storage::pump):│           │
│   ├─ udb: Arc<Database> (cloned from conn)               │           │
│   ├─ actor_id, cache: Mutex<DeltaPageIndex>              │           │
│   ├─ get_pages(pgnos)                                    │           │
│   └─ commit(dirty_pages, db_size_pages, now_ms)          │           │
│       on commit threshold:                               │           │
│       compactor::publish_compact_trigger(ups, actor_id)  │           │
│         → tokio::spawn(ups.publish(...))   // fire&forget│           │
└─────────────────────┬────────────────────────────────────┘           │
                      │ UPS                                            │
                      ▼ queue_subscribe(SqliteCompactSubject,          │
                                        "compactor")                   │
┌─ engine binary (HPA, N pods) ─────────────────────────────┐          │
│  Standalone service: compactor::start(config, pools)      │          │
│   ├─ UPS subscriber loop (TermSignal-aware)               │          │
│   ├─ /META/compactor_lease take/check/release                            │──────────┘
│   ├─ compactor::compact_default_batch                     │
│   │   (snapshot reads + COMPARE_AND_CLEAR)                │
│   └─ atomic_add /META/quota                               │
└───────────────────────────────────────────────────────────┘
```

Compactor is **not** a separate binary or crate. It's a `Standalone` service registered in the existing engine binary, same pattern as `pegboard_outbound` (`engine/packages/pegboard-outbound/src/lib.rs`). HPA scales the engine binary; UPS queue group balances compaction work across pods.

### Wire protocol after

Release shape:

```
get_pages(actor_id, pgnos)
commit(actor_id, dirty_pages, db_size_pages, now_ms) -> Ok
```

Two ops. No lifecycle. No fence inputs on the wire — pegboard exclusivity is the contract. Engine derives the next txid in-tx as `META.head_txid + 1`.

`commit` returns no payload (just success/error). The client doesn't need its assigned txid: under exclusivity there are no concurrent writers to disambiguate against, retry semantics don't need it (just re-read META if uncertain), and SQLite's own internal page-version state is independent of the storage txid. If diagnostics ever need it later, add an optional return field — saves a `u64` on the wire today.

Debug builds may carry optional `expected_generation` and `expected_head_txid` fields for invariant assertion (see "Debug-mode sentinels" below). Release builds skip the comparison entirely; the fields are parsed and ignored.

**Breaking changes are unconditionally acceptable. This system has not shipped to production.** No backwards compatibility, no migration period, no dual-running protocols. Wire shape, on-disk key layout, and `DBHead`/META schema are all free to change. The new protocol is a clean replacement.

### What's removed from `SqliteEngine`

The legacy `SqliteEngine` struct goes away entirely. The replacement is `ActorDb`, instantiated per-actor by the WS conn (see "What's kept" below).

- `open_dbs` — generation cache deleted. No fence to fast-fail on in release. Debug-mode sentinel reads `META.generation` in-tx (free, piggybacks on the existing META read).
- `pending_stages` — multi-chunk staging is gone. Replaced by single-shot commit.
- `compaction_tx` — replaced by `compactor::publish_compact_trigger` (UPS publish).
- `page_indices` (process-wide HashMap) — moved into `ActorDb` (per-actor instance, cached in the WS conn's `HashMap<actor_id, Arc<ActorDb>>`).
- `open()`, `close()`, `force_close()`, `ensure_open()` — methods deleted.
- `commit_stage_begin`, `commit_stage`, `commit_finalize` — collapsed into `commit`.

### What's kept

- `DeltaPageIndex` (PIDX cache) — perf cache only, no protocol meaning. The cache lives **inside `ActorDb`**, scoped to that actor's lifetime on the conn (one cache per actor, dropped when the actor's `ActorDb` is removed from the conn's HashMap or when the conn drops).

The crate exports a single per-actor type, `ActorDb`. There is no `Pump` struct, no process-wide registry, no per-conn wrapper inside sqlite-storage. The WS conn (in pegboard-envoy) owns the `HashMap<actor_id, Arc<ActorDb>>` directly:

```rust
// Exported from sqlite_storage::pump
pub struct ActorDb {
    udb: Arc<universaldb::Database>,
    actor_id: String,
    cache: parking_lot::Mutex<DeltaPageIndex>,
    /// Cached `/META/quota`. Loaded once on the first UDB tx (whichever of
    /// `get_pages` or `commit` arrives first), mutated in-process on every
    /// commit. Stale (over-estimates) is safe under pegboard exclusivity;
    /// under-estimates cannot occur. `None` until the first tx loads it.
    storage_used: parking_lot::Mutex<Option<i64>>,
    /// Bytes written across commits since the last metering rollup. Reset
    /// to 0 by the compactor on each pass.
    commit_bytes_since_rollup: parking_lot::Mutex<u64>,
    /// Bytes read across `get_pages` calls since the last metering rollup.
    /// Reset to 0 by the compactor on each pass.
    read_bytes_since_rollup: parking_lot::Mutex<u64>,
    /// Last time we published a compaction trigger for this actor. Used by
    /// the per-actor throttle to suppress redundant trigger publishes on
    /// hot actors. See "Compaction trigger" subsection.
    last_trigger_at: parking_lot::Mutex<Option<Instant>>,
}

impl ActorDb {
    pub fn new(udb: Arc<universaldb::Database>, actor_id: String) -> Self;
    pub async fn get_pages(&self, pgnos: Vec<u32>) -> Result<Vec<FetchedPage>>;
    pub async fn commit(
        &self,
        dirty_pages: Vec<DirtyPage>,
        db_size_pages: u32,
        now_ms: i64,
    ) -> Result<()>;
}
```

A commit that would push `storage_used` over `SQLITE_MAX_STORAGE_BYTES` is rejected with `SqliteStorageQuotaExceeded { remaining_bytes, payload_size }` (mirroring actor KV's error shape from `errors::Actor::KvStorageQuotaExceeded`). The check happens against the in-memory cache before any UDB writes.

Cold-cache cost: the first `get_pages` for a given actor on a given WS conn does a PIDX prefix scan inside its UDB tx. Subsequent calls on the same `ActorDb` hit RAM. Tracked via `sqlite_pump_pidx_cold_scan_total` metric.

### Why no active-actor tracking on the WS conn

An envoy can reconnect to a different pegboard-envoy worker node mid-flight while an actor is still active on the envoy host. When that happens, the new worker node never receives the original `CommandStartActor` for any actors that started before the reconnect — pegboard sends `start_actor` once when scheduling, not on every reconnect. So a per-conn `active_actors` HashMap (or any presence-tracking structure) would be empty/incomplete relative to what's actually running.

Treat the WS conn as **stateless w.r.t. actor identity**. There is no authoritative per-conn list of which actors are active and no `start_actor` handler. The only per-conn state is the perf cache `HashMap<actor_id, Arc<ActorDb>>`, populated lazily as `get_pages` / `commit` requests arrive. `command_stop_actor` is the only lifecycle command kept; its sole responsibility is to remove the entry from that HashMap (and thereby drop the cache). Stale entries that survive because `stop_actor` never arrived (envoy reconnected to a different worker before pegboard noticed) are bounded by WS-conn lifetime — they evict on conn drop.

### Quota enforcement

`ActorDb` carries an in-memory cache of `/META/quota` (the `storage_used: Mutex<Option<i64>>` field). The cache loads from UDB on the first request that opens a UDB tx — whichever of `get_pages` or `commit` arrives first. On all subsequent commits, the cap check is a local comparison against the cached value with no extra UDB read on the steady-state path.

Commit flow:

1. Take the cached `storage_used` value (loaded lazily on the first tx).
2. Compute `would_be = cached + delta_bytes` where `delta_bytes` is the sum of bytes added across META/PIDX/DELTA writes for this commit, computed before any UDB mutation.
3. If `would_be > SQLITE_MAX_STORAGE_BYTES` → reject with `SqliteStorageQuotaExceeded { remaining_bytes, payload_size }` (same shape as actor KV's `errors::Actor::KvStorageQuotaExceeded` in `engine/packages/pegboard/src/actor_kv/`).
4. Otherwise: proceed with commit, `atomic_add(/META/quota, +delta_bytes)`, and update the cache locally.

The cap is a Rust constant in `pump::quota`:

```rust
pub const SQLITE_MAX_STORAGE_BYTES: i64 = 10 * 1024 * 1024 * 1024; // 10 GiB
```

**Why the cache is safe.** The only writers to `/META/quota` are (a) commits on this exact `ActorDb` under pegboard exclusivity, and (b) the compactor (only ever decreases). The cached value is therefore always `>= true value`. Worst case: over-rejection (conservative — never lets a user write past the limit). The cache refreshes naturally on the next conn (a new `ActorDb` re-loads it).

**Hot-path overhead.** Zero new RTTs on steady-state commits. The first commit on a new `ActorDb` reads `/META/quota` alongside `/META/head` (parallelized via `tokio::try_join!`). One extra in-memory comparison per commit. The metering pipeline is not contacted on commit at all.

### META key split

`META` is split into four sub-keys, each with a single writer (or atomic semantics). All live under the existing per-actor prefix `[0x02][actor_id]`. (Free to do this — the system isn't shipped, so the on-disk key layout has zero compatibility constraints.)

```
/META/head             — head_txid, db_size_pages       (commit-owned, vbare blob)
/META/compact          — materialized_txid              (compaction-owned, vbare blob)
/META/quota            — sqlite_storage_used            (atomic counter, raw i64 LE)
/META/compactor_lease  — { holder_id, expires_at_ms }   (compaction lease, vbare blob)
```

Previously-stored fields are now Rust constants. `page_size` and `shard_size` live as `PAGE_SIZE` / `SHARD_SIZE` in `pump::keys`. The per-actor cap lives as `SQLITE_MAX_STORAGE_BYTES` in `pump::quota`. There is no `/META/static` key. There is no on-disk `schema_version`, `creation_ts_ms`, or origin tag — there is one schema, and any future bump detects version by presence/absence of new fields.

Optional: `generation` field on `/META/head` if kept for debug-mode sentinels. Not load-bearing in release.

`/META/quota` is **value-only fixed-width little-endian signed `i64`**, not a vbare blob — FDB atomic-add expects a fixed-width LE integer. Commits do `atomic_add(/META/quota, +bytes_written as i64)`; compaction does `atomic_add(/META/quota, (-bytes_freed as i64).to_le_bytes())`. Atomic adds compose without taking conflict ranges, so commit and compaction never conflict on quota.

**Atomic counter encoding.** The value at `/META/quota` is exactly 8 bytes. Increments encode `bytes_written` as `i64::to_le_bytes()`. Decrements encode the negative as `(-(bytes_freed as i64)).to_le_bytes()` so FDB's atomic-add sums them into a signed running total. Reads of `/META/quota` interpret the bytes as `i64::from_le_bytes`; the value should always be non-negative under correct operation. The field is signed so an out-of-order arithmetic could not corrupt the encoding, but FDB atomic-add is exact integer addition and the counter is correct as long as every code path that mutates billable bytes emits the matching atomic-add delta. There is no drift in steady state — a non-zero error means there is a bug in a quota-mutating code path, not entropy. Bugs get fixed at the call site, not by periodic recompute.

Other breaking changes:

- **Drop `next_txid`.** Single-shot commits derive `T = head_txid + 1` in-tx. The reservation counter only existed to support multi-chunk staging (allocate-then-stream-then-finalize). With single-shot, allocation and commit happen atomically in the same UDB tx — there is no allocated-but-not-yet-committed window.

### Hot-path key reads

| Op | Reads | Writes |
|---|---|---|
| `get_pages` | `/META/head` (db_size_pages) + PIDX scan + DELTA/SHARD blobs | none |
| `commit` (steady state) | `/META/head` + (PIDX upserts for dirty pgnos) | `/META/head` + DELTA chunks + PIDX upserts + `atomic_add(/META/quota, +bytes)` |
| `commit` (first on a new ActorDb, cold quota cache) | `/META/head` + `/META/quota` (in parallel via `try_join!`) + (PIDX upserts for dirty pgnos) | `/META/head` + DELTA chunks + PIDX upserts + `atomic_add(/META/quota, +bytes)` |
| compaction | `/META/compactor_lease` + `/META/head` + `/META/compact` + PIDX (snapshot) + DELTA blobs to fold + SHARD blobs being merged into | `/META/compactor_lease` (take) + `/META/compact` + SHARD writes + PIDX `COMPARE_AND_CLEAR` + DELTA deletes + `atomic_add(/META/quota, -bytes)` |
| takeover (release) | none | none |
| takeover (debug, `cfg(debug_assertions)`) | DELTA/PIDX/SHARD prefix scans for orphan classification (assert-only) | none |
| first commit (lazy META init) | `/META/head` (absent) | `/META/head` + DELTA chunks + PIDX upserts + initial `/META/quota` |

Steady-state hot-path reads cost a single key fetch (`/META/head`) within one tx — no `try_join!` is needed because there is only one key. The first commit on a new `ActorDb` reads two keys (`/META/head` + `/META/quota` for the in-memory quota cache load); those two gets must be issued concurrently via `tokio::try_join!(tx.get(/META/head), tx.get(/META/quota))`. UDB's `tx.get()` does NOT pipeline by itself; on FDB native, `try_join!` gets real parallelism, and on RocksDB it saves the await-between-sends gap. Without `try_join!` on that first-commit path, the two gets are serialized and add a real RTT.

Hot-path writes are unchanged in count: commit writes `head` (plus `quota` via atomic add, which doesn't take a conflict range).

### Debug-mode sentinels

Under `#[cfg(debug_assertions)]`, the engine asserts pegboard's exclusivity contract on every op:

```rust
#[cfg(debug_assertions)]
{
    if let Some(expected) = request.expected_generation {
        if head.generation != expected {
            tracing::error!(
                actor_id = %actor_id, expected, actual = head.generation,
                "sqlite generation fence mismatch — pegboard exclusivity violated"
            );
            return Err(SqliteStorageError::FenceMismatch { ... }.into());
        }
    }
    if let Some(expected) = request.expected_head_txid {
        if head.head_txid != expected {
            tracing::error!(
                actor_id = %actor_id, expected, actual = head.head_txid,
                "sqlite head_txid OCC mismatch — concurrent writer detected"
            );
            return Err(SqliteStorageError::FenceMismatch { ... }.into());
        }
    }
}
```

Release builds skip the comparison block entirely. The `expected_*` fields are parsed (small wire cost) but unused. No extra KV ops, no extra RTTs, no comparisons.

If pegboard exclusivity is ever violated in production, the result is undefined — the engine will not catch it. Acceptable per goal 5: defensive checks must not slow the hot path.

### Takeover

**There is no takeover work in release.** Pegboard's reassignment transaction does not touch sqlite-storage at all in release builds. The combination of (a) v2's atomic single-shot commits, (b) UDB tx isolation, and (c) pegboard exclusivity makes orphans impossible to produce in steady state — there is no half-state to reconcile. Whatever the previous host left in UDB is, by construction, a coherent v2 actor state.

The new envoy gets no setup signal at all. Pegboard reassigns the actor; the envoy starts receiving SQLite requests for it; on the first request, the WS conn lazily inserts an `ActorDb` into `actor_dbs`; on the first commit, that `ActorDb` seeds `/META/head` if missing as part of the commit's own UDB tx.

**Lazy first-commit META init.** The commit path must check whether `/META/head` exists at the start of its tx. If absent, this is the first commit on this actor — seed `/META/head` with `head_txid=0`, `db_size_pages=1`, and skip the atomic-add on `/META/quota` (initial state is zero, so leave the key absent — atomic-add will set it on first non-zero delta). One extra `tx.get(/META/head)` on the first commit; runs once per actor lifetime.

**Debug-only invariant check.** Under `#[cfg(debug_assertions)]`, `ActorDb::new` may run `takeover::reconcile(udb, actor_id)` to scan PIDX / DELTA / SHARD prefixes and assert no orphans exist. If the scan finds anything → loud structured error log identifying the violated invariant + panic in tests. This is a development-time invariant verification, not a production cleanup pass. Release builds skip this entirely; `ActorDb::new` does no UDB work.

The `takeover.rs` module exposes a single function: `pub async fn reconcile(udb: &Database, actor_id: &str) -> Result<()>`, gated `#[cfg(debug_assertions)]`. There is no `takeover::create_actor` because creation is folded into the lazy first-commit init.

The legacy STAGE/ key prefix from the multi-chunk staging protocol does not exist in the new design. Nothing in release cleans STAGE keys. Since SQLite v2 has not shipped, no actor in production has v2-format data; sqlite-storage exposes no migration helper. The actor v2 workflow's existing migration code (which writes SQLite state during actor v1 → actor v2) will need its destination schema updated to match this spec's v2 layout — that update is the workflow's concern, not sqlite-storage's.

### Compactor service

- Lives in `sqlite-storage::compactor`. Exposes `pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()>`, registered as `ServiceKind::Standalone` with `restart=true` in `engine/packages/engine/src/run_config.rs`. Same pattern as `pegboard_outbound` (see `engine/packages/pegboard-outbound/src/lib.rs:85-156`).
- Uses **UPS** (`engine/packages/universalpubsub/`), not NATS directly. UPS already supports queue-group semantics (`queue_subscribe`) on all drivers (memory, NATS, postgres). No UPS changes needed.
- Internal shape: construct `StandaloneCtx::new(...)`, get UPS via `ctx.ups()?`, get UDB via `ctx.udb()?`. Same plumbing as `pegboard_outbound`.
- Subscribes to `SqliteCompactSubject` with queue group `"compactor"` (typed subject struct in `compactor::subjects`).
- Select loop with `TermSignal::get()` for graceful shutdown. On shutdown, **release any held leases before exiting** (not optional — held leases stall the next compaction by up to TTL).
- On `NextOutput::Unsubscribed`: bail out and let the supervisor restart the service. Same behavior as `pegboard_outbound`.
- Stateless: each UPS message is independent. On receiving a trigger, the per-trigger handler runs in a `tokio::spawn`d task. The handler takes the actor's `/META/compactor_lease` (skipping if another pod holds it), reads META, decides if compaction is needed (`head_txid - materialized_txid` ≥ threshold), runs `compact_default_batch`, releases the lease, exits. Aborts on fatal error.
- HPA-scaled. Adding/removing pods is just engine binary instances churning their UPS connections.
- No leader election, no distributed coordination, no sweeper task in v1. The UDB-backed lease replaces all of these for cross-pod coordination.

**Test entrypoint convention.** `compactor::start` factors as a public `pub async fn start(config, pools) -> Result<()>` outer plus a `pub(crate) async fn run(udb, ups, term_signal) -> Result<()>` inner. Tests inject the UPS memory driver and an explicit UDB handle directly via the inner entrypoint, bypassing the engine's `Pools` plumbing.

**`CompactorConfig`.** Exposed from `sqlite_storage::compactor`:

```rust
pub struct CompactorConfig {
    pub lease_ttl_ms: u64,             // default 30_000 — must exceed FDB tx age (5s)
    pub lease_renew_interval_ms: u64,  // default 10_000 — TTL/3
    pub lease_margin_ms: u64,          // default 5_000  — TTL/6, must exceed FDB tx age
    pub compaction_delta_threshold: u32, // default 32 — head_txid - materialized_txid threshold
    pub batch_size_deltas: u32,        // default 32 — max deltas folded per pass
    pub max_concurrent_workers: u32,   // default 64 — per-pod tokio::spawn cap on triggers
    pub ups_subject: String,           // default "sqlite.compact"
    #[cfg(debug_assertions)]
    pub quota_validate_every: u32,     // debug only; default 16 — manually re-tally /META/quota every Nth pass
}

impl Default for CompactorConfig { /* ... */ }
```

The struct is registered inline in `engine/packages/engine/src/run_config.rs`, e.g.:

```rust
Service::new(
    "sqlite_compactor",
    ServiceKind::Standalone,
    |config, pools| Box::pin(sqlite_storage::compactor::start(config, pools, CompactorConfig::default())),
    true,
)
```

**Idle-actor stuck behavior is acceptable.** An actor that crosses the compaction threshold then goes idle and loses its trigger to a UPS hiccup will not compact until it next writes. This is acceptable because storage isn't actively growing while the actor is idle, and there is no urgency. A periodic safety sweep (e.g. a CronJob iterating actors that haven't been compacted recently) is deferred to future work.

### Metering rollup

After every successful compaction pass, the compactor rolls up per-actor storage metrics into the namespace-level metering pipeline using the same `MetricKey` structure actor KV uses (`engine/packages/pegboard/src/namespace/keys/metric.rs`). Three new variants:

- `SqliteStorageUsed { actor_name }` — current bytes in `/META/quota`; emitted every pass (point-in-time gauge).
- `SqliteCommitBytes { actor_name }` — bytes written across commits since the last rollup.
- `SqliteReadBytes { actor_name }` — bytes read across `get_pages` calls since the last rollup.

For commit/read byte counters, `ActorDb` maintains in-memory per-actor counters (`commit_bytes_since_rollup` / `read_bytes_since_rollup` — see "What's kept" above). The hot path increments these counters locally on each commit and `get_pages`; no UDB writes happen for metering on the hot path. The compactor reads + zeros these counters on each pass and emits via `atomic_add` against the `MetricKey`. The counters use `parking_lot::Mutex<u64>` for the same forced-sync-context reasons as the quota cache.

Round commit/read byte deltas to 10 KB chunks before emitting (matching actor KV's `KV_BILLABLE_CHUNK` convention — see `engine/packages/pegboard/src/actor_kv/mod.rs:164-166, 327-329`).

The compactor is already reading `/META/quota` for the pass, so emitting `SqliteStorageUsed` costs nothing extra. The commit/read byte counters live in envoy memory, not UDB — the compactor and envoy run in different processes, so the counter values must travel via the same channel as the compaction trigger. The UPS trigger payload carries `commit_bytes_since_rollup` / `read_bytes_since_rollup` snapshots that the envoy zeroes locally as it builds the message; the compactor reads those values out of the trigger and emits the metering atomic-adds.

**Idle-actor caveat.** Actors that never compact never emit metering. For low-activity actors this means stale billing snapshots until they next compact. Acceptable because: (a) low-activity actors have low storage churn so usage is roughly stable, (b) if billing freshness is needed for idle actors later, the existing `pegboard_actor_metrics` workflow can read `/META/quota` for SQLite actors as a one-line addition.

**Hot-path overhead.** Zero. `ActorDb` increments local counters on commit/read; the compactor pulls them out periodically. No metering UDB writes on the hot path.

### Compaction trigger

After every successful commit-that-crosses-threshold, pegboard-envoy calls `sqlite_storage::compactor::publish_compact_trigger(ups, actor_id)`. The helper internally `tokio::spawn`s a UPS publish — strictly fire-and-forget, must not be awaited before sending the WS commit response. The trigger is a hint; loss is tolerable because the next commit republishes (subject to the throttle described below).

UPS subject naming follows the existing convention (`pegboard::pubsub_subjects::ServerlessOutboundSubject` style): a typed struct implementing `Display`, owned by the storage crate.

**Per-actor throttle.** A naive "publish on every commit at-or-above threshold" floods the compactor: a hot actor doing 100 commits/sec at threshold = 100 redundant publishes/sec, each costing UPS publish + UPS deliver + a `/META/compactor_lease` read on the receiver. To bound this, `ActorDb` throttles publishes per actor:

- `last_trigger_at: parking_lot::Mutex<Option<Instant>>` — local-only, no UDB.
- On commit-crosses-threshold, check `now - last_trigger_at`. If `< trigger_throttle_ms` (default 500ms), skip the publish. Otherwise publish and update `last_trigger_at`.
- **First commit fires immediately.** Subsequent commits within the window are dropped. (This is throttle, not debounce — debounce would defer indefinitely under sustained load and starve hot actors of compaction.)

**Trigger-loss safety net.** If the throttle suppresses publishes for an extended window (e.g. all triggers landed in a UPS partition or got dropped on the receiver), an actor at-or-above threshold could go indefinitely without a trigger. Cap the throttle: if `now - last_trigger_at > trigger_max_silence_ms` (default 30s) AND the actor is still at-or-above threshold, force a publish regardless of recent activity. In practice this rarely fires — UPS isn't that flaky and compaction passes finish in seconds — but it closes the loop on trigger-loss recovery for actively-committing actors.

The throttle constants are constants in `pump::quota` (or `pump::trigger`) module, not on `CompactorConfig` — they're envoy-side, not compactor-side.

### Debug-only quota validation

Under `#[cfg(debug_assertions)]`, the compactor periodically verifies that `/META/quota`'s atomic-add running total matches a manually-tallied byte count from a full PIDX/DELTA/SHARD scan. Runs every Nth compaction pass per actor (default `quota_validate_every = 16`, exposed on `CompactorConfig`).

Procedure:

1. After the compaction pass completes, in a separate read-only UDB tx: scan PIDX + DELTA + SHARD prefixes, total billable bytes manually.
2. Read `/META/quota`.
3. Assert `manual_total == counter`. On mismatch → structured error log identifying actor + delta, panic in tests.

This is a development-time invariant verification of atomic-add correctness. It does NOT correct the counter. Drift (if observed) means a quota-mutating call site has a bug; the fix is at the bug site, not by recompute.

**Strictly debug-only.** Release builds skip this entirely — no extra scan, no extra read, zero overhead. Goal 5 applies.

## Concurrency model

The design uses two mechanisms total: pegboard exclusivity for envoy writers, and a UDB-backed lease for compaction. Plus one atomic op (`COMPARE_AND_CLEAR`) for the residual commit-vs-compaction PIDX race.

### Envoy writers: pegboard exclusivity

Per `engine/CLAUDE.md`: at most one envoy hosts an actor at a time. Pegboard's lost-timeout + ping protocol is the source of truth. Storage layer trusts this contract and does **not** add separate KV concurrency fences. Per goal 5, defensive in-tx checks for "two writers detected" are debug-only.

### Compaction lease

A compactor pod takes a UDB-backed lease before running compaction for an actor:

```
/META/compactor_lease → { holder_id: NodeId, expires_at_ms: i64 }
```

Lease take procedure:

```
1. Regular read (NOT snapshot read) of `/META/compactor_lease`.
   The regular read takes a conflict range — two pods racing the take get
   FDB OCC abort on the loser. A snapshot read here would let both pods
   "take" the lease.
2. If exists, holder != me, expires_at_ms > now: skip; another pod is working.
3. Else: write `/META/compactor_lease = { my_id, now + TTL }`.
4. Run compaction work under a CancellationToken (see Lease lifecycle).
5. On graceful exit, clear the lease so the next trigger doesn't wait for TTL.
```

**TTL > FDB tx age (5s).** Default 30s.

If a pod dies mid-compaction, the actor's compaction stalls for at most TTL before another pod takes over. Acceptable because compaction is throughput-bound, not latency-bound.

The lease eliminates concurrent compactions entirely. Compaction can use snapshot reads on SHARD blobs and plain `set()` for `/META/compact` — no atomic MAX, no quota reconciliation, no SHARD-content races to defend against.

### Lease lifecycle

The lease is held via a local timer, a cooperative cancellation token, and a periodic renewal task. **No /META/compactor_lease reads happen inside compaction work transactions.** Renewal is the only place /META/compactor_lease is read during a compaction pass.

Constants:

- `lease_ttl_ms = 30_000` (TTL = 30s)
- `lease_renew_interval_ms = 10_000` (renew every TTL/3 ≈ 10s)
- `lease_margin_ms = 5_000` (margin = TTL/6 ≈ 5s, chosen > FDB tx age (5s))

On lease take, the compactor computes `deadline = lease_acquired_at + TTL - margin` and arms a local `tokio::time::sleep_until(deadline)`. The compaction pass runs under a `CancellationToken`. The token is tripped by either:

- the local deadline timer firing (sleep_until completes), OR
- the renewal task observing a renewal failure.

Renewal task (runs every `lease_renew_interval_ms`):

1. Open a small UDB tx.
2. Regular-read /META/compactor_lease.
3. Assert `holder == me && expires_at_ms > now`. If either fails, the lease has been stolen or expired.
4. Write `expires_at_ms = now + TTL`. Commit.
5. On success: extend the local deadline. Replace the existing `sleep_until` with a fresh `sleep_until` at the new deadline.
6. On failure (lease stolen, UDB error, RPC timeout shorter than `deadline - now`): trip the cancellation token immediately.

Compaction work checks the cancellation token before each FDB tx. Token tripped → abort, do not start new work.

In-flight FDB transactions when the token trips are not aborted explicitly. They either commit successfully (within tx age, lease still valid) or abort on tx-age. Both outcomes are safe because the lease still grants exclusivity at commit time on the success path, and an aborted tx writes nothing on the failure path.

This design avoids per-tx lease re-validation reads inside compaction work. It also sidesteps the "lease expired but my tx commits anyway" pathology: any tx that completes within tx-age must have started while the lease was valid, and the renewal margin keeps the deadline ahead of in-flight commits.

### META key split (commit/compaction decoupling)

Even with the lease preventing concurrent compactions, commit and compaction still race because they run in different processes (envoy vs. compactor). Splitting META into per-owner sub-keys decouples them at the FDB conflict-range level (full key layout in the [META key split section](#meta-key-split) above):

```
/META/head             — commit-owned
/META/compact          — compaction-owned
/META/quota            — atomic counter
/META/compactor_lease  — compaction lease
```

**Commit writes:** `/META/head` + `atomic_add(/META/quota, +bytes_written)` + PIDX upserts + DELTA chunk writes.
**Compaction writes:** `/META/compact` + `atomic_add(/META/quota, -bytes_freed)` + conditional PIDX deletes (see below) + DELTA blob deletes + SHARD blob writes.

Compaction reads `/META/head.head_txid` (upper bound for which DELTAs are eligible) using a **snapshot read** — no conflict range taken.

`/META/quota` uses FDB atomic add. Two atomic adds compose without taking conflict ranges. Lease ensures no two compactions decrement at once, so the "double-decrement" concern doesn't apply.

Net effect: commits and compaction never conflict on any `/META/*` sub-key.

### PIDX deletes: COMPARE_AND_CLEAR

Compaction must delete PIDX entries for pages it folded into a shard. If a commit writes a *newer* PIDX entry for the same pgno between compaction's plan phase and its commit, blindly deleting would erase the new commit's claim.

Compaction uses FDB's `COMPARE_AND_CLEAR(key, expected_value)` atomic op. The op clears the key iff its current value equals `expected_value`, atomically at commit time. **It takes no read conflict range** — the value comparison happens during commit application, not via OCC.

```
Plan phase (snapshot reads, no conflicts):
  for each delta T in compaction window:
    decode T's LTX → page set
  for each pgno in union of page sets:
    snapshot_get(PIDX[pgno]) → owner_txid
    if owner_txid in {our K folded deltas}:
      add (pgno, owner_txid) to fold plan

Write phase (no conflicts on PIDX):
  set SHARD blobs
  for each (pgno, expected_txid) in fold plan:
    COMPARE_AND_CLEAR(PIDX[pgno], expected_txid_be_bytes)
  clear_range each folded DELTA's chunks
  set /META/compact, atomic_add /META/quota
```

Race resolution: if a commit writes `PIDX[5] = T_new` between compaction's snapshot and commit, the COMPARE_AND_CLEAR sees `T_new ≠ T_old` and no-ops. Newer commit's claim survives. The SHARD write is shadowed by the newer DELTA — harmless because PIDX shadows SHARD on read.

UDB needs to expose `COMPARE_AND_CLEAR`. FDB has it natively (`MutationType::COMPARE_AND_CLEAR`). Small wrapper-only addition in `engine/packages/universaldb/` if not already present.

### PIDX deletion atomicity

When compaction folds deltas into a shard, the old DELTA blobs **and** their PIDX `COMPARE_AND_CLEAR` ops must be in the same UDB tx as the shard write. Otherwise reads pay extra "stale-PIDX → shard fallback" round-trips until the entries are cleaned.

### Compaction concurrent with reads

UDB tx isolation: reads see consistent snapshot. If pegboard-envoy's PIDX cache points to a delta that compaction just deleted, the existing stale-PIDX fallback (`read.rs:144-150`) handles it — reads the shard instead, evicts the stale cache row. Self-healing.

### Shrink race during compaction

A commit that lowers `db_size_pages` orphans pages above the new EOF. CLAUDE.md requires shrink to delete above-EOF PIDX rows AND above-EOF SHARD blobs in the same tx as the commit.

This creates a race with compaction: if compaction reads `/META/head.db_size_pages` via snapshot at plan time, runs against the old EOF, then writes a SHARD that's now above EOF — that SHARD is leaked permanently and the quota counter drifts downward only.

**Fix:** the compactor's WRITE tx does a REGULAR (non-snapshot) read of `/META/head.db_size_pages` at the start of the write phase. A concurrent shrink commit then conflicts on `/META/head` and the compactor's tx aborts and retries with the new EOF. Cost: one extra in-tx read; recreates a brief `/META/head` conflict only during the write-phase window, not during the snapshot-heavy plan phase.

The other concern that comes up around shrink + compaction — "stale SHARD bytes for un-compacted-but-superseded pgnos" — is self-healing via PIDX shadowing on read and is not a bug. A SHARD blob whose bytes are obsolete is harmless as long as some PIDX row points to a newer DELTA, because reads go through PIDX first.

## Engine infrastructure dependency

This spec depends on a new `NodeId` type added to `rivet_pools::Pools`. The `NodeId` is generated at engine startup as `Uuid::new_v4()`, accessed via `pools.node_id() -> NodeId`. It is random, **not** derived from `HOSTNAME` or any other deployment-shaped identifier.

The `NodeId` infrastructure is a separate engine-wide change to `engine/packages/pools/` that lands before the compactor work in this spec.

The compactor uses `pools.node_id()` as the `/META/compactor_lease.holder_id` value. The lease's `holder_id` field is a `NodeId` (essentially a `Uuid`) — not the earlier proposed `LeaseHolder { pod_name, instance_uuid }` shape. All metrics emitted by the compactor and pump include a `node_id` label sourced from `pools.node_id()`.

## Hot-path latency analysis

Steady-state `get_pages` / `commit` reads only `/META/head` — a single key fetch, no `try_join!` needed. The first commit on a new `ActorDb` reads `/META/head` and `/META/quota` concurrently via `tokio::try_join!`; without it the two gets are serialized and add a real RTT.

| Op | Before | After | Change |
|---|---|---|---|
| `open()` | 1 RTT + 3 prefix scans + atomic write | gone | removed |
| `close()` | 1 RTT, in-mem cleanup | gone | removed |
| `get_pages` (warm cache) | 1 RTT, identical UDB ops | 1 RTT (`/META/head` + PIDX cache hit + DELTA/SHARD) | 0 |
| `get_pages` (cold cache, post-takeover) | 1 RTT (PIDX preloaded by `open`) | 1 RTT (PIDX prefix scan in-tx; tracked via `sqlite_pump_pidx_cold_scan_total`) | 0 RTT but extra in-tx scan once per WS conn |
| `commit` (steady state) | 1 RTT, identical UDB ops | 1 RTT, identical UDB ops | 0 |
| `commit` (first on a new ActorDb) | n/a | 1 RTT (`/META/head` + `/META/quota` via `try_join!`) | 0 (with `try_join!`) |
| `commit` (multi-chunk) | `2 + N` RTT × tx | 1 RTT × tx | -N to -(N+1) |
| `ensure_open` per op | HashMap lookup | gone | -sub-µs |
| Compaction trigger | `mpsc::send` (free) | `ups.publish` via `tokio::spawn` (~tens of µs, off path) | +negligible |

No UDB op gets heavier on the hot path. Multi-chunk commits get materially lighter. Cold-start latency drops by one RTT (no `open`). The cold-cache `get_pages` does an in-tx PIDX prefix scan instead of using preloaded data, but this happens once per WS connection (typically once per actor lifetime on a given envoy).

## Metrics

All metrics include a `node_id` label sourced from `pools.node_id()`.

**Pump-side**:

- `sqlite_pump_commit_duration_seconds` (histogram)
- `sqlite_pump_get_pages_duration_seconds` (histogram)
- `sqlite_pump_commit_dirty_page_count` (histogram)
- `sqlite_pump_get_pages_pgno_count` (histogram)
- `sqlite_pump_pidx_cold_scan_total` (counter) — incremented when `get_pages` runs against an empty per-conn cache and falls back to a PIDX prefix scan in-tx.

**Compactor-side**:

- `sqlite_compactor_lag_seconds{actor_id_bucket}` (histogram of `now - last_materialized_ts`)
- `sqlite_compactor_lease_take_total{outcome=acquired|skipped|conflict}` (counter)
- `sqlite_compactor_lease_held_seconds` (histogram)
- `sqlite_compactor_lease_renewal_total{outcome=ok|stolen|err}` (counter)
- `sqlite_compactor_pass_duration_seconds` (histogram)
- `sqlite_compactor_pages_folded_total` (counter)
- `sqlite_compactor_deltas_freed_total` (counter)
- `sqlite_compactor_compare_and_clear_noop_total` (counter)
- `sqlite_compactor_ups_publish_total{outcome=ok|err}` (counter)

**Quota**:

- `sqlite_storage_used_bytes` (gauge per actor, sampled)

**Billing metrics (UDB-backed namespace counters, not Prometheus)**:

These are emitted by the compactor on every pass via `atomic_add` against the namespace-level `MetricKey` structure (`engine/packages/pegboard/src/namespace/keys/metric.rs`), separate from the Prometheus metrics enumerated above. They feed the metering pipeline.

- `MetricKey::SqliteStorageUsed { actor_name }` — current bytes in `/META/quota` (point-in-time gauge).
- `MetricKey::SqliteCommitBytes { actor_name }` — commit bytes since last pass (rounded to 10 KB chunks, matching actor KV's `KV_BILLABLE_CHUNK`).
- `MetricKey::SqliteReadBytes { actor_name }` — `get_pages` bytes since last pass (rounded to 10 KB chunks).

See "Metering rollup" under the [Compactor service](#compactor-service) section for the rollup mechanism.

**Debug-only** (under `#[cfg(debug_assertions)]`):

- `sqlite_fence_mismatch_total` (counter) — pegboard exclusivity contract violated.
- `sqlite_quota_validate_mismatch_total` (counter) — manually-tallied bytes did not match `/META/quota` during a periodic compactor validation pass.
- `sqlite_takeover_invariant_violation_total{kind}` (counter) — orphan classification found a row that should not exist (kind = above_eof | above_head_txid | dangling_pidx_ref).

## Testing strategy

- **Per-module test scope.** `tests/pump_*.rs` for hot-path coverage (`pump_read.rs`, `pump_commit.rs`, `pump_keys.rs`); `tests/compactor_*.rs` for lease/compaction/UPS dispatch (`compactor_lease.rs`, `compactor_compact.rs`, `compactor_dispatch.rs`); `tests/takeover.rs` for takeover-tx coverage.
- **No mocks for storage paths.** All tests run against real UDB via `test_db()` (RocksDB-backed temp instance).
- **UPS dispatch tests use the UPS memory driver.** `engine/packages/universalpubsub/src/driver/memory/`. No real NATS broker required.
- **Crash-recovery tests** use `checkpoint_test_db()` + `reopen_test_db()` for real persisted-restart state.
- **Latency tests** live in a dedicated integration test binary because UDB caches `UDB_SIMULATED_LATENCY_MS` once via `OnceLock`; mixing latency and non-latency tests in the same binary corrupts the cached value across tests.
- **Failure-injection tests** use `MemoryStore::snapshot()`. Note that the `fail_after_ops` budget continues consuming after the first injected error.
- **Lease-expiry tests** use `tokio::time::pause()` + `advance()` for determinism.
- **COMPARE_AND_CLEAR conflict tests** verify the no-op path on stale-PIDX (commit writes `PIDX[pgno] = T_new` before compaction's CAS lands).
- **Test entrypoint convention.** `compactor::start` factors into a public `start(config, pools)` outer and a `pub(crate) async fn run(udb, ups, term_signal)` inner. Tests inject the memory-driver UPS directly via the inner entrypoint without going through the engine's `Pools` plumbing.

## Implementation strategy

**Stages do not need to leave the codebase in a working/compilable state at intermediate boundaries.** The rewrite is a single LLM-assisted greenfield effort. The stage breakdown organizes the work but does not gate intermediate ships. The legacy and new crates can coexist as non-compiling intermediate state during the rewrite; only the final delivery needs to compile and pass tests.

This is a rewrite, not an in-place edit. The scope of change (new wire shape, new key layout, new concurrency model, new compactor service) is too large for incremental modification. LLM-assisted greenfield is also more reliable when the spec is the source of truth.

### Stage 1: rename existing crate to legacy

```
git mv engine/packages/sqlite-storage engine/packages/sqlite-storage-legacy
```

Old code stays compilable and importable for reference. Don't delete anything yet.

### Stage 2: greenfield `engine/packages/sqlite-storage/`

Single crate, two top-level modules (`pump/` and `compactor/`) plus `takeover.rs`. No new crates.

```
sqlite-storage/src/
├── lib.rs                 — re-exports + crate-level docs
├── pump/                  — HOT PATH (used by pegboard-envoy)
│   ├── mod.rs             — exports ActorDb (the single per-actor handle)
│   ├── actor_db.rs        — ActorDb struct + new() constructor
│   ├── read.rs            — get_pages impl
│   ├── commit.rs          — commit impl (single-shot)
│   ├── keys.rs            — META sub-keys (head/compact/quota/compactor_lease, no /META/static), PIDX, DELTA, SHARD; PAGE_SIZE / SHARD_SIZE consts
│   ├── types.rs           — DBHead (no next_txid)
│   ├── udb.rs             — UDB wrappers (incl. COMPARE_AND_CLEAR)
│   ├── ltx.rs             — LTX V3 encode/decode  ← LIFTED
│   ├── page_index.rs      — DeltaPageIndex (RAM cache)  ← LIFTED
│   ├── quota.rs           — atomic-counter wrapper
│   ├── error.rs           — SqliteStorageError  ← LIFTED (pruned)
│   └── metrics.rs
├── compactor/             — BACKGROUND service (registered in run_config.rs)
│   ├── mod.rs             — re-exports
│   ├── subjects.rs        — SqliteCompactSubject typed wrapper
│   ├── publish.rs         — publish_compact_trigger(ups, actor_id)
│   ├── worker.rs          — start(config, pools) — UPS subscriber loop
│   ├── lease.rs           — /META/compactor_lease take/check/release
│   ├── compact.rs         — compact_default_batch — fold algorithm
│   ├── shard.rs           — per-shard fold + merge logic
│   └── metrics.rs
├── takeover.rs            — pegboard-side takeover-tx helper
└── test_utils/            — test_db, checkpoint_test_db  ← LIFTED
    └── mod.rs

tests/                     — all tests live here, not inline
├── pump_read.rs
├── pump_commit.rs
├── pump_keys.rs
├── compactor_lease.rs
├── compactor_compact.rs
├── compactor_dispatch.rs  — UPS memory-driver tests
└── takeover.rs
```

**Lift unchanged from `sqlite-storage-legacy/src/`:**

- `ltx.rs` → `pump/ltx.rs` — LTX V3 encoding/decoding. Battle-tested, format unchanged. Subtle correctness properties documented in `engine/CLAUDE.md` (zeroed 6-byte sentinel, varint page index, page-frame layout). Do not rewrite.
- `page_index.rs` → `pump/page_index.rs` — `DeltaPageIndex` data structure. Cache semantics carry forward unchanged.
- `error.rs` → `pump/error.rs` — error types. Lift, then prune variants that no longer apply (e.g., `FenceMismatch` becomes debug-only, multi-chunk error variants delete entirely).
- PIDX value encoding — raw big-endian `u64`. Same in the new design.
- `test_utils/` — lift unchanged.

**Do not lift:**

- The legacy quota fixed-point recompute math. `/META/quota` is a fresh FDB atomic counter; the legacy recompute math is **not** needed. It existed only because quota lived in the head's serialized blob and the encoded size depended on the field itself. With `/META/quota` as a separate atomic counter that property is gone, and the math is obsolete.

**Rewrite from scratch in `pump/`:**

- `actor_db.rs` — `ActorDb` struct (replaces legacy `SqliteEngine`). Per-actor handle owning UDB ref clone, actor_id, and `parking_lot::Mutex<DeltaPageIndex>` cache. Public surface: `new(udb, actor_id)`, `get_pages(pgnos)`, `commit(dirty_pages, db_size_pages, now_ms)`. No `open_dbs`, no `pending_stages`, no `compaction_tx`, no process-wide HashMap, no `Pump` struct.
- `commit.rs` — single-shot only. Delete `commit_stage_begin` / `commit_stage` / `commit_finalize`. The new `commit` is dramatically smaller. Reads `/META/head` only on the steady-state path; on the first commit for a new `ActorDb`, reads `/META/head` + `/META/quota` concurrently via `tokio::try_join!` to seed the in-memory quota cache.
- `read.rs` — simpler. No fence check, no `ensure_open` call. Reads `/META/head` only.
- `keys.rs` — new layout: `/META/{head,compact,quota,compactor_lease}` (no `/META/static`). Plus existing `/SHARD`, `/DELTA`, `/PIDX` unchanged. Owns the `PAGE_SIZE: u32 = 4096` and `SHARD_SIZE: u32 = 64` constants.
- `types.rs` — `DBHead` schema changes (drop `next_txid`, optional `generation` for debug-only). No `SqliteOrigin` enum. `SqliteMeta` shape is reduced to only the fields actually persisted on `/META/head` and `/META/compact`.
- `udb.rs` — add `COMPARE_AND_CLEAR` wrapper if `universaldb` doesn't already expose it. Otherwise lift unchanged.

**Not in `pump/`:**

- `open.rs` — delete entirely from new code. No `open()`, `close()`, `force_close()`, `ensure_open()`. Takeover logic moves to `takeover.rs` (Stage 4).
- `compaction/` — moves to `compactor/` (Stage 3).

All tests rewrite (wire shape change). New tests under `tests/`, not inline.

### Stage 3: greenfield `compactor/` module

Inside `sqlite-storage/src/compactor/`. Pure greenfield. Depends on `pump/` for storage primitives.

- `worker.rs` — `pub async fn start(config, pools) -> Result<()>`. Connects UPS, queue-subscribes `SqliteCompactSubject` with group `"compactor"`, runs select loop with `TermSignal`. Same shape as `engine/packages/pegboard-outbound/src/lib.rs:158-163`.
- `subjects.rs` — `SqliteCompactSubject` typed struct implementing `Display`. Convention from `pegboard::pubsub_subjects`.
- `publish.rs` — `publish_compact_trigger(ups, actor_id)`. Fire-and-forget; internally `tokio::spawn`s the publish so callers can't forget to detach.
- `lease.rs` — `/META/compactor_lease` take/check/release helpers. Pure UDB, no UPS.
- `compact.rs` — `compact_default_batch(&pump, actor_id)`. Port of `sqlite-storage-legacy/src/compaction/`, adapted for new key layout, lease-based concurrency, and `COMPARE_AND_CLEAR` for PIDX deletes.
- `shard.rs` — per-shard fold + merge logic. Lift the fold math from legacy `compaction/shard.rs`; rewrite the orchestration around it.

Service registration: add one line to `engine/packages/engine/src/run_config.rs`:

```rust
Service::new(
    "sqlite_compactor",
    ServiceKind::Standalone,
    |config, pools| Box::pin(sqlite_storage::compactor::start(config, pools)),
    true,
),
```

### Stage 4: debug-only invariant check (`takeover.rs`)

`sqlite-storage/src/takeover.rs`. Gated entirely behind `#[cfg(debug_assertions)]`. Not compiled in release.

- Public surface (debug only): `pub async fn reconcile(udb: &Database, actor_id: &str) -> Result<()>`.
- Behavior: scan PIDX / DELTA / SHARD prefixes; classify any rows as orphans (above EOF, above `head_txid`, dangling DELTA refs, etc.); if any are found → structured error log + panic in tests. Does NOT delete anything; this is invariant verification, not cleanup.
- Lift: orphan classification logic from `sqlite-storage-legacy/src/open.rs::build_recovery_plan` for the classification rules. Drop the mutation-builder code — debug-only just asserts.
- Wired into `ActorDb::new` under the same `#[cfg(debug_assertions)]` gate. Pegboard does not call this. Pegboard does not import `sqlite_storage::takeover` at all; it imports nothing sqlite-related on takeover.

Release build behavior: `ActorDb::new` does no UDB work. Trust v2 invariants. If an invariant violation actually occurs in production, behavior is undefined (acceptable per goal 5).

### Stage 5: rewire pegboard-envoy

Mostly net deletion.

- `engine/packages/pegboard-envoy/src/conn.rs` — **delete the `active_actors` HashMap entirely.** The conn holds no authoritative per-actor state. Add a single field `actor_dbs: scc::HashMap<String, Arc<sqlite_storage::pump::ActorDb>>` for the per-WS-conn cache. Entries are upserted lazily by the SQLite request handlers (first `get_pages` or `commit` for an actor on this conn calls `entry_async(...).or_insert_with(|| Arc::new(ActorDb::new(udb.clone(), actor_id)))`). See "Why no active-actor tracking on the WS conn" above for the reconnection rationale.
- `engine/packages/pegboard-envoy/src/actor_lifecycle.rs` — **delete `start_actor` entirely** (and the `open()` / `close()` / `force_close()` call sites at lines `189-201`, `237-250`). **Keep `stop_actor`**, but its sole responsibility shrinks to `conn.actor_dbs.remove_async(&actor_id).await` — no `close()` call, no `active_actors` mutation, no generation tracking.
- `engine/packages/pegboard-envoy/src/conn.rs` command dispatch — drop the `CommandStartActor` branch entirely (the WS conn doesn't react to start_actor; actor presence is implicit via the SQLite request stream). Keep the `CommandStopActor` branch routing to the new lightweight `stop_actor`.
- When pegboard destroys an actor (lifecycle teardown that clears `/META`, `/SHARD`, `/DELTA`, `/PIDX`), it must **also clear `/META/compactor_lease`** for that actor in the same teardown transaction. Otherwise dead lease keys accumulate in UDB indefinitely.
- `engine/packages/pegboard-envoy/src/sqlite_runtime.rs` — delete `CompactionCoordinator` spawn. Hold the conn's UDB ref and a UPS handle.
- `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs` — delete open/close/stage handlers. SQLite request handlers (`get_pages`, `commit`) look up or lazily insert the `Arc<ActorDb>` in `conn.actor_dbs`, then call methods directly: `actor_db.get_pages(pgnos).await?` / `actor_db.commit(...).await?`. After a commit-crosses-threshold, call `sqlite_storage::compactor::publish_compact_trigger(&ups, actor_id)`.

### Stage 6: wire-protocol schema

- New schema in `engine/sdks/schemas/envoy-protocol/`. The current schema is `v2.bare`; this is a VBARE bump to `v3.bare` (or whatever the next version is — confirm with `engine/CLAUDE.md` "VBARE migrations" rules).
- **Write the new protocol as a fresh schema. No reverse compatibility, no field-by-field converter** — breaking changes are unconditionally acceptable since the system has not shipped to production.
- Note: this contradicts the general `engine/CLAUDE.md` "VBARE migrations" guidance ("never byte-passthrough; always reconstruct"). That rule exists for production-shipped schemas; this one hasn't shipped, so the rule doesn't apply.
- Update `PROTOCOL_VERSION` constants in matched envoy-protocol crates.

### Stage 7: delete legacy

Once Stage 5 is complete and tests pass:

```
rm -rf engine/packages/sqlite-storage-legacy
```

Drop the workspace entry, update any remaining imports.

### Stage 8: update `engine/CLAUDE.md`

Update the SQLite-storage section in `engine/CLAUDE.md` to match the new design:

- **Remove**: the bullet that says "compaction must re-read META inside its write transaction and fence on `generation` plus `head_txid`." That note is obsoleted by the META key split — compaction writes `/META/compact` (its own key) while commits write `/META/head`, so no shared write target exists and the fence is unnecessary.
- **Remove**: the takeover "in one atomic_write" bullet entirely. There is no takeover work in release; pegboard's reassignment transaction does not touch sqlite-storage. v2's atomic single-shot commits make orphans impossible by construction. Debug builds run an invariant scan via `takeover::reconcile` (see Stage 4) but this is verification-only, not cleanup.
- **Verify** (still applies): "shrink writes must delete above-EOF PIDX rows and SHARD blobs in same commit/takeover transaction" — this rule is preserved and enforced (see the Shrink race during compaction subsection).
- **Update**: any "process-wide `OnceCell` SqliteEngine" reference becomes "per-actor `ActorDb` instances cached on the WS conn." Any `CompactionCoordinator` reference is gone (replaced by the standalone compactor service).
- **Remove**: STAGE-related notes (multi-chunk staging is gone; the STAGE/ key prefix does not exist in the new design).
- **Keep**: PIDX value encoding (raw big-endian `u64`) — unchanged.
- **Update test conventions**: tests live in `engine/packages/sqlite-storage/tests/`, not inline. This overrides any older "keep coverage inline" bullet.

## Open questions

- **PIDX-key-count optimization.** Today commit reads existing PIDX entries per pgno for quota math (`commit.rs:285-303`) when cache is cold. An incremental counter in META would skip those reads. Worth doing? Direct latency win on cold-cache commits, small META change. Not required by the three goals but serves goal 3.
- **Compactor lag SLO.** Need a target for `head_txid - materialized_txid` lag and HPA tuning. What's the alert threshold?
- **UPS partition handling.** If pegboard-envoy publishes during a UPS/NATS outage, the trigger is lost. Recovery: next commit republishes. Acceptable?
- **Single-shot commit size limit.** UDB chunks values internally but there's a practical upper bound. What's the cutoff before we'd need to reintroduce streaming? Likely above any actor's typical write set, but worth verifying.

## Future work

Out of scope for this spec but worth scoping next:

- **Migrate KV to SQLite.** Today actor KV (`actor_kv` ops) is a separate UDB-backed key/value store, distinct from the SQLite engine. With stateless SQLite in place and the compactor handling background work, the KV store becomes a candidate to fold into a single `_kv` table on the actor's SQLite database. Benefits: one storage backend per actor, KV transactions become real SQL transactions, KV reads benefit from the same PIDX/SHARD caching, no separate quota accounting. Open questions: backwards-compat for existing KV data, transactional semantics across what was previously two stores, whether the existing 128 KB KV value limit changes.

## Files affected

### Renamed
- `engine/packages/sqlite-storage/` → `engine/packages/sqlite-storage-legacy/` (Stage 1; deleted entirely in Stage 7).

### Greenfield in `sqlite-storage/src/pump/` (Stage 2)
- `mod.rs` — exports `ActorDb` (the per-actor handle, replaces legacy `SqliteEngine`).
- `actor_db.rs` — `ActorDb` struct: per-actor UDB ref + `actor_id` + `Mutex<DeltaPageIndex>` cache. Public surface: `new(udb, actor_id)`, `get_pages(pgnos)`, `commit(dirty_pages, db_size_pages, now_ms)`.
- `commit.rs` — single-shot only.
- `read.rs` — no fence, no `ensure_open`.
- `keys.rs` — new META sub-key layout (`head`/`compact`/`quota`/`compactor_lease`, no `/META/static`); owns `PAGE_SIZE` / `SHARD_SIZE` constants.
- `types.rs` — `DBHead` minus `next_txid`. No `SqliteOrigin`.
- `quota.rs` — atomic-counter wrapper; owns `SQLITE_MAX_STORAGE_BYTES` constant; performs cap enforcement on commit.
- `udb.rs` — adds `COMPARE_AND_CLEAR` wrapper if needed.

### Lifted unchanged into `sqlite-storage/src/pump/` (Stage 2)
- `ltx.rs`
- `page_index.rs`
- `error.rs` (with unused variants pruned)

### Greenfield in `sqlite-storage/src/compactor/` (Stage 3)
- `mod.rs` — re-exports.
- `worker.rs` — `start(config, pools)` UPS subscriber loop.
- `subjects.rs` — `SqliteCompactSubject`.
- `publish.rs` — `publish_compact_trigger(ups, actor_id)`.
- `lease.rs` — `/META/compactor_lease` take/check/release.
- `compact.rs` — `compact_default_batch` (lease + COMPARE_AND_CLEAR).
- `shard.rs` — per-shard fold logic (lifted math, rewritten orchestration).

### Other new files in `sqlite-storage/src/`
- `lib.rs` — re-exports `pump`, `compactor`, `takeover`.
- `takeover.rs` — debug-only invariant check (Stage 4); exports `reconcile` under `#[cfg(debug_assertions)]`. Not compiled in release.

### Tests (Stage 2-3)
- All test files live under `engine/packages/sqlite-storage/tests/`. Inline `#[cfg(test)] mod tests` blocks in `src/` are not used. This is the only acceptable test layout for this crate.
- `tests/pump_read.rs`, `tests/pump_commit.rs`, `tests/pump_keys.rs` — hot-path coverage.
- `tests/compactor_lease.rs`, `tests/compactor_compact.rs`, `tests/compactor_dispatch.rs` — lease, compaction, UPS dispatch (UPS memory-driver tests).
- `tests/takeover.rs` — debug-only invariant scan coverage (orphan classification asserts).

### Deleted (lived in legacy only)
- `sqlite-storage-legacy/src/open.rs` — takeover logic moves to Stage 4.
- `sqlite-storage-legacy/src/compaction/` — moves to `compactor/`.

### Modified
- `engine/packages/engine/src/run_config.rs` — register `sqlite_compactor` as `ServiceKind::Standalone` with `restart=true`, passing `CompactorConfig::default()` inline.
- Pegboard takeover code — **no changes in release.** Pegboard does not call into sqlite-storage on takeover. Debug builds only: `ActorDb::new` calls `sqlite_storage::takeover::reconcile` for invariant verification; pegboard remains untouched.
- `engine/packages/pegboard-envoy/src/actor_lifecycle.rs` — delete `open`/`close`/`force_close` call sites (lines `189-201`, `237-250`). The per-conn cache is dropped via the WS-state struct's `Drop`; no manual invalidation API.
- `engine/packages/pegboard/src/...` actor-destroy lifecycle — clear `/META/compactor_lease` for the actor in the same teardown transaction that clears `/META`, `/SHARD`, `/DELTA`, `/PIDX`.
- `engine/packages/pegboard-envoy/src/sqlite_runtime.rs` — delete `CompactionCoordinator` spawn; hold a UDB ref clone and a UPS handle on the conn.
- `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs` — delete open/close/stage handlers; SQLite request handlers lazily upsert into `conn.actor_dbs` and call `actor_db.get_pages(...)` / `actor_db.commit(...)` directly; call `compactor::publish_compact_trigger(...)` on commit threshold.
- `engine/packages/pools/` — adds `NodeId` type and `pools.node_id() -> NodeId` accessor (engine-wide change, lands before this spec's compactor work).
- `engine/CLAUDE.md` — Stage 8 updates (see Implementation strategy Stage 8 for the full list of bullets to remove, rewrite, and verify).

### Schema (Stage 6)
- `engine/sdks/schemas/envoy-protocol/v3.bare` — new schema version with collapsed protocol. Fresh schema; no field-by-field converter from v2 (the system has not shipped, breaking changes are unconditional).
