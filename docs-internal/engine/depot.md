# Depot Crash Course

How the Depot SQLite backend reads, writes, compacts, and fences branchable database storage. Read this before changing anything in `engine/packages/depot/`.

For VFS-side parity rules, see [sqlite-vfs.md](sqlite-vfs.md). For exact key formats, see [sqlite/storage-structure.md](sqlite/storage-structure.md).

## Storage Model

Depot stores SQLite pages in UDB/FDB. OSS Depot does not include object-backed cold storage.

| Row family | Holds | Owner |
|---|---|---|
| `DBPTR` / `BUCKET_PTR` | Current database and bucket branch pointers | Conveyer branch APIs |
| `BUCKET_CATALOG` | Database membership facts in bucket branches | Conveyer branch APIs |
| `BRANCHES` / `BUCKET_BRANCH` | Branch records, refcounts, pin floors, lifecycle generations | Conveyer, GC, workflow checks |
| `BR/{branch}/META/head` | Current database head | Commit path |
| `BR/{branch}/COMMITS` and `BR/{branch}/VTX` | Commit metadata and versionstamp-to-txid lookup | Commit path |
| `BR/{branch}/PIDX` and `BR/{branch}/DELTA` | Recent page-owner index and LTX delta chunks | Commit path |
| `BR/{branch}/SHARD` | Reader-visible hot shard versions | Workflow manager and reclaimer |
| `BR/{branch}/CMP/*` | Workflow root and staged hot output | Workflow manager and companions |
| `BR/{branch}/PITR_INTERVAL` | Automatic PITR interval coverage rows | Workflow hot install and reclaim |
| `RESTORE_POINT` and `DB_PIN` | User retained restore points and exact history pins | Restore point APIs and workflow proof |

The main invariant is simple: **commits write deltas directly to UDB; workflow compaction is the only publish/delete authority for compaction output.**

## Read Path

Reads resolve the database pointer to a database branch, build a branch-aware read plan, and fetch each page through FDB-backed coverage:

1. Read branch head or fork head metadata.
2. Return missing for pages above EOF.
3. Check PIDX and DELTA first.
4. If the DELTA is absent or reclaimed, fall back to the newest SHARD at or below the read cap.
5. Zero-fill only valid gaps inside the database size.

Missing required DELTA/SHARD coverage below EOF is a storage error. The in-process PIDX and branch ancestry caches are perf caches only; correctness comes from UDB rows and workflow revalidation.

## Write Path

SQLite commits call Depot through the conveyer path:

1. Resolve DBPTR and read the current branch head in the UDB transaction.
2. Encode dirty pages into LTX DELTA chunks.
3. Write COMMITS, VTX, DELTA, and PIDX rows.
4. Update META/head and quota counters.
5. After commit, update SQLITE_CMP_DIRTY and send a throttled DeltasAvailable wake when hot lag crosses thresholds.

The commit path does **not** publish SHARD rows or delete old history. It records new committed history and wakes workflow compaction.

## Workflow Compaction

Each active database branch has one DB manager workflow plus hot and reclaimer companions, all unique by database branch id.

- Hot jobs stage LTX shard blobs under `CMP/stage/{job_id}/hot_shard`; the manager validates the active job, copies output to reader-visible `SHARD`, advances `CMP/root`, writes selected `PITR_INTERVAL` rows, and compare-clears matching PIDX.
- Reclaim jobs delete hot rows only after the manager proves replacement coverage against branch pins, restore points, PITR intervals, PIDX, SHARD rows, lifecycle generation, and current branch state.

`CMP/root` watermarks are scheduling summaries, not deletion proof by themselves. `CompactionRoot` retains legacy cold watermark fields for persisted compatibility, but OSS Depot does not update or act on them.

## PITR And Restore

Automatic timestamp restore coverage is stored as `PITR_INTERVAL` rows selected during hot compaction from commit wall-clock timestamps and the effective bucket/database PITR policy. Expired interval rows are soft pins until reclaim compare-clears them.

Restore points are retained user tokens. Creating a restore point resolves a `SnapshotSelector` to exact branch, txid, versionstamp, and wall-clock metadata, then writes a `RestorePointRecord` and `DB_PIN(kind=RestorePoint)`. Deleting it removes that hard pin and recomputes branch pin floors.

Fork and restore use the same primitive: resolve a snapshot selector, derive a branch at that exact point, and let the caller decide whether to keep a fork or move the database pointer.

## Cross-References

- Key layout: [sqlite/storage-structure.md](sqlite/storage-structure.md)
- Component ownership: [sqlite/components.md](sqlite/components.md)
- VFS parity rules: [sqlite-vfs.md](sqlite-vfs.md)
- Storage metrics: [SQLITE_METRICS.md](SQLITE_METRICS.md)
