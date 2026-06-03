# depot

The per-database Depot engine. FDB is the hot tier; S3 is the cold tier (PITR + retention). LTX V3 file format throughout.

For implementation-level wiring (key encodings, RTT count for `commit`, LTX byte layout, compaction shard rules, test harness setup) see the `## Depot tests` and `## Pegboard Envoy` sections in `engine/CLAUDE.md`. The bullets here are the **architectural constraints** that shape what does and does not belong in this package.

## Hard constraints (binding floor)

These come from `r2-prior-art/.agent/research/sqlite/requirements.md` and supersede any contradiction in older specs.

- **Single writer per database.** Pegboard exclusivity (lost-timeout + ping protocol) holds. Never two writers on the same database at the same instant. Do not implement MVCC, page-versioned read-set tracking, optimistic conflict detection at commit, content-addressed dedup, or commit-intent logs. mvSQLite's PLCC / DLCC / MPC / versionstamps are explicitly out — single writer makes them dead weight.
- **No local SQLite files. Ever.** Not on disk, not on tmpfs, not as a hydrated cache file. The authoritative store is FDB (hot) and S3 (cold). The VFS speaks to them directly. Forks do not materialize local files. Anything that puts a real SQLite file on a pegboard node is out of scope.
- **Lazy read only.** No bulk pre-load at database open. Pages are fetched on demand from the hot tier. The per-WS-conn PIDX cache + flattened ancestry cache amortize the per-fetch cost. Fork warmup is a *background* cold→hot copy, not a synchronous bulk hydrate at first SQL statement.
- **Per-commit granularity.** The smallest addressable unit is a committed transaction. No sub-commit PITR, no WAL-frame-level shipping.

## Statelessness contract

- **The hot path (pegboard-envoy → Db) is pod-stateless.** Every request self-describes its fence (branch_id, expected generation/head_txid in debug) and runs against the current FDB state. In-memory state on `Db` is allowed only as a **perf cache** — never as the source of truth, never as a correctness fence, never as something that must survive across requests. WS conn drop = cache drop. No `open` / `close` lifecycle.
- **Workflow compaction is the only compaction authority.** Do not reintroduce standalone hot/cold/eviction compactor modules or tests.
- **Pegboard-envoy WS conn is stateless w.r.t. database identity.** Envoys can reconnect to a different worker mid-flight while a database is active; the new worker never sees the original actor start command. The only per-conn state is the perf-only `scc::HashMap<database_id, Arc<Db>>`, populated lazily by SQLite request handlers. No active-database registry, no presence tracking, no start handler. Stop only evicts the cache entry.
- **Defensive runtime checks for "this should never happen" are `#[cfg(debug_assertions)]` only.** Trust the surrounding contracts (pegboard exclusivity, FDB tx isolation, workflow uniqueness). Belt-and-suspenders fences that duplicate work the surrounding system already does belong in debug builds, not in release.

## Concurrency model

- **Pegboard exclusivity is the only writer fence in release.** No separate KV concurrency fence. Defensive in-tx "two writers detected" checks are `#[cfg(debug_assertions)]` only.
- **The DB manager workflow owns compaction publish and deletion authority.** Hot/cold/reclaimer companions only stage, upload, or delete manager-authorized work.
- **PIDX deletes use FDB `COMPARE_AND_CLEAR`** so commit-vs-compaction races no-op on stale entries. Takes no read conflict range.
- **Fork-vs-GC race is OCC, not margin-based.** Database fork derivation regular-reads the parent `restore_point_pin` and `depot::gc::read_branch_gc_pin_tx` inside the fork tx; concurrent GC pin advance aborts the fork via OCC. Hand-waved time margins are not a substitute.

## Storage layout

