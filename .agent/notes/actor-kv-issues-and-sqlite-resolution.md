# Actor KV Issues And SQLite Resolution

Date: 2026-02-24

## Scope

Full audit of actor KV usage across:

- `rivetkit-typescript/packages/rivetkit`
- `rivetkit-typescript/packages/workflow-engine`
- `rivetkit-typescript/packages/sqlite-vfs`
- `rivetkit-typescript/packages/traces`
- `rivetkit-typescript/packages/cloudflare-workers`
- `engine/packages/pegboard*` and API surfaces

Limits used as baseline:

- max key size: 2048 bytes
- max `kv put` batch entries: 128
- max `kv put` batch payload: 976 KiB (keys + values)
- max value size: 128 KiB
- max total actor KV storage: 1 GiB

## Confirmed Good

- File-system driver enforces key/value/batch/storage limits and validates list prefixes:
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/kv-limits.ts:3`
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/global-state.ts:1333`

## Findings

### High

1. Traces writes chunks that exceed KV value limits by default.
- Defaults: `DEFAULT_MAX_CHUNK_BYTES = 1024 * 1024` and target `512 * 1024`.
- Writes each chunk as one KV value via `driver.set`.
- This violates `max value size = 128 KiB`.
- Refs:
  - `rivetkit-typescript/packages/traces/src/traces.ts:63`
  - `rivetkit-typescript/packages/traces/src/traces.ts:546`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/traces-driver.ts:44`

2. Traces write queue can get permanently poisoned after one KV write failure.
- `writeChain` is a promise chain with no rejection recovery (`writeChain = writeChain.then(...)`).
- If one `driver.set` fails, subsequent queued writes are never attempted and `flush()` keeps failing.
- Refs:
  - `rivetkit-typescript/packages/traces/src/traces.ts:545`
  - `rivetkit-typescript/packages/traces/src/traces.ts:767`

3. SQLite VFS emits unsplit `putBatch` and `deleteBatch`.
- `xWrite` can write many chunks + metadata in one batch.
- `#delete` and `xTruncate` can delete many chunk keys in one batch.
- This can exceed 128 entries and/or 976 KiB payload.
- Refs:
  - `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts:856`
  - `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts:908`
  - `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts:979`

4. Workflow persistence/recovery sends unsplit write arrays.
- `storage.flush` builds unbounded `writes` then calls `driver.batch(writes)` once.
- `recover()` similarly accumulates metadata rewrites and sends one batch.
- Refs:
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:270`
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:346`
  - `rivetkit-typescript/packages/workflow-engine/src/index.ts:695`
  - `rivetkit-typescript/packages/workflow-engine/src/index.ts:722`

5. Workflow flush clears dirty flags before write success.
- `entry.dirty` and `metadata.dirty` are set to `false` before `driver.batch(writes)`.
- If batch fails, dirty markers are lost and later flushes can miss writes.
- Refs:
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:296`
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:308`
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:346`

6. State persistence can exceed batch limits and has failure-side bookkeeping risk.
- `savePersistInner` aggregates actor + all changed connections into one `entries` batch.
- It clears `connsWithPersistChanged` before `kvBatchPut`; if put fails, changed flags are lost.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts:422`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts:503`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts:512`

7. Queue paths can violate limits and default config exceeds value cap.
- Queue delete removes all selected messages in one `kvBatchDelete(keys)`.
- Queue message default max size is `1 MiB`, larger than actor KV `128 KiB` value cap.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:520`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:530`
  - `rivetkit-typescript/packages/rivetkit/src/actor/config.ts:226`

8. Cloudflare driver semantics diverge from engine KV constraints.
- No explicit engine-equivalent limit validation in Cloudflare KV helpers/driver paths.
- Batch operations are executed as per-key loops without explicit transaction grouping.
- Refs:
  - `rivetkit-typescript/packages/cloudflare-workers/src/actor-kv.ts:14`
  - `rivetkit-typescript/packages/cloudflare-workers/src/actor-driver.ts:226`
  - `rivetkit-typescript/packages/cloudflare-workers/src/actor-driver.ts:251`

### Medium

9. Workflow and traces prefix-deletes can exceed batch delete limits.
- Both `deletePrefix` paths list all matching keys, then issue one unsplit `kvBatchDelete`.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts:155`
  - `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts:166`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/traces-driver.ts:56`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/traces-driver.ts:65`

10. Connection cleanup swallows KV delete failures.
- `connDisconnected` catches KV delete errors, logs, and continues.
- Stale connection KV may remain without surfacing the failure to caller.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/connection-manager.ts:372`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/connection-manager.ts:379`

11. Queue metadata mutates before storage write and is not rolled back on write failure.
- Enqueue increments `nextId`/`size` before `kvBatchPut`.
- Dequeue decrements `size` before delete/metadata writes.
- If write fails, in-memory metadata can drift until a rebuild path runs.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:163`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:168`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:523`
  - `rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:531`

