# Depot Inspect API

Read-only operational admin API on `api-peer` for diagnosing Depot storage
issues. This is modeled after the epoxy debug routes, but the primary Depot
interface should return decoded internal state in a few big JSON blobs, with
pagination only for row families that can grow without bound.

## Goals

- Diagnose "where is this page", "why is this branch retained", "why is
  compaction stuck", and "what is the PITR/restore coverage" without shelling
  into FDB or attaching a debugger.
- Make the common path one request: an operator with a bucket/database or
  branch id should get the useful metadata in one bounded response.
- Provide paginated decoded scans for large row families.
- Keep raw FDB get/scan escape hatches for unknown or newly added key families.
- Keep the surface internal and unstable. This is not a customer API and is
  not part of any generated public SDK.

## Non-goals

- No mutations in the first pass. No forced compaction, rollback, deletes, or
  cache purges.
- No S3 body proxy. Return cold object keys, generation ids, hashes, and sizes.
- No compatibility promise. Response fields can change with Depot internals.
- No new auth model. These routes inherit `api-peer` protection.

## Conventions

- Mount under `/depot/inspect/...` so the debug nature is obvious in logs.
- Use `GET` only.
- Return JSON only.
- Include these root fields in every response:

```jsonc
{
  "node_id": "node-id",
  "generated_at_ms": 0,
  "scope": {}
}
```

- Encode arbitrary bytes as unpadded base64url in URLs and JSON. Epoxy uses
  standard base64 in path segments today, but that is awkward because `/` is a
  valid standard-base64 character.
- Encode versionstamps and hashes as lowercase hex plus base64url when useful:
  `{ "hex": "...", "base64url": "..." }`.
- Encode all `u64` and `i64` ids/counters as JSON numbers unless a client
  compatibility issue appears.
- Large blobs return bounded summaries and samples. Anything unbounded must be
  behind a paginated route.
- Paginated endpoints use `limit` and `cursor`. Default limit is `100`; hard
  cap is `1000`.
- Large binary values default to metadata only. Add `include_bytes=true` only
  where explicitly supported.

## Route Summary

| Route | Purpose |
|---|---|
| `GET /depot/inspect/summary` | Local node Depot health and high-level counters |
| `GET /depot/inspect/catalog` | Paginated bucket/database pointer catalog |
| `GET /depot/inspect/buckets/{bucket_id}` | Big decoded bucket JSON |
| `GET /depot/inspect/buckets/{bucket_id}/databases/{database_id}` | Big decoded database JSON |
| `GET /depot/inspect/branches/{branch_id}` | Big decoded branch JSON |
| `GET /depot/inspect/branches/{branch_id}/pages/{pgno}/trace` | Targeted page read planner trace |
| `GET /depot/inspect/branches/{branch_id}/rows/{family}` | Paginated decoded branch row-family scan |
| `GET /depot/inspect/raw/key/{key}` | Raw FDB get with best-effort decode |
| `GET /depot/inspect/raw/scan` | Paginated raw FDB prefix scan |
| `GET /depot/inspect/raw/decode-key/{key}` | Best-effort Depot key decoder |

## Big JSON Endpoints

The big endpoints are the primary operator interface. They should include all
basic metadata and bounded summaries, but never unbounded full scans.

### `GET /depot/inspect/summary`

Local node overview. This should be cheap and should not scan branch-heavy
prefixes by default.

Response includes:

- Cold-tier configuration summary.
- Counts for bucket pointers, database pointers, database branches, bucket
  branches, dirty branches, and queued compaction rows.
- Global quota counter.
- Small samples of dirty branches and queued compaction items.

### `GET /depot/inspect/catalog?bucket_id=&database_id=&limit=&cursor=`

Paginated decoded catalog across pointer rows. Filters are optional.
`database_id` is the external database name string, not `DatabaseBranchId`.

Response shape:

```jsonc
{
  "node_id": "...",
  "generated_at_ms": 0,
  "scope": { "kind": "catalog" },
  "buckets": [
    {
      "bucket_id": "...",
      "current_bucket_branch_id": "...",
      "last_swapped_at_ms": 0
    }
  ],
  "databases": [
    {
      "bucket_branch_id": "...",
      "database_id": "actor-or-sqlite-name",
      "current_database_branch_id": "...",
      "last_swapped_at_ms": 0
    }
  ],
  "next_cursor": null
}
```

This route is paginated because pointer/catalog rows can grow with customer
objects.

### `GET /depot/inspect/buckets/{bucket_id}?include_history=false&sample_limit=20`

Bucket-level debug blob. This resolves the current bucket pointer and decodes
the current bucket branch record.

