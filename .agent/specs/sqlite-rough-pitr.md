# SQLite Rough PITR + Two-Level Branching

## Revision: post-rollback-removal v4

This revision removes the rollback primitive from the storage API. Storage exposes only `fork_database` and `fork_namespace`; rollback semantics are implemented at the engine layer (pegboard's database abstraction, separately renamed to `Db` by US-065) by calling `fork_database` and flipping its own `database → current_database_id` mapping. With rollback gone, the mutable pointer indirection collapses: `database_id` IS the database branch id (one type, not two), `namespace_id` IS the namespace branch id (one type, not two). The `DBPTR` and `NSPTR` partitions (with `cur` + `history` sub-keys) are deleted along with `BranchState::Frozen`, the `last_swapped_at_ms` cache invalidation contract (fix #21), pointer history audit logs and their GC (fix #24), Frozen branch retention (fix #23), and the rolled-back-vs-commit race window. `fork_namespace` remains O(1) via lazy namespace-branch parent-chain inheritance, but the inheritance mechanism changes: a new `NSCAT/{namespace_id}/{database_id}` presence-marker partition records "this database is visible in this namespace branch" with cap-by-versionstamp filtering for fork_namespace AS-OF correctness. GC of orphaned databases is engine-owned; storage exposes `delete_database` / `delete_namespace` primitives the engine calls. All other v3 changes (uniform LSM flow, no tier model, hot retention floor) remain in force.

## Revision: post-tier-flattening v3

This revision removes the per-namespace tier model (T0/T1/T2) and replaces it with a single uniform LSM-shaped storage flow. Every namespace now behaves identically: hot-tier (FDB) DELTAs fold into versioned SHARDs, cold-tier (S3) image/delta/pin layers receive cold passes, and eviction reclaims hot-tier bytes once cold has caught up. The cold compactor and eviction compactor always run when triggered; small or idle namespaces simply produce little work. The mutable `tier_state` key is gone, the `Tier` enum and `NamespaceTierState` struct are gone, the `ensure_tier_at_least` CAS helper is gone, and DR posture is uniform across all namespaces (RPO bounded by cold-pass cadence, not by an opt-in tier setting). The hot-retention floor (renamed from `COMMITS_TIER0_RETENTION_MS` to `HOT_RETENTION_FLOOR_MS`) applies universally as the floor below which every namespace's hot-tier COMMITS + VTX rows are eligible for GC. All other v2 fixes (#1, #4, #5, #6, #7, #8, #11, #13, #14, #15, #17, #18, #19, #20, #21, #22, #23, #24, #25) remain in force.

## Revision: post-adversarial-review v2

This is a revision of v1 incorporating findings from architecture, performance, and operations reviews (`.agent/research/review-rough-pitr-{architecture,performance,operations}.md`). Critical fixes applied:

- DBPTR is per-namespace-branch (not global) — fixes namespace fork incoherence
- Added `root_versionstamp` on branch records (replaces nonexistent `created_at_versionstamp`)
- Added `/META/head_at_fork` snapshot for fresh-fork reads
- Added `VTX` versionstamp → txid index
- Added eviction OCC fence on `last_hot_pass_txid`
- Restructured Phase A to keep S3 PUTs out of FDB tx
- Reordered restore_to_bookmark; pin is post-rollback
- Added `pointer_snapshot.bare` for DR
- Added universal hot-tier retention rules (originally Tier 0 retention; v3 makes it uniform)
- Added eviction-index throttle
- Honest hot-path RTT accounting in §17
- Pinned bookmark create is async via cold compactor
- Bookmark resolution bridges namespace forks
- One-shard-per-image-file constraint
- schema_version dropped from FDB records (kept on S3 only)
- MAX_SHARD_VERSIONS_PER_SHARD cap
- Cold manifest chunked
- MAX_PINS_PER_NAMESPACE cap
- list_databases algorithm with tombstones
- Cache invalidation contract
- MAX_NAMESPACE_DEPTH = 16
- BranchState::Frozen restored
- DBPTR history GC rule
- 8 added test scenarios

Open issues flagged but not fixed: bookmark resolution complexity bound (worst case `MAX_FORK_DEPTH × MAX_NAMESPACE_DEPTH`), multi-region (future work), per-bookmark pin recompute O(n) on delete.

---

A new design extending the stateless SQLite storage architecture (`.agent/specs/sqlite-storage-stateless.md`) with lightweight point-in-time recovery at rough granularity (nearest checkpoint by default, exact precision via explicit pinning), two-level branch indirection for both namespaces and databases, and a uniform LSM-shaped two-level storage flow (FDB hot tier → S3 cold tier) that applies to every namespace.

## 1. Goals

1. **Rough PITR by default, exact PITR opt-in.** A user issuing a fork at an arbitrary `(timestamp_ms, txid)` resolves to the nearest preserved checkpoint without synchronous extra work. Exact-precision recovery requires an explicit `create_pinned_bookmark` call that synchronously writes a full image at that moment. The system does not pay the storage cost of preserving every commit's full materialization just because it might be the fork target.
2. **Single-layer immutable branch model.** A namespace id IS its namespace branch id; a database id IS its database branch id. There is no mutable pointer indirection: branches are immutable for life and deleted only when refcount reaches zero. Engine-layer "rollback" is implemented by calling `fork_database` to derive a new database id at the rollback point and flipping the engine's own `database → current_database_id` mapping. Storage exposes no rollback primitive.
3. **Two operations cover the full surface.** `fork_namespace`, `fork_database`. Both are O(1) metadata operations at call time. Lazy materialization on first divergent write keeps the fork itself genuinely zero-copy. Rollback at the engine level is implemented by calling `fork_database` and flipping the engine's database→database mapping. Storage exposes no rollback primitive.
4. **Three compactors with non-overlapping responsibilities.** Hot (FDB-internal SHARD folding), cold (FDB to S3 image upload), eviction (FDB clear once S3 has a durable image). Each holds its own lease; each writes a disjoint META sub-key.
5. **One uniform LSM-shaped storage flow.** Every namespace's data flows through the same pipeline: hot-tier DELTAs fold into versioned SHARDs (FDB), then cold-tier image/delta/pin layers (S3). Cold compactor and eviction compactor always run; idle or small namespaces simply have little to do. There is no per-namespace tier mode, no T0/T1/T2 distinction, and no CAS-promotion path. Operators tune cadence and quotas globally.
6. **Stateless hot path preserved.** Pegboard-envoy adds no per-database state for branching. The branch pointer chain is resolved per-request from FDB. The PIDX flattened-ancestry cache is the only addition, and it is per-conn perf cache, dropped on WS close.
7. **Storage layer exposes primitives only.** Cache invalidation on rollback, database restart on rollback, swap-in semantics, and quota for fork count are all upstream concerns. This package answers "what is the current state at this versionstamp on this branch?" and nothing else.
8. **Loss of FDB between cold passes is a hot-tier disaster, not a PITR failure.** S3 is for retention beyond the FDB cache window. Durability of a commit is still bounded by FDB.

## 2. Non-goals

- **Sub-commit PITR.** Granularity is per-commit. No WAL-frame-level shipping.
- **Multi-writer.** Pegboard exclusivity still holds at the database level. Forks produce new database IDs that have their own exclusivity owner; the source database is unaffected.
- **Cross-region replication.** Future work; called out in section 25.
- **Synchronous catalog materialization on namespace fork.** Namespace fork captures the database catalog as-of versionstamp V via lazy parent-chain resolution; it does not eagerly enumerate every database at fork time.
- **Migrating existing on-disk format.** This is a forward-only design. The format described here is the stateless v2 layout extended; the system has not shipped to production.
- **Cross-database bookmarks.** Bookmarks resolve in the context of a single database (or its namespace); resolving against a different database returns `BranchNotReachable`.
- **GC-by-wallclock-cutoff.** GC is dependency-graph-based; pin computation drives deletion eligibility, not "older than 30 days".

## 3. Inherited constraints

The following are binding floors carried over from prior docs. This spec does not relax any of them.

- `engine/packages/sqlite-storage/CLAUDE.md` — single writer per database (pegboard exclusivity is the only release-mode fence), no local SQLite files anywhere, lazy read only, per-commit granularity, all database state is per-branch under `[0x02][database_id][BR][branch_id]/<suffix>`, `MAX_FORK_DEPTH = 16`, `MAX_NAMESPACE_DEPTH = 16` (enforced in `fork_namespace`), `PITR/forking/restore_to_bookmark` are all the same primitive (branch-at-position).
- `.agent/specs/sqlite-storage-stateless.md` — pump/compactor split, `Db` per-database handle with PIDX cache, `/META/head` (commit-owned), `/META/compact` (hot-compactor-owned), `/META/quota` (atomic counter, raw i64 LE), `/META/compactor_lease`, `COMMITS/{txid_be:8}` carries `wall_clock_ms` only, `post_apply_checksum` lives on `/META/head`, `COMPARE_AND_CLEAR` for PIDX deletes, lease-via-local-timer-and-cancel-token, three-mechanism concurrency model.
- LTX V3 file format (zeroed 6-byte page-header sentinel before the varint page index, full-frame size accounting in offsets/sizes).
- `vbare::OwnedVersionedData` for every persisted/wire blob except `/META/quota` (which is a fixed-width LE atomic counter). Per `engine/CLAUDE.md`, FDB-resident records use the `vbare::OwnedVersionedData` enum-of-versions pattern; they do not carry a separate `schema_version: u32` field.
- Schema version on every persisted S3 object (`schema_version: u32` on every cold-tier vbare-encoded record). Applies only to S3-persisted records, not FDB-resident records.
- HWM `pending/{uuid}.marker` for orphan reconciliation across cold-pass crashes.
- S3 lifecycle policies are deployment-disabled; cold compactor's GC owns retention.
- Layer filenames omit content checksums (re-uploads are idempotent overwrites).

## 4. Architectural overview

```
External ID (== branch id)                                 Immutable Branch
──────────────────────────────                             ────────────────
namespace_id  ════════════════════════════════════════▶  /NSBRANCH/list/{namespace_id}/...  (parent + parent_versionstamp)
                                                         │
                                                         └─ immutable; deleted only when refcount = 0

database_id   ════════════════════════════════════════▶  /BRANCHES/list/{database_id}/...   (parent + parent_versionstamp)
                                                         │
                                                         └─ immutable; deleted only when refcount = 0

Database membership in a namespace branch (visibility):
   /NSCAT/{namespace_id}/{database_id}                   → presence marker (lazy parent walk on read)
```

There is no separate "branch id" type: the external `namespace_id` IS the namespace branch id, and the external `database_id` IS the database branch id. Branches are immutable for life. When the engine wants to "rollback" a database, it calls `fork_database` to derive a new database id at the rollback versionstamp and flips its own `database → current_database_id` mapping (engine-layer state, not storage). Storage exposes no rollback primitive.

`NSCAT/{namespace_id}/{database_id}` is a presence-marker partition recording "this database was created in (or forked into) this namespace branch." On `fork_namespace`, the new namespace branch starts with empty `NSCAT` and lazy-resolves to parent namespaces via parent walk capped by `parent_versionstamp` so a forked namespace does not see databases created in the source namespace after the fork point.

### Compactors

| Name | Inputs | Outputs | Trigger |
|---|---|---|---|
| Hot | DELTA blobs, current SHARD versions | New versioned SHARDs in FDB; deletes folded DELTAs | UPS, threshold-based |
| Cold | FDB SHARD versions, COMMITS, branch metadata | S3 image layers, S3 manifests | UPS, lag-derived |
| Eviction | Global eviction index | Clears database's hot-tier resident set | Periodic sweep |

All three compactors run unconditionally when triggered. There is no per-namespace mode that disables them. Idle or small namespaces simply produce little work for cold + eviction; the trigger predicates (lag thresholds, hot-cache window, OCC fences) are the only gates. Hot compactor is the same compactor described in the stateless spec, extended to produce versioned SHARDs and to consult the branch parent chain when deciding what to fold. Cold and eviction are new.

### Two fork levels

- **fork_database** — clones one database at a versionstamp. Source database unaffected. New `database_id` is allocated and IS the new database branch id; its `parent` is the source's id and `parent_versionstamp` is the fork point. Writes a `NSCAT/{target_namespace_id}/{new_database_id}` presence marker so the new database is visible in the target namespace.
- **fork_namespace** — clones an entire namespace at a versionstamp. New `namespace_id` is allocated and IS the new namespace branch id; its `parent` is the source's id and `parent_versionstamp` is the fork point. NSCAT entries are NOT eagerly copied; reads walk the namespace parent chain capped by `parent_versionstamp` for AS-OF correctness.

Both forks are O(1) metadata. Lazy materialization on first divergent write means the fork costs zero data bytes at fork time.

## 5. Data model

### 5.1 Identifiers

Two id types — and only two. The external id IS the branch id.

```rust
/// External, stable, never reassigned. Returned from create APIs.
/// Engine-edge clients hold these. Length-stable wire format.
/// IS the namespace branch id; there is no separate `NamespaceBranchId`
/// or `NamespacePointerId` type. The branch record at
/// `/NSBRANCH/list/{namespace_id}` is immutable; "rollback" semantically
/// is a fork at the engine layer.
pub struct NamespaceId(Uuid);

/// External, stable, never reassigned. Returned from `create_database` /
/// `fork_database`. IS the database branch id; there is no separate
/// `DatabaseBranchId` or `DatabasePointerId` type. The branch record at
/// `/BRANCHES/list/{database_id}` is immutable; "rollback" semantically
/// is a fork at the engine layer.
pub struct DatabaseId(Uuid);

/// Result of resolving a bookmark or wall-clock to a concrete versionstamp.
/// Passed into the two operations as the AS-OF coordinate.
pub struct ResolvedVersionstamp {
    pub versionstamp: [u8; 16],
    pub bookmark: Option<BookmarkRef>,
}
```

Why one type per entity: with rollback removed at the storage layer, branches are immutable for life and a database is bound to its branch one-for-one (a database's id never points at a different branch record over time). Same for namespaces. The previous v3 split of `NamespaceId` / `NamespacePointerId` / `NamespaceBranchId` (and the database analog) collapses to a single id type per entity. Engine-layer "rollback" is implemented by calling `fork_database` to allocate a new `database_id` and flipping the engine's database→database mapping; the old database id is unaffected.

Branch lifecycle: live while refcount > 0; deleted when refcount = 0 (and no descendant pin / bookmark pin holds retention). There is no `Frozen` lifecycle state — the rolled-back-vs-frozen retention scheme that v3 needed is unnecessary because old database ids are still referenced by pinned bookmarks or by descendant database branches and remain live until those references drop.

Forking is genuinely zero-copy — fork allocates a brand-new id (which IS the new branch id) plus a new immutable branch record whose `parent` field links into the existing graph. No data motion.

### 5.2 Versioned SHARDs

The hot tier carries multiple versions of each SHARD per branch. Versioning is per-branch and per-shard, not per-database or per-namespace.

```
SHARD/{shard_id_be:4}/{as_of_txid_be:8}  →  ltx_blob
```

`shard_id` is the existing 4-byte big-endian shard identifier carrying `SHARD_SIZE` pages. `as_of_txid` is the txid at which the SHARD's contents were materialized by a hot-compactor pass (i.e. the maximum txid folded into this SHARD plus implicit `+0` if no folding was needed). Multiple versions of the same `shard_id` coexist; reads pick the largest `as_of_txid <= read_txid`.

A read of pgno P at versionstamp V follows the standard PIDX-first path; on PIDX miss or fall-through to SHARD, the read does a reverse-range scan on `SHARD/{shard_id_for_P}/` constrained by `as_of_txid <= V`, returning the first hit.

Hot compactor produces a new versioned SHARD with `as_of_txid = max_folded_txid` rather than overwriting the previous version. The previous version is deletable only when the eviction compactor's predicate fires (newer version exists, older than hot-cache window, no descendant pin, no bookmark pin).

### 5.3 Namespace branch record

```rust
#[derive(Encode, Decode)]
pub struct NamespaceBranchRecord {
    /// Equals the external `NamespaceId`. Carried in the record for in-tx
    /// debug fence assertions.
    pub branch_id: NamespaceId,
    /// Parent at fork time. None for root namespaces. The fork's
    /// versionstamp is `parent_versionstamp` below.
    pub parent: Option<NamespaceId>,
    /// FDB versionstamp at which the parent was forked. Resolves
    /// reads to "before this version, see parent's commits".
    pub parent_versionstamp: Option<[u8; 16]>,
    /// FDB versionstamp at which THIS branch was created (its root point).
    /// Used by GC pin formula as the keep-floor when refcount > 0.
    /// For root branches with no parent, equals the genesis versionstamp.
    /// Also serves as the cap-by-versionstamp floor for NSCAT visibility:
    /// a NSCAT entry created at v <= root_versionstamp is inherited by
    /// descendants; one created at v > root_versionstamp on the source
    /// after a fork is invisible to descendants forked before that v.
    pub root_versionstamp: [u8; 16],
    /// The set of refcount and pin atomic-min keys is at known prefixes;
    /// this struct is the immutable side, mutable counters live alongside
    /// under `/refcount`, `/desc_pin`, `/bk_pin` sub-keys.
    pub created_at_ms: i64,
    pub created_from_bookmark: Option<BookmarkRef>,
}
```

There is no `state: BranchState` field and no `BranchState::Frozen` variant. Branches are live while refcount > 0 and deleted when refcount = 0 (subject to descendant + bookmark pin retention). With rollback removed, no branch ever transitions from "live" to "preserved-only-for-undo": engine-layer rollback creates a NEW database id and the old database id stays live as long as any reference (descendant, pinned bookmark, or engine-layer mapping) holds it.

### 5.4 Database branch record

```rust
#[derive(Encode, Decode)]
pub struct DatabaseBranchRecord {
    /// Equals the external `DatabaseId`.
    pub branch_id: DatabaseId,
    /// The namespace branch in which this database was created.
    /// `fork_database` writes a `NSCAT/{namespace}/{branch_id}` presence
    /// marker into this namespace at allocation time.
    pub namespace: NamespaceId,
    pub parent: Option<DatabaseId>,
    pub parent_versionstamp: Option<[u8; 16]>,
    /// FDB versionstamp at which THIS branch was created. Used by the GC
    /// pin formula and as the cap-by-versionstamp floor for NSCAT.
    pub root_versionstamp: [u8; 16],
    pub fork_depth: u8,  // 0 for root, capped at MAX_FORK_DEPTH = 16
    pub created_at_ms: i64,
    pub created_from_bookmark: Option<BookmarkRef>,
}
```

There is no `state: BranchState` field and no separate `DatabasePointer` struct. The external `DatabaseId` IS the database branch id; reads go straight from id to the immutable branch record at `/BRANCHES/list/{database_id}`. Engine-layer "rollback" allocates a new database id via `fork_database`; the engine flips its own `database → current_database_id` mapping (engine-owned, NOT a sqlite-storage concern).

### 5.5 Branch counters

Branch records are immutable except for deletion. Mutable counters and pins live in dedicated counter keys, atomic-add or atomic-min, never inside the immutable record. Counters are stored as raw fixed-width bytes (`i64 LE` for refcount, 16-byte BE for atomic-min versionstamps) per FDB atomic-op semantics; they are not vbare-encoded structs and carry no `schema_version`.

```text
[BRANCHES]/list/{database_id}/refcount  → i64 LE atomic-add
[BRANCHES]/list/{database_id}/desc_pin  → [u8; 16] atomic-min (descendant pin)
[BRANCHES]/list/{database_id}/bk_pin    → [u8; 16] atomic-min (bookmark pin)
```

Atomic-min on an absent key returns the sentinel `0xFF...FF` (infinity); the first atomic-min write overwrites that sentinel.

The single GC pin for a branch is `min(refcount > 0 ? branch.root_versionstamp : ∞, oldest_descendant_pin, oldest_bookmark_pin)`. Reads of the three counters are independent; the pin is computed at GC pass time, never persisted.

## 6. FDB schema

All keys live under the existing `[0x02]` prefix that the storage crate owns. The first byte after `[0x02]` distinguishes top-level partitions:

```
# NSCAT is the namespace catalog: a presence-marker partition recording
# "this database is visible in this namespace branch." Written by
# `fork_database` (and `create_database`) into the target namespace at
# allocation time. NOT written eagerly on `fork_namespace`; the new
# namespace branch starts with empty NSCAT and lazy-resolves to its parent
# namespace via parent walk capped by `parent_versionstamp` so a forked
# namespace does not see databases created in the source namespace AFTER
# the fork point. The 16-byte versionstamp suffix on each entry is what
# enables that cap-by-versionstamp filtering: list reads accept an entry
# only if its versionstamp <= the walking namespace's `parent_versionstamp`
# floor. Replaces v3's DBPTR + NSPTR pointer machinery.
[0x02][0x10] NSCAT /{namespace_id_uuid_be:16}/{database_id_uuid_be:16} → 16-byte FDB versionstamp at create time (via SetVersionstampedValue)

[0x02][0x20] BRANCHES /list/{database_id_uuid_be:16}                  → DatabaseBranchRecord (vbare-versioned)
[0x02][0x20] BRANCHES /list/{database_id_uuid_be:16}/refcount         → i64 LE atomic
[0x02][0x20] BRANCHES /list/{database_id_uuid_be:16}/desc_pin         → 16-byte versionstamp atomic-min
[0x02][0x20] BRANCHES /list/{database_id_uuid_be:16}/bk_pin           → 16-byte versionstamp atomic-min

[0x02][0x21] NSBRANCH /list/{namespace_id_uuid_be:16}                 → NamespaceBranchRecord (vbare-versioned)
[0x02][0x21] NSBRANCH /list/{namespace_id_uuid_be:16}/refcount        → i64 LE atomic
[0x02][0x21] NSBRANCH /list/{namespace_id_uuid_be:16}/desc_pin        → 16-byte versionstamp atomic-min
[0x02][0x21] NSBRANCH /list/{namespace_id_uuid_be:16}/bk_pin          → 16-byte versionstamp atomic-min
[0x02][0x21] NSBRANCH /list/{namespace_id_uuid_be:16}/database_tombstones/{database_id_uuid_be:16} → empty (database deletion in this namespace branch; see §10.5)

[0x02][0x30] BR /{database_id_be:16}/META/head                    → DBHead (vbare-versioned; commit-owned)
[0x02][0x30] BR /{database_id_be:16}/META/head_at_fork            → DBHead (vbare-versioned; written by fork to snapshot source's head AS-OF fork versionstamp; read by fresh forks until first commit)
[0x02][0x30] BR /{database_id_be:16}/META/compact                 → CompactState (vbare-versioned; hot-compactor-owned)
[0x02][0x30] BR /{database_id_be:16}/META/cold_compact            → ColdState (vbare-versioned; cold-compactor-owned)
[0x02][0x30] BR /{database_id_be:16}/META/quota                   → i64 LE atomic
[0x02][0x30] BR /{database_id_be:16}/META/compactor_lease         → Lease (vbare-versioned)
[0x02][0x30] BR /{database_id_be:16}/META/cold_lease              → Lease (vbare-versioned)
[0x02][0x30] BR /{database_id_be:16}/META/manifest                → BranchManifest (vbare-versioned; cross-cutting metadata)
[0x02][0x30] BR /{database_id_be:16}/COMMITS/{txid_be:8}          → CommitRow (vbare-versioned; carries wall_clock_ms + versionstamp)
[0x02][0x30] BR /{database_id_be:16}/VTX/{versionstamp_be:16}     → u64 BE txid (raw; secondary index for resolution + GC)
[0x02][0x30] BR /{database_id_be:16}/PIDX/{pgno_be:4}             → u64 BE owner_txid (raw)
[0x02][0x30] BR /{database_id_be:16}/DELTA/{txid_be:8}/{chunk_be:4} → ltx_chunk_blob
[0x02][0x30] BR /{database_id_be:16}/SHARD/{shard_id_be:4}/{as_of_txid_be:8} → ltx_shard_blob

[0x02][0x40] CTR  /quota_global                             → i64 LE (sum across all branches; optional)
[0x02][0x40] CTR  /eviction_index/{last_access_bucket_be:8}/{database_id} → empty
                                                            (bucket = floor(last_access_ts_ms / ACCESS_TOUCH_THROTTLE_MS);
                                                             re-keyed only when the bucket moves forward — see §12.3, §17)

[0x02][0x50] BOOKMARK /{database_id}/{bookmark_str}            → BookmarkRecord (vbare-versioned)
[0x02][0x50] BOOKMARK /{database_id}/{bookmark_str}/pinned     → PinnedBookmarkRecord (vbare-versioned; carries pin status: Pending|Ready|Failed)

[0x02][0x60] CMPC /enqueue/{ts_ms_be:8}/{database_id}/{kind:1} → empty (cold/eviction compactor work queue)
[0x02][0x60] CMPC /lease_global/{kind:1}                    → Lease (sweep-coordinator lease)
```

NSCAT cap-by-versionstamp implementation: each entry stores a 16-byte FDB versionstamp captured at create time via `SetVersionstampedValue`. When a namespace fork at versionstamp V_fork walks its parent's NSCAT to inherit visibility, it accepts only entries whose stored versionstamp <= V_fork. Newer entries in the source namespace (databases created AFTER V_fork) are filtered out. The new namespace's NSCAT is empty until it has its own `fork_database` / `create_database` calls that write into `NSCAT/{new_namespace_id}/...`. This is what gives `fork_namespace` AS-OF correctness without O(N) eager catalog materialization.

`database_tombstones` semantics: writing `NSBRANCH/{namespace_id}/database_tombstones/{database_id}` records "the database was deleted in this namespace." During NSCAT walk for visibility (§10.5), a database is hidden if any namespace branch on the walk path from the reading namespace up through its parents (including the reader itself, but stopping at the namespace branch where the NSCAT entry is found) carries a tombstone for it. This means `delete_database` in a source namespace IS visible to namespaces forked AFTER the deletion (because the tombstone sits on the source path before the fork's `parent_versionstamp` — the standard parent-chain inheritance). It is NOT visible to namespaces forked BEFORE the deletion, since their parent walk stops at the fork point. Tombstones do not retroactively delete the database in already-forked namespaces; they apply to the namespace branch they were written in and its descendants.

`{kind:1}` byte: `0x00 = cold`, `0x01 = eviction`. The hot compactor uses the existing `META/compactor_lease` per database; cold and eviction use either the per-branch lease (cold) or a global sweep lease (eviction).

The `VTX` index translates a 16-byte versionstamp to the 8-byte txid recorded in the corresponding `COMMITS/{txid_be:8}` row. Written on every commit (one extra mutation, no extra RTT). Used by:
- bookmark resolution (§9) to map a bookmark's encoded-versionstamp to a `txid` for SHARD/PIDX read planning.
- GC (§13) to map a versionstamp pin to a txid floor for COMMITS/DELTA range deletes.

`COMMITS/{txid_be:8}` value extends from the stateless spec to include the FDB versionstamp:

```rust
#[derive(Encode, Decode)]
pub struct CommitRow {
    pub wall_clock_ms: i64,
    /// FDB versionstamp captured at commit time via SetVersionstampedValue
    /// in the same tx. Length-stable 16 bytes. Unique across all commits
    /// in the cluster.
    pub versionstamp: [u8; 16],
    /// db_size_pages AS-OF this commit. Required so that fresh-fork
    /// /META/head_at_fork can be reconstructed by reading exactly one
    /// COMMITS row instead of replaying from the root (fix #4).
    pub db_size_pages: u32,
    /// post_apply_checksum AS-OF this commit. Same rationale.
    pub post_apply_checksum: u64,
}
```

The versionstamp is what bookmarks resolve against on lex comparison and what cross-branch `parent_versionstamp` refers to.

`DBHead` extends with the rolling `post_apply_checksum` and a `branch_id` field:

```rust
#[derive(Encode, Decode)]
pub struct DBHead {
    pub head_txid: u64,
    pub db_size_pages: u32,
    pub post_apply_checksum: u64,
    /// The database id this head sits on. Equals the database branch
    /// record's `branch_id`. Carried for in-tx fence assertions in debug.
    pub branch_id: DatabaseId,
}
```

`BranchManifest` carries cross-cutting metadata that needs to be visible per-database but is not commit-owned. The fields below are written by disjoint owners (commit, hot compactor, cold compactor, Db access-touch path); to avoid RMW conflicts on a single blob, the manifest is logically split by owner and stored as four sibling sub-keys under `BR/{database_id}/META/manifest/`:

```text
BR/{database_id}/META/manifest/cold_drained_txid  → u64 BE (cold-compactor-owned)
BR/{database_id}/META/manifest/last_hot_pass_txid → u64 BE (hot-compactor-owned)
BR/{database_id}/META/manifest/last_access_ts_ms  → i64 LE (Db-touched, throttled)
BR/{database_id}/META/manifest/last_access_bucket → i64 LE (last bucket key written into [GLOBAL]/eviction_index)
```

In code these are surfaced as a single `BranchManifest` struct read by joining the four sub-keys. The eviction compactor's OCC fence on `last_hot_pass_txid` (fix #6, §12.1, §15) uses this dedicated sub-key and is unaffected by writes to `cold_drained_txid` or access-touch fields.

```rust
#[derive(Encode, Decode)]
pub struct BranchManifest {
    /// Last txid the cold compactor uploaded an image for. Eviction
    /// compactor reads this to decide what's safe to drop from FDB.
    pub cold_drained_txid: u64,
    /// Last hot-compactor-pass txid. Used for cold-compactor lag
    /// derivation AND as the eviction-vs-hot OCC fence (fix #6).
    pub last_hot_pass_txid: u64,
    /// Last access timestamp (read or write) on this database. Used by
    /// eviction compactor to decide hot-cache eligibility. Updated lazily
    /// by Db on access; not load-bearing for correctness. Throttled
    /// by ACCESS_TOUCH_THROTTLE_MS (default 60_000) — per-conn cache
    /// suppresses writes when the bucket has not advanced.
    pub last_access_ts_ms: i64,
    pub last_access_bucket: i64,
}
```

## 7. S3 schema

Bucket layout, all paths under a fixed root configurable at deployment. Every namespace's data flows through the cold tier; small or idle namespaces simply produce small or infrequent layer files:

```
{root}/
  ns/{namespace_id_uuid_hex:32}/
    branch_record.bare                    ← snapshot of NamespaceBranchRecord
    catalog/
      {ns_versionstamp_hex:32}.bare       ← database catalog (NSCAT range) at this versionstamp (lazy)
  db/{database_id_uuid_hex:32}/
    branch_record.bare                    ← snapshot of DatabaseBranchRecord
    image/{as_of_txid_high_bytes_hex:8}/
      {shard_id_be_hex:8}-{as_of_txid_be_hex:16}.ltx      ← image layer (exactly one shard per file)
    pin/
      {versionstamp_hex:32}.ltx           ← pinned bookmark image layer (full DB at versionstamp)
    delta/
      {min_txid_be_hex:16}-{max_txid_be_hex:16}.ltx       ← cold-tier delta layer (consolidated DELTA range)
    cold_manifest/
      index.bare                           ← small mutable index: ordered list of chunk file names + last_pass metadata
      chunks/
        {pass_versionstamp_hex:32}.bare    ← immutable chunk: layer entries appended in this cold pass
  catalog_snapshot/
    {pass_versionstamp_hex:32}.bare       ← per-pass DR snapshot: enumerates known
                                              namespace branch records and database
                                              branch records observed at pass time, plus
                                              their NSCAT membership entries
  pending/
    {uuid}.marker                         ← HWM marker for in-progress cold pass; cleaned on next pass
```

Path conventions:

- `{...}_hex:N` is N hex chars (i.e. N/2 bytes).
- `image/{as_of_txid_high_bytes_hex:8}/...` adds a 4-byte high-order prefix bucket so `LIST` operations on a heavily-versioned database remain tractable. The bucket key is the high 4 bytes of `as_of_txid_be:8`.
- Filenames omit content checksums. Re-uploads are idempotent overwrites. Per-layer integrity is in the LTX V3 trailer + the `cold_manifest` chunk's `LayerEntry.checksum`.
- `pin/{versionstamp_hex:32}.ltx` is the only S3 object kind that holds a full-DB image at a specific versionstamp. Image layers under `image/` are per-shard.
- **Constraint (fix #15):** exactly one shard per image file. Multi-shard packing is forbidden in v1. Filename determinism is required for idempotent re-upload after lease loss; packing would let two passes with different shard-set choices produce different bytes for the same filename.
- **Cold manifest is chunked (fix #18).** Each cold pass appends a new immutable chunk under `cold_manifest/chunks/{pass_versionstamp_hex:32}.bare` containing only that pass's `LayerEntry`s and `BookmarkIndexEntry`s. The small mutable `cold_manifest/index.bare` carries an ordered list of chunk filenames plus `last_pass_at_ms` / `last_pass_versionstamp`. Readers fetch the index, then GET chunks lazily as needed (most reads only need the most recent chunks). Avoids the O(layers) full-manifest rewrite per pass.
- **Catalog snapshot (DR; replaces v3's pointer_snapshot).** Each cold pass writes a `catalog_snapshot/{pass_versionstamp_hex:32}.bare` capturing the set of `NamespaceBranchRecord`s and `DatabaseBranchRecord`s observed at pass time plus their NSCAT membership entries. Without v3's DBPTR/NSPTR pointer indirection there is no `(database_id → DatabaseBranchId)` mapping to record; the snapshot is just the immutable branch graph plus catalog membership. This is the DR catalog: FDB-disaster + S3-survival recovery walks the most recent `catalog_snapshot` to rebuild NSBRANCH + BRANCHES + NSCAT before replaying layers. DR posture is uniform across all namespaces; see §14.

```rust
/// Small mutable index file at `cold_manifest/index.bare`. Rewritten on
/// every cold pass; size is O(passes), not O(layers).
#[derive(Encode, Decode)]
pub struct ColdManifestIndex {
    pub schema_version: u32,
    pub branch_id: DatabaseId,
    /// Ordered list of chunk filenames (oldest first). Each entry refers
    /// to `cold_manifest/chunks/{name}.bare`.
    pub chunks: Vec<ColdManifestChunkRef>,
    pub last_pass_at_ms: i64,
    pub last_pass_versionstamp: [u8; 16],
}

#[derive(Encode, Decode)]
pub struct ColdManifestChunkRef {
    pub object_key: String,
    pub pass_versionstamp: [u8; 16],
    /// Coverage hint: lex range of versionstamps in this chunk's entries.
    /// Lets readers prune to "chunks that could cover bookmark V".
    pub min_versionstamp: [u8; 16],
    pub max_versionstamp: [u8; 16],
    pub byte_size: u64,
}

/// Immutable chunk file. One per cold pass.
#[derive(Encode, Decode)]
pub struct ColdManifestChunk {
    pub schema_version: u32,
    pub branch_id: DatabaseId,
    pub pass_versionstamp: [u8; 16],
    pub layers: Vec<LayerEntry>,
    pub bookmarks: Vec<BookmarkIndexEntry>,
}

#[derive(Encode, Decode)]
pub struct LayerEntry {
    pub kind: LayerKind,                  // Image | Delta | Pin
    pub shard_id: Option<u32>,            // present for Image; None for Delta and Pin
    pub min_txid: u64,
    pub max_txid: u64,                    // == min_txid for Image and Pin
    pub min_versionstamp: [u8; 16],
    pub max_versionstamp: [u8; 16],
    pub byte_size: u64,
    pub checksum: u64,                    // matches LTX V3 trailer
    pub object_key: String,               // full S3 key
}

#[derive(Encode, Decode)]
pub struct BookmarkIndexEntry {
    pub schema_version: u32,
    pub bookmark_str: String,             // 33-char wire format
    pub pinned: bool,
    pub pin_object_key: Option<String>,   // populated only if pinned
    pub pin_status: PinStatus,            // Pending | Ready | Failed (fix #13)
    pub created_at_ms: i64,
}

#[derive(Encode, Decode, Copy, Clone)]
pub enum PinStatus { Pending, Ready, Failed }

/// DR snapshot enumerating known immutable branches and namespace
/// catalog membership at pass time. Replaces v3's PointerSnapshot;
/// without DBPTR/NSPTR pointer indirection there is no `(database_id ->
/// branch_id)` mapping to record.
#[derive(Encode, Decode)]
pub struct CatalogSnapshot {
    pub schema_version: u32,
    pub pass_versionstamp: [u8; 16],
    pub namespaces: Vec<NamespaceBranchRecord>,
    pub databases: Vec<DatabaseBranchRecord>,
    /// Each entry: `(namespace_id, database_id, nscat_versionstamp)`.
    pub nscat: Vec<(NamespaceId, DatabaseId, [u8; 16])>,
}
```

`ns/{namespace_id}/branch_record.bare` and `db/{database_id}/branch_record.bare` are written exactly once per branch, at the moment the cold compactor first observes the branch. They are read for retention and cross-region replay. They are NOT the source of truth — FDB is. They exist so that an FDB disaster + S3 survival can reconstruct branch metadata far enough to enumerate which images exist; full recovery requires FDB.

## 8. The two operations

Both are O(1) metadata operations. Neither blocks on data movement. Neither holds a lease longer than a single FDB tx (or a single FDB tx + a synchronous S3 PUT in the case of pinned bookmarks). Both share a derive-new-branch-from-source-at-versionstamp primitive.

There is no `rollback_database` / `rollback_namespace` / `restore_to_bookmark` API. Engine-layer rollback semantics call `fork_database` and the engine flips its own database→database mapping.

The shared primitive, in pseudocode:

```rust
async fn derive_branch_at(
    udb: &Database,
    source_branch_id: DatabaseId,
    at_versionstamp: [u8; 16],
    new_branch_id: DatabaseId,
    target_namespace: NamespaceId,
    bookmark_ref: Option<BookmarkRef>,
) -> Result<()> {
    udb.run(|tx| async move {
        // Read source branch record. Used to assert fork depth budget and
        // to copy state forward.
        let source: DatabaseBranchRecord = vbare_get(tx, BRANCHES_list_key(source_branch_id)).await?;

        if source.fork_depth + 1 > MAX_FORK_DEPTH {
            return Err(SqliteStorageError::ForkChainTooDeep.into());
        }

        // OCC fence: regular-read source's bk_pin; if a concurrent GC
        // pass advances the pin past `at_versionstamp` we abort.
        let pin: [u8; 16] = atomic_min_get(tx, BRANCHES_list_bk_pin_key(source_branch_id)).await?;
        if pin > at_versionstamp {
            return Err(SqliteStorageError::ForkOutOfRetention.into());
        }

        // Fix #4: snapshot source's /META/head AS-OF at_versionstamp into
        // the new branch's /META/head_at_fork. This is the synthetic head
        // a fresh fork uses for reads until its first commit writes its
        // own /META/head.
        //
        // Resolving "head AS-OF at_versionstamp" needs head_txid for
        // at_versionstamp. We use the VTX index (fix #5):
        //   txid_at_v = VTX[at_versionstamp] → u64 BE
        // and read source's /META/head only for db_size_pages and
        // post_apply_checksum at that specific txid. The on-disk
        // /META/head reflects post-fork commits, so we must NOT just
        // copy it; instead we read COMMITS[txid_at_v] to get the historic
        // values via the embedded post_apply_checksum chain. (Concretely:
        // for each commit row we record post_apply_checksum and
        // db_size_pages on /META/head as it advances; recovering historic
        // values requires either storing them in CommitRow OR computing
        // via replay. v2 takes the simpler path: extend CommitRow to
        // carry db_size_pages and post_apply_checksum. See §6.)
        let txid_at_v: u64 = vtx_lookup(tx, source_branch_id, at_versionstamp).await?;
        let commit_at_v: CommitRow = vbare_get(
            tx, COMMITS_key(source_branch_id, txid_at_v)
        ).await?;
        let head_at_fork = DBHead {
            head_txid: txid_at_v,
            db_size_pages: commit_at_v.db_size_pages,
            post_apply_checksum: commit_at_v.post_apply_checksum,
            branch_id: new_branch_id,
        };
        vbare_set(tx, META_head_at_fork_key(new_branch_id), &head_at_fork).await?;

        // Allocate new branch. The external `new_branch_id` (a DatabaseId)
        // IS the branch id; there is no separate pointer record.
        let new_record = DatabaseBranchRecord {
            branch_id: new_branch_id,
            namespace: target_namespace,
            parent: Some(source_branch_id),
            parent_versionstamp: Some(at_versionstamp),
            // Fix #3: record this branch's own root versionstamp for the
            // GC pin formula.
            root_versionstamp: at_versionstamp,
            fork_depth: source.fork_depth + 1,
            created_at_ms: now_ms(),
            created_from_bookmark: bookmark_ref,
        };
        vbare_set(tx, BRANCHES_list_key(new_branch_id), &new_record).await?;

        // Refcount on source branch increments; child references parent.
        atomic_add(tx, BRANCHES_list_refcount_key(source_branch_id), 1);
        // Refcount on new branch starts at 1 (the namespace catalog entry
        // we are about to write references it; engine-layer mappings or
        // pinned bookmarks may add further references later).
        atomic_add(tx, BRANCHES_list_refcount_key(new_branch_id), 1);

        // Descendant pin on source: atomic-min the new branch's parent_versionstamp.
        atomic_min(tx, BRANCHES_list_desc_pin_key(source_branch_id), at_versionstamp);

        // Reads on the new branch resolve from /META/head_at_fork
        // (synthetic head) until first commit writes /META/head; PIDX/SHARD
        // miss falls through to source via parent_versionstamp cap (§10).

        // Note: `derive_branch_at` does NOT write the NSCAT entry. The
        // caller (`fork_database`) writes it after this returns so the
        // versionstamp suffix on NSCAT can be set via SetVersionstampedValue
        // in the same tx.

        Ok(())
    }).await
}
```

Note: `CommitRow` (§6) is extended to carry `db_size_pages: u32` and `post_apply_checksum: u64` so that the synthetic-head snapshot above can be reconstructed in one extra read. Bytes-on-the-wire grow by ~12 bytes/commit.

The two operations call this primitive (or its namespace twin) and write the appropriate catalog entry. Neither writes any pointer record — the immutable branch record IS the addressable entity.

### 8.1 fork_namespace

```rust
pub async fn fork_namespace(
    &self,
    source_namespace_id: NamespaceId,
    at: ResolvedVersionstamp,
) -> Result<NamespaceId> {
    // The new external NamespaceId IS the new namespace branch id.
    let new_namespace_id = NamespaceId(Uuid::new_v4());

    self.udb.run(|tx| async move {
        let source_branch: NamespaceBranchRecord =
            vbare_get(tx, NSBRANCH_list_key(source_namespace_id)).await?;

        // Enforce MAX_NAMESPACE_DEPTH (fix #22).
        let parent_depth = ns_branch_depth(tx, source_namespace_id).await?;
        if parent_depth + 1 > MAX_NAMESPACE_DEPTH {
            return Err(SqliteStorageError::NamespaceForkChainTooDeep.into());
        }

        // OCC fence on source's bk_pin.
        let pin = atomic_min_get(tx, NSBRANCH_list_bk_pin_key(source_namespace_id)).await?;
        if pin > at.versionstamp {
            return Err(SqliteStorageError::ForkOutOfRetention.into());
        }

        let new_branch = NamespaceBranchRecord {
            branch_id: new_namespace_id,
            parent: Some(source_namespace_id),
            parent_versionstamp: Some(at.versionstamp),
            // Fix #3: own root versionstamp. Also serves as the NSCAT
            // cap-by-versionstamp floor for inheritance.
            root_versionstamp: at.versionstamp,
            created_at_ms: now_ms(),
            created_from_bookmark: at.bookmark.clone(),
        };
        vbare_set(tx, NSBRANCH_list_key(new_namespace_id), &new_branch).await?;
        atomic_add(tx, NSBRANCH_list_refcount_key(source_namespace_id), 1);
        atomic_add(tx, NSBRANCH_list_refcount_key(new_namespace_id), 1);
        atomic_min(tx, NSBRANCH_list_desc_pin_key(source_namespace_id), at.versionstamp);

        // No NSCAT writes here. The new namespace starts with empty NSCAT
        // and lazy-resolves to the parent namespace's NSCAT via parent walk
        // (§10) capped by `parent_versionstamp = at.versionstamp`.
        // Databases created in the source AFTER `at.versionstamp` have a
        // versionstamp suffix > at.versionstamp on their NSCAT entries and
        // are filtered out of the fork's view.

        Ok(())
    }).await?;

    Ok(new_namespace_id)
}
```

WHY no eager NSCAT materialization: a namespace can hold thousands of databases. Eagerly copying every NSCAT entry at fork time is O(N) work synchronously on the fork tx, blowing the FDB tx budget for a "metadata-only" operation. The new namespace's empty NSCAT plus lazy parent-chain walk capped by `parent_versionstamp` gives an AS-OF snapshot of the database catalog without O(N) work. Databases subsequently created in the source namespace land in the source's NSCAT only, so they cannot bleed into the forked namespace. Cold compactor uploads the catalog snapshot asynchronously for retention.

### 8.2 fork_database

```rust
pub async fn fork_database(
    &self,
    source_namespace_id: NamespaceId,
    source_database_id: DatabaseId,
    at: ResolvedVersionstamp,
    target_namespace_id: NamespaceId,
) -> Result<DatabaseId> {
    // The new external DatabaseId IS the new database branch id.
    let new_database_id = DatabaseId(Uuid::new_v4());

    self.udb.run(|tx| async move {
        // Source database must be visible in source_namespace_id
        // (NSCAT presence + parent walk per §10.5).
        ensure_database_visible_in_namespace(
            tx, source_namespace_id, source_database_id
        ).await?;

        derive_branch_at(
            tx,
            source_database_id,
            at.versionstamp,
            new_database_id,
            target_namespace_id,
            at.bookmark.clone(),
        ).await?;

        // Write NSCAT presence marker for the new database in the target
        // namespace. The 16-byte versionstamp suffix is captured at commit
        // time via SetVersionstampedValue, so the entry is later filterable
        // by namespace forks taken at versionstamps before this commit.
        set_versionstamped_value(
            tx,
            NSCAT_key(target_namespace_id, new_database_id),
            // Placeholder — FDB substitutes the commit's versionstamp.
            VERSIONSTAMP_PLACEHOLDER,
        ).await?;

        Ok(())
    }).await?;

    Ok(new_database_id)
}
```

`ensure_database_visible_in_namespace(tx, ns, db)` reads `NSCAT/{ns}/{db}` and on absence walks the namespace parent chain (§10.5), honoring any `database_tombstones/{db}` along the chain. Returns `DatabaseNotFound` if the database is not visible.

WHY allow `target_namespace_id` parameter: forking a database cross-namespace is supported by the data model (the new branch's `namespace` is independent of the source's). The default in higher-level APIs may be "fork into source namespace" but the storage primitive accepts both. Cross-namespace `fork_database` writes the NSCAT entry only into the target namespace; the source namespace continues to see the source database, the target namespace sees the new forked database, and the source database remains undisturbed.

### 8.3 delete_database / delete_namespace

```rust
/// Mark a database as deleted in `namespace`. Refcount-decrement only.
/// The actual GC + S3 cleanup is engine-owned: the engine watches for
/// orphaned databases (refcount = 0 with no descendant or bookmark pin)
/// and calls this primitive when its application-level lifecycle dictates.
/// Storage does not enumerate orphans.
pub async fn delete_database(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
) -> Result<()>;

/// Same shape for namespaces.
pub async fn delete_namespace(
    &self,
    namespace: NamespaceId,
) -> Result<()>;
```

`delete_database` writes a `database_tombstones/{database_id}` entry under the namespace's NSBRANCH and atomic-add(-1) on the database's `BRANCHES/list/{database_id}/refcount`. The tombstone makes the database invisible to the namespace and to namespaces forked from it after the deletion (per the `database_tombstones` semantics in §6). Concrete cleanup of the database's keys + S3 layers happens via the standard GC pass once `refcount == 0` AND no descendant/bookmark pin holds retention; storage does not synchronously delete data.

GC of orphaned databases is engine-owned: the engine knows when a database id is no longer referenced by any database mapping or pinned bookmark and calls `delete_database` accordingly. Storage does not maintain an "is this database still in use by an database" view — that is engine-layer state.

## 9. Bookmarks

### Wire format

33 ASCII chars: `{timestamp_ms_hex_be:16}-{txid_hex_be:16}`. Lex order = chronological order within a single branch's parent chain. No branch identity in the wire format; bookmarks resolve in the context of a database (or its namespace).

```rust
pub struct BookmarkStr(String); // length-checked at construction

#[derive(Encode, Decode, Clone)]
pub struct BookmarkRef {
    pub bookmark: BookmarkStr,
    /// Resolved at create time. Pinned bookmarks store this; ephemeral
    /// bookmarks resolve at use time.
    pub resolved_versionstamp: Option<[u8; 16]>,
}
```

### Two classes

- **Ephemeral.** Created by `create_bookmark(namespace, database_id, t_ms)`. Resolution at use time finds the nearest preserved checkpoint (largest versionstamp `<=` the bookmark's encoded txid that survives in either FDB COMMITS or S3 image layers). If the nearest checkpoint is older than the GC pin, returns `BookmarkExpired`.
- **Pinned.** Created by `create_pinned_bookmark(namespace, database_id, t_ms)`. **Async via cold compactor (fix #13).** Synchronously: writes a `BOOKMARK/{database_id}/{bookmark_str}/pinned` record with `PinStatus::Pending`, atomic-min the database's `bk_pin` to that versionstamp, and enqueues a UPS message to the cold compactor. The cold compactor performs LTX encoding + S3 PUT of `pin/{versionstamp_hex:32}.ltx`, then transitions `PinStatus::Pending → Ready` (or `Failed`) in a follow-up FDB tx. Returns the bookmark string immediately. Caller may poll status via `bookmark_status`. GC cannot delete history below a pinned bookmark's versionstamp until the bookmark is deleted.
- **MAX_PINS_PER_NAMESPACE = 1024 (fix #19).** Enforced in-tx at `create_pinned_bookmark` time by reading the namespace's `pin_count` counter; exceeding the cap returns `SqliteStorageTooManyPins`. Pin lifecycle is explicit: `create_pinned_bookmark` to add, `delete_pinned_bookmark` to remove. The cold compactor's follow-up GC sweep deletes the `pin/{versionstamp_hex:32}.ltx` S3 object once `bk_pin` recompute confirms no remaining reference.

```rust
pub async fn create_bookmark(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
    at_ms: i64,
) -> Result<BookmarkStr>;

/// Async pin via cold compactor. Returns immediately; PinStatus is
/// queryable via `bookmark_status`. Pin contributes to bk_pin
/// atomic-min synchronously inside this fn's tx so retention is held
/// before the S3 PUT lands.
pub async fn create_pinned_bookmark(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
    at_ms: i64,
) -> Result<BookmarkStr>;

pub async fn delete_pinned_bookmark(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
    bookmark: BookmarkStr,
) -> Result<()>;

pub async fn bookmark_status(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
    bookmark: BookmarkStr,
) -> Result<PinStatus>;

pub async fn resolve_bookmark(
    &self,
    namespace: NamespaceId,
    database_id: DatabaseId,
    bookmark: BookmarkStr,
) -> Result<ResolvedVersionstamp>;
```

There is no `restore_to_bookmark` API. Engine-layer "restore" calls `resolve_bookmark` to get the AS-OF versionstamp, then `fork_database` at that versionstamp, then flips its own database→database mapping. The undo bookmark a v3 caller would have received from `restore_to_bookmark` is replaced by the source database id at the moment of the engine's flip — the source database is still live (refcount held by any descendant or pin), and the engine can flip its mapping back to it to "undo."

### Resolution algorithm

Resolution bridges both the database parent chain AND the namespace parent chain (fix #14). A bookmark created in namespace N1 against database D must continue to resolve correctly after `fork_namespace(N1) → N2` (the bookmark is still resolvable in N2 against D, since D is visible there via NSCAT inheritance).

```
resolve_bookmark(namespace_id, database_id, bookmark) -> ResolvedVersionstamp:
  1. parse bookmark = (ts_ms, txid_hex).
  2. ensure database is visible in namespace_id (NSCAT presence + parent
     walk; if not visible, return BranchNotReachable).
  3. namespace cap: walk namespace parents starting at namespace_id. For
     each NS_i with parent_versionstamp_i, accept i as the resolution-
     target namespace iff bookmark.versionstamp >= parent_versionstamp_i.
     Yields a ns_versionstamp_cap.
  4. walk database parent chain from database_id, capped by
     min(ns_versionstamp_cap, current database's parent_versionstamp_cap):
     for each branch in current..root:
       if a pinned bookmark exists at this exact (ts_ms, txid):
         return its stored versionstamp + bookmark ref
       look up VTX[bookmark_versionstamp] -> txid' (fix #5; O(1) instead
         of full COMMITS scan)
       if VTX present in this branch and txid' <= cap-derived txid:
         if COMMITS[txid'] present in FDB OR cold manifest covers txid':
           return that versionstamp + ephemeral bookmark
       fall through to parent (use parent_versionstamp as new cap)
  5. if exhausted without finding a preserved checkpoint:
       BookmarkExpired (when below GC pin) or BranchNotReachable (when
       no chain reaches the bookmark's lineage)
```

Resolution may touch S3 manifests if the FDB COMMITS row has been evicted; the cold manifest's `bookmarks_index` (chunked, fix #18) is the authoritative bookmark list.

### Complexity bound

Worst case is `MAX_FORK_DEPTH × MAX_NAMESPACE_DEPTH = 16 × 16 = 256` parent-chain hops. This is an open issue: in pathological deep-nested cases bookmark resolution latency grows multiplicatively. For typical workloads (small fork depths) the walk terminates at depth 1-2.

### `BranchNotReachable`

Resolving a bookmark whose stored ancestry does not include the caller's database branch returns `BranchNotReachable`. This enforces sender-scoping; a database cannot use a bookmark created on a different database's branch chain.

## 10. Read path

A `get_pages(pgnos)` call against `database_id` in `namespace_id`:

```
1. database_id IS the database branch id. Read BRANCHES/list/{database_id}
   directly to load the immutable branch record.
2. (Optional, depending on caller) verify the database is visible in
   namespace_id via NSCAT presence + parent walk (§10.5). Cached on Db
   for the lifetime of the connection.
3. Build (or cache-hit) a flattened ancestry view:
     ancestors = [database_id -> parent -> ... -> root]
     each entry carries (branch_id, parent_versionstamp_cap_for_reads).
4. Read /META/head on database_id (or /META/head_at_fork if the branch is
   fresh-fork — fix #4) -> head_txid, db_size_pages.
5. For each pgno in pgnos:
     a. Walk ancestry: for ancestor in [database_id, ...]:
          - PIDX read: PIDX[branch_id][pgno] -> owner_txid
            (cap reads on ancestor by `parent_versionstamp_cap_for_reads`)
          - if PIDX has an owner: load DELTA[branch_id][owner_txid]
          - else: SHARD read on (branch_id, shard_id_for(pgno)) with
            largest as_of_txid <= head_txid_at_ancestor_cap
          - if either hits, return the page
        if exhausted: page is unallocated -> zero page
6. On any hot-tier MISS (PIDX entry points at a DELTA blob that's been
   cold-evicted), fall through to S3:
     a. Load cold_manifest index + relevant chunk(s) (fix #18).
     b. Find layer whose (min_txid, max_txid) covers the owner_txid.
     c. GET the layer; decode; return the page.
   Cold reads do NOT happen on the steady-state read path; they happen
   only after eviction has cleared the FDB resident set for this database
   and the database wakes for read.
```

Per-conn perf cache: `Db` carries a flattened ancestry struct (the chain `[branch_id_i, parent_versionstamp_cap_i]`) computed once per `Db` lifetime and reused across `get_pages` calls. This is the only branching-related addition to `Db`'s in-memory state. With v3's DBPTR/NSPTR pointer-flip semantics gone, the ancestry cache is stable for the entire `Db` lifetime — branches are immutable, so the chain a `Db` resolves on first read is the chain it sees forever. There is no cache-invalidation tx contract (no `last_swapped_at_ms` parallel reads on every commit / get_pages); pegboard exclusivity guarantees no other writer can hijack the database's branch underneath the conn.

WHY no cache-invalidation contract: in v3 the contract existed because rollback flipped DBPTR/NSPTR pointers underneath a live conn, so the conn had to detect the swap and rebuild. With rollback removed at storage and pointer indirection collapsed, the database id IS the branch id; nothing the storage layer does will move the conn's database to a different branch record. The engine layer's database→database mapping flip is the engine's own concern; it tears down the old conn and opens a new one against the new database id. Storage need not coordinate.

WHY cache the ancestry: walking N parents per `get_pages` would do N FDB reads of `BRANCHES/list/{...}` on every read. The cache makes hot reads do at most 1 read per `Db` lifetime to materialize the chain.

### 10.5 Database enumeration (`list_databases`)

`list_databases(namespace_id) -> Vec<DatabaseId>` walks the namespace parent chain via NSCAT range scans:

```
list_databases(namespace_id):
  1. result_set = HashSet<DatabaseId>::new()
  2. tombstone_set = HashSet<DatabaseId>::new() (databases deleted in any
     namespace on the chain from the reader up to root)
  3. cap = read NSBRANCH(namespace_id).root_versionstamp (or +infinity if
     this is the read namespace itself; the cap floor advances as we walk
     up via each ancestor's parent_versionstamp).
  4. for ns in walk_parents(namespace_id) (including namespace_id itself):
       parent_vs_cap = ns.parent_versionstamp (or +infinity for the reader)
       for (database_id, nscat_versionstamp) in range_scan(NSCAT/{ns}/{*}):
         // Cap-by-versionstamp: the NSCAT entry is invisible to this
         // forked namespace if it was created AFTER the fork point.
         if nscat_versionstamp > parent_vs_cap: skip
         if database_id in tombstone_set or database_id in result_set: skip
         result_set.insert(database_id)
       for database_id in range_scan(NSBRANCH/{ns}/database_tombstones/{*}):
         tombstone_set.insert(database_id)
  5. return Vec::from(result_set)
```

Complexity: `O(databases-divergent-per-namespace × MAX_NAMESPACE_DEPTH)`. The cap-by-versionstamp filter on NSCAT entries (using the 16-byte versionstamp suffix written at create time via `SetVersionstampedValue`) is what gives `fork_namespace` AS-OF correctness: a database created in namespace N1 AFTER `fork_namespace(N1) → N2` has a NSCAT versionstamp greater than N2's `root_versionstamp`, so it is filtered out of N2's `list_databases`. Tombstones honor "database deleted in fork": writing `NSBRANCH/{ns}/database_tombstones/{database_id}` in a child namespace removes the database from `list_databases` results in that child even if NSCAT membership is inherited from a parent. Tombstones are namespace-scoped; they do not retroactively delete the database in the parent namespace.

`delete_database(namespace, database_id)` writes the tombstone in the namespace and decrements refcount on the database. If the database has been forked (descendants exist), the descendant pin keeps it alive at storage even though it is hidden from `list_databases` in the deleting namespace.

## 11. Write path

A `commit(dirty_pages, db_size_pages, now_ms)` call against `database_id`:

```
1. database_id IS the branch id. Direct lookup; no DBPTR/NSPTR
   indirection and no cache-invalidation contract.
2. /META/head read on database_id (or /META/head_at_fork for fresh fork).
3. T = head_txid + 1.
4. For each dirty page: write DELTA[database_id][T][chunk] blob.
5. PIDX upsert: PIDX[database_id][pgno] = T (raw u64 BE).
6. /META/head write with new head_txid = T, db_size_pages, post_apply_checksum.
7. COMMITS[database_id][T] write with wall_clock_ms = now_ms,
   versionstamp = SetVersionstampedValue(_), db_size_pages,
   post_apply_checksum.
8. VTX[database_id][versionstamp_be:16] = T (raw u64 BE), via
   SetVersionstampedKey (fix #5). One extra mutation, no extra RTT.
9. /META/quota atomic-add the byte delta.
10. /META/manifest/last_hot_pass_txid is hot-compactor-owned; commit does
    NOT touch it. Commit may write last_access_ts_ms via the throttled
    path (§12.3, fix #11): only when the access bucket has advanced.
11. If first commit on a fresh fork: clear /META/head_at_fork (it has
    been superseded).
12. If head_txid - materialized_txid >= compaction_delta_threshold:
      tokio::spawn(ups.publish(SqliteCompactSubject)).
```

Per-commit RTT savings vs v3: removing the cache-invalidation contract drops the parallel reads of `DBPTR.last_swapped_at_ms` and `NSPTR.last_swapped_at_ms` (-2 reads on every commit). On RocksDB / sequential-read backends this is two fewer wall-clock ms per commit; on FDB native the parallel-read fold-in already hid most of the cost, but the write-tx surface area shrinks.

WHY versionstamp on commit: bookmark resolution and fork's "AS-OF" semantics need a totally-ordered cluster-wide moment. FDB's versionstamp is the only thing that gives this ordering across databases, namespaces, and branches without a centralized counter. Each commit pays one extra `SetVersionstampedValue` mutation for COMMITS plus one `SetVersionstampedKey` mutation for VTX.

WHY VTX on commit: bookmark resolution (§9) and GC (§13) need to map versionstamp → txid. Without VTX, both paths require a full COMMITS scan. VTX is keyed by versionstamp BE so a single get/range-scan is `O(log N)`.

## 12. The three compactors

### 12.1 Hot compactor

Inherits the stateless spec's hot compactor with these changes:

1. **Produces versioned SHARDs.** A pass folds DELTA blobs in `[materialized_txid+1, head_txid]` into a new SHARD blob at `SHARD/{shard_id}/{max_folded_txid}`. The previous SHARD version is NOT deleted by the hot compactor; eviction or cold owns deletion.
2. **Uniform DELTA deletion.** Folded DELTAs are deleted only when `max_folded_txid <= cold_drained_txid`. Otherwise they stay in FDB until cold has uploaded them. The cold compactor's L1→L2 cursor (`cold_drained_txid`) is the single LSM gate for hot-tier DELTA reclamation.
3. **Universal hot-tier retention floor.** Hot pass GCs COMMITS + VTX rows older than `HOT_RETENTION_FLOOR_MS` (default 7 days, applied to every namespace). Bookmark resolution past the floor returns `BookmarkExpired` if no cold layer covers the target versionstamp. This is the LSM hot-tier write-buffer floor; it has no per-namespace knob.
4. **Updates `BranchManifest.last_hot_pass_txid`** (regular write) in its commit. The eviction compactor reads this regular-read inside its OCC fence (fix #6); a hot pass between eviction's plan and clear forces eviction abort, preventing torn read of a SHARD whose newer version is in flight.
5. **MAX_SHARD_VERSIONS_PER_SHARD = 32 (fix #17).** When a hot pass would produce a 33rd version of a shard (because eviction has not yet caught up), the hot pass force-evicts the oldest unpinned shard version inline before writing the new one. Pinned versions (covered by `desc_pin` or `bk_pin`) are never force-evicted; if all 32 slots are pinned, the hot pass aborts with `SqliteStorageShardVersionCapExhausted` and the operator is notified via metric. This bounds FDB byte amplification independently of cold compactor / eviction lag.
6. **Eviction OCC fence write surface (fix #6).** Adds `SHARD_RETENTION_MARGIN` constant (default 64 txids): eviction's safety predicate requires `last_hot_pass_txid - SHARD_RETENTION_MARGIN >= as_of_txid_being_evicted`. Margin protects against eviction clearing a shard version that a just-committed hot pass might still reference for fold continuity.

The hot compactor's lease key is `BR/{database_id}/META/compactor_lease`. Per-database (which IS per-database-branch in v4); a tenant with N forked databases has N independent hot-compactor leases.

Constants summary:

```
HOT_RETENTION_FLOOR_MS       = 7 * 24 * 60 * 60 * 1000  // 7 days
MAX_SHARD_VERSIONS_PER_SHARD = 32
SHARD_RETENTION_MARGIN       = 64                      // txids
ACCESS_TOUCH_THROTTLE_MS     = 60_000                  // 1 minute
MAX_NAMESPACE_DEPTH          = 16
MAX_PINS_PER_NAMESPACE       = 1024
```

### 12.2 Cold compactor

Always runs when triggered. Every namespace's data flows through the cold tier; small or idle namespaces simply produce small or infrequent passes.

Three-phase pass per branch (fix #7 restructures Phase A to keep S3 PUTs out of FDB tx):

```
Phase A — durability handoff for the pending marker:
  A.1. Take cold_lease (via local-timer + cancel-token; renewal is OUT
       of any FDB tx — fix carryover from §12.2 review).
  A.2. Generate uuid for this pass.
  A.3. Brief FDB tx (read+write):
         - Read /META/cold_compact, /META/compact, /META/manifest sub-keys.
         - Write /META/cold_compact.in_flight_uuid = uuid.
         Commit.
  A.4. PUT pending/{uuid}.marker to S3, OUTSIDE any FDB tx. The marker
       is self-describing: includes the planned set of object keys this
       pass will write, so stale-marker cleanup can delete leaked layers
       deterministically.
  A.5. Resume Phase A read tx (snapshot reads, bounded by 5s tx age):
         - Snapshot-read SHARD versions and DELTA chunks in drainable range.
         - Snapshot-read COMMITS + VTX rows in drainable range.
         - Snapshot-read /BRANCHES/list/{...} record.
         - Compute plan: which SHARD versions to upload as image layers,
           which DELTA ranges to consolidate into delta layers, which
           bookmarks need to be indexed, and which pending pinned bookmarks
           need their full-DB image written (fix #13).

Phase B — S3 only, no FDB tx:
  B.1. Encode + PUT image/{...}/{...}.ltx for each planned shard version
       (one shard per file — fix #15).
  B.2. Encode + PUT delta/{...}-{...}.ltx for each planned delta range.
  B.3. Encode + PUT pin/{versionstamp_hex:32}.ltx for each Pending pin.
  B.4. PUT branch_record.bare if not already present.
  B.5. Append new immutable chunk to cold_manifest/chunks/ (fix #18) and
       rewrite small cold_manifest/index.bare.
  B.6. PUT pointer_snapshot/{pass_versionstamp_hex:32}.bare (fix #9).
  B.7. List pending/, identify markers older than STALE_MARKER_AGE_MS,
       delete the listed object keys from each stale marker, then delete
       the marker itself.

Phase C — FDB write (regular tx, OCC fence):
  C.1. Cold lease is checked (renewal handled by background task; not in
       this tx).
  C.2. Regular-read /META/cold_compact.cold_drained_txid; assert == the
       value read in Phase A. If a concurrent hot pass advanced it,
       abort + restart.
  C.3. Update /META/cold_compact with new cold_drained_txid.
  C.4. Update /META/manifest/cold_drained_txid sub-key (no RMW with
       last_hot_pass_txid; fields are split into sub-keys per §6).
  C.5. For each Pending pin uploaded in B.3: transition
       BOOKMARK/{database_id}/{bookmark_str}/pinned.status = Ready.
  C.6. Clear /META/cold_compact.in_flight_uuid.
  C.7. Tx commits.
```

WHY split into A/B/C: S3 latency is unbounded relative to FDB tx-age (5s). Mixing them deadlocks. Phase A's brief FDB tx records the pending uuid into FDB so the S3 PUT in A.4 can be performed outside any tx; the durability handoff is "marker is durable iff its uuid is present in FDB", giving stale-marker cleanup a deterministic recovery target. Phase B holds no FDB tx; the only fence is the OCC regular-read in Phase C.

WHY OCC fence on `cold_drained_txid`: a hot-compactor pass running concurrently could shrink the drainable window; cold's plan must still be valid at write time.

### Cold compactor trigger and dedup

- Triggered via UPS subject `SqliteColdCompactSubject` with queue group `"cold_compactor"`.
- Trigger conditions: published when `head_txid - cold_drained_txid >= cold_drain_threshold` after a hot pass; published from the hot compactor (NOT from the envoy hot path).
- Burst-mode: signal derived from FDB lag in `/META/manifest.cold_drained_txid - /META/head.head_txid`, not a per-pod 5xx counter.
- Dedup: cold compactor checks `/META/cold_lease` before doing work; if held by another pod, skip.
- Lease shape mirrors hot: 30s TTL, 10s renew, 5s margin, local-timer + cancel-token + renewal task.

WHY no cron: cold runs on demand. Idle databases do not generate cold work. A trigger-loss safety net (force-publish if `now - last_cold_pass > cold_max_silence_ms`) lives on the hot-compactor pod, not on a separate cron.

### 12.3 Eviction compactor

Always runs when its predicate fires. Clears the FDB resident set when a branch is past the hot-cache window AND its tail is durably in S3.

Global eviction index, **bucketed by `ACCESS_TOUCH_THROTTLE_MS`** (fix #11):

```
[GLOBAL] eviction_index/{last_access_bucket_be:8}/{database_id}  →  empty
where last_access_bucket = floor(last_access_ts_ms / ACCESS_TOUCH_THROTTLE_MS)
```

Updated lazily by `Db` on read/write activity. Per-conn cache suppresses redundant writes: Db writes a new `last_access_ts_ms` (and re-keys the eviction index entry) only when the bucket has advanced past the cached value. Default `ACCESS_TOUCH_THROTTLE_MS = 60_000` (1 minute). At default settings, eviction-index churn drops by 60_000× per database at 1ms commit cadence and is bounded to ~1 write/min/database regardless of commit rate.

```
1. Take CMPC/lease_global/{kind=eviction}.
2. range_scan(eviction_index/, limit=BATCH_SIZE).
3. For each (last_access_bucket, database_id):
     // Snapshot OCC fence inputs (fix #6).
     a. Read /META/manifest/cold_drained_txid + /META/head.head_txid +
        /META/manifest/last_hot_pass_txid (regular reads).
        Record last_hot_pass_txid_at_plan = last_hot_pass_txid.
     b. If now - bucket_to_ts(last_access_bucket) < HOT_CACHE_WINDOW_MS:
          skip (still hot).
     c. Compute predicate "evictable":
          - newer SHARD version exists in FDB (eviction is shard-version-level)
          - older than HOT_CACHE_WINDOW_MS
          - cold_drained_txid >= max_folded_txid for this shard version
          - last_hot_pass_txid_at_plan - SHARD_RETENTION_MARGIN >= as_of_txid_being_evicted
          - no descendant pin (`desc_pin <= as_of_txid` -> NOT evictable)
          - no bookmark pin (`bk_pin <= as_of_txid` -> NOT evictable)
     d. Begin eviction tx:
          - Regular-read /META/manifest/last_hot_pass_txid; assert ==
            last_hot_pass_txid_at_plan. If a hot pass landed between
            plan and clear, abort the eviction tx and re-run from step a.
          - For each evictable shard version: clear_range the FDB key.
          - For each evictable DELTA covered by an uploaded delta layer:
            clear_range.
          - Tx commits.
     e. If database is fully evicted (no SHARD in FDB at all): remove from
        index; future read on this database lazy-rehydrates from S3.
     f. Lease renewal handled by background task, not in tx.
```

WHY a global index: per-database sweep would require iterating all databases. The global index is sortable by `last_access_bucket`, so eviction scans cold candidates first.

WHY the OCC fence on `last_hot_pass_txid` (fix #6): without it, eviction can clear a SHARD version mid-fold and lose pages. The fence aborts eviction if any hot pass committed between plan and clear; the SHARD_RETENTION_MARGIN gives a buffer for marginally-stale plans.

WHY the predicate gates on three conditions: any one being false means eviction would lose data. `desc_pin` means a fork descendant still depends on this version. `bk_pin` means a pinned bookmark exists. Both are atomic-min, so reads see the strictest constraint.

Eviction compactor lease is global, not per-database. Sweep work is bounded by `BATCH_SIZE` per pass; multiple pods do not need to parallelize across databases.

PIDX-and-shard deletes use `COMPARE_AND_CLEAR` semantics analogous to the inherited stateless invariant: clear is conditional on the value still being the planned-evicted value, so a hot pass that overwrote the slot during the plan-vs-clear window does not get its newer value clobbered.

## 13. GC

GC eligibility for any data tied to a branch (DELTA blob, SHARD version, COMMIT row, VTX entry, layer in S3):

```
delete_eligible(branch, versionstamp):
  let gc_pin = min(
    refcount > 0 ? branch.root_versionstamp : SENTINEL_INFINITY,
    branch.oldest_descendant_pin,
    branch.oldest_bookmark_pin,
  );
  versionstamp < gc_pin
```

Fix #3: the formula uses `branch.root_versionstamp` (added to both branch records), not the nonexistent `created_at_versionstamp`. For root branches with no parent, `root_versionstamp` equals the genesis versionstamp; for fork descendants it equals the `at_versionstamp` of derivation.

The three counters are independent atomic-min reads. GC pass computes the min on the fly; the result is never persisted.

For COMMITS / VTX / DELTA range deletes that are keyed by txid (not versionstamp), GC translates `gc_pin` to a txid floor by reading `VTX[gc_pin] -> txid_floor` (the txid of the commit at or just before the pin). Without VTX, this would be a full-COMMITS scan.

WHY no monotonic ratchet: pin recomputes per pass and can decrease when descendants are deleted. A branch whose only descendant gets deleted should immediately become GC-eligible at its tail.

### Branch deletion via refcount + pin

There is no `BranchState::Frozen` retention class and no pointer-history audit log. Branches are deleted by the standard refcount + pin pass: when a database's `refcount == 0` AND no `desc_pin` / `bk_pin` holds it, GC sweeps the entire branch (META, COMMITS, VTX, PIDX, DELTA, SHARD, S3 layers under `db/{database_id}/`). Same for namespace branches. Without rollback at storage, no branch ever transitions from "live" to "frozen waiting for undo": every observed-live branch is referenced by either an engine-layer mapping (refcount += 1 from the engine's `database → database_id` mapping), a descendant fork (`desc_pin`), or a pinned bookmark (`bk_pin`). Dropping the last reference makes the branch GC-eligible.

### Cold compactor follow-up sweep

After Phase C commits, cold runs a follow-up sweep that reads the three pin counters for the branch and DELETEs S3 objects whose `(min_versionstamp, max_versionstamp)` range falls entirely below the pin. Same OCC pattern; if a fork lands during the sweep that pulls the pin back, the sweep no-ops on the now-unrecoverable objects.

The cold manifest's chunked layout (fix #18) means the sweep updates the index file (removes chunk references that are entirely below the pin) and deletes the now-orphaned chunk + layer files. Manifest update precedes layer deletion so no manifest entry outlives its layer.

## 14. LSM-shaped storage

Every namespace flows through one uniform storage pipeline, shaped like a two-level LSM:

- **L0 — recent DELTAs in FDB (hot tier).** Each commit lands as a DELTA blob in `BR/{database_id}/DELTA/{txid_be:8}/{chunk_be:4}` plus a PIDX entry per dirty page. This is the write buffer.
- **L1 — folded SHARD versions in FDB (hot tier).** The hot compactor folds DELTAs into versioned SHARDs at `BR/{database_id}/SHARD/{shard_id_be:4}/{as_of_txid_be:8}`. Reads pick the largest `as_of_txid <= read_txid`. SHARD versions are bounded by `MAX_SHARD_VERSIONS_PER_SHARD = 32` per shard via inline force-eviction on overflow.
- **L2 — image / delta / pin layers in S3 (cold tier).** The cold compactor migrates folded SHARDs (image layers) and consolidated DELTA ranges (delta layers) to S3, plus pin layers for pinned bookmarks. Writes the small mutable `cold_manifest/index.bare` plus an immutable per-pass chunk file.
- **Reclamation.** The eviction compactor reclaims FDB hot-tier bytes once `cold_drained_txid` covers the SHARD version's range AND no descendant or bookmark pin holds the byte range. The hot compactor unconditionally GCs COMMITS + VTX rows older than `HOT_RETENTION_FLOOR_MS` so the hot tier does not grow without bound on a namespace that never accumulates pin pressure.

There is no per-namespace storage mode. The cold compactor and eviction compactor run unconditionally; an idle or small namespace simply has nothing to upload (cold) or evict (eviction). Operators tune cadence, lag thresholds, and quotas globally via the constants in §12.1.

### What's stored where

| Layer | Location | Contents |
|---|---|---|
| L0 (write buffer) | FDB | DELTA blobs, PIDX entries for commits not yet folded |
| L1 (folded hot) | FDB | versioned SHARDs at `SHARD/{shard_id}/{as_of_txid}`, COMMITS + VTX rows for the hot retention window |
| L2 (cold) | S3 | image layers (one shard per file), consolidated delta layers, pin layers for pinned bookmarks, branch_record, cold_manifest, pointer_snapshot |

COMMITS + VTX rows past `HOT_RETENTION_FLOOR_MS` are GC'd by the hot pass. Bookmark resolution past the floor falls through to the cold manifest and replays from S3 layers; if no cold layer covers the bookmark and no pin extends retention, resolution returns `BookmarkExpired`.

### DR posture (fix #9)

Uniform across all namespaces. FDB-region-loss + S3-survival recovery: restore FDB from FDB backup, replay images + deltas from S3, rebuild NSBRANCH + BRANCHES + NSCAT from the most recent `catalog_snapshot.bare`. RPO is bounded by cold-pass cadence: any commits since the last cold pass have only FDB durability and are lost on a full FDB-region failure. Operators wanting tighter RPO tune the cold drain threshold and the cold-pass schedule globally.

### Cost discussion

Storage cost is proportional to write volume + fork/pin density. A namespace that takes few commits and never forks produces few DELTAs, few folded SHARDs, and tiny cold layers (just `branch_record` and `pointer_snapshot` plus the manifest index per pass). A heavy-write namespace produces proportionally more DELTAs (briefly), more SHARD versions, and more image/delta layers in S3. Pinned bookmarks add one pin layer per pin, plus extended retention on the layers that cover the pin. Operators can tune cadence (cold drain threshold, hot retention floor, eviction batch size) globally; per-namespace tuning is not part of this design.

## 15. Concurrency invariants

| Invariant | Mechanism |
|---|---|
| Single writer per database | Pegboard exclusivity (release); debug sentinel reads `head.branch_id` |
| Hot vs cold compactor on same database | Separate FDB-backed leases, disjoint META keys (`/META/compactor_lease` vs `/META/cold_lease`) |
| Cold pass A/C consistency | OCC regular-read on `/META/cold_compact.cold_drained_txid` |
| Eviction vs concurrent hot pass (fix #6) | Eviction OCC regular-read on `/META/manifest/last_hot_pass_txid` inside eviction tx; abort if hot advanced. SHARD_RETENTION_MARGIN protects marginal stales. |
| Fork vs concurrent GC | OCC regular-read on source branch's `bk_pin` inside fork tx |
| Eviction vs concurrent fork | Eviction reads `desc_pin` regular (not snapshot); fork's atomic-min triggers OCC abort on eviction tx |
| Refcount accuracy | Atomic-add only; cross-tx writes use atomic counters so concurrent fork + delete add their own deltas without RMW conflicts |
| Pin advance | Atomic-min only; commits never have to read pin in the hot path |
| PIDX + versioned-SHARD deletion under concurrent commit | `COMPARE_AND_CLEAR` (extends to eviction's shard-version clear) |
| Multi-bookmark pin | Each bookmark contributes via atomic-min; deletion reads bookmark and runs a per-pass full recompute (pass is rare; O(pins-in-namespace)) |
| NSCAT visibility under concurrent fork_namespace | NSCAT entries carry FDB versionstamps via `SetVersionstampedValue`; namespace forks cap reads at the new namespace's `parent_versionstamp` so a database created in the source after the fork is invisible to the fork. No coordination required between concurrent fork_database (in source) and fork_namespace (off source). |

## 16. Failure modes

| Failure | Detection | Recovery |
|---|---|---|
| Cold pass crashes between Phase B and Phase C | Pending marker `pending/{uuid}.marker` older than `STALE_MARKER_AGE_MS` | Next cold pass sees stale marker, re-uploads/overwrites layers (idempotent), deletes marker, retries Phase C |
| Cold pass crashes mid-Phase B | Same as above | Same; partial S3 PUTs are overwritten on retry |
| Cold pass crashes mid-Phase C | FDB tx aborts; lease still held | Lease expires after TTL; next pod retries from Phase A |
| Hot pass crashes mid-pass | FDB tx aborts; lease still held via local timer + cancel | Lease expires after TTL; next trigger pod re-runs |
| Eviction pass crashes mid-clear | FDB tx aborts | Eviction index entry not removed; next sweep retries |
| Fork at GC'd point | Fork tx OCC abort on `bk_pin` regular-read mismatch | Return `ForkOutOfRetention` to caller |
| Bookmark resolution at GC'd point | Resolver finds no preserved checkpoint at or before bookmark | Return `BookmarkExpired` |
| Pinned bookmark S3 PUT fails | Cold compactor pass observes error and writes `PinStatus::Failed` | `create_pinned_bookmark` already returned; status query / cold-pass UPS retries; caller may `delete_pinned_bookmark` to clean up (fix #13) |
| Eviction races concurrent hot pass | Eviction tx OCC abort on `last_hot_pass_txid` mismatch | Re-plan from current state (fix #6) |
| Concurrent fork_database + delete_database | Both atomic-add on source's refcount; fork's `desc_pin` atomic-min holds retention even if delete decrements refcount to 0 | Either order is valid; deletion only completes via GC once descendant chain resolves and pins drop |
| FDB disaster + S3 survival | All cold layers + branch records + cold manifests + `catalog_snapshot.bare` still in S3 | Reconstruct NSBRANCH + BRANCHES + NSCAT from latest catalog_snapshot; reconstruct per-branch records from `branch_record.bare`; replay images + deltas onto a fresh FDB; lossy: any commits since last cold pass are gone (RPO bounded by cold-pass cadence) |
| Cold lease holder dies during a pass | TTL expires; next take wins | Idempotent S3 layout means retried PUTs are safe |
| Hot lease holder dies during a pass | Same as cold | Same |
| Cross-pod schema mismatch (rolling deploy) | Cold pass reader path encounters newer schema | Reader code retains old-version paths for one full retention window past rollout (binding constraint from CLAUDE.md) |
| Eviction compactor lags badly | FDB hot tier grows unboundedly | Quota check at commit rejects; user sees `SqliteStorageQuotaExceeded` |
| Bookmark refers to a database whose only descendant is a fork | Resolver walks parent chain; if `parent_versionstamp >= bookmark.versionstamp`, hit the parent | Standard parent fall-through |

## 17. Hot-path latency analysis

This section is rewritten for v4 to reflect the cache-invalidation contract removal (-2 reads vs v3) and the collapse of pointer indirection.

### Cost added per commit vs stateless spec

A v4 commit reads + writes (relative to the stateless spec):

- **+1 read:** `/META/manifest/last_access_bucket` for the throttled-access-touch decision.
- **+1 read:** `BRANCHES/list/{database_id}` (cached on per-conn; first commit only).
- **+1 write:** VTX entry (`SetVersionstampedKey`, no extra RTT — adds to the existing commit batch).
- **+1 write (throttled):** `last_access_ts_ms` + eviction index re-key, only when `ACCESS_TOUCH_THROTTLE_MS` bucket has advanced.

Removed vs v3:

- **-1 read:** DBPTR `last_swapped_at_ms` (cache invalidation contract gone — branches are immutable).
- **-1 read:** NSPTR `last_swapped_at_ms` (same).

No tier-state read on the commit path. Hot, cold, and eviction compactors all run unconditionally; per-commit cost is identical for every namespace.

On FDB native, all reads in a single tx can be issued via `try_join!` and observe one wall-clock RTT (the slowest read). On RocksDB or under FDB tail latency, parallel reads serialize per tx and the extra reads each add real wall-clock ms — and the v3→v4 -2-reads delta directly improves wall-clock commit latency on those backends.

### Steady-state RTTs

| Operation | RTTs (FDB native, parallel reads) | RTTs (worst-case sequential) | Notes |
|---|---|---|---|
| `get_pages` (warm cache, non-evicted) | 1 | 1 | head read; PIDX/SHARD blob fetch piggybacks. No cache-invalidation parallel reads. |
| `get_pages` (warm cache, evicted shard) | 1 + 1 S3 GET | 1 + 1 S3 GET | cold-tier path |
| `get_pages` (cold cache, fresh Db) | 1 + N | 1 + N | N = ancestry depth, capped by `MAX_FORK_DEPTH × MAX_NAMESPACE_DEPTH` |
| `commit` (steady state) | 1 | 1 | head + DELTA + PIDX + COMMITS + VTX + atomic-add quota — fold into the single FDB tx batch |
| `commit` (first on new Db) | 1 (with try_join!) | 2 | head + manifest in parallel |
| `fork_database` | 1 | 1 | Single FDB tx (with VTX lookup + head_at_fork write + NSCAT versionstamped write) |
| `fork_namespace` | 1 | 1 | Single FDB tx |
| `delete_database` | 1 | 1 | Single FDB tx (refcount atomic-add + tombstone write) |
| `delete_namespace` | 1 | 1 | Single FDB tx |
| `create_bookmark` (ephemeral) | 0 (in-memory) | 0 | Derivable from cached versionstamp + now_ms |
| `create_pinned_bookmark` | 1 FDB tx, returns immediately | 1 | Async via cold compactor (fix #13); bookmark string returned with PinStatus::Pending |
| `bookmark_status` | 1 | 1 | Single FDB read |
| `resolve_bookmark` | 1 (single branch) up to MAX_FORK_DEPTH × MAX_NAMESPACE_DEPTH | 256 | Worst case bridges both database and namespace fork chains (fix #14) |

### Fork latency

- O(1) FDB metadata operation. Wall-clock latency dominated by single-tx round-trip plus the OCC regular-read of source's `bk_pin` plus the VTX lookup + COMMITS read for `/META/head_at_fork` snapshot (fix #4).
- Adds ~3 extra reads vs v1: VTX[at_versionstamp], COMMITS[txid_at_v], ns_branch_depth walk for MAX_NAMESPACE_DEPTH check.
- Empirically tens of milliseconds; sub-100ms p99 expected.
- No data motion, no S3 PUT, no LTX encoding.

### Pinned bookmark latency (fix #13)

- `create_pinned_bookmark` returns in 1 FDB tx (≈ tens of ms). The bookmark is usable immediately with `PinStatus::Pending`.
- Cold compactor performs LTX encoding (FDB read of every page) + S3 PUT in a background pass. For a 1 GiB DB at typical S3 PUT speeds, wall-clock to `Ready` is seconds to tens of seconds. Caller does NOT block on the WS conn handler.
- `bookmark_status` polls `PinStatus`. If exact-undo guarantee is required before continuing, callers wait for `Ready`; otherwise the pin contributes to `bk_pin` immediately, so retention is held even while the S3 PUT is in flight.
- Multi-database pinning storms are absorbed by the cold compactor's existing rate-limit (UPS queue group + lease).

## 18. Quota and billing

Per-database `/META/quota` semantics from the stateless spec carry over for the hot tier. Branching adds:

- **Per-fork quota.** Each fork allocates a new database or namespace and starts at zero `/META/quota`. The source database's quota is unaffected. Open question 24.4: do forks count against the source's quota for billing? Storage layer does not enforce this; engine edge does.
- **Cold-tier quota.** S3 storage is quota-tracked in a separate counter `/CTR/cold_quota_global` (or per-namespace; open question). Cold compactor atomic-adds on every PUT and atomic-subs on every DELETE. Pinned bookmarks contribute to cold quota.
- **Hot-quota delta on engine-layer rollback.** Engine-layer rollback calls `fork_database`; the new database's `/META/head` is empty until the first divergent write. The fork itself adds zero hot quota.

Engine-edge billing model is open. Storage layer exposes counters: `quota_per_database`, `cold_quota_per_database`, `fork_count_per_namespace`, `pin_count_per_namespace`. Engine reads these and decides what to charge for.

## 19. Authorization model

Storage layer does not implement authorization. The two operations and the bookmark APIs are caller-trusted. The engine edge enforces:

- Caller may only operate on databases/namespaces in their tenancy.
- Cross-namespace fork requires permission on both source and target namespaces.
- Bookmark resolution scoped to caller's databases (sender-scoped — a bookmark for database D1 cannot be resolved by database D2 even within the same caller's tenancy unless explicit sharing semantics exist at the engine edge).

Storage returns `BranchNotReachable` when ancestry walks would cross a database's branch chain, which the engine edge translates to a permissions error.

## 20. Observability

### Metrics (Prometheus, all labeled with `node_id`)

- `sqlite_branch_fork_total{op=database|namespace, outcome=ok|out_of_retention|too_deep|err}` — fork operation count.
- `sqlite_branch_delete_total{op=database|namespace, outcome=ok|err}` — delete operation count.
- `sqlite_bookmark_create_total{kind=ephemeral|pinned, outcome=ok|err}`.
- `sqlite_bookmark_resolve_total{outcome=ok|expired|unreachable|err}`.
- `sqlite_bookmark_resolve_duration_seconds` (histogram).
- `sqlite_branch_ancestry_walk_depth` (histogram).
- `sqlite_branch_pin_advance_total{kind=desc|bookmark}`.
- `sqlite_cold_pass_duration_seconds` (histogram, three sub-labels: `phase=A|B|C`).
- `sqlite_cold_pass_layers_uploaded_total{kind=image|delta|pin}`.
- `sqlite_cold_pass_bytes_uploaded_total`.
- `sqlite_cold_lease_take_total{outcome=acquired|skipped|conflict}`.
- `sqlite_eviction_pass_duration_seconds` (histogram).
- `sqlite_eviction_pass_shards_cleared_total`.
- `sqlite_eviction_pass_deltas_cleared_total`.
- `sqlite_pending_marker_orphan_cleaned_total` — stale HWM markers cleaned.
- `sqlite_pin_status{status=pending|ready|failed}` — gauge per pinned bookmark async-PUT status (fix #13).
- `sqlite_eviction_occ_abort_total{reason=hot_pass_advanced|desc_pin|bk_pin}` — eviction tx aborted due to OCC fence (fix #6).
- `sqlite_shard_versions_per_shard` — histogram; alert near `MAX_SHARD_VERSIONS_PER_SHARD` (fix #17).
- `sqlite_dr_posture{recoverable_from=s3|fdb_backup_only}` — namespace-level DR observability.
- `sqlite_pinned_bookmark_count_per_namespace` — gauge; alert near `MAX_PINS_PER_NAMESPACE` (fix #19).
- `sqlite_cold_lag_versionstamps` — per-database gauge of `head_versionstamp - cold_drained_versionstamp`.
- `sqlite_bookmark_resolution_chain_depth` — histogram of total parent-chain hops including ns + database (fix #14).

### Debug APIs

- `debug::dump_database_ancestry(database_id) -> Vec<(database_id, parent_versionstamp)>` — dumps the full chain.
- `debug::dump_branch_pins(database_id) -> BranchPins` — refcount, desc_pin, bk_pin.
- `debug::list_bookmarks(database_id) -> Vec<BookmarkIndexEntry>` — both ephemeral (resolved at call time) and pinned.
- `debug::dump_cold_manifest(database_id) -> ColdManifest` — for retention diagnostics.
- `debug::estimate_gc_pin(database_id) -> [u8; 16]` — what GC would compute right now.

## 21. Testing strategy

Inherits from stateless spec test conventions.

- All tests live in `engine/packages/sqlite-storage/tests/`. No inline `#[cfg(test)] mod tests` in `src/`.
- Fork tests use real UDB via `test_db()`.
- Cold-tier tests use `ColdTier::Filesystem` (local filesystem stand-in for S3) and the UPS memory driver. No real S3 required.
- Pinned-bookmark tests verify the synchronous PUT path with a filesystem ColdTier.
- Lease tests use `tokio::time::pause()` + `advance()`.
- OCC fence tests inject staged FDB transactions to provoke fork-vs-GC and cold-A-vs-hot-pass races.
- Failure-injection tests use `MemoryStore::snapshot()` for cold pass crash recovery (Phase B crash, Phase C crash, lease expiry).
- Eviction tests verify the predicate's three gates independently (descendants pin, bookmark pin, hot-cache window).
- Bookmark resolution tests verify ephemeral resolution, pinned resolution, expired path, unreachable path, and parent fall-through.
- Schema-version-skew tests verify reader code retains old-version paths.

New test files to add:

- `tests/fork_database.rs`, `tests/fork_namespace.rs` — forks at all combinations of (root, depth-1, depth-N) sources; OCC fence races.
- `tests/bookmarks.rs` — ephemeral, pinned, parent-chain resolution, namespace-fork bridging (fix #14).
- `tests/cold_compactor.rs` — Phase A/B/C orchestration, OCC fence, pending markers, pending-marker-outside-FDB-tx (fix #7).
- `tests/eviction_compactor.rs` — predicate gates, global index sweep, OCC fence on `last_hot_pass_txid` (fix #6), throttle (fix #11).
- `tests/gc.rs` — pin computation, dependency-graph deletion, refcount-driven branch deletion.
- `tests/list_databases.rs` — database enumeration with NSCAT cap-by-versionstamp, tombstones across deep ns fork chain.

### Fault-injection scenarios (fix #25)

The most critical fault-injection tests:

1. **`tests/cold_compactor_5xx_phase_b.rs`** — S3 returning 5xx during Phase B uploads. Assert: cold pass aborts cleanly, lease released, retry from Phase A is idempotent (re-uploads succeed via deterministic keys), `sqlite_s3_request_failures_total{op=put}` increments.
2. **`tests/cold_compactor_phase_a_pending_put_5s.rs`** — S3 5s p99 latency on Phase A pending marker PUT. Assert: Phase A's brief FDB tx commits FIRST (records uuid into `/META/cold_compact.in_flight_uuid`), then S3 PUT happens outside the tx (fix #7); FDB tx age never exceeds 80% of 5s budget.
3. **`tests/cold_compactor_lease_loss_b_to_c.rs`** — Cold compactor lease lost between Phase B (uploads complete) and Phase C (FDB write). Pod B picks up: assert pod B's pass is idempotent (overwrites are safe by deterministic keys), pod A's leaked layers are deleted via the self-describing pending marker.
4. **`tests/eviction_during_active_read.rs`** — Eviction lands during active read on a still-hot database. Assert: throttled access-touch (fix #11) keeps `last_access_bucket` current; eviction skips on the bucket gate; no torn reads.
5. **`tests/concurrent_fork_during_eviction.rs`** — `fork_database` lands while eviction is mid-pass on the source database. Assert: fork's `desc_pin` atomic-min is observed by the eviction tx OCC fence; eviction either re-plans (if it has not committed) or no-ops on the now-pinned versions.
6. **`tests/gc_pin_recompute_under_bookmark_delete_race.rs`** — `delete_pinned_bookmark` raced against `fork_database` at the to-be-deleted bookmark's versionstamp. Assert: either fork sees the pin as still-active (succeeds), or fork sees pin gone (aborts with ForkOutOfRetention only if GC has already run); never silent data loss.
7. **`tests/dr_replay_from_s3_alone.rs`** — FDB region failure simulated via fresh empty FDB; restore from `catalog_snapshot.bare` + cold manifest chunks + image/delta/pin layers. Assert: NSBRANCH + BRANCHES + NSCAT rebuilt; reads on reconstructed databases return correct data up to `catalog_snapshot.pass_versionstamp`. RPO = (now - last cold pass) for any commits after the snapshot.

## 22. Implementation strategy

This work builds on the stateless spec's deliverables. It is a large but mechanical extension of the storage crate.

### Stage 1: immutable branch records (no fork yet)

- Add BRANCHES/NSBRANCH/NSCAT key partitions.
- Migrate `Db` to read BRANCHES record on first `get_pages`/`commit`. Cache the database id (= branch id) on `Db`.
- Bump the on-disk META layout to live under `BR/{database_id}/`. Existing creation paths allocate a root branch + NSCAT presence marker on first commit.
- All existing tests pass (single-database behavior identical).

### Stage 2: bookmarks

- Wire format, ephemeral creation, ephemeral resolution.
- COMMITS row gains versionstamp via `SetVersionstampedValue`.
- BOOKMARK key partition.
- No pinned bookmarks yet.

### Stage 3: fork_database and fork_namespace

- `derive_branch_at` primitive.
- New `database_id` / `namespace_id` allocation (which IS the new branch id).
- `MAX_FORK_DEPTH` and `MAX_NAMESPACE_DEPTH` enforcement.
- `delete_database` / `delete_namespace` (refcount + tombstone).

### Stage 4: cold compactor

- Phase A/B/C scaffolding.
- LTX V3 layer encoding.
- ColdManifest/BookmarkIndex/CatalogSnapshot vbare schema.
- HWM markers.
- Lease + OCC fences.
- Filesystem-backed ColdTier for tests.
- S3 driver behind config flag.

### Stage 5: pinned bookmarks

- `create_pinned_bookmark` async PUT path via cold compactor.
- bk_pin atomic-min on creation; recompute on deletion.

### Stage 6: eviction compactor

- Global eviction_index.
- Predicate computation.
- Lease + sweep loop.
- `last_access_ts` updates from `Db`.

### Stage 7: cold-tier read fall-through

- Read path detects FDB miss, loads cold manifest, GETs from S3.
- Per-conn cold-manifest perf cache (LRU, bounded).

### Stage 8: optional periodic checkpoints

- Optional cron-equivalent (configurable cadence) for periodic image PUT, useful for short-window PITR without explicit pins. Cadence is a global operator knob, not a per-namespace mode.

### Stage 9: documentation rollout

- Create `docs-internal/engine/sqlite/` per section 23.
- Update `engine/packages/sqlite-storage/CLAUDE.md` to point at the new docs folder.

Stages do not need to leave the codebase in a working state at intermediate boundaries; the rewrite is greenfield-friendly. Final delivery must compile, pass tests, and be feature-complete.

## 23. Documentation requirement

The implementation MUST create a new folder `docs-internal/engine/sqlite/` containing exactly five files. The spec INSTRUCTS this creation; the spec itself does not write the docs.

- `docs-internal/engine/sqlite/storage-structure.md` — full FDB and S3 key layout (per-database, per-branch, per-namespace, global), indirection layers, versioned SHARDs. Read by anyone touching key formats.
- `docs-internal/engine/sqlite/components.md` — pump (hot path), hot compactor, cold compactor (Phase A/B/C), eviction compactor. Per-component responsibilities and lease ownership.
- `docs-internal/engine/sqlite/vfs-brief.md` — high-level VFS interaction. KEPT BRIEF: full VFS details remain in the package docs / `docs-internal/engine/sqlite-vfs.md`. This file links to those.
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md` — binding constraints (single writer, no local files, lazy read, per-commit granularity), architectural rationale for: rough-PITR-by-default vs Neon's exact-PITR (cost), pages-self-describing insight, why versioned SHARDs (avoiding read-your-writes ambiguity on rollback), why two-level indirection (stable external IDs across rollback).
- `docs-internal/engine/sqlite/comparison-to-other-systems.md` — Neon, CF DO, Snowflake, LiteFS, Litestream, mvSQLite, Turso. For each: what we share, what we diverge on, with the *why*.

Additionally, the implementation MUST update `engine/packages/sqlite-storage/CLAUDE.md` to:

1. Reference the new `docs-internal/engine/sqlite/` folder under "Reference Docs".
2. Add a maintenance bullet: "When changing FDB or S3 key layout, branch metadata, or compactor responsibilities, update `docs-internal/engine/sqlite/{storage-structure,components,constraints-and-design-decisions}.md` in the same change."

This spec does NOT write any of those files. It only commits the contract that the implementation will produce them.

## 24. Open questions

1. `HOT_CACHE_WINDOW_MS` default (configurable per database or per namespace?). Current draft: 7 days, namespace-level.
2. Per-fork billing model. Storage exposes counters; engine edge decides. Proposal: forks count against source quota until first divergent write; after divergence, charged against the fork's tenancy.
3. Multi-region cold tier. S3 buckets are single-region by default. Cross-region replication is future work; the current spec assumes single-region S3.
4. Cold compactor periodic-image cadence default. Proposal: per-shard image every 30 days OR `head_txid - last_image_txid >= IMAGE_TXID_THRESHOLD`, whichever fires first.
5. Bookmark resolution complexity bound. Worst case is `MAX_FORK_DEPTH × MAX_NAMESPACE_DEPTH = 256` parent-chain hops (fix #14). For pathological deep-nested namespaces this dominates `resolve_bookmark` latency. Mitigation candidates: a per-bookmark cached resolution, or a resolution-target hint stored in `BookmarkRef`.
6. `MAX_PINS_PER_NAMESPACE` default. Currently 1024 (fix #19). Higher values pose `delete_pinned_bookmark` recompute cost; lower values constrain real workloads.
7. Pin recompute on delete is O(pins-in-namespace) (fix #19, deferred). At 1024 pins per namespace, deletion blocks for ~ms but is rare. Future optimization: maintain a sorted in-FDB index of pin versionstamps so the new floor is computable in O(log N).
8. GC of orphaned databases is engine-owned. Storage primitives `delete_database` / `delete_namespace` only handle refcount-based cleanup. The engine's database-lifecycle subsystem decides when a `database_id` no longer has any database referencing it (the engine's own `database → current_database_id` mapping is the source of truth), and calls `delete_database` accordingly. Storage does not enumerate "orphaned databases" because storage has no view of which databases are bound to live databases versus historical database mappings.

## 25. Future directions

- **Multi-region cold tier.** Cross-region S3 replication; reads served from the regional bucket; pinned bookmarks visible globally.
- **Namespace sharding.** As namespace count grows, sharding across FDB key prefixes becomes necessary; the immutable-branch model makes this transparent to callers.
- **Bookmark sharing across databases.** Today bookmarks are sender-scoped. A future API could allow sharing a pinned bookmark with another database for cross-database restore.
- **Async fork warmup.** Fork is zero-copy at fork time but cold reads on the new branch fall through to the parent's S3 layers. A background warmup pass could pre-copy the parent's resident set into the new branch's FDB keys; deferred because the existing PIDX-flattened-ancestry cache + Option F client-side read cache make hot-path reads tolerable.
- **Compactor for COMMITS rows.** A heavy-write namespace may produce a lot of COMMITS rows in FDB. A per-database COMMITS compaction pass that keeps only "preserved checkpoint" rows could reclaim FDB space; deferred until COMMITS row volume becomes a measured problem.
- **Versioned schema migration.** When schema evolves, the cold compactor reads old version + writes new version on every pass. A "force migrate all branches now" admin op may be useful for accelerating rollout windows.
- **Snapshot-isolated reads across history.** Today `get_pages` returns the head's view; a future read API could resolve at an arbitrary versionstamp without taking a fork.
- **Cross-database consistency snapshots.** A namespace-wide AS-OF read primitive that returns consistent state across all databases at versionstamp V, useful for backups and analytical consumers.

## 26. Files affected

### Greenfield additions to `engine/packages/sqlite-storage/src/`

- `pump/branch.rs` — BRANCHES/NSBRANCH/NSCAT read helpers; ancestry walk.
- `pump/branch_cache.rs` — flattened ancestry per-conn perf cache.
- `pump/bookmark.rs` — ephemeral and pinned bookmark APIs; resolution algorithm.
- `pump/operations/fork_database.rs`
- `pump/operations/fork_namespace.rs`
- `pump/operations/delete_database.rs`
- `pump/operations/delete_namespace.rs`
- `pump/operations/derive_branch.rs` — shared primitive.
- `compactor/cold/mod.rs` — phase A/B/C orchestration.
- `compactor/cold/phase_a.rs`
- `compactor/cold/phase_b.rs`
- `compactor/cold/phase_c.rs`
- `compactor/cold/manifest.rs` — ColdManifest/LayerEntry/BookmarkIndexEntry vbare types.
- `compactor/cold/lease.rs` — `META/cold_lease` take/check/release.
- `compactor/cold/markers.rs` — pending-marker HWM helpers.
- `compactor/cold/s3.rs` — S3 driver (or filesystem stand-in).
- `compactor/eviction/mod.rs` — sweep loop.
- `compactor/eviction/index.rs` — global eviction-index access.
- `compactor/eviction/predicate.rs` — three-gate predicate.
- `gc/mod.rs` — pin computation, dependency-graph deletion (used by cold + eviction).

### Modifications

- `pump/keys.rs` — add BRANCHES/NSBRANCH/NSCAT/CTR/BOOKMARK/CMPC partitions; extend BR partition under `BR/{database_id}/`.
- `pump/types.rs` — `DBHead` gains `branch_id`; new `DatabaseBranchRecord`, `NamespaceBranchRecord`, `BookmarkRef`, `BranchManifest`. No pointer / `BranchState` types.
- `pump/commit.rs` — write `COMMITS[T]` with versionstamp; update `last_access_ts_ms`.
- `pump/read.rs` — ancestry walk; cold-tier fall-through on FDB miss.
- `pump/db.rs` — branch resolution on first request; ancestry cache field. No cache-invalidation contract (branches are immutable).
- `compactor/worker.rs` — extend to dispatch hot/cold/eviction triggers (separate UPS subjects).
- `compactor/subjects.rs` — add `SqliteColdCompactSubject` and the eviction subject.
- `engine/packages/engine/src/run_config.rs` — register `sqlite_cold_compactor` and `sqlite_eviction_compactor` as Standalone services.
- `engine/packages/sqlite-storage/CLAUDE.md` — reference `docs-internal/engine/sqlite/` and add the maintenance rule (per section 23).

### Engine-side change (NOT a sqlite-storage concern)

Pegboard's database abstraction (renamed to `Db` by US-065) owns an `database_id → current_database_id` mapping. This is engine-layer state, not storage layer state. Engine-layer "rollback" is implemented by:

1. Calling `resolve_bookmark` or otherwise picking an AS-OF versionstamp.
2. Calling `fork_database(at)` to allocate a new database id at that versionstamp.
3. Updating the engine's own `database_id → current_database_id` mapping to point at the new database id.
4. Tearing down any existing `Db` connection and opening a new one against the new database id.

The engine is also responsible for "is this database orphaned?" decisions and calls `delete_database` when the engine determines a database id is no longer referenced by any database mapping or pinned bookmark. Storage exposes the `delete_database` / `delete_namespace` primitives only; storage does not enumerate orphans.

This spec does not specify the engine-side data layout or trigger logic for that mapping. It is documented separately in pegboard / engine docs, not in `docs-internal/engine/sqlite/`.

### New documentation (created by implementation, not by this spec)

- `docs-internal/engine/sqlite/storage-structure.md`
- `docs-internal/engine/sqlite/components.md`
- `docs-internal/engine/sqlite/vfs-brief.md`
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md`
- `docs-internal/engine/sqlite/comparison-to-other-systems.md`

### Tests

- `tests/fork_database.rs`, `tests/fork_namespace.rs`.
- `tests/bookmarks.rs`, `tests/cold_compactor.rs`, `tests/eviction_compactor.rs`.
- `tests/gc.rs`.
- `tests/branch_ancestry.rs` — ancestry cache correctness, parent-chain depth.
- `tests/list_databases.rs` — NSCAT walk, cap-by-versionstamp filtering, tombstones.

### Schema (vbare)

- New `*.bare` schema versions for cold-tier types. Each persisted S3 vbare object carries `schema_version: u32`. Reader code retains old-version paths for at least one full retention window past rollout (binding constraint).

## 27. Divergences from prior art

| System | What we share | What we diverge on | Why |
|---|---|---|---|
| Neon | Layer model (image + delta), branch-as-pointer + branch-as-restore, dependency-graph GC | Rough-PITR-by-default vs exact-PITR; FDB hot tier is authoritative (Neon's pageserver is); single-layer immutable branches (Neon's branch records are mutable, ours are write-once + delete-on-refcount-zero). | Neon assumes Postgres-grade exact PITR is required; for database workloads, rough is sufficient and dramatically cheaper. FDB-as-source-of-truth eliminates the need for a separate pageserver layer. |
| CF Durable Objects (SQLite) | Bookmark-as-time-token concept, "snapshot when log >= db size" image-rebuild rule | DO has 3-of-5 follower quorum; we use FDB. DO has no fork primitive; we have two. DO has no cold tier; we always use S3 as L2. | FDB durability replaces the multi-replica WAL stream. Forks were never a DO design goal. |
| Snowflake | Time-travel + zero-copy clone via metadata-only branching | Snowflake is OLAP and per-table; we are per-database SQLite. Snowflake's FDN is internal; we expose primitives. | Application boundaries: databases vs warehouses. The metadata-only-clone model carries; the consumer surface is different. |
| LiteFS | LTX V3 file format, HWM pending markers | LiteFS uses local SQLite files + multi-replica WAL stream; we forbid local files. LiteFS has no PITR. | Local files conflict with statelessness. PITR is built around branches in our design, not WAL replicas. |
| Litestream | LTX V3, `Pos{TXID, PostApplyChecksum}` rolling checksum | Litestream is single-replica WAL shipping to S3; no fork, no branch. | Litestream's value is "incremental backup of one DB to S3"; ours is "branchable storage primitive". |
| mvSQLite | (deliberately none) | We are single-writer; mvSQLite's PLCC/DLCC/MPC/versionstamps/content-addressed-dedup are dead weight under exclusivity. | Pegboard exclusivity removes the multi-writer race surface mvSQLite was designed to handle. |
| Turso (libSQL) | Branching primitive (point-in-time fork) | Turso has local SQLite files with replication; we forbid local files. Turso PITR is per-database, with rollback as a storage operation; ours pushes rollback up. | Local-file constraint same as LiteFS. Storage exposes only `fork_database` (next row). |
| CF DO / Turso / Neon (rollback push-up) | (n/a) | **Storage exposes no rollback primitive.** Rollback semantics are implemented at the engine layer by calling `fork_database` and flipping the engine's `database → current_database_id` mapping. CF DO, Turso, and Neon all expose rollback as a storage operation. | Storage-level rollback forces a mutable pointer indirection (cache-invalidation contract on every commit + frozen state machine + pointer history audit log + rolled-back-vs-commit race window). Fork-only storage avoids all of that. The engine layer already owns the database lifecycle and is the natural home for "what database is this database currently bound to?" — pushing rollback up there matches that ownership boundary, and storage gets a strictly simpler invariant set in return: branches are immutable for life, period. |

The unifying divergence: most prior art assumes either (a) the underlying SQLite file is on a node, (b) writers are multi-replica, (c) PITR must be exact, or (d) rollback is a storage primitive. Our binding constraints reject all four. Single-layer immutable branches + tiered cold storage + rough PITR by default + engine-layer rollback are the architectural answers to those rejections.
