# depot

Per-database storage engine for Rivet's SQLite-on-FDB system. Owns durability (FDB hot tier), retention (S3 cold tier, 30-day PITR), branching, restore points, and compaction.

Depot does **not** own the SQLite VFS, the wire protocol to envoys, or actor lifecycle. Those live in `depot-client`, `pegboard-envoy`, and `pegboard`. This crate is a Rust library; it has no main loop of its own. Code that wants storage holds a `depot::Db` and calls methods on it. Workflows in this crate (`registry()`) are registered into the engine's gasoline runtime.

For architectural invariants (single-writer, no-local-files, lazy-read-only, per-commit granularity) see `CLAUDE.md`. This README is the **map**: where things live and how data flows.

## Top-level layout

```
src/
  lib.rs              entry point — exports `Db`, registers workflows
  conveyer/           the Db itself: storage layer, branches, reads, commits
  compaction/         shared compaction types + companion infra
  workflows/          gasoline workflows (manager + 3 companions)
  cold_tier/          S3 / filesystem / disabled / faulty backends
  gc/                 branch GC pin computation
  burst_mode.rs       hot quota cap, branch-lag driven
  inspect.rs          internal /depot/inspect/* read-only debug surface
  metrics.rs          shared Prometheus metrics
  doctor.rs           diagnostic helpers
  takeover.rs         debug-only takeover reconcile
  fault/              test-only fault injection (test-faults feature)
```

## The Db type

`depot::Db` (defined in `conveyer/db.rs`, re-exported as `depot::conveyer::Db`) is the only handle anyone outside this crate should hold. It is:

- **Pod-stateless.** Every method self-describes its fence; in-memory state is perf cache only.
- **Bucket-scoped.** Constructed with a bucket id (engine namespace id); resolves DBPTR lazily.
- **Cheap to drop.** WS conn drop = cache drop, no `close()`.

The Db is constructed by `pegboard-envoy` per WebSocket connection and cached on the conn in an `scc::HashMap<database_id, Arc<Db>>`. New Db instances seed BUCKET_PTR/BUCKET_BRANCH lazily on first use.

## conveyer/ — the storage layer

`conveyer` is depot's "real work" module. Everything below is reached through `Db` methods.

| Path | Owns |
|---|---|
| `db.rs` | `Db` struct, cache snapshot, lifecycle |
| `commit.rs` + `commit/` | commit apply, branch init, dirty-marker signaling, truncate cleanup |
| `read.rs` + `read/` | `get_pages` planning, PIDX/cache lookup, SHARD fallback, cold reads, fill workers |
| `branch.rs` + `branch/` | branch resolution, bucket catalog, fork/derive, lifecycle rollback |
| `restore_point.rs` + `restore_point/` | restore point create / delete / resolve / restore / recompute |
| `pitr_interval.rs` | per-branch `PITR_INTERVAL` rows for time-based coverage |
| `policy.rs` | PITR + shard-cache policy resolution (db → bucket → defaults) |
| `quota.rs` | `/META/quota` atomic-add counter accounting |
| `history_pin.rs` | `DB_PIN` records (restore point, db fork, bucket fork) |
| `page_index.rs` | PIDX entry encoding (big-endian page → big-endian txid) |
| `ltx.rs` | LTX V3 file format encode/decode |
| `keys.rs` | branch-local key prefixes (`BR/{branch_id}/...`) |
| `constants.rs` | tunables: retention windows, fork depth, shard size |
| `udb.rs` | UDB helpers (subspace-aware range scans) |
| `types.rs` + `types/` | persisted payload structs (branch, restore points, compaction, etc.) |
| `metrics.rs` | conveyer-internal Prometheus metrics |
| `error.rs` | `SqliteStorageError` variants |
| `debug.rs` | historical-read debug helpers (debug builds only) |

## compaction/ — companion glue

