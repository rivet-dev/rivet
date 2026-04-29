# Engine Notes

## VBARE migrations

When changing a versioned VBARE schema, follow the existing migration pattern.

1. Never edit an existing published `*.bare` schema in place. Add a new versioned schema instead.
2. Update the matching `versioned.rs` like this:
   - If the bytes did not change, deserialize both versions into the new wrapper variant:

   ```rust
   6 | 7 => Ok(ToClientMk2::V7(serde_bare::from_slice(payload)?))
   ```

   - If the bytes did change, write the conversion field by field.

   - Do not do this:

   ```rust
   let bytes = serde_bare::to_vec(&x)?;
   serde_bare::from_slice(&bytes)?
   ```
5. If a new VBARE union version keeps old variants byte-identical, append new variants at the end and gate v2-only variants when serializing back to v1.
6. If a nested payload like `CommandStartActor` changes shape, write explicit v1<->v2 conversions for both `ToEnvoy` and `ActorCommandKeyData` instead of assuming same-bytes compatibility.
6a. Never rely on byte-identical wire layout across versions. Every cross-version converter must reconstruct the target type field-by-field, even when versions appear identical today. No `serde_bare::to_vec` + `from_slice` shortcuts and no `impl_versioned_same_bytes!`-style macros that reuse bytes across versions.
7. For manual `vbare::OwnedVersionedData` impls whose latest schema version is greater than `1`, return `vec![Ok]` from both converter hooks or `serialize(version)` still treats the type as version `1`.
3. Verify the affected Rust crate still builds.
4. For the runner protocol specifically:
   - Bump both protocol constants together:
     - `engine/packages/runner-protocol/src/lib.rs` `PROTOCOL_MK2_VERSION`
     - `rivetkit-typescript/packages/engine-runner/src/mod.ts` `PROTOCOL_VERSION`
   - Update the Rust latest re-export in `engine/packages/runner-protocol/src/lib.rs` to the new generated module.
5. For any Rust VBARE protocol crate, bump the protocol constant together with the matching latest generated/schema wiring (`generated::vN`, latest re-exports, `protocol.rs`/`versioned.rs` if present, and the corresponding `engine/sdks/schemas/.../vN.bare` file).

## Epoxy durable keys

- All epoxy durable state lives under per-replica subspaces (`keys::subspace(replica_id)` for v2, `keys::legacy_subspace(replica_id)` for read-only legacy data). Shared key types (`KvValueKey`, `KvBallotKey`, etc.) live in `engine/packages/epoxy/src/keys/keys.rs` and new tuple segment constants go in `engine/packages/universaldb/src/utils/keys.rs`.
- UniversalDB low-level `Transaction::get`, `set`, `clear`, and `get_ranges_keyvalues` do not apply the transaction subspace automatically; pack subspace bytes yourself or use the higher-level range helpers.
- UniversalDB simulated latency for benchmarks comes from `UDB_SIMULATED_LATENCY_MS`, which `Database::txn(...)` reads once via `OnceLock`, so set it before process startup.
- When adding fields to epoxy workflow state structs, mark them `#[serde(default)]` so Gasoline can replay older serialized state.
- Epoxy integration tests that spin up `tests/common::TestCtx` must call `shutdown()` before returning.

## Test snapshots

Use `test-snapshot-gen` to generate and load RocksDB snapshots of the full UDB KV store for migration and integration tests. Scenarios produce per-replica RocksDB checkpoints stored under `engine/packages/test-snapshot-gen/snapshots/` (git LFS tracked). In tests, use `test_snapshot::SnapshotTestCtx::from_snapshot("scenario-name")` to boot a cluster from snapshot data. See `docs-internal/engine/TEST_SNAPSHOTS.md` for the full guide.

## Engine test flakes

- If a full engine test sweep fails during workflow-worker startup with `ActiveWorkerIdxKey` and `bad code, found 2`, treat it as a sporadic harness issue and retry the affected test once.

## SQLite storage tests

