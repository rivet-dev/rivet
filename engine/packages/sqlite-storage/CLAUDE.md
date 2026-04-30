# sqlite-storage

The per-actor SQLite storage engine. FDB is the hot tier; S3 is the cold tier (PITR + retention). LTX V3 file format throughout.

For implementation-level wiring (key encodings, RTT count for `commit`, LTX byte layout, compaction shard rules, test harness setup) see the `## SQLite storage tests` and `## Pegboard Envoy` sections in `engine/CLAUDE.md`. The bullets here are the **architectural constraints** that shape what does and does not belong in this package.

## Hard constraints (binding floor)

These come from `r2-prior-art/.agent/research/sqlite/requirements.md` and supersede any contradiction in older specs.

- **Single writer per database.** Pegboard exclusivity (lost-timeout + ping protocol) holds. Never two writers on the same actor at the same instant. Do not implement MVCC, page-versioned read-set tracking, optimistic conflict detection at commit, content-addressed dedup, or commit-intent logs. mvSQLite's PLCC / DLCC / MPC / versionstamps are explicitly out — single writer makes them dead weight.
- **No local SQLite files. Ever.** Not on disk, not on tmpfs, not as a hydrated cache file. The authoritative store is FDB (hot) and S3 (cold). The VFS speaks to them directly. Forks do not materialize local files. Anything that puts a real SQLite file on a pegboard node is out of scope.
- **Lazy read only.** No bulk pre-load at actor open. Pages are fetched on demand from the hot tier. The per-WS-conn PIDX cache + flattened ancestry cache amortize the per-fetch cost. Fork warmup is a *background* cold→hot copy, not a synchronous bulk hydrate at first SQL statement.
- **Per-commit granularity.** The smallest addressable unit is a committed transaction. No sub-commit PITR, no WAL-frame-level shipping.

## Statelessness contract

- **The hot path (pegboard-envoy → ActorDb) is pod-stateless.** Every request self-describes its fence (branch_id, expected generation/head_txid in debug) and runs against the current FDB state. In-memory state on `ActorDb` is allowed only as a **perf cache** — never as the source of truth, never as a correctness fence, never as something that must survive across requests. WS conn drop = cache drop. No `open` / `close` lifecycle.
- **Both compactors (hot + cold) are pod-stateless.** All coordination state lives in FDB or S3. A compactor pod can crash mid-pass and the next pod that takes the lease picks up correctly. Pods churn freely; HPA scales without drain modes.
  - Hot compactor: stateless. Throttle (`last_trigger_at`) lives on the envoy's `ActorDb`, not on the compactor.
  - Cold compactor: stateless. Burst-mode signal is **derived from FDB lag** (`head_txid - cold_drained_txid`), not from a per-pod 5xx-ratio tracker. Debug `validate_quota` cadence is **derived from `materialized_txid % N`**, not from a per-pod pass-count counter.
- **Pegboard-envoy WS conn is stateless w.r.t. actor identity.** Envoys can reconnect to a different worker mid-flight while an actor is active; the new worker never sees the original `CommandStartActor`. The only per-conn state is the perf-only `scc::HashMap<actor_id, Arc<ActorDb>>`, populated lazily by SQLite request handlers. No `active_actors` HashMap, no presence tracking, no `start_actor` handler. `stop_actor` only evicts the cache entry.
- **Defensive runtime checks for "this should never happen" are `#[cfg(debug_assertions)]` only.** Trust the surrounding contracts (pegboard exclusivity, FDB tx isolation, lease-based compactor exclusion). Belt-and-suspenders fences that duplicate work the surrounding system already does belong in debug builds, not in release.

## Concurrency model

