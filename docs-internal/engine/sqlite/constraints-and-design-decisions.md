# SQLite Constraints And Design Decisions

This page records the constraints that shape the PITR/forking storage design. These are not incidental implementation details.

## Binding Constraints

- **Single writer per database.** Pegboard exclusivity is the release-mode concurrency fence. Storage does not implement multi-writer conflict resolution.
- **No local SQLite files.** The durable database state is in FDB plus cold-tier objects. Local files would make storage stateful and non-migratable.
- **Lazy reads.** Forks do not copy data. Reads walk branch ancestry and hydrate from FDB or cold layers only when needed.
- **Per-commit granularity.** PITR targets commits/versionstamps, not individual WAL frames inside a commit.
- **FDB is the hot source of truth.** S3 is retained history and disaster-recovery material, not the synchronous commit authority.
- **Branches are immutable.** A namespace id is its namespace branch id, and a database id is its database branch id.
- **Rollback is engine-owned.** Storage exposes fork primitives; the engine decides which database id an database currently uses.
- **Persisted wire/storage records use vbare.** Raw fixed-width bytes are reserved for atomic counters and simple indexes such as `VTX`.

## Rough PITR By Default

The design keeps rough PITR cheap by preserving enough history for branch-at-position recovery without writing a full image for every commit. Exact recovery is opt-in through pinned bookmarks, which ask the cold compactor to upload a full image for that versionstamp.

Compared with Neon's exact-PITR posture, this trades precision for lower steady-state cost. That fits Rivet Database-style workloads where "fork near this point" is usually enough, and exact bookmarks can be created explicitly for critical moments.

## Pages Are Self-Describing

LTX layers carry page numbers and checksums. That lets the system move bytes between DELTA, SHARD, and cold image/delta/pin layers without a separate opaque page map inside the object. FDB PIDX remains the hot routing index, while cold manifests describe which layer ranges can satisfy a historical read.

The result is an LSM-shaped flow:

- L0: DELTAs in FDB.
- L1: versioned SHARDs in FDB.
- L2: image, delta, and pin layers in S3.

## Why Versioned SHARDs

Overwriting one SHARD key loses the older materialized image that forks and bookmarks may still need. Versioned SHARDs keep multiple materializations side by side:

```text
SHARD/{shard_id}/{as_of_txid}
```

Reads choose the largest `as_of_txid <= read_txid`. Eviction deletes old versions only after cold coverage exists and descendant/bookmark pins allow it.

This avoids read-your-writes ambiguity after a fork. A child branch can refer to the parent's state at a specific versionstamp while the parent continues compacting and writing newer SHARD versions.

## Why Immutable Branch Ids

Earlier pointer-based designs needed mutable DBPTR/NSPTR rows, pointer history, frozen branch state, and cache invalidation when rollback swapped a pointer. The v4 model removes that storage-layer rollback primitive.

Now the external id is the branch id. Forking creates a new immutable branch record with a parent link. Engine-layer rollback is just: fork a new database at the target versionstamp, then update the engine-owned database mapping.

This keeps storage invariants simple:

- A `DatabaseId` never points to a different branch later.
- A `NamespaceId` never points to a different branch later.
- Branch records are write-once and then removed only by refcount/pin GC.
- Storage does not need pointer-history audit logs or rollback cache invalidation.

## Why Lazy Namespace Catalog Inheritance

Namespace forks must be metadata-only. Eagerly copying every database membership entry would make fork time proportional to namespace size.

`NSCAT` solves this with parent walks. A namespace fork starts with an empty local catalog and inherits parent catalog entries whose versionstamp is at or before the fork's `parent_versionstamp`. Tombstones hide inherited entries only along the relevant namespace path.

## Why Split Manifest Fields

`BranchManifest` is logical, but its fields are stored under separate subkeys because different components own them:

- Cold compactor owns `cold_drained_txid`.
- Hot compactor owns `last_hot_pass_txid`.
- Access-touch owns `last_access_ts_ms` and `last_access_bucket`.

Split keys avoid unrelated read-modify-write conflicts and give eviction a precise OCC fence on `last_hot_pass_txid`.

## Why Phase A/B/C Cold Compaction

FDB transactions have strict age limits; S3 latency does not. Cold compaction therefore separates:

- Phase A: short FDB handoff plus planning.
- Phase B: S3-only uploads.
- Phase C: short FDB finalization with an OCC fence.

The pending marker bridges crashes between those phases. It records the object keys a pass may leak so the next pass can clean them deterministically.
