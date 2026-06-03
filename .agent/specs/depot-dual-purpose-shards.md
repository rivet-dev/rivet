# Depot Dual-Purpose Shards

Design for treating Depot branch shards as both:

- hot FDB materialization waiting to be published to cold storage
- the FDB cache of data whose durable copy may already live in cold storage

The important property: **when cold storage is disabled, hot compaction still works and all live data remains in FDB**. Cold storage becomes an optional eviction target, not a dependency for correctness.

## Goals

- Keep the read model simple: `PIDX -> DELTA`, otherwise `SHARD`, otherwise optional cold storage.
- Let hot compaction fold deltas into FDB shard rows even when cold storage is disabled.
- Let cold compaction publish existing FDB shard rows to cold storage without changing reader-visible bytes.
- Let FDB shard rows act as a cache after cold publish.
- Never delete an FDB shard row unless every required page version is either superseded, unpinned, or covered by a published cold ref.
- Avoid a separate "upload staging" data model. The branch shard row is the upload input and the hot-cache line.
- Support rollback to an arbitrary timestamp inside a configurable PITR window without requiring a user-created restore point for every target.

## Storage Model

Keep the existing branch keys:

```text
BR/{branch}/DELTA/{txid}/{chunk}
BR/{branch}/PIDX/{pgno}
BR/{branch}/SHARD/{shard_id}/{as_of_txid}
BR/{branch}/CMP/COLD_SHARD/{shard_id}/{as_of_txid}
BR/{branch}/CMP/ROOT
```

Interpretation:

- `DELTA` is the recent write log.
- `PIDX` routes pages that still live in `DELTA`.
- `SHARD/{shard_id}/{as_of_txid}` is the FDB materialized shard version.
- `COLD_SHARD/{shard_id}/{as_of_txid}` is a ref proving that exact shard version was published to cold storage.
- `CompactionRoot.hot_watermark_txid` is the highest txid folded into FDB shard rows.
- `CompactionRoot.cold_watermark_txid` is the highest txid whose shard rows were published to cold storage.

Do not treat `cold_watermark_txid` as required for hot compaction. It only controls cold coverage and later FDB shard eviction.

## Shard States

Each `(branch_id, shard_id, as_of_txid)` FDB shard row is in one of these states:

```text
HotOnly
  branch SHARD exists
  no matching published cold ref

HotAndCold
  branch SHARD exists
  matching cold ref exists

ColdOnly
  branch SHARD has been evicted
  matching cold ref exists
```

There is intentionally no `ColdPending` reader-visible state. A cold upload may be in progress, but readers still see `HotOnly` until `COLD_SHARD` is published transactionally.

`COLD_SHARD` indicates cold coverage, not cache residency. Cache residency is determined by whether the FDB `SHARD/{shard_id}/{as_of_txid}` row currently exists.

Concrete example:

```text
txid 1..100 committed as DELTA
hot compaction writes SHARD/0/100 and clears PIDX rows <= 100

With cold enabled:
  cold compaction uploads SHARD/0/100, publishes COLD_SHARD/0/100
  eviction may later clear SHARD/0/100

With cold disabled:
  no COLD_SHARD/0/100 is published
  SHARD/0/100 stays in FDB
```

This makes the FDB shard row like a packed moving box: before shipping, it is the thing waiting to be shipped; after shipping, the same box remains the local copy until we choose to free the space.

## Shard Cache Policy

Shard caching is separate from PITR retention.

```text
shard_cache_retention_ms = 7 days by default
```

After a shard has a matching published `COLD_SHARD` ref, the FDB `SHARD` row becomes a cache entry for the cold object. It should remain in FDB until cache eviction decides it is cold enough to remove.

Cache identity:

```text
cache key      = SHARD/{shard_id}/{as_of_txid}
cold ref key   = COLD_SHARD/{shard_id}/{as_of_txid}
payload        = exact LTX bytes referenced by ColdShardRef.content_hash
```

`SHARD` rows do not need a separate cache-state record. Existence of the row means resident. Absence of the row plus presence of a matching `COLD_SHARD` ref means cold-backed cache miss.

Default rationale:

- Use a 5 minute PITR interval because Amazon RDS uploads transaction logs every 5 minutes for PITR.
- Use a 7 day PITR restore window because Azure SQL Database and Google Cloud SQL Enterprise default to 7 days.
- Use a 7 day shard cache retention to match the PITR restore window by default. Operators can tune cache retention separately from PITR retention when cost or cold-read latency requires it.

Access tracking:

- Use branch-level access buckets as the eviction index.
- Do not track per-shard access initially.
- Compute the current bucket as `now_ms / ACCESS_TOUCH_THROTTLE_MS`.
- Reads that hit FDB `SHARD` or cold-backed `COLD_SHARD` should touch the branch access bucket only when the bucket advances.
- Cache fill should also touch the branch access bucket only when the bucket advances.
- Many reads inside the same bucket cause at most one metadata write per branch.
- Any shard read refreshes the whole branch cache window. This is intentional for the first implementation.

Cache eviction eligibility:

- A matching `COLD_SHARD/{shard_id}/{as_of_txid}` ref exists.
- The cold ref's `content_hash`, `shard_id`, and `as_of_txid` match the FDB `SHARD` bytes being evicted.
- No restore point, fork, or PITR interval coverage depends on the FDB shard as the only local copy.
- The shard or branch has not been accessed within `shard_cache_retention_ms`.
- A newer FDB shard version exists for normal latest-state reads, or the evicted version is otherwise cold-covered for every read target that can still resolve to it.
- Eviction is skipped when cold storage is disabled.

Read-through cache behavior:

1. Read misses `PIDX` and FDB `SHARD`.
2. Read finds a matching `COLD_SHARD` ref.
3. Read fetches and validates the cold object.
4. Read returns the requested page.
5. After returning the page to the caller, enqueue a best-effort cache fill task that writes the fetched shard bytes back to `SHARD/{shard_id}/{as_of_txid}` and updates cache access metadata.

The cache fill should be safe to race with eviction and hot compaction:

- Cache fill must not run on the user-facing read latency path. Use a bounded background queue drained by worker tasks.
- If the queue is full, skip cache fill and increment `sqlite_shard_cache_fill_total{outcome="skipped_queue_full"}`.
- Coalesce duplicate fills by `(branch_id, shard_id, as_of_txid)` while a fill is queued or running.
- Revalidate that the `COLD_SHARD` ref is still published and still points to the same object hash before writing.
- Write the exact cold object bytes under the matching shard version key.
- Use compare-and-set or an idempotent set guarded by the cold ref hash. If another writer has already filled the same key with identical bytes, treat it as success.
- If the key exists with different bytes, leave it alone and report a consistency error.
- Do not advance hot or cold watermarks from cache fill.
- Treat cache fill failure as non-fatal to the read.
- Track cache-fill bytes in cache metrics. Do not fail user reads because the background cache fill fails quota or size checks.

Eviction transaction:

1. Read the candidate FDB `SHARD` row.
2. Read the matching `COLD_SHARD` ref.
3. Decode or hash the FDB bytes and verify the cold ref matches.
4. Re-read pin floors or history pins needed for the candidate version.
5. Verify the access bucket is still older than `shard_cache_retention_ms`.
6. Compare-and-clear the `SHARD` row with the exact bytes read at plan time.
7. Leave the `COLD_SHARD` ref in place.

Eviction must never clear `COLD_SHARD`. Cold object retirement is separate reclaim work and only applies when a newer cold ref supersedes it and no restore point, fork, or PITR interval coverage can resolve to the old version.

Cold-disabled behavior:

- Hot compaction still writes FDB `SHARD` rows.
- Cold planning is skipped.
- No `COLD_SHARD` refs are published.
- Cache eviction is disabled because every FDB `SHARD` row is durable data, not a cold-backed cache entry.

Current code has pieces of this model, but not the full policy. It already has FDB shard rows, cold refs, cold fallback reads, and branch access buckets for eviction. It does not currently rehydrate an evicted FDB shard after reading from cold storage.

Metrics:

- `sqlite_shard_cache_read_total{outcome=fdb_hit|cold_hit|miss}`
- `sqlite_shard_cache_fill_total{outcome=scheduled|succeeded|failed|skipped_queue_full|skipped_duplicate|skipped_no_cold_ref}`
- `sqlite_shard_cache_fill_bytes_total`
- `sqlite_shard_cache_eviction_total{outcome=cleared|skipped_no_cold_ref|skipped_recent_access|skipped_retained|failed}`
- `sqlite_shard_cache_resident_bytes`
- `sqlite_shard_cache_cold_read_duration_seconds`

Metric labels must stay low-cardinality. Do not label by bucket id, database id, branch id, shard id, restore point id, or object key.

## Interval PITR Coverage

Restore points are exact and user-owned. A restore point retains its target until it is deleted. Internally, that means a restore point owns a hard history pin.

We also need implicit interval coverage for rough timestamp rollback:

```text
pitr_snapshot_interval_ms = 5 minutes by default
pitr_snapshot_retention_ms = 7 days by default
```

Interval coverage points are selected lazily by hot compaction. There is no timer that snapshots idle databases every interval. For each branch, hot compaction looks at commit metadata and picks the latest commit at or before each interval boundary inside the PITR restore window. Rollback to timestamp `T` resolves to the latest retained commit at or before `T`; it cannot restore between two commits because SQLite state only changes at commit boundaries.

If a database has no commits for an hour, no new interval coverage points are created for that hour. A rollback timestamp inside the quiet period resolves to the last retained commit before that timestamp.

These interval coverage points act like soft pins:

- Hot compaction must materialize FDB `SHARD` coverage for interval txids in the PITR window.
- Reclaim must not delete the `COMMITS`, `VTX`, `DELTA`, or `SHARD` coverage needed to restore those interval txids.
- After `pitr_snapshot_retention_ms`, interval coverage expires automatically.
- Restore points outlive the interval retention window and remain until deleted.

Interval coverage storage:

- Commits already store `CommitRow.wall_clock_ms`; use this timestamp for interval selection and timestamp restore resolution.
- Add an interval coverage index keyed by bucket branch and interval bucket.
- Each row stores selected `txid`, `versionstamp`, `wall_clock_ms`, and expiry timestamp.
- Hot compaction updates interval coverage rows when it observes commits for a bucket.
- Reclaim deletes expired interval coverage rows after `pitr_snapshot_retention_ms`.
- Reclaim treats unexpired interval coverage rows like retention inputs when deciding which `COMMITS`, `VTX`, `DELTA`, and `SHARD` rows can be deleted.

Suggested key shape:

```text
BR/{branch}/PITR_INTERVAL/{bucket_start_ms_be:8}
  -> PitrIntervalCoverage { txid, versionstamp, wall_clock_ms, expires_at_ms }
```

Timestamp resolution:

1. Resolve effective PITR policy for the bucket/database.
2. Reject timestamps older than `now - pitr_snapshot_retention_ms` with `RestoreTargetExpired`.
3. Find the newest interval coverage row with `bucket_start_ms <= timestamp_ms`.
4. If the selected commit's `wall_clock_ms > timestamp_ms`, walk backward to the previous coverage row.
5. Return the selected commit's `txid` and `versionstamp`.
6. If no row exists, return `RestoreTargetExpired`.

The interval policy is separate from cache eviction. A shard can be old and cold-covered, but still not evictable from FDB if cold storage is disabled and interval coverage still depends on it.

Concrete example:

```text
interval = 5 minutes
retention = 7 days

10:00 bucket -> txid 120
10:05 bucket -> txid 181
10:10 bucket -> txid 260

rollback to 10:07 resolves to the latest retained commit at or before 10:07
compaction treats txid 181 like a soft pin until it ages past 7 days
```

Quiet-period example:

```text
10:00 commit -> txid 120
10:01 commit -> txid 121
10:17 commit -> txid 122
interval = 5 minutes

hot compaction may retain:
10:00 bucket -> txid 121
10:15 bucket -> txid 122

rollback to 10:12 resolves to txid 121
rollback to 10:18 resolves to txid 122
```

## Terminology Rename

Do a full user-facing rename to restore point terminology.

Also rename Depot-local "bucket" terminology to "bucket". This is scoped to Depot storage APIs, types, keys, metrics, tests, and docs only. Do not rename Rivet-wide bucket concepts outside Depot.

Preferred terms:

