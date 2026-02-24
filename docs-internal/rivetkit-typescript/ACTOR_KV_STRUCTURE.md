# Actor KV Structure

This document is the canonical reference for the actor KV keyspace in `rivetkit-typescript`.

## Master Tree (Quick View)

```text
persisted data (1)/
  actor-persist

connections (2)/
  {conn_id}/
    conn-persist

inspector token (3)/
  token

user kv (4)/
  {user_key}/
    value

queue messages (5)/
  {message_id_u64_be}/
    message

queue metadata (6)/
  metadata

workflow (7)/
  names (1)/
    {name_index}/
  history (2)/
    {location_segments...}/
  workflow fields (3)/
    state (1)
    output (2)
    error (3)
    version (4)
    input (5)
  entry metadata (4)/
    {entry_id}/

traces (8)/
  data (1)/
    {bucket_start_sec}/
      {chunk_id}/

sqlite (9)/
  metadata (0)/
    {file_tag}/
  chunks (1)/
    {file_tag}/
      {chunk_index_u32_be}/
```

The sections below provide the full details and storage semantics.

## Known Issues (As Of 2026-02-24)

For the full tracked audit, see `.agent/notes/actor-kv-issues-and-sqlite-resolution.md`.

High-impact issues currently tracked:

- Traces chunk defaults are larger than actor KV value limits (default chunks can exceed 128 KiB), and traces writes currently use single-value puts per chunk.
- Traces write queue can get stuck after a single KV write failure (`writeChain` rejection is not recovered), causing subsequent trace flushes to fail.
- SQLite VFS can emit unsplit `putBatch`/`deleteBatch` calls that exceed batch limits (128 entries / 976 KiB).
- Workflow/state persistence paths can build unsplit write arrays that exceed batch limits, and workflow dirty flags are cleared before write success.
- Queue paths can exceed batch limits, and default queue message size (1 MiB) exceeds actor KV value limit (128 KiB).
- Cloudflare worker KV backend currently diverges from engine KV limit/atomicity semantics (no explicit equivalent guardrails in driver paths).

Important error-handling concerns:

- Some cleanup/persistence paths log KV errors without propagating them, which can leave stale or divergent state.
- Some in-memory metadata is mutated before KV writes and is not rolled back if the write fails.
- Engine websocket KV tunnel currently returns raw error strings from storage/validation errors.
- Quota checks are conservative near the 1 GiB limit (overwrite-in-place updates may fail even when resulting size would still fit).

It covers:

- top-level actor key prefixes in `rivetkit`
- nested workflow key prefixes in `workflow-engine`
- nested traces key prefixes in `traces` (OpenTelemetry storage)
- nested SQLite VFS keys in `sqlite-vfs`
- the physical Rust-side storage envelope used by engine `actor_kv`

## Logical Key Tree (Per Actor)

Each actor has an isolated logical keyspace. The first byte determines the top-level namespace.

```text
[1] PERSIST_DATA
[2] CONN_PREFIX + utf8(conn_id)
[3] INSPECTOR_TOKEN
[4] KV + user_key_bytes
[5] QUEUE_PREFIX + u64_be(message_id)
[6] QUEUE_METADATA
[7] WORKFLOW_PREFIX + workflow_engine_key
[8] TRACES_PREFIX + traces_key
[9] SQLITE_PREFIX + sqlite_vfs_key
```

## `[7]` Workflow Nested Keys

Inside `WORKFLOW_PREFIX`, keys are fdb-tuple packed:

```text
[1, name_index]                // name registry
[2, ...location_segments]      // history
[3, field]                     // workflow metadata field
[4, entry_id]                  // entry metadata
```

Workflow metadata fields:

```text
[3, 1] state
[3, 2] output
[3, 3] error
[3, 4] version
[3, 5] input
```

## `[8]` Traces Nested Keys (OTEL)

Inside `TRACES_PREFIX`, traces are stored as chunked fdb tuples:

```text
[1, bucket_start_sec, chunk_id]
```

## `[9]` SQLite Nested Keys

Inside `SQLITE_PREFIX`, SQLite VFS keys are byte-encoded:

```text
[9, 0, file_tag]                       // metadata key
[9, 1, file_tag, u32_be(chunk_index)]  // chunk key
```

`file_tag` values:

```text
0 main
1 journal
2 wal
3 shm
```

Legacy SQLite keys still exist for old data and are resolved in `sqlite-vfs/src/vfs.ts`.

## Physical Engine Storage Envelope

On the engine side (`pegboard::actor_kv`), each logical key above is wrapped and stored as:

```text
Subspace: (RIVET, PEGBOARD, ACTOR_KV, actor_id)

metadata: (KeyWrapper(logical_key), METADATA) -> KvMetadata
data:     (KeyWrapper(logical_key), DATA, chunk_index) -> bytes
```

`DATA` chunks are 10,000 bytes in the Rust engine actor KV implementation.

## Source Files

- `packages/rivetkit/src/actor/instance/keys.ts`
- `packages/workflow-engine/src/keys.ts`
- `packages/traces/src/traces.ts`
- `packages/sqlite-vfs/src/kv.ts`
- `packages/sqlite-vfs/src/vfs.ts` (legacy SQLite keys)
- `engine/packages/pegboard/src/keys/actor_kv.rs`
- `engine/packages/pegboard/src/actor_kv/mod.rs`
