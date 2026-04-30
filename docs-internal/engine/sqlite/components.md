# SQLite Storage Components

SQLite storage is split into the pump hot path plus three compactors. Each component owns a narrow write surface and lease model.

## Pump

The pump is the request path used by the SQLite VFS.

Responsibilities:

- Resolve namespace/database branch ancestry for reads.
- Commit dirty pages as LTX DELTA chunks under `BR/{database_id}/DELTA/{txid}/{chunk}`.
- Write PIDX owner rows for dirty pages.
- Write `COMMITS/{txid}` and `VTX/{versionstamp}` in the same commit transaction.
- Maintain `META/head`, quota counters, access-touch manifest fields, and the global eviction index bucket.
- Create namespaces, create databases, fork namespaces, fork databases, and write branch records/catalog markers.
- Create and resolve bookmarks. Pinned bookmarks publish cold-compactor work and start as `PinStatus::Pending`.

Lease ownership: none. Correctness relies on Pegboard single-writer exclusivity for a live database plus FDB transaction fences. The pump must not take compactor leases.

## Hot Compactor

The hot compactor folds recent DELTA rows into versioned SHARD rows inside FDB.

Responsibilities:

- Read unmaterialized DELTAs and current SHARD versions.
- Write new `SHARD/{shard_id}/{as_of_txid}` rows instead of overwriting a single shard key.
- Delete folded DELTAs only after `cold_drained_txid` covers them.
- Update `META/compact` and `META/manifest/last_hot_pass_txid`.
- Enforce `MAX_SHARD_VERSIONS_PER_SHARD` by evicting the oldest unpinned version inline when safe.
- GC hot `COMMITS` and matching `VTX` rows below the hot retention floor when pin math permits.
- Publish cold-compactor work when hot/cold lag crosses the drain threshold.

Lease ownership: per-database `BR/{database_id}/META/compactor_lease`.

## Cold Compactor

The cold compactor moves durable history from FDB into S3 or the filesystem cold-tier test backend.

Responsibilities:

- Upload SHARD image layers, DELTA layers, pinned bookmark images, branch records, cold manifest chunks, and catalog snapshots.
- Advance `META/cold_compact` and `META/manifest/cold_drained_txid` only after cold objects are durable.
- Transition uploaded pinned bookmarks from `Pending` to `Ready`, or to `Failed` on upload failure.
- Rewrite manifest metadata during follow-up sweeps and delete cold objects that fall below the branch GC pin.
- Clean stale `pending/{uuid}.marker` objects by deleting the object keys recorded in the marker.

Lease ownership: per-database `BR/{database_id}/META/cold_lease`, with local timer renewal and a cancel token. UPS uses `SqliteColdCompactSubject` and queue group `cold_compactor`.

### Phase A

Phase A is the durability handoff and planning phase:

1. Take the cold lease.
2. Record `in_flight_uuid` in `META/cold_compact` in a short FDB transaction.
3. PUT `pending/{uuid}.marker` outside any FDB transaction.
4. Snapshot-read branch state, SHARDs, DELTAs, COMMITS, VTX rows, bookmarks, and branch records to build the upload plan.

The marker PUT is deliberately outside FDB transaction age limits.

### Phase B

Phase B is S3-only:

1. Upload image, delta, and pin LTX files.
2. Upload branch records if absent.
3. Write the immutable manifest chunk and rewrite the small manifest index.
4. Write the catalog snapshot.
5. Delete stale pending markers and their listed object keys.

No FDB transaction is open during this phase.

### Phase C

Phase C is the FDB finalizer:

1. Check the cold lease.
2. Regular-read `cold_drained_txid` and assert it still matches the Phase A plan.
3. Advance cold state and manifest cursor fields.
4. Mark successfully uploaded pins `Ready`.
5. Clear `in_flight_uuid`.

If the OCC fence fails, the pass retries from Phase A. Uploaded objects are idempotent because object keys are deterministic.

## Eviction Compactor

The eviction compactor clears hot-tier FDB bytes once the cold tier has durable coverage.

Responsibilities:

- Scan `CTR/eviction_index` oldest bucket first.
- Plan SHARD and PIDX clears only when the database is outside the hot-cache window.
- Require a newer SHARD version, cold-drain coverage, hot-pass margin, and no descendant/bookmark pin before clearing.
- Regular-read `META/manifest/last_hot_pass_txid` during planning and again during clear to avoid racing hot compaction.
- Use compare-and-clear semantics for planned SHARD/PIDX values.
- Remove the eviction-index key only when the branch is fully evicted.

Lease ownership: global `CMPC/lease_global/{kind=eviction}`. Sweeps are batch-limited; multiple pods coordinate through the single global lease.

## Ownership Summary

| Component | Main writes | Lease |
|---|---|---|
| Pump | `META/head`, `COMMITS`, `VTX`, `PIDX`, `DELTA`, branch records, bookmarks | None |
| Hot compactor | `SHARD`, `META/compact`, `META/manifest/last_hot_pass_txid` | `META/compactor_lease` |
| Cold compactor | S3 objects, `META/cold_compact`, `META/manifest/cold_drained_txid`, pin status | `META/cold_lease` |
| Eviction compactor | Clears FDB SHARD/PIDX/DELTA rows, eviction index | `CMPC/lease_global/{kind=eviction}` |

The components share branch metadata and pin counters, but each mutable manifest field has one owner.