- **Restore point**: user-facing retained restore target.
- **PITR interval coverage**: automatic restore coverage selected by compaction from commit timestamps. It expires by the PITR retention window.
- **History pin**: internal retention row used by restore points, forks, and PITR interval coverage.
- **Restore window**: how far back timestamp restore is supported by interval PITR.
- **Bucket**: Depot-local grouping boundary that currently appears as "bucket" in Depot code.

Current restore point code:

- `RestorePointId` is the retained restore point id wrapper.
- `RestorePointRef` records resolved restore point context.
- `RestorePointRecord` is the retained restore point record.
- `DbHistoryPinKind::RestorePoint` protects restore point history.
- `create_restore_point` writes the retained restore point record and history pin.
- `delete_restore_point` removes the retained restore point record and history pin.
- `resolve_restore_point` resolves retained restore point records only.
- `restore_database_to_restore_point` is the legacy restore API until selector-based `restore_database` replaces it.
- `owner_restore_point` records the restore point that owns a history pin.
- `restore_point_key` stores retained restore point records.
- `branches_restore_point_pin_key` stores the per-branch restore point pin floor.
- `BucketId` -> `BucketId` inside Depot boundaries.
- `BucketBranchId` -> `BucketBranchId`.
- `BucketBranchRecord` -> `BucketBranchRecord`.
- `BucketCatalogDbFact` -> `BucketCatalogDbFact`.
- `BucketForkFact` -> `BucketForkFact`.
- `DbHistoryPinKind::BucketFork` -> `DbHistoryPinKind::BucketFork`.
- `owner_bucket_branch_id` -> `owner_bucket_branch_id`.
- `bucket_branches_*` keys/helpers -> `bucket_branches_*`.
- `BUCKET_BRANCH`, `BUCKET_FORK_PIN`, and `BUCKET_CHILD` key labels -> bucket-named labels.
- `MAX_BUCKET_DEPTH` -> `MAX_BUCKET_DEPTH`.
- `MAX_RESTORE_POINTS_PER_BUCKET` -> `MAX_RESTORE_POINTS_PER_BUCKET`.

Migration rule: this has not shipped, so do clean renames with no old-name shims or dead Depot-local bucket APIs inside Depot. Remove old Depot-local bucket names from storage, API, tests, docs, metrics, and debug output in the same change.

## API Shape

There are two separate concepts:

- **PITR retention policy** controls automatic interval snapshots: interval duration and retention duration.
- **Shard caching policy** controls when cold-covered FDB shard rows may be evicted from FDB.

They should not share one knob. PITR retention says "keep rollback coverage." Cache retention says "keep local hot copies after cold publish."

Current restore point behavior:

- `create_restore_point(at_ms)` creates the retained user-facing restore point.
- `resolve_restore_point(restore_point)` resolves retained restore point records only.
- `fork_database(...)` already accepts a resolved versionstamp, so it can fork from a resolved restore point or another resolver output.

Clean terminology:

- **PITR interval coverage**: automatic restore coverage selected by compaction from commit timestamps. It expires by the PITR retention window.
- **History pin**: internal retention row used by restore points, forks, and PITR interval coverage.

Target storage API should be selector-based, not parallel one-off methods. Remove the old restore point API surface instead of keeping wrappers.

```rust
struct PitrPolicy {
	interval_ms: i64,
	retention_ms: i64,
}

struct ShardCachePolicy {
	retention_ms: i64,
}

enum SnapshotSelector {
	Latest,
	AtTimestamp { timestamp_ms: i64 },
	RestorePoint { restore_point: RestorePointId },
}

struct ResolvedRestoreTarget {
	restore_point: Option<RestorePointId>,
	versionstamp: [u8; 16],
	txid: u64,
	wall_clock_ms: i64,
	kind: SnapshotKind,
}

enum SnapshotKind {
	Head,
	Interval,
	RestorePoint,
}
```

Policy storage:

```text
BUCKET/{bucket_id}/POLICY/PITR
  -> PitrPolicy
BUCKET/{bucket_id}/POLICY/SHARD_CACHE
  -> ShardCachePolicy
BUCKET/{bucket_id}/DB_POLICY/{database_id}/PITR
  -> PitrPolicy override
BUCKET/{bucket_id}/DB_POLICY/{database_id}/SHARD_CACHE
  -> ShardCachePolicy override
```