- **All PITR database state is per-branch.** Db resolves DBPTR, caches the branch id as a perf cache, and writes hot-path data under top-level `[0x02][0x30]/{branch_id}/<suffix>` keys.
- **Db is bucket-scoped.** New Db instances receive the engine namespace id as a Depot bucket id, lazily seed BUCKET_PTR/BUCKET_BRANCH, and write DBPTR under the resolved bucket branch.
- **Database pointer resolution walks bucket branch parents on DBPTR miss.** Bucket-branch tombstones return `DatabaseNotFound` and must not fall back to legacy database-scoped storage.
- **Db branch and PIDX caches must be invalidated when DBPTR moves.** Rollback swaps can make cached branch id, quota, and PIDX rows stale; resolve DBPTR as the source of truth before using cached branch-local state.
- **Db async perf caches use async-safe primitives.** Use `tokio::sync::RwLock` for mutable cache snapshots, keep `DeltaPageIndex` unwrapped because its `scc::HashMap` owns concurrency, and use atomics for synchronous metering snapshots.
- **Db branch read caches publish atomically.** Keep branch id, ancestry, access bucket, and PIDX index in one `CacheSnapshot` under `cache_snapshot`.
- **Debug-only `Db` constructor reconciliation must stay non-blocking.** Schedule takeover reconcile on the current Tokio runtime or a detached background thread; never join it from `Db::new*`.
- **Legacy database-scoped storage is not a read fallback.** Db reads use branch-scoped META, COMMITS, VTX, PIDX, DELTA, and SHARD keys.
- **Legacy database-scoped key helpers are v1 pegboard compatibility only.** Do not use `meta_head_key`, `delta_prefix`, `pidx_delta_prefix`, or `shard_prefix` for current Depot invariant scans.
- **Branch ancestry reads use branch-aware sources.** The PIDX cache is safe only when the read plan has one source branch; multi-branch ancestry reads must scan PIDX with branch identity.
- **Flattened ancestry caches store versionstamp caps.** Resolve cached parent versionstamps to txids inside each read transaction before PIDX/SHARD lookup.
- **Ancestor PIDX reads are capped by fork point.** Ignore PIDX owners newer than that source's cap, scan capped DELTA history for requested pages, then fall through to the latest SHARD version at or below the cap.
- **Workflow compaction read fallback preserves hot-tier order.** PIDX/DELTA wins, branch SHARD fallback is next, and `CMP/cold_shard` refs are considered only before legacy cold manifest layers.
- **Conveyer read domains live behind the `conveyer/read.rs` facade.** Keep read planning, PIDX/cache helpers, SHARD fallback, cold-tier reads, and transaction scan helpers under `conveyer/read/*.rs`.
- **Sparse in-range reads zero-fill only when no source exists.** Corrupted or broken source blobs must return an explicit read error, not a zero page.
- **PIDX-owned DELTA gaps are broken source coverage.** Missing delta chunks may fall back only to valid SHARD or cold coverage; otherwise return `ShardCoverageMissing` or a decode error.
- **Compaction cold-shard reads must revalidate live refs.** After fetching cold bytes, re-read the `CMP/cold_shard` ref under `Serializable`; missing or changed refs return `ShardCoverageMissing`, not sparse zero-fill.
- **Read-path tests should seed branch-owned storage.** Use `Db::commit` or `BR/{branch}` keys, not pre-PITR database-scoped `META`/`PIDX`/`DELTA`/`SHARD` keys.
- **Conveyer branch domains live behind the `conveyer/branch.rs` facade.** Keep branch resolution, bucket catalog/list/delete, fork/derive, lifecycle rollback, and shared branch helpers under `conveyer/branch/*.rs`.
- **Conveyer commit domains live behind the `conveyer/commit.rs` facade.** Keep commit apply, branch initialization, dirty-marker signaling, truncate cleanup, and transaction helpers under `conveyer/commit/*.rs`.
- **Workflow cold storage is opt-in through Rivet config.** When absent, FDB SHARD data remains the durable source of truth and cold upload/delete/reclaim planning must stay disabled.
- **Cold-disabled reads must fail on cold-only coverage.** If a read finds a `CMP/cold_shard` or legacy cold candidate but `Db` has no cold tier, return `ShardCoverageMissing` instead of zero-filled bytes.
- **Cold-ref reads refill the FDB shard cache in the background.** Tests that need SHARD absence should disable fill workers or assert before the background queue drains.
- **Shard-cache fill workers use cloned `async-channel` receivers.** Do not reintroduce a shared `Mutex<mpsc::Receiver>` around worker dispatch.
- **Shard-cache fill idle waits pre-arm `Notify` before checking `outstanding`.** `notify_waiters()` does not store permits.
- **Shard-cache metrics use fixed outcome labels only.** Keep branch, database, shard, object, restore point, and bucket identifiers out of metric labels.
- **Workflow compaction integration tests configure filesystem cold storage through `TestCtx`.** Do not use runtime global cold-tier overrides for integration coverage.
- **Fault scenarios install workflow cold-tier test overrides by branch id.** Use `test_hooks::install_workflow_cold_tier_for_test` only when the workflow must share a fault-controller-backed tier with VFS reads.
- **Truncate cleanup prunes the boundary SHARD instead of blindly deleting it.** `shard_id = pgno / SHARD_SIZE`, so page 64 and page 65 both map to shard 1.
- **Debug historical reads cannot trust PIDX.** PIDX is the current owner map, so `debug::read_at` scans DELTA history up to the target txid before falling through to SHARD/cold layers.
- **Fresh fork branches use `/META/head_at_fork` until first commit.** The first local commit treats it as the previous `DBHead`, writes `/META/head`, and clears `/META/head_at_fork` in the same transaction.
- **PITR tunable constants live in `conveyer/constants.rs`.** Import shared limits and retention windows from there instead of duplicating literals.
- **PITR and shard cache policy storage lives in `conveyer/policy.rs`.** Payloads and defaults live in `conveyer/types/policy.rs`; database overrides fall back to bucket policies, then defaults.
- **PITR interval coverage storage lives in `conveyer/pitr_interval.rs`.** Payloads live in `conveyer/types/compaction.rs`, and branch-local keys use `BR/{branch}/PITR_INTERVAL/{bucket_start_ms_be}`.
- **Restore-point creation must re-read the target `COMMITS/{txid}` row in the pin-write transaction.** Missing history aborts with `RestoreTargetExpired`; never write a Ready pin for deleted history.
- **Workflow compaction key helpers live in `conveyer/keys.rs`.** Branch-local manifest/staging keys stay under `BR/{branch_id}/CMP`, while global pin/proof/dirty indexes use reserved partitions `0x70..=0x75`.
- **Workflow compaction signal and durable state contracts live in `compaction/types.rs`.** Keep them serde-compatible for Gasoline runtime use; do not add vbare helpers unless they become persisted or BARE wire payloads.
- **Workflow compaction uses one module per workflow.** Keep `DbManagerWorkflow`, `DbHotCompacterWorkflow`, `DbColdCompacterWorkflow`, and `DbReclaimerWorkflow` in `workflows/{db_manager,db_hot_compacter,db_cold_compacter,db_reclaimer}.rs`; shared helpers live under `compaction/`.
- **Workflow compaction input fingerprints use SHA-256 over length-prefixed chunks.** Preserve the existing field order when adding fingerprint inputs.
- **Workflow compaction branch workflows use `DATABASE_BRANCH_ID_TAG` as their only stable unique tag.** The manager stores required companion workflow ids before entering its durable loop.
- **Workflow compaction manager input may carry actor context for logs.** Keep workflow uniqueness branch-only, and pass runtime actor ids through `DbManagerInput.actor_id` when available.
- **Workflow compaction jobs carry the database branch lifecycle generation.** Stage, publish, and reclaim activities must reject stale work when the branch record is missing, frozen, or generation-changed.
- **Workflow compaction manager planning runs through `RefreshManager` activities.** Recompute branch lag from FDB, record one active job in durable state, then signal the persistent companion workflow.
- **Workflow compaction planned jobs use typed lanes.** `RefreshManagerOutput` carries concrete hot/cold/reclaim planned-job structs; dispatch helpers should not unpack a generic input-range enum.
- **Workflow compaction manager active jobs use typed lanes.** Store dispatched hot/cold/reclaim work under `DbManagerState.active_jobs` with concrete input ranges.
- **Workflow compaction manager stops use explicit reasons.** Store stop requests as `ManagerStopReason` and route explicit destroy plus branch-not-live refresh through the shared stop effect.
- **Workflow compaction manager helpers orchestrate only.** Keep FDB snapshots, UDB writes, and cold-tier object calls inside activities.
- **Workflow compaction manager orchestration is effect-driven.** Signal and refresh helpers emit `ManagerEffect` values; the executor owns workflow API calls, activities, timestamps, and signal dispatch.
- **Workflow compaction joined signals expose `database_branch_id()`.** Manager and companion loops should filter unrelated branch signals once before variant-specific handling.
- **Workflow companion signal handlers are kind-specific.** Hot, cold, and reclaim loops choose typed signal handlers first; storage work stays inside activities.
- **Workflow force-compaction state lives in `ForceCompactionTracker`.** Manager code should use tracker methods for request, result, attempted-job, and completed-job bookkeeping.
- **Workflow force-compaction tests use `DepotCompactionTestDriver` under `depot/test-faults`.** Wait on durable `force_compactions.recent_results`, not planner deadlines or arbitrary timing thresholds.
- **Workflow fault-hook tests should assert durable forced results.** Manager retries can surface a terminal error and still settle later state in the same forced cycle.
- **Workflow hot compaction writes only staged LTX blobs under `CMP/stage/{job_id}/hot_shard`.** Successful staged results stay as the manager's active hot job until the manager install path publishes or rejects them.
- **Workflow hot install runs in the DB manager.** It accepts only the active job fingerprint, copies staged hot shards to reader-visible `SHARD`, advances `CMP/root`, and clears matching PIDX rows with `COMPARE_AND_CLEAR`.
- **Workflow hot planning treats `DB_PIN` records as exact coverage targets.** Preserve pinned txids in `HotJobInputRange.coverage_txids`; do not round them up to the latest head.
- **Workflow hot planning treats selected PITR intervals as exact coverage targets.** Include selected interval txids in coverage fingerprints and publish their `PITR_INTERVAL` rows during hot install.
- **Workflow FDB reclaim is manager-planned and reclaimer-executed.** Recompute DB pins, PIDX dependencies, and SHARD coverage before signaling the reclaimer, then revalidate them in the delete activity and clear hot rows with `COMPARE_AND_CLEAR`.
- **Workflow shard cache eviction is a reclaimer lane.** Plan it from effective `ShardCachePolicy` plus branch access buckets, require matching `COLD_SHARD` refs, and never clear cold refs during FDB SHARD eviction.
- **Future DB pins can rely on older cold-backed SHARD coverage.** A pin at txid 2 does not retain SHARD txid 1 by exact match; eviction is safe only because the matching `CMP/cold_shard` remains readable.
- **Workflow FDB reclaim treats unexpired `PITR_INTERVAL` rows as soft pins.** Expired interval rows are reclaim inputs and must be compare-cleared with the planned value.
- **Workflow reclaim commit scans are bounded by exact `branch_commit_key` ranges.** Do not full-scan `branch_commit_prefix` and locally filter high txids.
- **Workflow FDB reclaim resolves bucket fork facts before deletion.** Treat missing or ambiguous `BUCKET_FORK_PIN`/`BUCKET_CHILD` proof as a retention blocker, and materialize exact `DB_PIN(kind=BucketFork)` records before planning reclaim.
- **Workflow cold-object reclaim retires FDB refs before S3 deletes.** Reclaimer jobs carry exact `ColdShardRef` identity, wait the grace window, mark `DeleteIssued`, delete the exact object key, then leave a completed retired record so the key cannot be republished.
- **Workflow repair cleanup runs through the lifecycle-fenced reclaimer.** Stale hot completions clear exact staged FDB rows, stale cold completions delete unreferenced S3 objects, and live cold refs are retained with structured corruption logs when S3 is missing or a retired record is `DeleteIssued`.
- **Workflow cold cleanup intent is config-independent.** Schedule orphan cleanup from the manager; activities decide whether configured cold storage exists and safely no-op or reject when disabled.
- **Workflow cold publish rejection cleans uploaded outputs first.** Matching active-job publish failures and stale cold completions both route uploaded refs through reclaimer orphan cleanup.
- **Workflow compaction wakeups are injected into `Db` as a signaler.** The commit path owns `SQLITE_CMP_DIRTY` admission, while runtime callers decide how to create or target the unique manager workflow.
- **Dirty-marker cleanup proves idle from workflow `CMP/root`.** Do not use legacy `/META/compact` materialized txid as a fallback.
- **`Db` constructors do not take UPS.** Keep workflow wakeups behind `CompactionSignaler`; do not reintroduce legacy compactor PubSub wiring.
- **Truncate cleanup fences observed PIDX and SHARD values before writing.** Snapshot-selected rows must be re-read with Serializable isolation before cleanup clears or rewrites them so concurrent workflow output cannot be clobbered and quota accounting retries cleanly.
- **UDB helper scans return logical conveyer keys.** Apply the supplied `Subspace` to the physical range scan, then strip it before returning rows to callers.
- **Depot inspect decode and pagination logic lives in `depot::inspect`.** `api-peer` should only mount thin internal `/depot/inspect/...` handlers and must not expose this surface through public SDKs.
- **Conveyer persisted payload structs use `serde::{Serialize, Deserialize}` as the serde_bare/vbare-compatible derive pattern.** Add `OwnedVersionedData` wrappers when introducing encode/decode helpers.
- **Conveyer type domains live behind the `conveyer/types.rs` facade.** Add branch, restore_point, compaction, history-pin, cold-manifest, storage, page, and id payloads under `conveyer/types/*.rs` and re-export public names from the facade.
- **META splits into single-writer sub-keys:** `/META/head` (commit-owned), `/META/quota` (atomic-add counter, raw i64 LE, not vbare), and workflow `CMP/root` (manager-owned compaction manifest).
- **Access-touch uses manifest sub-keys only.** Route commit/read touches through `touch_access_if_bucket_advanced`; do not enqueue the legacy eviction index.
- **Burst-mode quota uses FDB-derived branch lag.** Route hot quota cap decisions through `depot::burst_mode`; do not add per-pod cold-tier health state or tier gates.
- **Burst-mode cold lag uses workflow `CMP/root.cold_watermark_txid`.** Do not read or write legacy `branch_manifest_cold_drained_txid_key` in production paths.
- **Legacy eviction helpers are removed.** Workflow reclaimer planning owns deletion eligibility from current manifest, pins, PIDX, SHARD, and cold refs.
- **`COMMITS/{txid_be}` stores `CommitRow` via `SetVersionstampedValue`; `VTX/{versionstamp}` is written via `SetVersionstampedKey` and maps to raw u64 BE txid.**
- **Hot retention clears `COMMITS` and matching `VTX` rows together.** Do this inside the hot compactor write tx and keep quota accounting paired with the cleared keys.
- **Branch records live under `[BRANCHES]/list/{branch_id}` with FDB atomic-add refcount plus `desc_pin` and `restore_point_pin` atomic-min keys.** GC reads these scalars instead of walking the descendant tree.
- **Database branch records are current-only vbare payloads.** Do not add legacy decode paths that default missing `lifecycle_generation`.
- **Sweeping a forked database branch releases the parent fork retention.** Delete the parent `DB_PIN(kind=DatabaseFork)`, recompute `desc_pin`, and decrement the parent refcount in the sweep transaction.
- **Empty `restore_point_pin` state is key absence, not `[0; 16]`.** Clear `branches_restore_point_pin_key` when deleting the last restore point so a later `MutationType::ByteMin` can initialize from the next real versionstamp.
- **Branch GC pin computation lives in `depot::gc`.** Use it for cold sweeps, hot-history cleanup, and debug estimates instead of duplicating refcount/root/desc/restore point pin math.
- **Bucket catalog entries store branch ids with 16-byte versionstamped values.** `list_databases` walks bucket parents, caps inherited BUCKET_CATALOG rows by `parent_versionstamp`, and lets database tombstones mask inherited visibility.
- **Database tombstones store 16-byte versionstamped values.** `list_databases` applies the same bucket parent cap to tombstones so deletions after a bucket fork do not hide databases in the older fork.
- **Branch pin atomic-min writes use `MutationType::ByteMin`** because versionstamps are 16-byte lexicographic big-endian values.
- **Cold and eviction behavior is unconditional.** There is no per-bucket tier state or promotion path.
- **PITR, forking, and `restore_database` are all the same primitive: branch-at-position.** PITR creates a new branch at the resolved restore point; the broader system (pegboard) decides whether to swap the database's head pointer onto it.
- **`MAX_FORK_DEPTH = 16`.** Deeper trees indicate misuse.