- **Pegboard exclusivity is the only writer fence in release.** No separate KV concurrency fence. Defensive in-tx "two writers detected" checks are `#[cfg(debug_assertions)]` only.
- **Hot compactor and cold compactor on the same branch hold separate FDB-backed leases.** They write disjoint META sub-keys (hot owns `META/head` + `META/compact`; cold owns `META/cold_compact`). Quota is FDB atomic-add — composes without conflict ranges.
- **Cold compactor must regular-read `META/compact.materialized_txid`** (not snapshot) so a concurrent hot pass aborts cold's write phase via OCC. Snapshot reads on the hot side are fine; commit/compaction never share a write target.
- **Hot compactor enforces `MAX_SHARD_VERSIONS_PER_SHARD` inside its write tx.** At cap, clear the oldest unpinned SHARD version before writing the new version; if every version is pinned by `desc_pin` or `bk_pin`, return `ShardVersionCapExhausted`.
- **Lease lifecycle uses local timer + cancellation token + periodic renewal task.** No in-tx lease re-validation reads inside compaction work transactions; renewal is the only place leases are touched during a pass.
- **PIDX deletes use FDB `COMPARE_AND_CLEAR`** so commit-vs-compaction races no-op on stale entries. Takes no read conflict range.
- **Fork-vs-GC race is OCC, not margin-based.** `fork()` regular-reads parent's `META/manifest.retention_pin_txid` inside the fork tx; concurrent GC pin advance aborts the fork via OCC. Hand-waved time margins are not a substitute.

## Storage layout

- **All PITR actor state is per-branch.** ActorDb resolves APTR, caches the branch id as a perf cache, and writes hot-path data under top-level `[0x02][0x30]/{branch_id}/<suffix>` keys.
- **ActorDb is namespace-scoped.** New ActorDb instances receive the engine namespace id, lazily seed NSPTR/NSBRANCH, and write APTR under the resolved namespace branch.
- **Actor pointer resolution walks namespace branch parents on APTR miss.** Namespace-branch tombstones return `ActorNotFound` and must not fall back to legacy actor-scoped storage.
- **ActorDb branch and PIDX caches must be invalidated when APTR moves.** Rollback swaps can make cached branch id, quota, and PIDX rows stale; resolve APTR as the source of truth before using cached branch-local state.
- **Legacy actor-scoped storage is compatibility fallback only.** New ActorDb writes use branch-scoped META, COMMITS, VTX, PIDX, DELTA, and SHARD keys.
- **Branch ancestry reads use branch-aware sources.** The PIDX cache is safe only when the read plan has one source branch; multi-branch ancestry reads must scan PIDX with branch identity.
- **Flattened ancestry caches store versionstamp caps.** Resolve cached parent versionstamps to txids inside each read transaction before PIDX/SHARD lookup.
- **Ancestor PIDX reads are capped by fork point.** Ignore PIDX owners newer than that source's cap and fall through to the latest SHARD version at or below the cap.
- **Debug historical reads cannot trust PIDX.** PIDX is the current owner map, so `debug::read_at` scans DELTA history up to the target txid before falling through to SHARD/cold layers.
- **Fresh fork branches use `/META/head_at_fork` until first commit.** The first local commit treats it as the previous `DBHead`, writes `/META/head`, and clears `/META/head_at_fork` in the same transaction.
- **PITR tunable constants live in `pump/constants.rs`.** Import shared limits and retention windows from there instead of duplicating literals.
- **Pump persisted payload structs use `serde::{Serialize, Deserialize}` as the serde_bare/vbare-compatible derive pattern.** Add `OwnedVersionedData` wrappers when introducing encode/decode helpers.
- **META splits into single-writer sub-keys:** `/META/head` (commit-owned), `/META/compact` (hot compactor-owned), `/META/cold_compact` (cold compactor-owned), `/META/quota` (atomic-add counter, raw i64 LE — not vbare), `/META/manifest` (branch metadata), `/META/compactor_lease`, `/META/cold_lease`. Disjoint owners; commit/compaction never conflict on a META sub-key.
- **Access-touch uses manifest sub-keys plus the global eviction index.** Route commit/read touches through `touch_access_if_bucket_advanced`; it is branch-scoped, throttled by `ACCESS_TOUCH_THROTTLE_MS`, and not part of branch quota accounting.
- **Burst-mode quota uses FDB-derived branch lag.** Route hot quota cap decisions through `sqlite_storage::burst_mode`; do not add per-pod cold-tier health state or tier gates.
- **Eviction compactor coordination is global.** The service lives under `compactor/eviction/`, takes `CMPC/lease_global/{kind=eviction}`, and scans `CTR/eviction_index` in `batch_size` chunks before predicate-specific clearing.
- **Eviction predicate is shard-version scoped.** Require a newer FDB SHARD version, hot-cache age, cold-drain coverage, `last_hot_pass_txid - SHARD_RETENTION_MARGIN >= as_of_txid`, and no `desc_pin` or `bk_pin` at or below that txid before clearing.
- **Eviction clears are plan-then-fence.** Capture expected SHARD/PIDX values at plan time, then regular-read `last_hot_pass_txid` in the clear tx and use `COMPARE_AND_CLEAR` for every planned key.
- **Fully evicted branches exit the eviction index.** Clear `CTR/eviction_index` only when the current SHARD rows are all covered by the planned compare-and-clear set.
- **`COMMITS/{txid_be}` stores `CommitRow` via `SetVersionstampedValue`; `VTX/{versionstamp}` is written via `SetVersionstampedKey` and maps to raw u64 BE txid.**
- **Hot retention clears `COMMITS` and matching `VTX` rows together.** Do this inside the hot compactor write tx and keep quota accounting paired with the cleared keys.
- **Branch records live under `[BRANCHES]/list/{branch_id}` with FDB atomic-add refcount plus `desc_pin` and `bk_pin` atomic-min keys.** GC reads these scalars instead of walking the descendant tree.
- **Branch GC pin computation lives in `sqlite_storage::gc`.** Use it for cold sweeps, hot-history cleanup, and debug estimates instead of duplicating refcount/root/desc/bookmark pin math.
- **Namespace catalog entries store branch ids with 16-byte versionstamped values.** `list_databases` walks namespace parents, caps inherited NSCAT rows by `parent_versionstamp`, and lets database tombstones mask inherited visibility.
- **Branch pin atomic-min writes use `MutationType::ByteMin`** because versionstamps are 16-byte lexicographic big-endian values.
- **Cold and eviction behavior is unconditional.** There is no per-namespace tier state or promotion path.
- **PITR, forking, and `restore_to_bookmark` are all the same primitive: branch-at-position.** PITR creates a new branch at the resolved bookmark; the broader system (pegboard) decides whether to swap the actor's head pointer onto it.
- **`MAX_FORK_DEPTH = 16`.** Deeper trees indicate misuse.

