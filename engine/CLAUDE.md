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

Use `test-snapshot-gen` to generate and load RocksDB snapshots of the full UDB KV store for migration and integration tests. Scenarios produce per-replica RocksDB checkpoints stored under `engine/packages/test-snapshot-gen/snapshots/` as normal checked-in fixture files. In tests, use `test_snapshot::SnapshotTestCtx::from_snapshot("scenario-name")` to boot a cluster from snapshot data. See `docs-internal/engine/TEST_SNAPSHOTS.md` for the full guide.

## Workflow debugging

- `rivet-engine wf get <WORKFLOW_ID>` shows workflow state, tags, error, input/output.
- `rivet-engine wf history <WORKFLOW_ID> --print-ts` shows full event history with timestamps.
- `rivet-engine wf list` scans the workflow index and often hits FDB "transaction too old" on staging/prod. Use UDB direct queries instead.
- To find a workflow by tags, query UDB directly from the engine-shell pod:
```bash
# Find pegboard_runner_pool workflow for a namespace
rivet-engine udb -q 'ls 0/1/2/workflow/by_name_and_tag/pegboard_runner_pool/str:namespace_id/str:NAMESPACE_ID'

# Other useful workflow lookups
rivet-engine udb -q 'ls 0/1/2/workflow/by_name_and_tag/pegboard_runner_pool_metadata_poller/str:namespace_id/str:NAMESPACE_ID'
rivet-engine udb -q 'ls 0/1/2/workflow/by_name_and_tag/pegboard_runner_pool_error_tracker/str:namespace_id/str:NAMESPACE_ID'
rivet-engine udb -q 'ls 0/1/2/workflow/by_name_and_tag/pegboard_actor/str:actor_id/str:ACTOR_ID'
```
- `rivet-engine wf revive -n WORKFLOW_NAME` wakes dead workflows matching the name.
- `rivet-engine wf wake <WORKFLOW_ID>` wakes a specific sleeping workflow.

## Engine test flakes

- If a full engine test sweep fails during workflow-worker startup with `ActiveWorkerIdxKey` and `bad code, found 2`, treat it as a sporadic harness issue and retry the affected test once.

## Build metadata

- `rivet-build-meta`'s build script re-stamps the git SHA and build timestamp on every commit, so everything downstream of it recompiles each commit. Only crates that nothing but the `rivet-engine` binary pulls in may depend on it.
- The binary reads the constants once and passes them into `rivet_config::Config::load`. Everywhere else reads them through `rivet_config::Config::build_meta()`.

## Runtime protocol version sync

- Compiled runtime protocol versions are assembled in `rivet_build_meta::compiled_runtime_protocols()` rather than the config crate, so a protocol bump does not recompile everything that depends on `rivet-config`.
- The fleet only advances a runtime protocol version once every process heartbeating an older version is gone. A newly deployed process holds back to the oldest live version, so it must scan the protocol subspace in ascending version order.
- Heartbeat timestamps are written with `MutationType::Max`, which compares values as little-endian integers, so `ProtocolVersionKey`'s value codec must be little-endian too.
- Once a protocol has a call site that can speak more than one version, read the version from `rivet_config::Config::protocols()`. The `*_protocol::PROTOCOL_VERSION` constant is the compiled ceiling, not what the fleet has agreed to speak.

## Metrics

- RivetKit core records process-wide Prometheus metrics (actor, SQLite, VFS, tokio runtime) but does not serve them over HTTP. Users render them with `registry.routes.prometheusMetrics()` in TypeScript or `Registry::prometheus_metrics()` in Rust and mount the response on their own router. There is no per-actor `/gateway/<actor_id>/metrics` route and no serverless `/metrics` route.
- Track SQLite cold-read, VFS, storage, and preload optimization ideas in `docs-internal/engine/SQLITE_OPTIMIZATIONS.md`.
- Track SQLite cold-read optimization implementation and per-step benchmark deltas in `scripts/ralph/prd.json`.

## Depot tests