12. Workflow driver “atomicity” comment is stronger than implementation.
- Uses `Promise.all([workflow batch put, saveState])`.
- This is concurrent, but not truly atomic across both operations.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts:189`
  - `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts:192`

13. Runner tunnel returns raw internal errors to clients.
- `ws_to_tunnel_task` sends `err.to_string()` for KV errors, with TODO noting concern.
- Also performs KV operations inline in websocket task (TODO for queue/background thread).
- Refs:
  - `engine/packages/pegboard-runner/src/ws_to_tunnel_task.rs:224`
  - `engine/packages/pegboard-runner/src/ws_to_tunnel_task.rs:246`
  - `engine/packages/pegboard-runner/src/ws_to_tunnel_task.rs:321`

14. Engine storage-limit check excludes metadata overhead.
- Validation payload accounting does not include metadata row bytes.
- Existing TODO acknowledges this.
- Refs:
  - `engine/packages/pegboard/src/actor_kv/utils.rs:63`
  - `engine/packages/pegboard/src/actor_kv/mod.rs:273`

15. Storage quota checks are conservative for overwrites.
- Storage checks use `current_total + payload_size` and do not subtract replaced key/value sizes.
- Near quota, overwrite-in-place updates can be rejected even when final size would still fit.
- Refs:
  - `engine/packages/pegboard/src/actor_kv/utils.rs:63`
  - `engine/packages/pegboard/src/actor_kv/utils.rs:70`
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/kv-limits.ts:45`
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/kv-limits.ts:55`

16. Initial actor KV bootstrap paths bypass local limit validation in some drivers.
- File-system create/load-or-create writes initial KV entries through `#putKvEntriesInDb` without `validateKvEntries`.
- Cloudflare bootstrap writes initial entries directly with `kvPut`.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/global-state.ts:263`
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/global-state.ts:468`
  - `rivetkit-typescript/packages/rivetkit/src/drivers/file-system/global-state.ts:590`
  - `rivetkit-typescript/packages/cloudflare-workers/src/actor-handler-do.ts:433`
  - `rivetkit-typescript/packages/cloudflare-workers/src/actor-handler-do.ts:435`

17. HTTP KV get surfaces validation failures as generic server errors.
- `api-peer` path forwards `actor_kv::get` errors via `?`.
- User/input errors can bubble as 500-style errors instead of clear 4xx mapping.
- Refs:
  - `engine/packages/api-peer/src/actors/kv_get.rs:44`
  - `engine/packages/api-peer/src/actors/kv_get.rs:87`

### Low

18. Manager KV get encoding path appears inconsistent for binary keys.
- Manager router decodes incoming path key as base64 bytes.
- Remote manager driver re-encodes that key as UTF-8 text instead of base64 when calling engine API.
- Engine API expects base64 in path.
- Refs:
  - `rivetkit-typescript/packages/rivetkit/src/manager/router.ts:346`
  - `rivetkit-typescript/packages/rivetkit/src/remote-manager-driver/mod.ts:387`
  - `engine/packages/api-peer/src/actors/kv_get.rs:72`

19. Workflow default message delete path hides individual failures.
- `Promise.allSettled` returns only successful IDs; failed deletes are not surfaced loudly.
- Refs:
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:117`
  - `rivetkit-typescript/packages/workflow-engine/src/storage.ts:131`

20. Limit-focused regression coverage is thin in core KV test suites.
- Current pegboard/engine KV tests are mostly happy-path CRUD/list behavior.
- Refs:
  - `engine/packages/pegboard/tests/kv_operations.rs:1`
  - `engine/packages/engine/tests/actors_kv_crud.rs:1`

## SQLite Resolution (Without Breaking SQLite Atomicity)

1. Keep SQLite as the atomicity authority.
- Return `SQLITE_OK` only if all KV sub-batches succeed.
- On any sub-batch failure, return `SQLITE_IOERR_*` so WAL/journal rollback semantics remain intact.

2. Add deterministic sub-batching in SQLite VFS.
- Split by:
  - max entries per batch: 128
  - max payload per batch: 976 KiB (sum of key/value sizes)
  - max key size: 2048 bytes
- Apply to all `putBatch` and `deleteBatch` fanout paths.

3. Preserve safe write ordering.
- Write chunk entries first.
- Write metadata (size key) last.

4. Fail closed.
- Do not swallow partial failures.
- Surface the error and let caller fail the SQLite operation.

5. Optional future enhancement.
- Engine-level multi-op transactional API to reduce failure windows further.