| File | Owns |
|---|---|
| `types.rs` | shared persisted state: `DbManagerState`, `ManagerActiveJobs`, signals, planned-job shapes |
| `shared.rs` | shared planners: `plan_hot_job`, `plan_cold_job`, `plan_reclaim_job`, FDB snapshot reads |
| `companion.rs` | shared loop helpers for the three companion workflows |
| `test_driver.rs` | `DepotCompactionTestDriver` (force-compaction tests, `test-faults` only) |
| `test_hooks.rs` | debug-only hooks for race tests |

## workflows/ — the four gasoline workflows

One module per workflow. The manager + three companions form a branch-scoped quartet, dispatched on first commit and identified by the `DATABASE_BRANCH_ID_TAG` unique tag.

```
DbManagerWorkflow         (db_manager.rs)
  ├─ dispatches once at startup ─►
  ├─ DbHotCompacterWorkflow   (db_hot_compacter.rs)
  ├─ DbColdCompacterWorkflow  (db_cold_compacter.rs)
  └─ DbReclaimerWorkflow      (db_reclaimer.rs)
```

- **DbManagerWorkflow** — planning + publish + delete authority. Each iteration: listen for signals (deltas, job-finished, force-compaction, destroy) → run `RefreshManager` activity (FDB snapshot, recompute planned jobs) → emit `ManagerEffect`s → dispatch work to companions.
- **DbHotCompacterWorkflow** — stages hot-shard LTX blobs under `CMP/stage/{job_id}/hot_shard`. Never publishes. The manager installs.
- **DbColdCompacterWorkflow** — uploads cold objects via the `ColdTier` trait. Manager publishes refs. Only runs when `sqlite.workflow_cold_storage` is configured.
- **DbReclaimerWorkflow** — deletes FDB SHARD/PIDX/COMMIT rows, retires `CMP/cold_shard` refs, deletes S3 objects after grace window, runs shard-cache eviction.

### Manager lifecycle

Hot compaction is signal-driven. Cold and reclaim run on their own one-shot deadlines that signals re-arm.

| Field | Interval | Behavior |
|---|---|---|
| `next_cold_check_at_ms` | `MANAGER_COLD_COMPACTION_INTERVAL_MS` (2min) | Set to `Some(now+interval)` by any signal when currently `None`. Cleared to `None` when the deadline fires (or a `ForceCompaction(cold)` runs). |
| `next_reclaim_check_at_ms` | `MANAGER_RECLAIM_INTERVAL_MS` (10min) | Same shape as cold. |

The listen waits until the soonest of those two deadlines or the next signal. With both `None` the listen blocks until a signal arrives.

**Per-iteration wake triggers** (computed after `RefreshManager`):

- `hot` ← any signal received this iteration. Hot has no timer; it only dispatches when a signal woke us.
- `cold` ← cold deadline elapsed, or `ForceCompaction(cold)` is pending.
- `reclaim` ← reclaim deadline elapsed, or `ForceCompaction(reclaim)` is pending.

`manager_effects_after_refresh` only dispatches a planned hot/cold/reclaim job when the matching trigger is set and the `active_jobs.*` slot is empty.

**`schedule_next_wake` rules:**

- A trigger that fired (cold or reclaim) clears its deadline to `None`.
- If any signal arrived this iteration, any `None` cold/reclaim deadline is armed at `now + interval`. Already-armed deadlines are left alone.

**Initial state.** Fresh `DbManagerState::new` has both deadlines `None`. The manager parks on a deadline-less listen until the first signal arrives, then arms cold/reclaim for their first cycles.