- For Depot key layout, component responsibilities, VFS interaction, design constraints, and prior-art comparisons, read `docs-internal/engine/sqlite/`.
- `depot` tests live in `engine/packages/depot/tests/`; do not add inline module test blocks.
- Run `depot` tests against temp RocksDB-backed UniversalDB via `test_db()`, `checkpoint_test_db(...)`, and `reopen_test_db(...)` instead of mocked storage paths.
- `depot` PIDX entries are stored as the PIDX key prefix plus a big-endian `u32` page number, with the value encoded as a raw big-endian `u64` txid.
- `depot` `/META/quota` is a fixed-width little-endian `i64` atomic counter; do not vbare-encode it.
- `depot` `/META/compactor_lease` is held with a local timer, cancellation token, and periodic renewal task; compaction work transactions must not revalidate the lease in-tx.
- `depot` compaction PIDX deletes use `COMPARE_AND_CLEAR` so stale entries no-op when commits race compaction.
- `depot` LTX V3 files end the page section with a zeroed 6-byte page-header sentinel before the varint page index, and the index offsets/sizes refer to the full on-wire page frame.
- `depot` LTX decoders should validate the varint page index against the actual page-frame layout instead of trusting footer offsets alone.
- `depot` `get_pages(...)` should keep `/META/head`, cold PIDX loads, and DELTA/SHARD blob fetches inside one UDB transaction, then decode each unique blob once and evict stale cached PIDX rows that now need SHARD fallback.
- `depot` fast-path commits should update an already-cached PIDX in memory after the store write, but must not load PIDX from store just to mutate it or the one-RTT path is gone.
- `depot` shrink writes must delete above-EOF PIDX rows and fully-above-EOF SHARD blobs inside the same commit/takeover transaction; compaction only cleans partial shards by filtering pages at or below `head.db_size_pages`.
- `depot` compaction should choose shard passes from the live PIDX scan, then delete DELTA blobs by comparing all existing delta keys against the remaining global PIDX references so multi-shard and overwritten deltas only disappear when every page ref is gone.
- `depot` metrics all live in the single flat `depot::metrics` module. Compaction pass duration/totals and per-pass volume (shards installed, cold refs published, reclaimed keys/bytes, lag) are recorded from the workflow `#[activity]` bodies via `metrics::record_*` helpers, never inside `#[workflow]` fns.
- `depot` quota accounting should treat only `/META/head`, SHARD, DELTA, and PIDX keys as billable; `/META/quota` tracks the sum with signed atomic-add deltas.
- `depot` latency tests that depend on `UDB_SIMULATED_LATENCY_MS` should live in a dedicated integration test binary, because UniversalDB caches that env var once per process with `OnceLock`.

## UniversalDB throttling

- Bulk background transactions bound their load with `tx.charge_throttle(name, kind)` at the top of the transaction body; UniversalDB then charges what the transaction reads and writes automatically. Do not hand-derive a charge from the rows a pass selected.
- Reads are charged per attempt (including attempts that never commit) and writes once on commit. Any new charging path must preserve that split; charging writes per attempt silently strangles the budget.
- Automatic write bytes only cover what an operation submits, so a `clear_range` must report the removed volume with `tx.charge_throttle_bytes(...)`. `COMPARE_AND_CLEAR` carries its value and is already covered.
- `tx.check_throttle(...)` answers from process-local state and reads nothing, so it can be called before opening a transaction. Charging without checking is valid and is how a pass that must not yield still keeps the estimate honest.
- Budgets are per `(throttle name, read|write)` in `runtime.udb_throttle_bytes_per_second`; depot compaction keeps its own `sqlite.compaction_{read,write}_bytes_per_second` keys, which win over the map.

## Pegboard Envoy

- Write new actor-hosting engine tests under `engine/packages/engine/tests/envoy/`; do not add new legacy runner tests under `engine/packages/engine/tests/runner/`.
- `PegboardEnvoyWs::new(...)` is constructed per websocket request, so SQLite dispatch translates the actor id to a 1:1 SQLite `database_id` at the boundary and caches per-database `Db` handles on the WS conn.
- Restored hibernatable WebSockets must rebuild runtime WebSocket handlers from callbacks and call `on_open`; pre-sleep NAPI callbacks are not reusable after actor wake.
- `pegboard-envoy` SQLite websocket handlers must validate page numbers, page sizes, and duplicate dirty pages at the websocket trust boundary and return `SqliteErrorResponse` for unexpected failures instead of bubbling them through the shared connection task.
- `pegboard-envoy` forwards `CommandStartActor` without local SQLite side effects; `CommandStopActor` only evicts the WS conn's cached SQLite `Db`.

## API routing

- `api-public` owns cross-datacenter forwarding for external requests; `api-peer` handlers should be local datacenter operations and must not add forwarding requirements.