## Cold tier (S3)

- **Cold tier holds history for PITR and 30-day retention; never the durability boundary.** A commit is durable as soon as the FDB write completes. Loss of FDB data between cold passes is a hot-tier disaster, not a PITR failure. RPO = FDB durability.
- **GC is a dependency-graph walk, not a wall-clock cutoff.** A layer is deletable only when older than `PITR_WINDOW_MS` AND not pinned by any descendant branch's fork point. No monotonic ratchet on `retention_pin_txid` — pin recomputes per pass, decreases when descendants delete.
- **Frozen branches use `created_at_ms`-based retention, not `head_txid - window`.** Their `head_txid` is fixed; window math against it would drift forward and GC their history out from under live descendants.
- **Layer filenames omit content checksum** (`delta/{min_txid}-{max_txid}.ltx`). Re-uploads after lease loss overwrite cleanly. Per-layer checksum still lives in the LTX V3 trailer + `LayerEntry.checksum`.
- **HWM `pending/{uuid}.marker` objects gate orphan cleanup.** A pending marker older than `STALE_MARKER_AGE_MS` indicates a crashed prior pass; the next pass deletes the marker and its associated layer file before continuing.
- **Cold Phase B rewrites the pending marker before uploads.** Phase A's handoff marker is overwritten with the complete image, delta, pin, manifest, and pointer snapshot object list once the snapshot plan is known.
- **Cold Phase A reuses `in_flight_uuid` on retry.** Phase B manifest index writes replace the chunk ref for that pass UUID so crashes after upload can re-run idempotently.
- **Cold compactor pass runs as Phase A (FDB read tx) → Phase B (S3-only, no FDB tx) → Phase C (FDB write tx with regular-read OCC fence on `cold_drained_txid`).** Phase A/C tx-age budget is independent of S3 latency.
- **Cold Phase C is the only phase that advances `cold_drained_txid`, clears `in_flight_uuid`, and flips uploaded pinned bookmarks to `Ready`.**
- **Cold Phase B pin upload failures mark pending bookmarks `Failed`; Phase C OCC failures leave them `Pending` for retry.**
- **Fork warmup copies parent image layer bytes into the child branch prefix.** Child manifests must own child-prefixed object keys so later child sweeps cannot delete parent-owned cold objects.
- **Cold follow-up sweeps rewrite manifest chunks/index before deleting layer objects.** Treat zero layer versionstamps as unknown and not reclaimable.
- **Cold compactor service code lives under `compactor/cold/`.** Its UPS queue group is `cold_compactor`, and its per-branch lease uses `BR/{branch_id}/META/cold_lease` with the same 30s/10s/5s local-renewal shape as the hot compactor.
- **Schema version on every persisted S3 object** (`schema_version: u32` on `ColdManifest`, `BookmarkIndex`, `BranchColdState`). Cold compactor reads old version + writes new version on every pass; reader code retains old-version paths for at least one full retention window past rollout.
- **Cold compactor tests inject a concrete cold tier.** The service default is `DisabledColdTier` until runtime config selects filesystem or S3, so test hooks must pass `FilesystemColdTier` explicitly when a pass should write objects.
- **ColdTier object keys are relative S3-style keys.** Reject empty object keys, absolute paths, and `..`; use `FilesystemColdTier` for local tests and `FaultyColdTier` for injected latency or failures.
- **Cold read fall-through keeps ColdTier GETs outside UDB transactions.** `ActorDb::new_with_cold_tier` supplies the backend and read-side manifests are cached per connection.

