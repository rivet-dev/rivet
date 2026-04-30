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
- **Lease lifecycle uses local timer + cancellation token + periodic renewal task.** No in-tx lease re-validation reads inside compaction work transactions; renewal is the only place leases are touched during a pass.
- **PIDX deletes use FDB `COMPARE_AND_CLEAR`** so commit-vs-compaction races no-op on stale entries. Takes no read conflict range.
- **Fork-vs-GC race is OCC, not margin-based.** `fork()` regular-reads parent's `META/manifest.retention_pin_txid` inside the fork tx; concurrent GC pin advance aborts the fork via OCC. Hand-waved time margins are not a substitute.

## Storage layout

- **All PITR actor state is per-branch.** ActorDb resolves APTR, caches the branch id as a perf cache, and writes hot-path data under top-level `[0x02][0x30]/{branch_id}/<suffix>` keys.
- **ActorDb is namespace-scoped.** New ActorDb instances receive the engine namespace id, lazily seed NSPTR/NSBRANCH/tier_state, and write APTR under the resolved namespace branch.
- **Legacy actor-scoped storage is compatibility fallback only.** New ActorDb writes use branch-scoped META, COMMITS, VTX, PIDX, DELTA, and SHARD keys.
- **Branch ancestry reads use branch-aware sources.** The PIDX cache is safe only when the read plan has one source branch; multi-branch ancestry reads must scan PIDX with branch identity.
- **PITR tunable constants live in `pump/constants.rs`.** Import shared limits and retention windows from there instead of duplicating literals.
- **Pump persisted payload structs use `serde::{Serialize, Deserialize}` as the serde_bare/vbare-compatible derive pattern.** Add `OwnedVersionedData` wrappers when introducing encode/decode helpers.
- **META splits into single-writer sub-keys:** `/META/head` (commit-owned), `/META/compact` (hot compactor-owned), `/META/cold_compact` (cold compactor-owned), `/META/quota` (atomic-add counter, raw i64 LE — not vbare), `/META/manifest` (branch metadata), `/META/compactor_lease`, `/META/cold_lease`. Disjoint owners; commit/compaction never conflict on a META sub-key.
- **`COMMITS/{txid_be}` stores `CommitRow` via `SetVersionstampedValue`; `VTX/{versionstamp}` is written via `SetVersionstampedKey` and maps to raw u64 BE txid.**
- **Branch records live under `[BRANCHES]/list/{branch_id}` with FDB atomic-add refcount plus `desc_pin` and `bk_pin` atomic-min keys.** GC reads these scalars instead of walking the descendant tree.
- **Branch pin atomic-min writes use `MutationType::ByteMin`** because versionstamps are 16-byte lexicographic big-endian values.
- **Namespace branch derivation mirrors actor branch pin/refcount updates but leaves child `tier_state` absent.** `ensure_tier_at_least` handles lazy tier inheritance or promotion later.
- **PITR, forking, and `restore_to_bookmark` are all the same primitive: branch-at-position.** PITR creates a new branch at the resolved bookmark; the broader system (pegboard) decides whether to swap the actor's head pointer onto it.
- **`MAX_FORK_DEPTH = 16`.** Deeper trees indicate misuse.

## Cold tier (S3)

- **Cold tier holds history for PITR and 30-day retention; never the durability boundary.** A commit is durable as soon as the FDB write completes. Loss of FDB data between cold passes is a hot-tier disaster, not a PITR failure. RPO = FDB durability.
- **GC is a dependency-graph walk, not a wall-clock cutoff.** A layer is deletable only when older than `PITR_WINDOW_MS` AND not pinned by any descendant branch's fork point. No monotonic ratchet on `retention_pin_txid` — pin recomputes per pass, decreases when descendants delete.
- **Frozen branches use `created_at_ms`-based retention, not `head_txid - window`.** Their `head_txid` is fixed; window math against it would drift forward and GC their history out from under live descendants.
- **Layer filenames omit content checksum** (`delta/{min_txid}-{max_txid}.ltx`). Re-uploads after lease loss overwrite cleanly. Per-layer checksum still lives in the LTX V3 trailer + `LayerEntry.checksum`.
- **HWM `pending/{uuid}.marker` objects gate orphan cleanup.** A pending marker older than `STALE_MARKER_AGE_MS` indicates a crashed prior pass; the next pass deletes the marker and its associated layer file before continuing.
- **Cold compactor pass runs as Phase A (FDB read tx) → Phase B (S3-only, no FDB tx) → Phase C (FDB write tx with regular-read OCC fence on `cold_drained_txid`).** Phase A/C tx-age budget is independent of S3 latency.
- **Schema version on every persisted S3 object** (`schema_version: u32` on `ColdManifest`, `BookmarkIndex`, `BranchColdState`). Cold compactor reads old version + writes new version on every pass; reader code retains old-version paths for at least one full retention window past rollout.

## Bookmarks

- **Bookmark wire format is 33-char `{timestamp_ms_hex_be:016}-{txid_hex_be:016}`.** Branch identity is **not** in the wire format; bookmarks are interpreted relative to a branch context (actor's current head by default, explicit `branch_id` argument otherwise).
- **Pump records carry bookmarks as `BookmarkStr`, not raw `String`.** The wrapper validates the 33-character ASCII wire format at construction and decode.
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
- Latency tests that depend on `UDB_SIMULATED_LATENCY_MS` must live in a dedicated integration test binary because UDB caches the env var once per process via `OnceLock`.

## Specs

- `.agent/specs/sqlite-storage-stateless.md` — base architecture (hot tier only, two compactors, pegboard-envoy stateless).
- `.agent/specs/sqlite-pitr-fork.md` — branches, bookmarks, forking, S3 cold tier, retention. Extends the stateless spec.
- `r2-prior-art/.agent/research/sqlite/requirements.md` — the binding constraint floor (citing here for traceability; same constraints are duplicated above).
- `r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md` — Option F: client-side VFS read-cache, hydration, `sqlite_read_many`, stride prefetch. Orthogonal but complementary to PITR/fork; the steady-state hot-path read latency in this spec depends on Option F shipping for fork descendants to be tolerable.
