# SQLite Constraints And Design Decisions

This page records the constraints that shape the PITR/forking storage design. These are not incidental implementation details.

## Binding Constraints

- **Single writer per database.** Pegboard exclusivity is the release-mode concurrency fence. Storage does not implement multi-writer conflict resolution.
- **No local SQLite files.** The durable database state is in FDB. Local files would make storage stateful and non-migratable.
- **Lazy reads.** Forks do not copy data. Reads walk branch ancestry and hydrate from FDB DELTA/SHARD rows only when needed.
- **Per-commit granularity.** PITR targets commits/versionstamps, not individual WAL frames inside a commit.
- **FDB is the source of truth.** OSS Depot has no object-backed cold tier.
- **Branches are immutable.** A bucket id is its bucket branch id, and a database id is its database branch id.
- **Rollback is engine-owned.** Storage exposes fork primitives; the engine decides which database id a database currently uses.
- **Persisted wire/storage records use vbare.** Raw fixed-width bytes are reserved for atomic counters and simple indexes such as `VTX`.

## Rough PITR By Default

The design keeps rough PITR cheap by preserving enough FDB history for branch-at-position recovery without writing a full image for every commit. Exact recovery is opt-in through restore points, which write FDB history pins that workflow compaction must preserve.

Compared with Neon's exact-PITR posture, this trades precision for lower steady-state cost. That fits Rivet Database-style workloads where "fork near this point" is usually enough, and exact restore points can be created explicitly for critical moments.

## Pages Are Self-Describing

LTX layers carry page numbers and checksums. That lets the system move bytes between DELTA and SHARD rows without a separate opaque page map. FDB PIDX remains the hot routing index.

The result is an LSM-shaped flow:

- L0: DELTAs in FDB.
- L1: versioned SHARDs in FDB.

## Why Versioned SHARDs

Overwriting one SHARD key loses the older materialized image that forks and restore points may still need. Versioned SHARDs keep multiple materializations side by side:

```text
SHARD/{shard_id}/{as_of_txid}
```

Reads choose the largest `as_of_txid <= read_txid`. Reclaim deletes old versions only after descendant/restore-point pins and PITR coverage allow it.

This avoids read-your-writes ambiguity after a fork. A child branch can refer to the parent's state at a specific versionstamp while the parent continues compacting and writing newer SHARD versions.

## Why Immutable Branch Ids

Earlier pointer-based designs needed mutable DBPTR/BUCKET_PTR rows, pointer history, frozen branch state, and cache invalidation when rollback swapped a pointer. The v4 model removes that storage-layer rollback primitive.

Now the external id is the branch id. Forking creates a new immutable branch record with a parent link. Engine-layer rollback is just: fork a new database at the target versionstamp, then update the engine-owned database mapping.

This keeps storage invariants simple:

- A `DatabaseId` never points to a different branch later.
- A `BucketId` never points to a different branch later.
- Branch records are write-once and then removed only by refcount/pin GC.
- Storage does not need pointer-history audit logs or rollback cache invalidation.

## Why Lazy Bucket Catalog Inheritance

Bucket forks must be metadata-only. Eagerly copying every database membership entry would make fork time proportional to bucket size.

`BUCKET_CATALOG` solves this with parent walks. A bucket fork starts with an empty local catalog and inherits parent catalog entries whose versionstamp is at or before the fork's `parent_versionstamp`. Tombstones hide inherited entries only along the relevant bucket path.

## Why Split Manifest Fields

`BranchManifest` is a legacy logical struct whose fields are stored under separate subkeys:

- Workflow compaction owns current publish/delete decisions through `CMP/root` and manager state.
- Access-touch owns `last_access_ts_ms` and `last_access_bucket`.
- Legacy hot pass keys may exist in older data, but workflow reclaim does not use them as deletion fences.

Split keys avoid unrelated read-modify-write conflicts.