## Cold tier (S3)

- **Cold tier holds history for PITR and 30-day retention; never the durability boundary.** A commit is durable as soon as the FDB write completes. Loss of FDB data between cold passes is a hot-tier disaster, not a PITR failure. RPO = FDB durability.
- **GC is a dependency-graph walk, not a wall-clock cutoff.** A layer is deletable only when older than `PITR_WINDOW_MS` AND not pinned by any descendant branch's fork point. No monotonic ratchet on `retention_pin_txid` — pin recomputes per pass, decreases when descendants delete.
- **Frozen branches use `created_at_ms`-based retention, not `head_txid - window`.** Their `head_txid` is fixed; window math against it would drift forward and GC their history out from under live descendants.
- **Layer filenames omit content checksum** (`delta/{min_txid}-{max_txid}.ltx`). Re-uploads after lease loss overwrite cleanly. Per-layer checksum still lives in the LTX V3 trailer + `LayerEntry.checksum`.
- **Legacy cold compactor handoff state is not active.** Do not add new `pending` marker, `in_flight_uuid`, or restore point UPS handoff paths.
- **Schema version on every persisted S3 object** (`schema_version: u32` on `ColdManifest`, `RestorePointIndex`, `BranchColdState`). Cold compactor reads old version + writes new version on every pass; reader code retains old-version paths for at least one full retention window past rollout.
- **Cold compactor tests inject a concrete cold tier.** The service default is `DisabledColdTier` until runtime config selects filesystem or S3, so test hooks must pass `FilesystemColdTier` explicitly when a pass should write objects.
- **ColdTier object keys are relative S3-style keys.** Reject empty object keys, absolute paths, and `..`; use `FilesystemColdTier` for local tests and `FaultyColdTier` for injected latency or failures.
- **ColdTier implementations live under `src/cold_tier/`.** Keep `mod.rs` as the trait/shared-type facade and put disabled, filesystem, S3, config, and fault-injection behavior in focused files.
- **FaultyColdTier requires an explicit node id.** Use a real node id or test-specific label so injected-failure metrics never fall back to `unknown`.
- **FaultyColdTier controller faults are test-only.** `DropArtifact` on GET returns a missing object; on PUT it writes first and then drops the acknowledgement with an error.
- **S3 missing-object handling uses typed SDK service errors.** Check `GetObjectError::is_no_such_key`; do not match `Display` strings.
- **Cold read fall-through keeps ColdTier GETs outside UDB transactions.** `Db::new_with_cold_tier` supplies the backend and read-side manifests are cached per connection.