- `sqlite-storage` tests live in `engine/packages/sqlite-storage/tests/`; do not add inline module test blocks.
- Run `sqlite-storage` tests against temp RocksDB-backed UniversalDB via `test_db()`, `checkpoint_test_db(...)`, and `reopen_test_db(...)` instead of mocked storage paths.
- `sqlite-storage` PIDX entries are stored as the PIDX key prefix plus a big-endian `u32` page number, with the value encoded as a raw big-endian `u64` txid.
- `sqlite-storage` storage usage counters are fixed-width little-endian `i64` atomic counters; use `/META/storage_used_live` for live data and `/META/storage_used_pitr` for PITR overhead, not vbare.
- `sqlite-storage` `/META/compactor_lease` is held with a local timer, cancellation token, and periodic renewal task; compaction work transactions must not revalidate the lease in-tx.
- `sqlite-storage` compaction PIDX deletes use `COMPARE_AND_CLEAR` so stale entries no-op when commits race compaction.
- `sqlite-storage` LTX V3 files end the page section with a zeroed 6-byte page-header sentinel before the varint page index, and the index offsets/sizes refer to the full on-wire page frame.
- `sqlite-storage` LTX decoders should validate the varint page index against the actual page-frame layout instead of trusting footer offsets alone.
- `sqlite-storage` `get_pages(...)` should keep `/META/head`, cold PIDX loads, and DELTA/SHARD blob fetches inside one UDB transaction, then decode each unique blob once and evict stale cached PIDX rows that now need SHARD fallback.
- `sqlite-storage` fast-path commits should update an already-cached PIDX in memory after the store write, but must not load PIDX from store just to mutate it or the one-RTT path is gone.
- `sqlite-storage` shrink writes must delete above-EOF PIDX rows and fully-above-EOF SHARD blobs inside the same commit/takeover transaction; compaction only cleans partial shards by filtering pages at or below `head.db_size_pages`.
- `sqlite-storage` compaction should choose shard passes from the live PIDX scan, then delete DELTA blobs by comparing all existing delta keys against the remaining global PIDX references so multi-shard and overwritten deltas only disappear when every page ref is gone.
- `sqlite-storage` forks must copy any source DELTA rows referenced by checkpoint PIDX entries into the destination actor; destination PIDX rows must not point at source-only DELTAs.
- `sqlite-storage` metrics should record compaction pass duration and totals in `compactor/worker.rs`, while shard outcome metrics such as folded pages, deleted deltas, delta gauge updates, and lag stay in `compactor/shard.rs` to avoid double counting.
- `sqlite-storage` live quota accounting should treat only `/META/head`, SHARD, DELTA, and PIDX keys as billable; `/META/storage_used_live` tracks the sum with signed atomic-add deltas.
- `sqlite-storage` admin operation state is persisted under actor-scoped `/META/admin_op/{operation_id}` records; do not track operation source-of-truth in compactor pod memory.
- `sqlite-storage` latency tests that depend on `UDB_SIMULATED_LATENCY_MS` should live in a dedicated integration test binary, because UniversalDB caches that env var once per process with `OnceLock`.

## SQLite PITR + Forking

- PITR is logical recovery only; it is not a backup against FoundationDB cluster loss.
- Restore writes `/META/restore_in_progress` in the same transaction as the first destructive live-state clear.
- Fork refcount increments must commit before releasing the source compactor lease, and decrements must commit after copied data is safe.
- Checkpoint creation uses the head txid captured during compaction planning, not a later live head.
- Restore recomputes quota by scanning current state and applying `atomic_add(delta)` to live/PITR counters.
- Commits must reject while `/META/restore_in_progress` exists, even if pegboard suspension should already block traffic.
- Live bytes and PITR overhead stay split between `/META/storage_used_live` and `/META/storage_used_pitr`.

## Pegboard Envoy

- `PegboardEnvoyWs::new(...)` is constructed per websocket request, so SQLite dispatch uses per-actor `ActorDb` instances cached on the WS conn and populated lazily by `get_pages` or `commit`.
- Restored hibernatable WebSockets must rebuild runtime WebSocket handlers from callbacks and call `on_open`; pre-sleep NAPI callbacks are not reusable after actor wake.
- `pegboard-envoy` SQLite websocket handlers must validate page numbers, page sizes, and duplicate dirty pages at the websocket trust boundary and return `SqliteErrorResponse` for unexpected failures instead of bubbling them through the shared connection task.
- `pegboard-envoy` forwards `CommandStartActor` without local SQLite side effects; `CommandStopActor` only evicts the WS conn's cached `ActorDb`.
