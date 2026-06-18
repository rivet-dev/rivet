# Depot crash course

How the Depot SQLite backend reads, writes, compacts, and fences branchable database storage. Read this before changing anything in `engine/packages/depot/`.

For VFS-side parity rules, see [sqlite-vfs.md](sqlite-vfs.md). For exact key formats, see [sqlite/storage-structure.md](sqlite/storage-structure.md).

## Storage model

Depot stores SQLite pages in UDB first. S3 is optional cold storage for workflow-published shard objects.

| Row family | Holds | Owner |
|---|---|---|
| `DBPTR` / `BUCKET_PTR` | Current database and bucket branch pointers | Conveyer branch APIs |
| `BUCKET_CATALOG` | Database membership facts in bucket branches | Conveyer branch APIs |
| `BRANCHES` / `BUCKET_BRANCH` | Branch records, refcounts, pin floors, lifecycle generations | Conveyer, GC, workflow checks |
| `BR/{branch}/META/head` | Current database head | Commit path |
| `BR/{branch}/COMMITS` and `BR/{branch}/VTX` | Commit metadata and versionstamp-to-txid lookup | Commit path |
| `BR/{branch}/PIDX` and `BR/{branch}/DELTA` | Recent page-owner index and LTX delta chunks | Commit path |
| `BR/{branch}/SHARD` | Reader-visible hot shard versions and cold-backed shard-cache rows | Workflow manager, cache fill, reclaimer |
| `BR/{branch}/CMP/*` | Workflow manifest, cold refs, retired cold objects, staged hot output | Workflow manager and companions |
| `BR/{branch}/PITR_INTERVAL` | Automatic PITR interval coverage rows | Workflow hot install and reclaim |
| `RESTORE_POINT` and `DB_PIN` | User retained restore points and exact history pins | Restore point APIs and workflow proof |

The main invariant is simple: **commits write deltas directly to UDB; workflow compaction is the only publish/delete authority for compaction output.**

## Read path

Reads resolve the database pointer to a database branch, build a branch-aware read plan, and fetch each page through the hot path first:

```text
1. Read branch head or fork head metadata.
2. Return missing for pages above EOF.
3. Check PIDX and DELTA first.
4. If the DELTA is absent or reclaimed, fall back to the newest SHARD at or below the read cap.
5. If no FDB SHARD covers the page, locate a matching workflow CMP/cold_shard ref and read the cold object.
6. If the cold read succeeds, enqueue a bounded background fill to restore the matching FDB SHARD cache row.
```

Cold storage is optional. If only cold coverage can satisfy a page and the `Db` has no configured cold tier, reads fail with `ShardCoverageMissing` instead of inventing zero-filled bytes.

The in-process PIDX, branch-id, ancestry, and shard-cache fill queues are perf caches only. Correctness comes from UDB rows and workflow revalidation.

## Write path

SQLite commits call Depot through the conveyer path:

```text
1. Resolve DBPTR and read the current branch head in the UDB transaction.
2. Use the fast path for commits whose encoded LTX bytes and metadata fit in one UDB transaction.
3. For large commits, write immutable LTX bytes under DELTA_OBJ chunks before the publish transaction.
4. Publish COMMITS, VTX, PIDX, quota, META/head, and either DELTA chunks or DELTA_MANIFEST plus DELTA_PAGEIDX rows.
5. After commit, update SQLITE_CMP_DIRTY and send a throttled DeltasAvailable wake when lag crosses thresholds.
```

The commit path does **not** publish SHARD rows, upload cold objects, or delete old history. It only records new committed history and wakes workflow compaction.

## Workflow compaction

Each active database branch has one DB manager workflow plus hot, cold, and reclaimer companions, all unique by database branch id.

The manager owns planning and durable publication:

- Hot jobs stage LTX shard blobs under `CMP/stage/{job_id}/hot_shard`; the manager validates the active job, copies output to reader-visible `SHARD`, advances `CMP/root`, writes selected `PITR_INTERVAL` rows, and compare-clears matching PIDX.
- Cold jobs upload deterministic objects at `db/{branch}/shard/{shard_id}/{txid}-{job_id}-{hash}.ltx`; the manager publishes `CMP/cold_shard` refs only after revalidating branch lifecycle, manifest generation, pins, proof state, and covered inputs.
- Reclaim jobs delete hot rows only after the manager proves replacement coverage. They also retire cold refs, wait the grace window, mark deletes issued, delete exact S3 keys, and leave completed retired records so object keys are not republished.
- Shard-cache eviction is a reclaimer lane. It clears only FDB `SHARD` rows that have matching `CMP/cold_shard` refs and are not retained by restore points, forks, or unexpired PITR interval coverage.

`CMP/root` watermarks are scheduling summaries, not deletion proof by themselves. Deletes re-read the exact pins, PIDX dependencies, SHARD coverage, lifecycle generation, and manifest generation inside the delete transaction.

## PITR and restore

Automatic timestamp restore coverage is stored as `PITR_INTERVAL` rows selected during hot compaction from commit wall-clock timestamps and the effective bucket/database PITR policy. Expired interval rows are soft pins until reclaim compare-clears them.

Restore points are retained user tokens. Creating a restore point resolves a `SnapshotSelector` to exact branch, txid, versionstamp, and wall-clock metadata, then writes a `RestorePointRecord` and `DB_PIN(kind=RestorePoint)`. Deleting it removes that hard pin and recomputes branch pin floors.

Fork and restore use the same primitive: resolve a snapshot selector, derive a branch at that exact point, and let the caller decide whether to keep a fork or move the database pointer.

## Cross-references

- Key layout: [sqlite/storage-structure.md](sqlite/storage-structure.md)
- Component ownership: [sqlite/components.md](sqlite/components.md)
- VFS parity rules: [sqlite-vfs.md](sqlite-vfs.md)
- Storage metrics: [SQLITE_METRICS.md](SQLITE_METRICS.md)