## Restore Points

- **Restore point wire format is 33-char `{timestamp_ms_hex_be:016}-{txid_hex_be:016}`.** Branch identity is not in the wire format.
- **Restore points are retained `RestorePointRecord` rows.** Do not add non-retained restore point create or resolve fallback paths.
- **Conveyer records carry restore points as `RestorePointId`, not raw `String`.** The wrapper validates the 33-character ASCII wire format at construction and decode.
- **Use `RestorePointId::format(ts_ms, txid)` and `RestorePointId::parse()`** instead of hand-formatting or slicing restore point strings.
- **Restore point creation is FDB-only.** `PinStatus::Ready` means `DB_PIN(kind=RestorePoint)` exists, and current records keep `pin_object_key: None` without legacy cold-compactor UPS.
- **Restore point creation resolves `SnapshotSelector` first.** Persist the exact resolved txid/versionstamp as the restore point record and `DB_PIN`; do not mint retained tokens from caller-supplied timestamps.
- **Restore point deletion removes the restore point key, deletes `DB_PIN(kind=RestorePoint)`, decrements `pin_count`, and recomputes branch `restore_point_pin` without legacy cold-compactor UPS.**
- **Empty recomputed restore point pins clear `branches_restore_point_pin_key`.** Do not write a zero sentinel because it blocks later `MutationType::ByteMin` initialization.
- **Snapshot selector resolution returns `ResolvedRestoreTarget`.** Latest reads branch head metadata, timestamp reads unexpired `PITR_INTERVAL` rows, and restore point reads retained restore point records without VTX fallback.
- **Selector-based fork and restore derive from `ResolvedRestoreTarget.database_branch_id`.** The target can be an ancestor branch, not only the current DBPTR branch.
- **Restore point lifecycle code uses the conveyer facade pattern.** Keep `conveyer/restore_point.rs` as the public `Db` facade and put creation, deletion, restore, resolution, and recompute code under `conveyer/restore_point/*.rs`.
- **Restore-to-restore-point captures the undo commit before rollback, then swaps DBPTR and writes the undo restore point in one UDB transaction.**
- **Restore point resolution carries bucket fork caps into database branch ancestry.** Do not use recursive DBPTR resolution when resolving inherited restore points; direct-walk bucket parents so parent commits after `parent_versionstamp` stay unreachable.
- **Lex order = chronological order within a single branch's parent chain.** Across sibling branches, restore points are not orderable in any meaningful way.
- **Restore points are sender-scoped.** A caller resolving a restore point on another database's branch returns `BranchNotReachable`. Cross-database isolation is enforced at the engine edge, not in this package.