## Bookmarks

- **Bookmark wire format is 33-char `{timestamp_ms_hex_be:016}-{txid_hex_be:016}`.** Branch identity is **not** in the wire format; bookmarks are interpreted relative to a branch context (actor's current head by default, explicit `branch_id` argument otherwise).
- **Pump records carry bookmarks as `BookmarkStr`, not raw `String`.** The wrapper validates the 33-character ASCII wire format at construction and decode.
- **Use `BookmarkStr::format(ts_ms, txid)` and `BookmarkStr::parse()`** instead of hand-formatting or slicing bookmark strings.
- **Ephemeral bookmark creation is read-only.** Format the caller timestamp with the current branch head txid; only pinned bookmarks write `BOOKMARK/{actor_id}/{bookmark}/pinned`.
- **Pinned bookmark creation is two-phase.** The request tx writes `PinStatus::Pending`, branch `bk_pin`, and namespace `pin_count`; the cold compactor UPS message makes the S3 pin layer and later flips status.
- **Pinned bookmark deletion removes both bookmark keys, decrements `pin_count`, recomputes branch `bk_pin`, and publishes cold-compactor cleanup.**
- **Restore-to-bookmark captures the undo commit before rollback, then writes the undo pinned bookmark after APTR moves.**
- **Bookmark resolution carries namespace fork caps into actor branch ancestry.** Do not use recursive APTR resolution when resolving inherited bookmarks; direct-walk namespace parents so parent commits after `parent_versionstamp` stay unreachable.
- **Lex order = chronological order within a single branch's parent chain.** Across sibling branches (forks of the same parent), bookmarks are not orderable in any meaningful way. APIs do not support cross-branch comparison.
- **Bookmarks are sender-scoped.** A caller resolving a bookmark on another actor's branch returns `BranchNotReachable`. Cross-actor isolation is enforced at the engine edge, not in this package.

## What we don't import from prior art

Single-writer + no-local-file + lazy-read-only constraints rule out most of the multi-writer machinery in mvSQLite and the local-file path in LiteFS / libSQL / DO SQLite. We import:

- LTX V3 file format (Litestream / LiteFS).
- Layer model: delta + image (Neon).
- Branch-as-pointer + branch-as-restore (Neon).
- Dependency-graph GC (Neon issue #707 cautionary tale).
- `Pos{TXID, PostApplyChecksum}` rolling checksum (LiteFS / Litestream).
- Bookmark-as-time-token concept (CF DO).
- HWM pending markers (LiteFS).
- "Snapshot when log >= db size" image-rebuild rule (CF DO SRS).

We explicitly do **not** import:

- mvSQLite PLCC / DLCC / MPC / versionstamps / content-addressed dedup (single-writer).
- LiteFS / libSQL local SQLite file (no local files).
- LiteFS / DO multi-replica WAL stream (FDB durability replaces it).
- CF DO 3-of-5 follower quorum (FDB durability replaces it).
- Hydration-at-actor-open (lazy read only).
- WAL-frame-level shipping (per-commit granularity).

## Errors

- All failable functions return `anyhow::Error`. Use `.context(...)` instead of `anyhow!`.
- Public error variants on this package's surface are `RivetError`-derived (`SqliteStorageError::*`).
- Keep `SqliteStorageError` downcastable with a manual `Display`/`Error` impl when using `RivetError` derive; envoy and VFS inspect typed variants.
- Quota cap rejection uses `SqliteStorageQuotaExceeded { remaining_bytes, payload_size }` mirroring actor KV's shape.
- Bookmark-out-of-retention returns `BookmarkExpired`; bookmark on unreachable branch returns `BranchNotReachable`; fork at GC'd point returns `ForkOutOfRetention`; deeper than `MAX_FORK_DEPTH` returns `ForkChainTooDeep`.

## Testing

- All tests live under `engine/packages/sqlite-storage/tests/`. **No inline `#[cfg(test)] mod tests` in `src/`.**
- Tests run against real UDB via `test_db()` (RocksDB-backed temp instance). No mocks for storage paths.
- Cold-tier tests use `ColdTier::Filesystem` (local filesystem stand-in for S3). UPS dispatch tests use the UPS memory driver. No real S3 required.
- Crash recovery tests use `checkpoint_test_db()` + `reopen_test_db()` for real persisted-restart state.
- Failure-injection tests use `MemoryStore::snapshot()`. The `fail_after_ops` budget keeps decrementing past the first injected error.
- Lease-expiry and time-window tests use `tokio::time::pause()` + `advance()` for determinism.
- Use a nil namespace `ActorDb` to exercise branch-scoped hot compaction through public `compact_default_batch`.
- Latency tests that depend on `UDB_SIMULATED_LATENCY_MS` must live in a dedicated integration test binary because UDB caches the env var once per process via `OnceLock`.

## Metrics

- Prometheus metrics live with their owner module (`pump::metrics` or `compactor::metrics`) and must include a `node_id` label.

## Specs

- `.agent/specs/sqlite-storage-stateless.md` — base architecture (hot tier only, two compactors, pegboard-envoy stateless).
- `.agent/specs/sqlite-pitr-fork.md` — branches, bookmarks, forking, S3 cold tier, retention. Extends the stateless spec.
- `r2-prior-art/.agent/research/sqlite/requirements.md` — the binding constraint floor (citing here for traceability; same constraints are duplicated above).
- `r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md` — Option F: client-side VFS read-cache, hydration, `sqlite_read_many`, stride prefetch. Orthogonal but complementary to PITR/fork; the steady-state hot-path read latency in this spec depends on Option F shipping for fork descendants to be tolerable.