**Timestamps.** `ctx.create_ts()` returns workflow *creation* time and never advances. To compute scheduling deadlines, `RefreshManager` returns `refreshed_at_ms` (the activity's `ctx.ts()`, which is real recorded execution time). The workflow uses that as `now_ms` for `schedule_next_wake`.

## cold_tier/ — S3 backend abstraction

| File | Owns |
|---|---|
| `mod.rs` | `ColdTier` trait + relative-key validation |
| `disabled.rs` | `DisabledColdTier` — default, fails all ops |
| `filesystem.rs` | `FilesystemColdTier` — local-fs stand-in for S3 (tests + dev) |
| `s3.rs` | `S3ColdTier` — production backend |
| `faulty.rs` | `FaultyColdTier` — fault-injection wrapper around any backend |
| `config.rs` | runtime selection from Rivet config |

`Db::new_with_cold_tier` injects the backend. `Db` reads/writes cold via this trait; nothing else touches S3 directly.

## Other top-level modules

- **`burst_mode.rs`** — hot quota cap. Reads FDB-derived branch lag + workflow `CMP/root.cold_watermark_txid` and decides whether to reject commits. Per-pod stateless.
- **`gc/`** — branch GC pin computation. Refcount + root + desc-pin + restore-point-pin math. Used by cold sweeps and debug estimates.
- **`inspect.rs`** — read-only debug surface. `api-peer` mounts thin `/depot/inspect/...` handlers that call into here. Not a public SDK.
- **`metrics.rs`** — shared Prometheus metrics (all labeled with `node_id`).
- **`doctor.rs`** — diagnostic helpers for ad-hoc investigation.
- **`takeover.rs`** — debug-only, reconciles takeover on Db construction.
- **`fault/`** — fault-injection actions, points, controller. Gated behind the `test-faults` feature.

## Data routing

### Commit path (write)

```
pegboard-envoy WS handler
       │  validates page numbers + sizes at trust boundary
       ▼
Db::commit (conveyer/commit/apply.rs)
       │  one UDB tx:
       │   • appends DELTA blob
       │   • updates PIDX rows
       │   • writes COMMITS/VTX row (versionstamped)
       │   • updates /META/head + /META/quota
       │   • writes SQLITE_CMP_DIRTY marker if newly dirty
       ▼
CompactionSignaler → unique DbManagerWorkflow (by branch_id tag)
       │  signals DeltasAvailable
       ▼
manager refresh → maybe plans hot job → dispatches to DbHotCompacterWorkflow
```

Commit returns durable after FDB commit, *not* after cold upload. RPO = FDB durability.

### Read path

```
SQLite VFS xRead (depot-client)
       ▼
Db::get_pages (conveyer/read/plan.rs)
       │  one UDB tx for hot-tier reads:
       │   • read /META/head + PIDX rows (for current branch)
       │   • walk ancestry caps for fork descendants
       │   • fetch DELTA/SHARD blobs
       │   • fall through to CMP/cold_shard refs if missing
       ▼
ColdTier::get  ──────► S3 / filesystem  (outside UDB tx)
       │  on cold hit, schedules background FDB shard-cache refill
       ▼
returns FetchedPage vec, sparse zero-fill only when no source exists
```

### Compaction path

```
manager RefreshManager activity (FDB snapshot)
       │  computes planned_{hot,cold,reclaim}_job from snapshot + policy
       ▼
manager dispatches RunHotJob / RunColdJob / RunReclaimJob signals
       ▼
companion workflow runs activity, signals *JobFinished back
       ▼
manager InstallHotOutput / PublishColdOutput / FinishReclaimJob
       │  one UDB tx per install/publish:
       │   • copies staged SHARD → reader-visible SHARD
       │   • advances CMP/root
       │   • clears matching PIDX rows with COMPARE_AND_CLEAR
       ▼
on cold publish reject: routes uploaded refs through reclaimer orphan cleanup
```

## Where to read next

For deeper detail beyond this map:

- `CLAUDE.md` (this dir) — binding invariants and design constraints.
- `docs-internal/engine/sqlite/storage-structure.md` — exact FDB + S3 key layout.
- `docs-internal/engine/sqlite/components.md` — component responsibilities in prose.
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md` — PITR / branching / retention / cold tier rationale.
- `.agent/specs/depot-stateless.md` and `.agent/specs/sqlite-pitr-fork.md` — design specs.

For tests, see `engine/packages/depot/tests/` (integration) and `engine/packages/depot-client/` (VFS-level integration + fault injection).