## What we don't import from prior art

Single-writer + no-local-file + lazy-read-only constraints rule out most of the multi-writer machinery in mvSQLite and the local-file path in LiteFS / libSQL / DO SQLite. We import:

- LTX V3 file format (Litestream / LiteFS).
- Layer model: delta + image (Neon).
- Branch-as-pointer + branch-as-restore (Neon).
- Dependency-graph GC (Neon issue #707 cautionary tale).
- `Pos{TXID, PostApplyChecksum}` rolling checksum (LiteFS / Litestream).
- RestorePoint-as-time-token concept (CF DO).
- HWM pending markers (LiteFS).
- "Snapshot when log >= db size" image-rebuild rule (CF DO SRS).

We explicitly do **not** import:

- mvSQLite PLCC / DLCC / MPC / versionstamps / content-addressed dedup (single-writer).
- LiteFS / libSQL local SQLite file (no local files).
- LiteFS / DO multi-replica WAL stream (FDB durability replaces it).
- CF DO 3-of-5 follower quorum (FDB durability replaces it).
- Hydration-at-database-open (lazy read only).
- WAL-frame-level shipping (per-commit granularity).

## Errors

- All failable functions return `anyhow::Error`. Use `.context(...)` instead of `anyhow!`.
- Public error variants on this package's surface are `RivetError`-derived (`SqliteStorageError::*`).
- Keep `SqliteStorageError` downcastable with a manual `Display`/`Error` impl when using `RivetError` derive; envoy and VFS inspect typed variants.
- Quota cap rejection uses `SqliteStorageQuotaExceeded { remaining_bytes, payload_size }` mirroring database KV's shape.
- RestorePoint-out-of-retention returns `RestoreTargetExpired`; restore_point on unreachable branch returns `BranchNotReachable`; fork at GC'd point returns `ForkOutOfRetention`; deeper than `MAX_FORK_DEPTH` returns `ForkChainTooDeep`.

## Testing

- All test bodies live under `engine/packages/depot/tests/`. Source files may keep only tiny `#[cfg(test)] #[path = "..."] mod tests;` shims for private access.
- Tests run against real UDB via `test_db()` (RocksDB-backed temp instance). No mocks for storage paths.
- Shared depot integration-test helpers live in `tests/common/mod.rs`; use them for temp UDB creation, memory UPS construction, `Db` construction, and raw key reads instead of redefining helpers per test file.
- Cold-tier tests use `ColdTier::Filesystem` (local filesystem stand-in for S3). UPS dispatch tests use the UPS memory driver. No real S3 required.
- Workflow cold-publish e2e tests may need a short-lived restore point to pin commit metadata until cold publish; delete it before asserting shard-cache eviction.
- Workflow compaction tests using real `Db::commit` should assert versionstamp-independent invariants; real UDB versionstamps do not encode txid bytes.
- Workflow force-compaction tests should wait for manager companion workflow ids before signaling so the manager is in its durable loop.
- Workflow compaction race tests can use debug-only workflow `test_hooks`; use stored `Notify` permits with `notify_one()` when a notification may arrive before the waiter arms.
- Workflow compaction skeleton tests should use `BumpSubSubject::WorkflowCreated { tag }` for workflow-row waits; keep `wait_until` for observations without notification hooks.
- Workflow cold-publish e2e tests may need short-lived restore points to keep `COMMITS` rows alive until cold publish validates them; delete the pins before testing reclaim.
- Crash recovery tests use `checkpoint_test_db()` + `reopen_test_db()` for real persisted-restart state.
- Failure-injection tests use `MemoryStore::snapshot()`. The `fail_after_ops` budget keeps decrementing past the first injected error.
- Shared test error helpers should search `anyhow::Error::chain()` for typed `SqliteStorageError` causes so wrapped context does not hide the expected variant.
- Lease-expiry and time-window tests use `tokio::time::pause()` + `advance()` for determinism.
- Use a nil bucket `Db` to exercise branch-scoped hot compaction through public `compact_default_batch`.
- Latency tests that depend on `UDB_SIMULATED_LATENCY_MS` must live in a dedicated integration test binary because UDB caches the env var once per process via `OnceLock`.
- SQLite VFS integration and fault-injection tests live in `engine/packages/depot-client/` so they exercise the full VFS.
- Depot fault-injection APIs live only behind `depot/test-faults`; enable the feature from dev/test dependencies only.
- Production fault-leak checks run through `engine/packages/depot/scripts/check-production-fault-leaks.sh`.
- Depot fault-controller tests live in `tests/fault_controller.rs` and run with `cargo test -p depot --features test-faults --test fault_controller`.
- Commit fault-hook tests should search the `anyhow` error chain because UDB transaction failures wrap injected fault errors.

## Metrics

- Prometheus metrics live with their owner module or in shared `depot::metrics`, and must include a `node_id` label.
- Shared depot metrics live in `depot::metrics`; cold-tier and takeover code must not import legacy compactor metrics.

## Reference Docs

- `docs-internal/engine/sqlite/storage-structure.md` - FDB and S3 key layout.
- `docs-internal/engine/sqlite/components.md` - conveyer, hot compactor, cold compactor, and eviction responsibilities.
- `docs-internal/engine/sqlite/vfs-brief.md` - SQLite VFS interaction summary and links to VFS docs.
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md` - PITR, branching, retention, and cold-tier rationale.
- `docs-internal/engine/sqlite/comparison-to-other-systems.md` - comparison against Neon, Cloudflare Durable Objects, Snowflake, LiteFS, Litestream, mvSQLite, and Turso.
- When changing FDB or S3 key layout, branch metadata, or compactor responsibilities, update `docs-internal/engine/sqlite/{storage-structure,components,constraints-and-design-decisions}.md` in the same change.

## Specs

- `~/.agents/specs/depot-stateless.md` — base architecture (hot tier only, two compactors, pegboard-envoy stateless).
- `~/.agents/specs/sqlite-pitr-fork.md` — branches, restore points, forking, S3 cold tier, retention. Extends the stateless spec.
- `r2-prior-art/.agent/research/sqlite/requirements.md` — the binding constraint floor (citing here for traceability; same constraints are duplicated above).
- `r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md` — Option F: client-side VFS read-cache, hydration, `sqlite_read_many`, stride prefetch. Orthogonal but complementary to PITR/fork; the steady-state hot-path read latency in this spec depends on Option F shipping for fork descendants to be tolerable.
