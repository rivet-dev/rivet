# SQLite Storage Components

Depot is split into the conveyer hot path plus workflow compaction. Workflow compaction is the only compaction publish/delete authority.

## Conveyer

The conveyer is the request path used by the SQLite VFS.

Responsibilities:

- Resolve bucket/database branch ancestry for reads.
- Commit dirty pages as LTX DELTA chunks under `BR/{database_id}/DELTA/{txid}/{chunk}`.
- Write PIDX owner rows for dirty pages.
- Write `COMMITS/{txid}` and `VTX/{versionstamp}` in the same commit transaction.
- Maintain `META/head`, quota counters, and access-touch manifest fields.
- Update `SQLITE_CMP_DIRTY/{database_branch_id}` and send throttled `DeltasAvailable` workflow wakeups when hot lag crosses compaction thresholds.
- Create buckets, create databases, fork buckets, fork databases, and write branch records/catalog markers.
- Create and resolve restore points. Pinned restore points write FDB pins directly and start as `PinStatus::Ready`.

Lease ownership: none. Correctness relies on Pegboard single-writer exclusivity for a live database plus FDB transaction fences. The conveyer must not take compactor leases.

## Workflow Compaction

The workflow compaction path uses one persistent DB manager plus hot and reclaim companion workflows per database branch.

Responsibilities:

- Coalesce commit wakeups through `SQLITE_CMP_DIRTY/{database_branch_id}` and `DeltasAvailable` signals.
- Plan hot jobs from current FDB state instead of trusting signal payloads.
- Carry the branch lifecycle generation through planned jobs and reject stale stage, publish, or reclaim work after branch deletion or recreation.
- Have the hot companion write staged shard blobs under `CMP/stage/{job_id}/hot_shard`.
- Install matching hot job output by copying staged blobs to reader-visible `SHARD`, advancing `CMP/root`, and compare-and-clearing expected PIDX rows.
- Have the reclaimer delete only manager-authorized FDB rows and stale staged output.
- Keep automatic PITR interval coverage and retained restore point pins live until reclaim can prove they are no longer needed.
- Stop the manager and companion workflows through `DestroyDatabaseBranch` when a database branch is no longer live.

Lease ownership: none. Gasoline workflow uniqueness uses only the database branch id tag.

## Ownership Summary

| Component | Main writes | Lease |
|---|---|---|
| Conveyer | `META/head`, `COMMITS`, `VTX`, `PIDX`, `DELTA`, branch records, restore points | None |
| Workflow DB manager | `CMP/root`, live `SHARD`, `PITR_INTERVAL`, matching PIDX clears | None |
| Workflow companions | Staged hot output and manager-authorized FDB cleanup | None |

The components share branch metadata and pin counters, but each mutable manifest field has one owner.
