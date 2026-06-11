# SQLite Constraints And Design Decisions

This page records the constraints that shape the PITR/forking storage design. These are not incidental implementation details.

## Binding Constraints

- **Single writer per database.** Pegboard exclusivity is the release-mode concurrency fence. Storage does not implement multi-writer conflict resolution.
- **No local SQLite files.** The durable database state is in UDB. Local files would make storage stateful and non-migratable.
- **Lazy reads.** Forks do not copy data. Reads walk branch ancestry and hydrate from UDB DELTA/SHARD rows only when needed.
- **Per-commit granularity.** PITR targets commits/versionstamps, not individual WAL frames inside a commit.
- **UDB is the source of truth.** OSS Depot has no object-backed cold tier.
- **Branches are immutable.** A bucket id is its bucket branch id, and a database id is its database branch id.
- **Rollback is engine-owned.** Storage exposes fork primitives; the engine decides which database id a database currently uses.
- **Persisted wire/storage records use vbare.** Raw fixed-width bytes are reserved for atomic counters and simple indexes such as `VTX`.

## Rough PITR By Default

The design keeps rough PITR cheap by preserving enough UDB history for branch-at-position recovery without writing a full image for every commit. Exact recovery is opt-in through restore points, which write UDB history pins that workflow compaction must preserve.

Compared with Neon's exact-PITR posture, this trades precision for lower steady-state cost. That fits Rivet Database-style workloads where "fork near this point" is usually enough, and exact restore points can be created explicitly for critical moments.

## Pages Are Self-Describing

LTX layers carry page numbers and checksums. That lets the system move bytes between DELTA and SHARD rows without a separate opaque page map. UDB PIDX remains the hot routing index.

The result is an LSM-shaped flow:

- L0: DELTAs in UDB.
- L1: versioned SHARDs in UDB.

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

## Why Fork Alignment And Watermark Delta Retention

Delta retention and fork resolution are designed as one mechanism. They must be
read together; changing one without the other corrupts data.

### The trade

The simple option is to retain every `DELTA` chunk for the full PITR window so a
fork or restore can reconstruct any historical txid. That makes storage scale
with the retention window and forces reclaim to prove, per shard, that a
replacement shard version exists before deleting any delta. In practice that
proof starves: it requires a shard version at exactly the current watermark for
every shard a delta touches, which a multi-shard database rarely satisfies, so
deltas accumulate without bound.

Instead the delta rule is trivial:

> A `DELTA` chunk is reclaimable once its txid is at or below the hot watermark,
> with no per-shard or PIDX proof.

This is sound because hot compaction's install transaction advances
`CMP/root.hot_watermark_txid` **and** publishes the shard coverage for every
covered txid at or below the new watermark atomically. "Below the watermark"
therefore means "reconstructable from the newest `SHARD` version at or below
that txid" for covered points, and only for covered points.

### Fork alignment is the constraint that keeps the trade sound

A fork, pin, or restore may only target a txid that is one of:

- above the watermark (its deltas still exist; creating the pin makes the next
  hot install stage shard coverage for that exact txid), or
- an already-covered txid at or below the watermark: the watermark itself, a
  retained `PITR_INTERVAL` representative, or an existing `DB_PIN`.

A caller-supplied versionstamp that lands between covered points is **snapped
down** to the newest covered point at or below it. Because reads resolve "newest
`SHARD` version at or below the read cap", a snapped fork reads exactly that
covered snapshot. `AtTimestamp` resolution already reads `PITR_INTERVAL` rows, so
timestamp forks were always aligned; the fence (`conveyer::coverage`) and
`snap_covered_target` extend the same rule to caller-supplied-versionstamp forks,
rollbacks, restore points, and bucket-fork pins.

Removing either half breaks the other. Without alignment, a fork could pin a
below-watermark txid whose deltas reclaim just deleted, and reads would silently
zero-fill or serve stale shard bytes. Without the simple delta rule, the
per-shard proof and full-window delta retention return.

### Recorded coverage, not clock synchronization

Alignment never compares wall-clock time across machines. The points a fork may
align to are durable rows recorded per database branch:

- `PITR_INTERVAL` rows: hot compaction selects one representative commit per
  interval bucket and writes `(txid, versionstamp, wall_clock_ms, expires_at_ms)`.
- `DB_PIN` rows: concrete `(txid, versionstamp)` for restore points, database
  forks, and bucket forks.

Snapping then compares **UDB versionstamps** (monotonic, globally
ordered commit tokens) against those recorded rows: it picks the covered row
with the largest txid whose `versionstamp <= fork_versionstamp`. No clock is
consulted in that decision. A bucket fork carries one `fork_versionstamp`, and
each inherited database resolves it independently against that database branch's
own recorded rows.

Wall-clock appears in two single-node, serializable-fenced places only: hot
compaction buckets commits into PITR intervals by each commit's own recorded
`wall_clock_ms`, and the fence plus `snap_covered_target` skip interval rows
whose `expires_at_ms` is past the local `now_ms`. Both reads are Serializable, so
a fork racing reclaim either conflicts and retries or observes a consistent
snapshot; a skewed clock can only admit or reject a fork slightly early or late,
never resolve it to the wrong data.