Effective policy lookup:

1. Read database override.
2. If missing, read bucket policy.
3. If missing, use defaults.

Defaults:

```text
PitrPolicy { interval_ms: 5 minutes, retention_ms: 7 days }
ShardCachePolicy { retention_ms: 7 days }
```

Core functions:

```rust
set_bucket_pitr_policy(bucket_id, PitrPolicy)
set_database_pitr_policy(bucket_id, database_id, Option<PitrPolicy>)
set_bucket_shard_cache_policy(bucket_id, ShardCachePolicy)
set_database_shard_cache_policy(bucket_id, database_id, Option<ShardCachePolicy>)
resolve_restore_target(bucket_id, database_id, SnapshotSelector) -> ResolvedRestoreTarget
create_restore_point(bucket_id, database_id, SnapshotSelector) -> RestorePointId
delete_restore_point(bucket_id, database_id, RestorePointId)
fork_database(source_bucket, source_database_id, SnapshotSelector, target_bucket) -> String
restore_database(bucket_id, database_id, SnapshotSelector) -> RestorePointId
```

API semantics:

- `resolve_restore_target(AtTimestamp)` returns the latest retained commit at or before `timestamp_ms`.
- `create_restore_point(AtTimestamp)` resolves the timestamp first, then writes a hard restore-point history pin for that resolved commit.
- Database policy overrides bucket policy. A `None` database policy clears the override and falls back to bucket defaults.
- `fork_database(...)` resolves the selector, then calls the existing `derive_branch_at` path with the resolved versionstamp.
- `restore_database(...)` resolves the selector, captures and pins an undo restore point for current head, then rolls back.
- If no interval coverage or commit metadata is retained for the requested timestamp, return `RestoreTargetExpired`.

Implementation detail: interval coverage rows should be stored separately from explicit restore point records so automatic retention expiry can delete interval coverage without touching user-owned restore points.

Old restore point API removal:

- Delete the legacy restore-only APIs when selector-based `resolve_restore_target` and `restore_database` land.

Errors:

- `RestoreTargetExpired`: timestamp or restore point target is outside retained coverage.
- `RestorePointNotFound`: named restore point does not exist.
- `RestorePointTooDeep`: restore point/fork ancestry would exceed depth limits.
- `TooManyRestorePoints`: bucket restore point cap exceeded.
- `ShardCacheCorrupt`: FDB `SHARD` bytes differ from the matching `COLD_SHARD` ref.

Workflow changes:

- Manager reads effective bucket/database PITR and shard cache policies during planning.
- Hot planning includes interval coverage txids inside the PITR restore window.
- Hot install writes FDB `SHARD` rows for head, restore point/fork txids, and interval coverage txids.
- Reclaim treats interval coverage as expiring retention and restore points/forks as hard retention.
- Manager computes `cold_storage_enabled` from depot config or cold-tier configuration.
- If `cold_storage_enabled` is false, manager does not call `plan_cold_job` and does not set `active_cold_job`.
- Reclaim planning must still run when cold is disabled. Skipped cold work must not block reclaim.
- Eviction planning checks actual `COLD_SHARD` refs before clearing FDB `SHARD` rows.
- Read path schedules background cache fill after cold reads.

## Read Path

Reader order should be:

1. Read head and branch read plan.
2. If `PIDX` owns the page, read the owning `DELTA`.
3. If the delta is missing or there is no `PIDX`, read the latest FDB `SHARD` at or below the requested txid.
4. If the FDB shard is missing, read the latest published `COLD_SHARD` ref at or below the requested txid, then fetch from cold storage.
5. If cold storage is disabled and the FDB shard is missing, return an explicit corruption or missing-coverage error.

The existing stale-`PIDX` fallback remains valid because hot compaction writes the FDB shard before clearing the `PIDX` row. Cold fallback is only a second fallback after the FDB shard cache miss.

## Hot Compaction

Hot compaction is the only path that creates reader-visible FDB shard rows.

Algorithm:

1. Select deltas in `(hot_watermark_txid, head_txid]`.
2. Decode selected deltas and group pages by `shard_id`.
3. Select output coverage txids: current head, restore point/fork txids, and interval PITR txids still inside retention.
4. Merge each group over the latest existing FDB shard version at or below each output txid.
5. Stage the encoded shard bytes under job-scoped staging keys.
6. Install by writing `SHARD/{shard_id}/{as_of_txid}`.
7. Compare-and-clear covered `PIDX` rows.
8. Advance `hot_watermark_txid`.

This path must not call the cold tier. If cold storage is disabled, the system still reaches a compact FDB representation by replacing many deltas with shard rows.

## Cold Compaction

Cold compaction is a publisher for existing FDB shard rows.

Algorithm:

1. Select FDB shard rows where `as_of_txid == hot_watermark_txid` and `as_of_txid > cold_watermark_txid`.
2. Upload the exact shard bytes to cold storage using content-addressed or content-hashed object keys.
3. Revalidate the root, input fingerprint, and uploaded refs.
4. Publish `COLD_SHARD/{shard_id}/{as_of_txid}` refs.
5. Advance `cold_watermark_txid`.

If the cold tier is disabled, planning should produce no cold job. Do not run a job that reaches `DisabledColdTier::put_object`, because failure would only churn workflow state without changing correctness.

## Reclaim And Eviction

Separate deletion into two categories.

Delta reclaim:

- May delete `DELTA`, `COMMIT`, and old `PIDX` rows once the relevant pages are covered by FDB shard rows and no pin requires the raw delta history.
- Does not require cold coverage.
- This is what lets cold-disabled deployments avoid unbounded delta growth while keeping data in FDB.

FDB shard eviction:

- May delete an FDB shard row only if a matching published `COLD_SHARD` ref exists and no pin requires the FDB row as the only local copy.
- Must be disabled automatically when cold storage is disabled because no cold refs can be published.
- Should be driven by access age and cache pressure, not by the cold watermark alone.

Old shard-version cleanup:

- If a shard version is unpinned and superseded by a newer FDB shard version, it can be deleted as history cleanup.
- If an explicit or interval pin can read that version only through the FDB shard row, keep it unless a matching cold ref exists.

## Invariants

- A page at a readable txid has at least one source: `DELTA`, FDB `SHARD`, or published cold ref.
- `hot_watermark_txid >= cold_watermark_txid`.
- `cold_watermark_txid` only advances after all expected cold refs for the selected range are published.
- Clearing `PIDX` is safe only after the corresponding FDB shard row is installed.
- Clearing an FDB shard row is safe only after a matching cold ref is published.
- Cold upload retries are idempotent. Cold publish is the only reader-visible transition.
- Disabled cold storage means "never offload from FDB," not "never compact deltas."
- Interval PITR pins expire by wall-clock retention. Restore point pins expire only when the restore point is deleted.

## Implementation Notes

- Keep `branch_shard_key` as the canonical hot cache and cold-upload input.
- Update cold planning to short-circuit when no configured cold tier exists.
- Make eviction eligibility check for the actual `COLD_SHARD` ref, not just a watermark.
- Add branch metadata for interval PITR policy and the latest selected interval bucket.
- Prefer naming around `hot_shard` or `fdb_shard` where code currently implies the row is only staging for cold storage.
- Keep `ColdShardRef` content hashes tied to exact FDB shard bytes so cold reads can validate the object if needed.
- Metrics should distinguish hot compaction lag, cold publish lag, and FDB shard cache eviction.

## Tests

- Cold disabled: many commits, hot compaction runs, deltas are reclaimed, reads succeed from FDB shards, no cold job is attempted.
- Cold enabled: hot compaction publishes FDB shards, cold compaction publishes refs, eviction clears FDB shards, reads fall through to cold storage.
- Cold upload failure: FDB shards remain readable and `cold_watermark_txid` does not advance.
- Stale `PIDX`: cached PIDX points to reclaimed delta, read falls back to FDB shard.
- Evicted hot cache: no FDB shard exists, read falls back to matching cold ref.
- Bad eviction guard: attempting to evict a shard without a matching cold ref is rejected.
- Pinned history: old shard versions are retained until the pin is removed or cold coverage exists.
- Interval PITR: commits across several interval buckets create soft coverage txids, rollback resolves by timestamp, and reclaim deletes them only after interval retention expires.
