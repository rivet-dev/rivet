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
- Before issuing an Epoxy operation with scoped `target_replicas`, validate the local replica is in scope or forward to an in-scope datacenter first.

## Test snapshots

Use `test-snapshot-gen` to generate and load RocksDB snapshots of the full UDB KV store for migration and integration tests. Scenarios produce per-replica RocksDB checkpoints stored under `engine/packages/test-snapshot-gen/snapshots/` (git LFS tracked). In tests, use `test_snapshot::SnapshotTestCtx::from_snapshot("scenario-name")` to boot a cluster from snapshot data. See `docs-internal/engine/TEST_SNAPSHOTS.md` for the full guide.

## Engine test flakes

- If a full engine test sweep fails during workflow-worker startup with `ActiveWorkerIdxKey` and `bad code, found 2`, treat it as a sporadic harness issue and retry the affected test once.

## Metrics

- RivetKit core exposes per-actor Prometheus metrics at `/gateway/<actor_id>/metrics`, gated by `_RIVET_METRICS_TOKEN`; prefer this endpoint for actor and VFS performance tuning metrics.

## SQLite storage tests

- In `sqlite-storage` failure-injection tests, inspect state with `MemoryStore::snapshot()` because store calls still consume the `fail_after_ops` budget after the first injected error.
- Keep `sqlite-storage` integration coverage inline in the module test blocks and run it against temp RocksDB-backed UniversalDB via `test_db()` plus real `SqliteEngine` methods instead of mocked storage paths.
- For `sqlite-storage` background task coordinators, inject the worker future in tests so dedup and restart behavior can be verified without depending on the real worker implementation.
- `sqlite-storage` PIDX entries are stored as the PIDX key prefix plus a big-endian `u32` page number, with the value encoded as a raw big-endian `u64` txid.
- When lazily populating `sqlite-storage` caches with `scc::HashMap::entry_async`, drop the vacant entry before awaiting a store load, then re-check `entry_async` before inserting.
- `sqlite-storage` takeover should batch orphan DELTA/STAGE/PIDX cleanup with the bumped META write in one `atomic_write`, then evict the actor's cached PIDX so later reads reload cleaned state.
- `sqlite-storage` LTX V3 files end the page section with a zeroed 6-byte page-header sentinel before the varint page index, and the index offsets/sizes refer to the full on-wire page frame.
- `sqlite-storage` LTX decoders should validate the varint page index against the actual page-frame layout instead of trusting footer offsets alone.
- `sqlite-storage` `get_pages(...)` should keep META, cold PIDX loads, and DELTA/SHARD blob fetches inside one `db.run(...)` transaction, then decode each unique blob once and evict stale cached PIDX rows that now need SHARD fallback.
- `sqlite-storage` fast-path commits should update an already-cached PIDX in memory after the store write, but must not load PIDX from store just to mutate it or the one-RTT path is gone.
- `sqlite-storage` shrink writes must delete above-EOF PIDX rows and fully-above-EOF SHARD blobs inside the same commit/takeover transaction; compaction only cleans partial shards by filtering pages at or below `head.db_size_pages`.
- `sqlite-storage` fast-path cutoffs should use raw dirty-page bytes, and slow-path finalize must accept larger encoded DELTA blobs because UniversalDB chunks logical values internally.
- `sqlite-storage` compaction should choose shard passes from the live PIDX scan, then delete DELTA blobs by comparing all existing delta keys against the remaining global PIDX references so multi-shard and overwritten deltas only disappear when every page ref is gone.
- `sqlite-storage` compaction must re-read META inside its write transaction and fence on `generation` plus `head_txid` before updating `materialized_txid` or quota fields, so takeover and commits cannot rewind the head.
- `sqlite-storage` metrics should record compaction pass duration and totals in `compaction/worker.rs`, while shard outcome metrics such as folded pages, deleted deltas, delta gauge updates, and lag stay in `compaction/shard.rs` to avoid double counting.
- `sqlite-storage` quota accounting should treat only META, SHARD, DELTA, and PIDX keys as billable, and META writes need fixed-point `sqlite_storage_used` recomputation because the serialized head size includes the usage field itself.
- `sqlite-storage` crash-recovery tests should snapshot RocksDB with `checkpoint_test_db(...)` and reopen it with `reopen_test_db(...)` so takeover cleanup runs against a real persisted restart state.
- `sqlite-storage` latency tests that depend on `UDB_SIMULATED_LATENCY_MS` should live in a dedicated integration test binary, because UniversalDB caches that env var once per process with `OnceLock`.

## Pegboard Envoy

- Write new actor-hosting engine tests under `engine/packages/engine/tests/envoy/`; do not add new legacy runner tests under `engine/packages/engine/tests/runner/`.
- `PegboardEnvoyWs::new(...)` is constructed per websocket request, so shared sqlite dispatch state such as the `SqliteEngine` and `CompactionCoordinator` must live behind a process-wide `OnceCell` instead of per-connection fields.
- Restored hibernatable WebSockets must rebuild runtime WebSocket handlers from callbacks and call `on_open`; pre-sleep NAPI callbacks are not reusable after actor wake.
- `pegboard-envoy` SQLite websocket handlers must validate page numbers, page sizes, and duplicate dirty pages at the websocket trust boundary and return `SqliteErrorResponse` for unexpected failures instead of bubbling them through the shared connection task.
- SQLite start-command schema dispatch should probe actor KV prefix `0x08` at startup instead of persisting a schema version in pegboard config or actor workflow state.

## API routing

- `api-public` owns cross-datacenter forwarding for external requests; `api-peer` handlers should be local datacenter operations and must not add forwarding requirements.