Response includes:

- Current `BucketPointer`.
- Current `BucketBranchRecord`.
- Parent ancestry with `parent_versionstamp` caps.
- Bucket policies: PITR and shard-cache policy.
- Catalog summary: count, bounded sample, and link to paginated catalog route.
- Tombstone summary: count and bounded sample.
- Child/fork fact summary: count and bounded sample.
- Bucket proof epoch.
- Optional pointer history sample when `include_history=true`.

### `GET /depot/inspect/buckets/{bucket_id}/databases/{database_id}?sample_limit=20`

Pointer-scoped database summary. This is the main starting point when an
operator has an actor/database name but not the internal branch UUID.

Response includes:

- Resolved bucket branch id and decoded `DatabasePointer`.
- Current `DatabaseBranchRecord`.
- Branch ancestry.
- `META/head` and `META/head_at_fork`.
- Last commit metadata.
- Branch quota and global quota.
- Effective PITR and shard-cache policies, including bucket/database overrides.
- Restore-point summary: count, newest, oldest, and bounded sample.
- Pin summary: refcount, descendant pin, restore-point pin, computed GC pin,
  count by pin kind, and bounded sample.
- PITR coverage summary: earliest/latest wall-clock coverage, interval count,
  and bounded sample.
- Compaction summary: `CMP/root`, dirty marker, manifest access-touch keys,
  workflow ids, active jobs, retry cursors, planning deadlines, force-compaction
  tracker summary, and stop state.
- Row-family summaries for commits, pidx, deltas, shards, cold shards, retired
  objects, staged hot shards, and PITR intervals. Each summary includes count
  when cheap, bounded sample, and a link to `/rows/{family}`.
- Links to the current branch endpoint, page trace endpoint template, and raw
  scan prefix templates.

### `GET /depot/inspect/branches/{branch_id}?sample_limit=20`

Branch-scoped version of the database debug blob. Use this when the operator
already has the `DatabaseBranchId` from logs, workflow state, or a raw key.

Response includes everything from the database endpoint that can be derived
from `branch_id` alone:

- `DatabaseBranchRecord`.
- `META/head` and `META/head_at_fork`.
- Branch ancestry.
- Quota, pins, computed GC pin, manifest access keys, dirty marker.
- Restore points that reference this branch.
- PITR coverage summary.
- Full compaction/workflow summary.
- Row-family summaries with bounded samples and `/rows/{family}` links.

This route should not require knowing the external bucket/database pointer.
If the reverse catalog lookup is available, include it as best-effort
`known_pointers`.

## Targeted Diagnostic Endpoint

### `GET /depot/inspect/branches/{branch_id}/pages/{pgno}/trace?at_txid=&at_versionstamp=&include_bytes=false`

Single-page read planner trace. This is intentionally separate from the big
JSON endpoints because it runs targeted planner/debug logic.

Rules:

- Exactly one of `at_txid` or `at_versionstamp` may be provided.
- If neither is provided, read at branch head.
- Use the production read planner with a debug trace sink. Do not reimplement
  planner behavior in `api-peer`.
- `include_bytes=false` returns `sha256`, size, and source metadata only.

Response shape:

```jsonc
{
  "node_id": "...",
  "generated_at_ms": 0,
  "scope": { "kind": "page", "branch_id": "...", "pgno": 1 },
  "read_cap": {
    "txid": 0,
    "versionstamp": { "hex": "...", "base64url": "..." }
  },
  "outcome": "found",
  "source": {
    "kind": "delta",
    "branch_id": "...",
    "txid": 0,
    "shard_id": null,
    "object_key": null
  },
  "steps": [
    {
      "kind": "pidx_lookup",
      "branch_id": "...",
      "result": "found",
      "details": {}
    }
  ],
  "bytes": {
    "size": 4096,
    "sha256": { "hex": "...", "base64url": "..." },
    "base64url": null
  }
}
```

`outcome` values: `found`, `zero_fill`, `above_eof`, `missing`, `error`.
`source.kind` values: `delta`, `hot_shard`, `cold_shard`, `ancestor`,
`zero_fill`.

## Paginated Row-Family Endpoint

### `GET /depot/inspect/branches/{branch_id}/rows/{family}?limit=&cursor=&include_bytes=false&...`

This single endpoint covers large decoded scans. It exists so the route surface
does not grow one endpoint per Depot row family.

Supported `family` values:

| Family | Extra filters | Output |
|---|---|---|
| `commits` | `before_txid`, `after_txid`, `include_vtx` | Decoded `CommitRow` plus optional VTX check |
| `pidx` | `from_pgno` | `pgno -> owner_txid` rows |
| `deltas` | `before_txid`, `after_txid` | Delta chunk metadata, optional LTX header and bytes hash |
| `shards` | `shard_id`, `before_txid` | Reader-visible hot shard or shard-cache row metadata |
| `cold-shards` | `shard_id`, `before_txid` | Decoded `ColdShardRef` records |
| `retired-cold-objects` | `state` | Decoded `RetiredColdObject` records |
| `pitr-intervals` | `from_ms`, `to_ms` | Decoded `PitrIntervalCoverage` rows |
| `pins` | `kind` | Decoded `DbHistoryPin` rows plus GC pin summary |
| `staged-hot-shards` | `job_id` | Staged hot shard metadata and optional bytes hashes |

Response shape:

```jsonc
{
  "node_id": "...",
  "generated_at_ms": 0,
  "scope": {
    "kind": "branch_rows",
    "branch_id": "...",
    "family": "commits"
  },
  "rows": [
    {
      "key": "base64url",
      "decoded": {}
    }
  ],
  "next_cursor": null
}
```

Every row includes the raw FDB key in base64url so an operator can jump from
typed output to the raw route.

This endpoint is paginated because these families are unbounded:

- `commits`
- `pidx`
- `deltas`
- `shards`
- `cold-shards`
- `retired-cold-objects`
- `pitr-intervals`
- `pins`
- `staged-hot-shards`

## Raw Endpoints

### `GET /depot/inspect/raw/key/{key}`

Raw FDB get. `{key}` is unpadded base64url of the exact FDB key.

Response shape:

```jsonc
{
  "node_id": "...",
  "generated_at_ms": 0,
  "scope": { "kind": "raw_key" },
  "key": "base64url",
  "value": {
    "exists": true,
    "size": 0,
    "base64url": "..."
  },
  "decoded": {
    "family": "BR.COMMITS",
    "path": {},
    "value": {}
  }
}
```

`decoded` is best-effort. Decode errors should appear in `decoded.error`
instead of failing the whole request unless the raw key itself is invalid.

### `GET /depot/inspect/raw/scan?prefix=&start_after=&limit=&decode=true`

Raw FDB prefix scan. `prefix` and `start_after` are unpadded base64url keys.
This route is always paginated.

Response shape:

```jsonc
{
  "node_id": "...",
  "generated_at_ms": 0,
  "scope": { "kind": "raw_scan" },
  "prefix": "base64url",
  "rows": [
    {
      "key": "base64url",
      "value_size": 0,
      "value": "base64url",
      "decoded": null
    }
  ],
  "next_cursor": null
}
```

Default `decode=true` for Depot-owned prefixes. If decoding is too expensive
for a scan, callers can pass `decode=false`.

### `GET /depot/inspect/raw/decode-key/{key}`

Decode a raw FDB key without reading the value. Useful when an operator has a
key from logs or FDB tooling and wants to know the Depot family and path fields.

## Implementation Notes

- Add `depot.workspace = true` to `engine/packages/api-peer/Cargo.toml`.
- Add `engine/packages/api-peer/src/depot.rs` and mount from
  `engine/packages/api-peer/src/router.rs` after epoxy routes.
- Keep request/response structs in `api-peer` unless another crate needs to
  call these endpoints as typed Rust APIs. Do not add them to public OpenAPI.
- Prefer decoded helper functions inside `depot::conveyer::debug` for storage
  reads and key-family decoding. `api-peer` should mostly parse HTTP params and
  shape JSON responses.
- Add a trace sink to the real read path for the page trace endpoint. The trace
  sink should be optional and zero-cost for normal reads.
- Use snapshot reads for inspect endpoints unless a value requires serializable
  semantics to match production behavior.
- Bound all scans. No unbounded `WantAll` scans in API handlers.
- Never expose these routes through `api-public`.

## Error Handling

Use normal `RivetError` responses through `api-builder`.

Recommended codes:

- `depot.inspect_invalid_key`
- `depot.inspect_invalid_cursor`
- `depot.inspect_bucket_not_found`
- `depot.inspect_database_not_found`
- `depot.inspect_branch_not_found`
- `depot.inspect_decode_failed`
- `depot.inspect_scan_limit_exceeded`

Decode failures for optional `decoded` fields should be returned inline instead
of converted to a top-level error.

## Implementation Order

1. Raw key, raw scan, and raw key decoder.
2. Catalog plus big bucket/database/branch metadata responses.
3. Paginated `/rows/{family}` endpoint for unbounded decoded scans.
4. Page trace through the production read path.

This order gives immediate operational value while keeping the route surface
small.
