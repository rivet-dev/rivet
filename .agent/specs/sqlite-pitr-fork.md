# SQLite Point-in-Time Recovery + Forking

> **Revision:** v2 (post-adversarial-review). Architecture, performance, and operations critics surfaced ~40 findings against v1; the most severe (broken GC pin algorithm, O(N) fork-chain reads, hot-quota-wedge on S3 outage, orphan-layer races, fork-vs-GC OCC race) are addressed in this revision. Reviews live at `.agent/research/review-{architecture,performance,operations}.md`.

## Goals

1. **Continuous point-in-time recovery.** Any committed transaction within the retention window is recoverable. Granularity is per-commit. Mirrors Cloudflare Durable Objects' SRS contract: "we can restore to any point in time by replaying the change log from the last snapshot."
2. **Bookmarks** as the user-facing identity for points in time. Lexicographically sortable within a single branch's parent chain, cheap to obtain (`get_current_bookmark()`), cheap to resolve (`get_bookmark_for_time(t)`). The wire format omits branch_id; bookmarks are always interpreted relative to a branch context (the actor's current head, or an explicit `branch_id` argument).
3. **Forking** as a first-class primitive at the storage layer. A fork is a new branch that points at a parent branch + parent_txid; storage cost is metadata-only at fork time (copy-on-write at the layer level). PITR is the same primitive: "restore to bookmark B" creates a branch at B; the broader system decides whether to swap the actor onto it.
4. **S3 is the cold tier; FoundationDB is the hot tier.** Hot tier holds the live working set. Cold tier holds history for PITR. Steady-state hot path never hits S3. Historical reads do.
5. **30-day retention by default**, enforced by a dependency-graph GC. Layers are deletable only when older than the retention window AND not pinned by any descendant branch's fork point. The pin tracks the *minimum* of all live descendants' fork points and decreases when descendants delete (no monotonic ratchet).
6. **Two compactors with separate triggers and leases.** The hot compactor folds DELTA into SHARD inside FoundationDB on every commit-crosses-threshold trigger (existing — see `sqlite-storage-stateless.md`). The cold compactor drains compacted DELTAs into immutable LTX layer files in S3 on every commit-crosses-cold-threshold trigger. Both are UPS-driven; neither has a periodic cron.
7. **Storage library scope only.** Implement and test inside `engine/packages/sqlite-storage/`. Schema must support PITR + forking; broader system integration (pegboard fork API, billing for forked storage, bookmark UX) is out of scope.
8. **Recovery point objective is bounded by hot-tier durability, not cold-tier cadence.** A commit is durable as soon as the hot-tier (FDB) write completes — that is the same RPO as the existing stateless spec. Cold tier is the *retention horizon*, not the durability boundary. Loss of FDB data between cold passes is treated as a hot-tier disaster, not a PITR failure.
9. **Breaking changes are unconditionally acceptable.** The system has not shipped to production.

## Non-goals

- Multi-region replication of hot tier. SRS replicates each commit to 5 followers in distinct datacenters before acking; we do not. We rely on FoundationDB durability for hot-path commits, period.
- WAL frame-level shipping. We ship at the per-commit DELTA grain.
- Cross-actor PITR (restoring multiple actors to a coordinated point). Each actor is independent.
- Hot read-replicas. Hot tier is single-writer per pegboard exclusivity.
- Compression beyond LTX V3's per-page LZ4 in v1.
- Sub-commit PITR granularity. Smallest addressable unit is a committed transaction.
- Forking from arbitrarily old points. Fork-from-bookmark is bounded by the retention window plus a small safety margin (`GC_FORK_MARGIN_TXIDS`); older fork points return `ForkOutOfRetention`.
- Cross-actor branch references. A branch's parent must be in the same actor.
- Resurrecting deleted-but-still-cold branches. v1: `delete_branch` is the trim point.

## Inherited constraints

This spec sits on top of the constraints in `r2-prior-art/.agent/research/sqlite/requirements.md` (the binding floor for any SQLite storage spec):

1. **Single writer per database.** Pegboard exclusivity holds. There is no concurrent writer across actors, connections, or processes. We do not implement MVCC, page-versioned read-set tracking, optimistic conflict detection at commit, or content-addressed dedup. mvSQLite's PLCC/DLCC/MPC machinery is explicitly skipped — see [Prior-art divergences](#prior-art-divergences) below.
2. **No local SQLite files. Ever.** Not on disk, not on tmpfs, not as a hydrated cache file. The authoritative store is FoundationDB (hot) and S3 (cold); the VFS speaks to them directly. Forks do not materialize local files.
3. **Lazy read only.** No bulk pre-load at actor open. Pages are fetched on demand from the hot tier (FDB), with the per-actor PIDX cache + (post-fork) flattened ancestry cache amortizing the per-fetch cost. Fork warmup is a *background* cold→hot copy, not a synchronous bulk hydrate.

These constraints rule out any design built around a local SQLite file (LiteFS-shape, libSQL embedded replicas, Turso embedded replicas) and any "hydrate the whole DB at resume" path. They are why our cold tier is consulted only on miss — there is no "warm the cache before first SQL statement" phase.

## Dependency on Option F

Hot-path read latency in this spec inherits whatever the underlying read path delivers. The "Option F" track (`r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md`, US-020/021/025/026/027) is the *steady-state* hot-path read optimization track:

- US-020 / US-021: enable `read_cache` by default + bump `PRAGMA cache_size`.
- US-026: add `sqlite_read_many(actor_id, file_tag, ranges)` envoy op so one round-trip carries many pages.
- US-027: VFS-level stride prefetch predictor for sequential scans.

Option F is **orthogonal but complementary** to PITR/fork. It does not depend on this spec, and this spec does not depend on it for correctness, but the two together are what makes fork-descendant reads tolerable: Option F gets one-page misses down to a few ms; this spec then gets parent fall-through down to (depth × few ms) instead of (depth × tens of ms). Without Option F, the latency table below is optimistic.

If Option F is not shipping, the parent-fall-through path in this spec will be slow regardless of fork warmup. State this dependency in the implementation plan.

## Prior art

This design is a hybrid of:

- **Cloudflare Durable Objects SRS** ([blog](https://blog.cloudflare.com/sqlite-in-durable-objects/)) — bookmark concept (lex-sortable, timestamp-derived), retention semantics, snapshot-when-log >= db-size rule, "marked for deletion" GC model. CF does not expose forking publicly; we do, and we explicitly diverge by making `restore_to_bookmark` non-destructive (creates a new branch) instead of CF's destructive in-place rewrite. See [Divergences from CF DO](#divergences-from-cf-do) below.
- **Neon Postgres pageserver** — the layer file model (delta layers, image layers), branch-as-pointer (`ancestor_timeline_id`, `ancestor_lsn`), branch-as-restore, GC as a dependency graph (Neon issue #707 is the cautionary tale). Single most important architectural import.
- **Litestream v0.5** — LTX V3 file format (we already use it), `Pos{TXID, PostApplyChecksum}` flat positioning model, 4-tier wall-clock-aligned compaction (L0 raw / L1 30s / L2 5m / L3 1h).
- **LiteFS** — `<minTXID>-<maxTXID>.ltx` filename convention, `(TXID, PostApplyChecksum)` tuple as the position invariant, CRC-ISO-64 rolling checksum, HWM (high-water-mark) pattern for in-flight pending markers.
- **Turso bottomless** — generation `.dep` chain (parent-pointer chain), max 100-hop chain depth (we cap at 16; see [Fork chain bounds](#fork-chain-bounds) below), batched (~15s) granularity.

Detailed research reports under `.agent/research/{cf-durable-objects-sqlite,neon-postgres,litestream,litefs,turso-libsql}.md`. Companion analysis of the broader SQLite-on-remote-storage prior-art landscape (LiteFS, libSQL/Turso, mvSQLite, dqlite, Litestream VFS, sql.js-httpvfs, absurd-sql) is in `r2-prior-art/.agent/research/sqlite/prior-art.md`.

### Prior-art divergences

We explicitly **do not** import the following from prior art:

| Prior art | Mechanism we skip | Reason |
|---|---|---|
| mvSQLite PLCC (page-level OCC) | `PLCC_READ_SET_SIZE_THRESHOLD = 2000` page-version check at commit | Single-writer; no concurrent writer to conflict with |
| mvSQLite DLCC | Distributed lock-based concurrency control | Single-writer |
| mvSQLite MPC (multi-phase commit) | `COMMIT_MULTI_PHASE_THRESHOLD = 1000` 5-step commit ceremony | Single-writer; our commit is single-shot under FDB tx (stateless spec) |
| mvSQLite versionstamps | 80-bit FDB monotonic versionstamps as page version keys | We use per-branch `txid: u64` BE; pegboard exclusivity makes it monotonic |
| mvSQLite content-addressed dedup | `(page_number, version) -> page_hash` + `page_hash -> page_content` | Storage-cost concern only; defer until billing data shows it matters |
| LiteFS / libSQL local SQLite file | Real on-disk SQLite file behind FUSE / pluggable WAL | Hard constraint #2 ("no local files") |
| LiteFS multi-replica WAL stream | HTTP/2 stream + Heartbeat / Handoff frames | We rely on FDB durability; no peer replica protocol |
| CF DO 5-follower sync replication | 3-of-5 quorum-ack on every commit before app sees success | We rely on FDB; commit blocks on FDB write only |
| Turso `.dep` chain depth 100 | Generation-level COW via parent UUID chain | We cap at 16 (see [Fork chain bounds](#fork-chain-bounds)); deeper than 16 indicates misuse |

What we DO import (as a recap):

- mvSQLite's "read_many for batched page fetch" — but as Option F's `sqlite_read_many` on the hot path, not in this spec.
- Neon's layer model (delta + image), branch-as-LSN-pointer, dependency-graph GC.
- Litestream's `Pos{TXID, checksum}` flat positioning + 4-tier compaction ladder.
- LiteFS's `(TXID, PostApplyChecksum)` rolling-checksum invariant + HWM pending markers.
- CF DO's bookmark-as-time-token concept (with our own wire format).

## Conceptual model

### Branches

The unit of identity is a **branch**, not an actor. An actor's storage is "the chain of branches reachable from its current head by following parent pointers."

```rust
pub struct Branch {
    pub branch_id: Uuid,
    pub actor_id: String,
    pub parent: Option<BranchParent>,
    pub created_at_ms: i64,
    pub state: BranchState,
    /// Atomic-min counter: the smallest `parent_txid` of any live direct
    /// child branch. Updated by `fork()` (atomic-min) and `delete_branch()`
    /// (recompute by scanning live children — rare; tx-scoped).
    /// Used by GC to compute the retention pin in O(1) instead of walking
    /// the descendant tree.
    pub oldest_descendant_parent_txid: Option<u64>,
}

pub struct BranchParent {
    pub parent_branch_id: Uuid,
    pub parent_txid: u64,
    pub parent_checksum: u64, // CRC-ISO-64 at parent_txid
}

pub enum BranchState {
    /// Normal mutable branch. Pegboard can host an actor on it; commits append.
    Live,
    /// Branch is read-only (e.g. a sibling left after `restore_to_bookmark`).
    /// Retention is timestamp-based (`created_at_ms` floor), not head-relative.
    /// See "Garbage collection" for the predicate.
    Frozen,
    /// Branch will be deleted on the next cold compactor pass.
    /// Hot tier already cleared. Cold tier prefix delete is still pending.
    /// Reads transparently fall through to live siblings or fail with
    /// BranchNotReachable.
    Tombstoned { tombstoned_at_ms: i64 },
    /// Branch and its cold-tier prefix are both gone.
    Deleted { deleted_at_ms: i64 },
}
```

A new actor starts with one `Live` branch (the head). Forks create new `Live` branches with `parent` set. PITR also creates a `Live` branch at the target point; the *original* branch transitions to `Frozen` only if the broader system swaps the actor's head pointer onto the new branch (and elects to freeze the predecessor).

### Fork chain bounds

Reads on a fork descendant fall through to the parent at `parent_txid`. To bound worst-case read RTT, the chain depth from any branch back to a root is capped:

```rust
pub const MAX_FORK_DEPTH: u32 = 16;
```

`fork()` rejects with `ForkChainTooDeep` when the new branch would exceed this depth. Sixteen is enough for typical "fork → branch → fork → restore" sequences plus headroom; deeper trees indicate misuse (e.g. programmatic-loop forking) and the cold compactor can flatten them via image-layer materialization in a future pass.

The depth is precomputed and stored on the branch record so `fork()` is O(1).

### Bookmarks

Bookmark is a 33-character lexicographic-sortable string:

```
{timestamp_ms_hex_be:016}-{txid_hex_be:016}
```

Total: 16 + 1 + 16 = 33 chars. Example: `0000018f3a2c1234-0000000000004b1f`.

Branch identity is **not** in the wire format. A bookmark is interpreted relative to a branch context:
- `get_current_bookmark()` returns a bookmark on the actor's current head branch.
- `get_bookmark_for_time(t)` walks the actor's current head branch and its parent chain, returning a bookmark on whichever ancestor branch contains the commit closest to `t`.
- `resolve_bookmark(b, branch_ctx)` accepts an explicit branch context (default: actor's current head) and returns `Position { branch_id, txid, checksum }`.
- `restore_to_bookmark(b)` resolves the bookmark relative to the actor's current head, creates a new `Live` branch at the resolved position, returns its `branch_id`.

Lex order = chronological order **within a single branch's parent chain**. Across sibling branches (forks of the same parent), bookmarks are not orderable in any meaningful way; this is documented and the API surfaces don't support cross-branch comparison.

Bookmarks are sender-scoped: a bookmark resolved by an unauthorized actor on another actor's branch returns `BranchNotReachable` (verified at the engine edge per CLAUDE.md trust boundaries).

### Forking

`fork(point)` is metadata-only:

1. Open one FDB tx.
2. **Regular-read** the parent branch's `META/manifest.retention_pin_txid`. If `point.txid <= retention_pin_txid + GC_FORK_MARGIN_TXIDS`, abort with `ForkOutOfRetention`. Regular read takes a conflict range — concurrent GC pin advancement aborts this fork via OCC, not a vibes-based 1-minute margin.
3. Compute new branch's depth = `parent.depth + 1`. If `> MAX_FORK_DEPTH`, abort with `ForkChainTooDeep`.
4. Allocate `new_branch_id = Uuid::new_v4()`.
5. Write `[BRANCHES]/list/{new_branch_id}` with `parent = Some(BranchParent { parent_branch_id: point.branch_id, parent_txid: point.txid, parent_checksum: point.checksum })`.
6. Atomic-min the parent's `oldest_descendant_parent_txid` with `point.txid`.
7. Atomic-add the parent's refcount by +1.
8. Commit.

Steady-state cost: 1 regular read, 1 normal write, 2 atomic ops, all in one tx. No data copy.

`restore_to_bookmark(b)` is the same primitive: resolve the bookmark, then call `fork(Position)`. Returns the new branch_id. The actor's `[BRANCHES]/head` pointer is **not** updated by storage; the broader system (pegboard) does that swap if it wants destructive in-place restore semantics.

### Divergences from CF DO

Documented up front so callers know what's different:

| Aspect | CF DO | This spec | Why |
|---|---|---|---|
| Bookmark wire format | 4-segment 66-char (`...-...-...-<128-bit hash>`) | 2-segment 33-char | Branch context replaces the trailing hash; per-branch order is what matters |
| Restore destructive? | Yes (overwrites in place on next session) | No (creates a sibling branch; broader system swaps) | Fork is the same primitive; CF-style destructive restore is a layered API on top |
| Forking exposed | No public API | Yes | Differentiating feature |
| Restore granularity | Per-commit | Per-commit | Match |
| Retention default | 30 days | 30 days | Match |
| Lex-sort = chronological | Within a single DO's linear log | Within a single branch's parent chain | Sibling branches break global lex order; documented |
| `getBookmarkForTime` | Function of (timestamp) | Function of (timestamp, branch_ctx) | Branch context required |
| Old bookmarks survive a destructive restore | Yes (CF retains the pre-restore log) | Yes (the old branch becomes Frozen, not deleted) | Different mechanism, equivalent observable behavior |

A future `restore_to_bookmark_destructive` API can wrap fork + head-swap + freeze-then-tombstone-old-branch as a single operation if exact CF DO equivalence is needed.

## Architecture

```
┌─ pegboard-envoy (per WS conn) ───────────────────────────┐
│  scc::HashMap<actor_id, Arc<ActorDb>>                    │
│   ↓                                                       │
│  ActorDb (per actor):                                    │
│   ├─ resolved branch_id from [BRANCHES]/head              │
│   ├─ flattened parent index (BTreeMap, see "Read path")   │
│   ├─ rolling checksum cached on /META/head                │
│   └─ cold-tier client (S3 or fs or disabled)              │
└──────────────────────────────────────────────────────────┘
                          │
                ┌─────────┴──────────┐
                │                    │
                ▼                    ▼
┌─ FoundationDB (hot) ──┐  ┌─ S3 / Filesystem (cold) ─────┐
│ Per-branch:           │  │ Per-branch:                  │
│ /META/head            │  │ manifest.v1.vbare            │
│ /META/compact         │  │ layers/delta/{min}-{max}.ltx │
│ /META/cold_compact    │  │ layers/image/{sh}-{ts}.ltx   │
│ /META/quota           │  │ pending/{uuid}.marker        │
│ /META/manifest        │  │ bookmarks.v1.vbare           │
│ /META/compactor_lease │  │ state.v1.vbare               │
│ /META/cold_lease      │  └──────────────────────────────┘
│ /COMMITS/{txid}       │ (txid → wall_clock_ms)
│ /DELTA/{txid}/{chunk} │
│ /PIDX/delta/{pgno}    │
│ /SHARD/{shard_id}     │
└───────────────────────┘
                ▲                    ▲
                │                    │
        ┌───────┴────────┐  ┌────────┴───────┐
        │ Hot compactor  │  │ Cold compactor │
        │ UPS-driven on  │  │ UPS-driven on  │
        │ delta thresh   │  │ cold thresh    │
        └────────────────┘  └────────────────┘
```

Both compactors are UPS-driven; **neither has a periodic cron**. The cold trigger fires when the hot compactor's pass observes that drained-but-not-cold-drained delta count exceeds `cold_compact_delta_threshold` (default 1024 deltas — 32x the hot threshold).

## On-disk layout (FoundationDB hot tier)

Per-actor prefix `[0x02][actor_id]`. Two subspace bytes within:
- `[BRANCHES]` (e.g. `0x10`): branch metadata.
- `[BR]` (e.g. `0x11`): per-branch storage.

```
[0x02][actor_id][BRANCHES]/head                          → current head branch_id
[0x02][actor_id][BRANCHES]/list/{branch_id}              → Branch (vbare blob)
[0x02][actor_id][BRANCHES]/list/{branch_id}/refcount     → atomic i64 (FDB add)
[0x02][actor_id][BRANCHES]/list/count                    → atomic i64 — total live branch count
[0x02][actor_id][BR][branch_id]/META/head                → DBHead (head_txid, db_size_pages, post_apply_checksum)
[0x02][actor_id][BR][branch_id]/META/compact             → MetaCompact (materialized_txid)
[0x02][actor_id][BR][branch_id]/META/cold_compact        → MetaColdCompact (cold_drained_txid, last_image_txid)
[0x02][actor_id][BR][branch_id]/META/quota               → atomic i64 LE — hot-tier bytes
[0x02][actor_id][BR][branch_id]/META/manifest            → BranchManifest (parent, retention_pin_txid, depth, oldest_descendant_parent_txid)
[0x02][actor_id][BR][branch_id]/META/compactor_lease     → hot compactor lease
[0x02][actor_id][BR][branch_id]/META/cold_lease          → cold compactor lease
[0x02][actor_id][BR][branch_id]/COMMITS/{txid_be:8}      → wall_clock_ms (i64 LE — exact)
[0x02][actor_id][BR][branch_id]/DELTA/{txid_be:8}/{chunk_idx_be:4} → LTX bytes
[0x02][actor_id][BR][branch_id]/PIDX/delta/{pgno_be:4}   → owning txid (u64 BE)
[0x02][actor_id][BR][branch_id]/SHARD/{shard_id_be:4}    → LTX bytes
```

### Key changes from v1

- **Rolling checksum moves onto `/META/head`** as a `post_apply_checksum: u64` field. Was on `COMMITS/{T}` in v1; that costs an extra in-tx read on every commit. Now the checksum is colocated with `head_txid` and read once per commit (no extra get). Per-tx update: take old checksum from current `/META/head`, fold this commit's `(pgno, page_bytes)` updates via XOR, write new value back with the new `head_txid`. Same cost as updating `head_txid` itself.
- **`COMMITS/{T}` value reduces to a single `i64 LE`** (wall_clock_ms only). The checksum belongs on `/META/head` (as the rolling chain) and on cold layer trailers (per-layer). Hot path never reads `COMMITS/*` to compute a checksum.
- **`BranchManifest.oldest_descendant_parent_txid: Option<u64>`** added. Atomic-min from `fork()`, recomputed during `delete_branch()`. GC reads this scalar instead of walking children.
- **`BRANCHES/list/count`** atomic counter for `count_branches()` API + per-actor branch cap enforcement.

### CommitRecord (gone)

v1's `CommitRecord { wall_clock_ms, checksum }` is split:
- `wall_clock_ms` lives at `COMMITS/{T}` (exact, hot tier).
- `checksum` is the rolling `post_apply_checksum` on `/META/head` (latest only) plus on each cold layer's LTX trailer (per-layer).

Hot-path consequence: commit is back to one `META/head` read + one `META/head` write. The COMMITS write piggybacks on the same UDB tx as the META update. No new RTTs vs the stateless spec.

### BranchManifest

```rust
pub struct BranchManifest {
    pub branch_id: Uuid,
    pub parent: Option<BranchParent>,
    pub created_at_ms: i64,
    pub state: BranchState,
    pub depth: u32,
    /// Lower bound below which layers/data may be GC'd.
    /// Recomputed by GC each pass.
    pub retention_pin_txid: u64,
    /// Smallest `parent_txid` across all live direct children.
    /// `None` when no live children. Maintained as atomic-min on
    /// fork-create; recomputed by `delete_branch` (rare, scan-bound).
    pub oldest_descendant_parent_txid: Option<u64>,
}
```

## On-disk layout (S3 cold tier)

```
s3://{bucket}/actors/{actor_id}/branches/{branch_id}/
    manifest.v1.vbare                                # ColdManifest
    state.v1.vbare                                   # BranchColdState (cold_drained_txid mirror, cold_bytes)
    layers/
        delta/{min_txid_be:16}-{max_txid_be:16}.ltx  # NO checksum in filename
        image/{shard_id_be:8}-{as_of_txid_be:16}.ltx
    pending/{uuid_v4}.marker                         # in-flight upload markers (HWM pattern from LiteFS)
    bookmarks.v1.vbare                               # sparse BookmarkIndex
```

### Layer file naming

Layer filenames omit the checksum: `delta/{min_txid}-{max_txid}.ltx`. This is a deliberate change from v1 (where filenames included a content checksum) to make re-uploads idempotent at the S3 key level: if a pass crashes after upload but before manifest commit, the next pass overwrites the same key cleanly instead of producing a sibling orphan with a different checksum.

Per-layer checksum still exists in the LTX V3 trailer (`pre_apply_checksum`, `post_apply_checksum`) and in the `LayerEntry.checksum` manifest field.

To handle two passes producing different bytes for the same `(min_txid, max_txid)` (e.g. one pass got materialized_txid=120 and folded T=100..120; the other got materialized_txid=130): the second pass cannot happen, because cold pass uses regular-read on `META/cold_compact.cold_drained_txid` for OCC fencing (see [Cold compactor](#cold-compactor) below).

### Pending markers (HWM)

Before uploading a layer, the cold compactor writes a small marker object at `pending/{uuid}.marker` containing `{branch_id, min_txid, max_txid, started_at_ms, lease_holder}`. After successful manifest commit, the marker is deleted.

On lease takeover, the new pod lists `pending/*` and:
- Markers older than `STALE_MARKER_AGE_MS` (default 600s = 10 minutes): delete both the marker and the matching `layers/delta/{min_txid}-{max_txid}.ltx` if present (orphan from a crashed pass).
- Newer markers: leave alone (peer pod might still be working — but won't be, because we hold the lease now; defensive only).

### Schema versioning

Every persisted S3 object includes a `schema_version: u32` field via vbare. Migration rule (added to `engine/CLAUDE.md`):

> **Cold tier schema migrations.** When bumping `ColdManifest`/`BranchColdState`/`BookmarkIndex` schema version, the cold compactor reads the old version and writes the new version on every pass. The reader keeps prior-version code paths in tree for at least one full retention window (30 days) past the new version's rollout.

### ColdManifest

```rust
pub struct ColdManifest {
    pub schema_version: u32,             // = 1
    pub branch_id: Uuid,
    pub layers: Vec<LayerEntry>,
    pub last_modified_ms: i64,
}

pub struct LayerEntry {
    pub kind: LayerKind, // Delta | Image
    pub min_txid: u64,
    pub max_txid: u64,
    pub size_bytes: u64,
    pub pre_apply_checksum: u64,
    pub post_apply_checksum: u64,
    pub s3_key: String,                  // relative to branch root
    pub created_at_ms: i64,
    pub shard_range: Option<(u32, u32)>, // image only
}
```

Manifest is rewritten on every pass. For long-lived actors with many L0+L1+L2+L3 layers, the manifest can reach 50-100 KB.

**Future optimization (not v1):** append-only manifest segments + periodic compaction. v1 single-PUT works at <= 1M actor scale; revisit when scaling beyond.

### BookmarkIndex (cold)

Sparse index for `get_bookmark_for_time(t)` when `t` predates hot `COMMITS`. Entries written every N commits or every M seconds (whichever first; default N=64, M=10s) by the cold compactor.

```rust
pub struct BookmarkIndex {
    pub schema_version: u32, // = 1
    pub branch_id: Uuid,
    pub entries: Vec<BookmarkIndexEntry>,
}

pub struct BookmarkIndexEntry {
    pub wall_clock_ms: i64,
    pub txid: u64,
    pub post_apply_checksum: u64,
}
```

**Bookmark gap fallback.** For an actor offline N days then back online, hot `COMMITS/*` covers only the recent (post-online) window, and `BookmarkIndex` has no entries during the offline period because no cold pass ran. To resolve a bookmark in the gap:

1. Lookup hot `COMMITS/{T}` (recent only).
2. Lookup `BookmarkIndex` (sparse but covers all "active" history).
3. **Gap fallback:** binary-search layer file names for the wall-clock range. Each `delta/{min_txid}-{max_txid}.ltx` carries `created_at_ms` in `LayerEntry`. Find the layer covering `t`, GET its trailer (HTTP Range), extract the exact `(txid, wall_clock_ms)` pair from the LTX trailer. +1 S3 GET per lookup; bounded.

This means `get_bookmark_for_time` always succeeds within the retention window, regardless of whether the actor was active.

### BranchColdState

```rust
pub struct BranchColdState {
    pub schema_version: u32, // = 1
    /// Mirror of FDB's META/cold_compact.cold_drained_txid. Written by cold
    /// compactor after S3 layers are committed; read at lease takeover for
    /// orphan reconciliation.
    pub cold_drained_txid: u64,
    pub cold_bytes: u64,
    pub last_image_pass_at_ms: i64,
}
```

## Hot path

The hot path is **identical to the stateless spec** except keys are now branch-prefixed and the rolling checksum is maintained on `/META/head`. RTT counts are unchanged.

### `commit`

Reads:
- `[BR][branch_id]/META/head` → DBHead (head_txid, db_size_pages, post_apply_checksum).

Writes (in same UDB tx):
- DELTA chunks under new `txid = head_txid + 1`.
- PIDX upserts for dirty pgnos.
- `/META/head` updated with new (head_txid, db_size_pages, post_apply_checksum).
- `/COMMITS/{T}` = wall_clock_ms.
- `atomic_add(/META/quota, +delta_bytes)`.

The rolling checksum update folds this commit's `(pgno, page_bytes)` updates into the prior `post_apply_checksum` via CRC-ISO-64 XOR — same algorithm as LiteFS / Litestream `Pos{TXID, PostApplyChecksum}`. No prior-record read needed because the previous checksum is in the `/META/head` we just read.

Hot path RTT count: 1 (same as stateless spec).

### `get_pages` with parent fall-through

Rationale: a fork descendant reading pages it hasn't modified must walk up the parent chain. v1 specified "lookup parent's PIDX, fall through" without bounding the walk; that's O(chain_depth) per page read.

v2 uses **flattened ancestry**:

On `ActorDb` construction (first request), build a `FlattenedAncestry`:

```rust
pub struct FlattenedAncestry {
    /// One per ancestor in chain order, root first.
    pub ancestors: Vec<AncestorView>,
}

pub struct AncestorView {
    pub branch_id: Uuid,
    pub as_of_txid: u64,
    pub pidx_cache: parking_lot::Mutex<DeltaPageIndex>, // lazy
}
```

- `ancestors[0]` = oldest ancestor, capped at its own `parent_txid` (or full history if root).
- `ancestors[N-1]` = self, no cap.
- Resolved at construction by walking `[BRANCHES]/list/{branch_id}.parent` up to MAX_FORK_DEPTH.

Read algorithm for a single pgno:
1. For each ancestor from N-1 (self) to 0 (root):
   - Lookup `[BR][ancestor.branch_id]/PIDX/delta/{pgno}` → if hit AND `txid <= ancestor.as_of_txid`, fetch DELTA, return.
   - Lookup `[BR][ancestor.branch_id]/SHARD/{pgno / 64}` → if hit AND covers pgno as-of `as_of_txid`, return.
2. Else fall through to cold tier (see below).

PIDX caches are filled lazily; first access on each ancestor triggers one prefix scan + cache population. Subsequent reads on the same WS conn are RAM-only.

**Worst case:** 16 ancestors × 2 in-tx gets per pgno = 32 reads per pgno on cold cache. Mitigated by:
- Most pages don't cross more than 1-2 fork boundaries in practice.
- Cache amortizes after first access (one cold scan per ancestor per WS conn lifetime).
- `MAX_FORK_DEPTH = 16` caps the worst case.

For chains deeper than 2-3 forks, the cold compactor materializes image layers in descendant branches via [fork warmup](#fork-warmup) below to flatten read cost.

### Fork warmup

When `fork()` runs, it enqueues a UPS message to the cold compactor: `WarmupRequest { actor_id, new_branch_id, parent, parent_txid }`. The cold compactor's pass for this actor will:

1. Read all image layers in the parent's cold manifest that cover txids <= parent_txid.
2. Copy their byte ranges (snapshot reads from cold) into the new branch's cold tier under fresh image-layer keys, all at `as_of_txid = parent_txid`.

After warmup, the new branch's cold tier holds a full snapshot at `parent_txid`; subsequent reads on the new branch never need to walk past `ancestors[N-1]` (self) for pages with no local writes — they hit self's cold-image-from-parent and return without touching the parent's tier.

Warmup is a background job; the new branch is usable immediately, but reads during the warmup window pay parent fall-through cost. SLA target: warmup completes within `cold_pass_interval_secs / 2`.

## Cold compactor

Standalone service, registered same shape as hot compactor:

```rust
Service::new(
    "sqlite_cold_compactor",
    ServiceKind::Standalone,
    |config, pools| Box::pin(sqlite_storage::cold_compactor::start(config, pools, ColdCompactorConfig::default())),
    true,
)
```

### Trigger model (UPS-only, no cron)

- **Drain trigger:** the *hot* compactor publishes `SqliteColdCompactSubject { actor_id, branch_id }` whenever its pass advances `materialized_txid` past `cold_compact_delta_threshold` deltas beyond `cold_drained_txid` (default threshold: 1024 deltas).
- **Warmup trigger:** `fork()` publishes a warmup payload directly.
- **GC trigger:** the cold compactor self-publishes a GC payload at the end of each successful drain pass for the same branch (GC always rides on a drain pass).
- **No periodic cron.** Adding a cron creates a thundering-herd risk at top-of-hour and a leader-election problem; the UPS-driven model relies on commit activity to drive everything. Idle actors do not need cold passes (their hot tier is stable).

Per-actor throttle: 60s window, 2-hour safety net (matches v1 numbers; review didn't push back on these).

### Lease

Separate UDB key (`META/cold_lease`). Same TTL/renewal lifecycle as hot compactor. Hot and cold leases are independent.

### Pass procedure (Phase A / B / C)

**Phase A: FDB read tx (target <2s)**

1. Acquire `META/cold_lease`.
2. List `pending/*` markers from S3. For each marker older than `STALE_MARKER_AGE_MS`: delete its associated layer file + marker. Reconciles orphans from prior crashed passes.
3. Snapshot-read `META/head.head_txid`, `META/compact.materialized_txid`, `META/cold_compact.cold_drained_txid`.
4. Compute `drain_window = (cold_drained_txid + 1) ..= min(materialized_txid, cold_drained_txid + DRAIN_BATCH_LIMIT)`. Bound the window by:
   - **Txid count**: at most `DRAIN_BATCH_LIMIT_TXIDS` (default 1024).
   - **Byte count**: at most `DRAIN_BATCH_LIMIT_BYTES` (default 64 MB) — sum of DELTA chunk sizes.
   The byte bound is the operative one for high-page-count commits.
5. Snapshot-read all DELTA chunks in `drain_window` into process memory. Read existing image-layer plans (which shards are stale).
6. Phase A tx commits cleanly (pure read tx; trivially within tx-age).

**Phase B: S3-only (no FDB tx)**

7. For each pending warmup request: fetch parent layers, write into self's cold prefix (parallel S3 ops).
8. Encode the drain window into one or more LTX V3 layer files (split if `DRAIN_BATCH_LIMIT_BYTES` exceeded).
9. For each layer to upload:
   - Write `pending/{uuid}.marker` with payload.
   - PUT layer to `layers/delta/{min_txid}-{max_txid}.ltx`. Single-PUT for <= 16 MB; multipart for larger.
   - HEAD the uploaded object to verify ETag/size.
10. Maybe rebuild image layers per the "log >= db size" rule: compute affected shards, snapshot-read SHARD blobs from FDB (no tx — quick out-of-tx read is fine), encode + upload as image layers.
11. Update `BookmarkIndex` (rewrite) and `BranchColdState` (rewrite) to S3.
12. PUT new `manifest.v1.vbare`. **Manifest is the commit point** — it is the index that names every layer.

**Phase C: FDB write tx (target <3s)**

13. **Regular-read** `META/cold_compact.cold_drained_txid`. Assert it equals the value seen in Phase A. If not, abort: another pod (lease was lost and reacquired by a different pod between Phase A and Phase C). Phase B's S3 work is now orphaned; the pending markers will be reconciled on the next pass.
14. Single FDB tx writes:
    - `META/cold_compact.cold_drained_txid = max_txid` (regular write, conflicts on concurrent racers via prior step 13).
    - Clear `DELTA/{T}/*` for T in drain_window.
    - Clear `COMMITS/{T}` for T < retention_pin_txid (read first, see GC below).
15. Delete `pending/{uuid}.marker` objects in S3 (out of tx, post-commit).
16. Run [GC](#garbage-collection) (Phase C-2).
17. Release lease.

**Tx-age budget:**
- Phase A: read-only, snapshot reads, FDB tx is short.
- Phase B: no FDB tx; only S3 work. Lease-renewal task ticks every 10s independently.
- Phase C: write tx, ~3s budget for clears (1024 DELTA chunks × ~5 KV ops each = ~5k operations; well within FDB write tx limits).

If Phase C exceeds 5s tx age, split into multiple write txs (clear in batches of 256 DELTAs). The Phase A→C OCC fence on `cold_drained_txid` makes this safe — partial Phase C still advances the cursor only at the end.

### `cold_drained_txid` monotonicity

To prevent a bug zeroing the cursor (review ops #10), Phase C does:

- Regular-read prior `cold_drained_txid` (already done in step 13).
- Write new value. FDB OCC ensures any concurrent write fails.
- Debug assertion: `new > prior`. Panic in tests; log + skip pass in release.

### Compaction levels

Following Litestream:
- **L0**: raw drain windows (32-1024 commits or up to 64 MB each).
- **L1**: 30s wall-clock buckets.
- **L2**: 5min buckets.
- **L3**: 1hr buckets.

Each pass that ends Phase B writes one or more L0 layers. After every Nth L0 (or every M seconds, whichever first), the same pass extends Phase B to also produce L1 by merging recent contiguous L0s. Same for L2/L3 at lower frequencies. Each level merge is a self-contained read-N-layers, write-1-layer, update-manifest sequence.

### Garbage collection

GC runs as Phase C-2, holding the cold lease.

For a single branch:

1. Compute `retention_pin_txid`:
   - `time_floor_txid` = txid corresponding to `(now_ms - PITR_WINDOW_MS)`. For `Live` branches: lookup via `BookmarkIndex`. For `Frozen` branches: clamped at `created_at_ms - PITR_WINDOW_MS` lookup (see below).
   - `descendant_floor_txid` = `oldest_descendant_parent_txid` (read once, atomic-min counter).
   - `pin = min(time_floor_txid, descendant_floor_txid.unwrap_or(u64::MAX))`.
   - **No monotonic ratchet.** v1 had a "never advance past prior pin" rule that made delete_branch ineffective; v2 drops it. The pin recomputes fully each pass.
2. Update `META/manifest.retention_pin_txid = pin`.
3. Walk `ColdManifest.layers`. Layers with `max_txid < pin` are GC-eligible.
4. Delete eligible S3 objects via batch `DeleteObjects` (1k per request); rewrite manifest without those entries; update `BranchColdState.cold_bytes`.
5. Walk and clear `COMMITS/{T}` for T < pin.

#### Frozen-branch retention

A `Frozen` branch (e.g. the predecessor of a `restore_to_bookmark` swap) has a stable `head_txid`. Its retention is **not** `head_txid - window` (that drags forward over time). Instead:

```rust
fn frozen_pin(branch: &Branch, now_ms: i64) -> u64 {
    // Frozen branch's head_txid is fixed; window is wall-clock-relative
    // to when it was frozen (or created), not to "now".
    // If created/frozen >= 30 days ago AND no live descendants → all txids
    // are GC-eligible (branch transitions to Tombstoned).
    if now_ms - branch.created_at_ms > PITR_WINDOW_MS {
        u64::MAX // delete everything
    } else {
        0 // keep everything
    }
}
```

Frozen branches with live descendants keep the descendant-pinned txids and nothing else, regardless of wall clock. This fixes review arch #2.

#### Branch deletion cascade

`delete_branch(branch_id)`:

1. Validate: branch must have refcount = 0 (no live children). Return `BranchPinned` if not.
2. Transition state to `Tombstoned`. Decrement parent's refcount (atomic).
3. **Clear hot tier prefix synchronously** (`[BR][branch_id]/*`). Bounded by branch size; runs in the same tx as state transition.
4. Schedule cold delete: cold compactor's next pass for the parent picks up the tombstone, walks the descendant's S3 prefix, deletes layers in batches.
5. After cold prefix is cleared, transition state to `Deleted`. Final state.

Concurrent `fork(C)` while `delete_branch(C)` is in flight:
- `delete_branch` reads C's `state` with regular read inside its tx.
- `fork` reads parent's `state` with regular read inside its tx.
- FDB OCC: whichever commits first wins. The loser sees an updated state and retries or fails (`fork` fails with `BranchNotReachable`).

#### Tombstoned actors

When pegboard destroys an entire actor:

1. Actor's destroy tx writes a per-actor `[BRANCHES]/tombstone` marker.
2. Cold compactor's next pass for any branch in this actor sees the marker, recursively tombstones every branch, schedules cold prefix delete.
3. `actors/{actor_id}/branches/*` cleanup runs as a workflow (gasoline job) so it survives pod restarts and bounded retries on S3 errors. The workflow batch-deletes via `DeleteObjects`.

This avoids the v1 problem where actor-destroy inline-deleted 70k+ S3 objects.

## Concurrency model

### Hot vs cold compactor

**Disjoint META sub-keys:**
- Hot writes `META/head`, `META/compact`, `PIDX/*`, `DELTA/*` (writes), `SHARD/*`, `COMMITS/*` (writes).
- Cold writes `META/cold_compact`, `DELTA/*` (clears only), `COMMITS/*` (clears only — for txids < retention_pin).

**Quota counter is atomic-add.** Both compactors do `atomic_add(/META/quota, ...)`. Atomic ops compose; no conflict range.

**Explicit invariant:** the hot compactor MUST NOT modify `DELTA/{T}` for any T at-or-below the highest-ever-written `materialized_txid`. Once folded, deltas are read-only until cold drain clears them. Debug assert this in commit and compaction.

**Cold's regular read of `materialized_txid`** in Phase A is a regular read (NOT snapshot), so a concurrent hot pass advancing `materialized_txid` between cold's plan and cold's Phase C aborts cold via OCC. Cold retries on the next trigger.

### Fork during pass

Fork writes one new `BRANCHES/list/{new_branch_id}` + atomic-add refcount + atomic-min `oldest_descendant_parent_txid`. None of these conflict with hot or cold compactor work on the same branch. Fork's regular-read of `META/manifest.retention_pin_txid` is the OCC fence against GC.

### Refcount race

Concurrent `fork(at C)` + `delete_branch(C)`:
- Both read C's state with regular read.
- FDB OCC serializes; loser fails.
- After serialization, `fork` either sees C's state == Live (succeeds) or != Live (fails with `BranchNotReachable`).

## API surface

```rust
impl ActorDb {
    // Hot path:
    pub async fn get_pages(&self, pgnos: Vec<u32>) -> Result<Vec<FetchedPage>>;
    pub async fn commit(&self, dirty_pages: Vec<DirtyPage>, db_size_pages: u32, now_ms: i64) -> Result<()>;

    // Bookmarks:
    pub async fn get_current_bookmark(&self) -> Result<Bookmark>;
    pub async fn get_bookmark_for_time(&self, t_ms: i64) -> Result<Bookmark>;
    pub async fn resolve_bookmark(&self, b: &Bookmark) -> Result<Position>;
    pub async fn resolve_bookmark_in_branch(&self, b: &Bookmark, branch_id: Uuid) -> Result<Position>;

    // Branching:
    pub async fn fork(&self, fork_point: ForkPoint) -> Result<Uuid>;
    pub async fn restore_to_bookmark(&self, b: &Bookmark) -> Result<Uuid>;

    // Branch admin:
    pub async fn list_branches(&self, cursor: Option<Cursor>, limit: u32) -> Result<(Vec<Branch>, Option<Cursor>)>;
    pub async fn count_branches(&self) -> Result<u64>;
    pub async fn delete_branch(&self, branch_id: Uuid) -> Result<()>;
    pub async fn swap_head(&self, new_head: Uuid) -> Result<()>;

    // Operator-only debug:
    #[cfg(any(debug_assertions, feature = "operator"))]
    pub async fn debug_describe_bookmark(&self, b: &Bookmark) -> Result<BookmarkResolutionTrail>;
}

pub struct BookmarkResolutionTrail {
    pub bookmark: Bookmark,
    pub resolved_position: Position,
    pub branch_chain_walked: Vec<Uuid>,
    pub source_layer_keys: Vec<String>,
    pub computed_checksum: u64,
    pub stored_checksum: u64,
}
```

### Pagination

`list_branches(cursor, limit)` paginates with a cursor for actors with many branches. Default per-actor branch cap: `MAX_BRANCHES_PER_ACTOR = 1024` (enforced at `fork()`).

## Hot-path latency analysis

| Op | RTT count | Change vs stateless spec |
|---|---|---|
| `get_pages` (warm cache, head branch) | 1 | 0 |
| `commit` (steady state) | 1 | 0 (rolling checksum on `/META/head`, no extra read) |
| `get_pages` (warm cache, fork descendant, page touched in branch) | 1 | 0 |
| `get_pages` (warm cache, fork descendant, page NOT touched in branch) | 1 | +0 (parent PIDX in flattened cache, RAM-only) |
| `get_pages` (cold WS conn, fork descendant, page NOT touched in branch) | 2-N | +1 per uncached ancestor, capped at MAX_FORK_DEPTH=16 |
| `get_pages` (historical, txid < cold_drained_txid) | 2-3 | +1-2 (cold-layer fetch from S3) |
| `get_pages` (historical with fork warmup) | 1-2 | -N vs without warmup (image layer pre-staged) |
| `get_current_bookmark` | 1 | 1 fresh op |
| `get_bookmark_for_time(t)` (recent, hot COMMITS) | 1 | 1 fresh op |
| `get_bookmark_for_time(t)` (older, cold BookmarkIndex) | 2 (manifest + index GET) | rare path |
| `get_bookmark_for_time(t)` (gap fallback) | 3 (manifest + layer trailer GET) | rare path |
| `fork` | 2-3 (regular read + Branch write + atomic-add + atomic-min) | metadata only |
| `restore_to_bookmark` | same as fork | same |

The COMMITS write piggybacks on the same UDB tx as the META update — no extra RTT.

## Quota and metering

### Hot-tier quota

`META/quota`: atomic counter, FDB-side, per branch.

Cap: `SQLITE_HOT_MAX_BYTES = 10 GiB` per branch. Exceeded → `SqliteStorageQuotaExceeded`.

### Burst mode (S3 outage handling)

When the cold compactor cannot drain (S3 5xx ratio exceeds threshold), the hot tier accumulates DELTAs that should have been drained. v1 wedged commits at the 10 GiB cap; v2 has burst mode:

```rust
pub const HOT_BURST_MULTIPLIER: i64 = 2; // → 20 GiB during S3 outage
pub const COLD_DEGRADED_THRESHOLD_5XX_RATIO: f64 = 0.5;
pub const COLD_DEGRADED_WINDOW_PASSES: u32 = 3;
```

- Cold compactor tracks per-pod S3 5xx ratio over the last 3 passes.
- When ratio exceeds threshold, flip a per-pod gauge `sqlite_cold_tier_degraded_pods`.
- Hot tier observes the gauge (via UPS broadcast or polling); when set, raise the quota cap to `SQLITE_HOT_MAX_BYTES * HOT_BURST_MULTIPLIER`.
- After cold tier recovers (5xx ratio normal for 1 full pass), revert the cap.
- During burst mode, commits keep flowing; ops sees the gauge and is alerted.

User-facing: documented as "during S3 outages, the per-branch hot quota is temporarily raised to 20 GiB; commits are not blocked unless the burst cap is also exceeded."

### Cold-tier accounting

`BranchColdState.cold_bytes` updated by cold compactor on every pass. **Periodic reconciliation:** a daily job lists `branches/{branch_id}/layers/`, sums `Content-Length`, overwrites `cold_bytes` with ground truth. Emits `sqlite_cold_bytes_reconciliation_drift` for ops visibility.

### Billing for forked branches

v1 billed total `cold_bytes` per branch — double-charges shared parent pages, triple-charges grandchildren. v2 fixes this:

- A branch's billable cold bytes = sum of byte sizes of layer files **owned by this branch** (i.e. created by this branch's cold passes, not pinned-by-this-branch from parent).
- Fork descendants start at 0 billable cold bytes; only diverged commits accrue.
- Fork warmup layers count toward the descendant (warmup bytes are "owned" by the descendant in v2, since it copied them).

This matches Neon's billing model. Documented in `MetricKey::SqliteColdBytes { actor_name, branch_id }`.

## Authorization model

Per CLAUDE.md trust boundaries: `client <-> engine` is untrusted.

Bookmark + branch authorization:
- `get_current_bookmark`, `commit`, `get_pages` operate on the actor's currently-bound branch (whatever pegboard told this conn).
- `resolve_bookmark`, `restore_to_bookmark`, `fork(at_bookmark)`: caller must be authorized for the target actor; the storage library trusts that pegboard-envoy already validated this. The storage library does NOT enforce cross-actor isolation — that's at the engine edge.
- `resolve_bookmark_in_branch(b, branch_id)`: caller must own `branch_id` (verified by checking `actor_id` association). Reject `BranchNotReachable` otherwise.

Within an actor, all branches are accessible to that actor's authorized callers — siblings included. There is no per-branch ACL in v1.

## Observability

Metrics added by this spec (`node_id` labels everywhere, per stateless spec):

### Cold compactor

- `sqlite_cold_compactor_pass_duration_seconds{outcome}` — histogram.
- `sqlite_cold_compactor_pass_failures_total{stage=upload|manifest|fdb_commit|gc}` — counter.
- `sqlite_cold_compactor_lease_take_total{outcome=acquired|skipped|conflict}` — counter.
- `sqlite_cold_compactor_lease_held_seconds` — histogram.
- `sqlite_cold_compactor_lease_renewal_total{outcome=ok|stolen|err}` — counter.
- `sqlite_cold_compactor_lease_steals_total{branch_id}` — counter.
- `sqlite_cold_drain_lag_txids{actor_id_bucket}` — gauge: `head_txid - cold_drained_txid`.
- `sqlite_cold_drain_lag_seconds{actor_id_bucket}` — gauge: wall-clock equivalent via BookmarkIndex.
- `sqlite_cold_layers_uploaded_total{level=L0|L1|L2|L3|image}` — counter.
- `sqlite_cold_layer_bytes_uploaded_total{level}` — counter.
- `sqlite_cold_layer_orphan_count{branch_id}` — gauge: pending markers > stale age.
- `sqlite_cold_tier_degraded_pods` — gauge.
- `sqlite_cold_bytes_reconciliation_drift` — counter.

### S3

- `sqlite_s3_request_duration_seconds{op,outcome}` — histogram.
- `sqlite_s3_inflight_multipart_uploads` — gauge.
- `sqlite_s3_5xx_ratio` — derived rolling.

### Branches

- `sqlite_branch_count_per_actor` — histogram (sampled).
- `sqlite_branch_count_total{state}` — gauge.
- `sqlite_branch_gc_eligible_count{state=Tombstoned}` — gauge.
- `sqlite_fork_total{outcome=ok|chain_too_deep|out_of_retention|branch_unreachable}` — counter.

### PITR resolution

- `sqlite_pitr_resolve_duration_seconds{path=hot|cold|gap_fallback}` — histogram.
- `sqlite_bookmark_resolve_failures_total{reason=expired|branch_unreachable|gap}` — counter.

### Debug-only

- `sqlite_refcount_drift_total` — counter.
- `sqlite_cold_drained_txid_regressed_total` — counter (panic in debug, log in release).

## Failure modes

| Failure | Detection | Recovery |
|---|---|---|
| S3 down for 1h | `cold_drain_lag_txids` climbing | Burst-mode hot quota engages at 2x cap; ops alerted via gauge; cold compactor backs off; commits keep flowing |
| S3 slow (p99 = 5s) | `cold_compactor_pass_duration_seconds` p99 | Phase A/B/C structure isolates FDB tx-age from S3 latency; lease-renewal task ticks during Phase B |
| Cold pod loses lease mid-pass | New pod takes lease | Stale `pending/*` markers + matching layer files cleaned at start of next pass |
| Multipart PUT incomplete | S3 lifecycle policy `AbortIncompleteMultipartUpload` after 1d (deployment requirement) | Auto-aborted; metric gauge tracks in-flight |
| Manifest PUT fails after layer upload | Pending marker persists past stale-age | Next pass cleans the orphan layer |
| FDB Phase C fails after S3 manifest commit | OCC abort on Phase C; Phase B's S3 work is "ahead" | Next pass re-reads `cold_drained_txid` (still old), redrains the same window — manifest already references those layers, so layer keys collide on the same `(min_txid, max_txid)` and overwrite cleanly |
| `cold_drained_txid` zeroed by bug | Phase C debug assert | Panic in tests; in release, the regular-read fence fails OCC and the pass aborts |
| Bookmark older than 30d window | `resolve_bookmark` returns `BookmarkExpired` | Caller's responsibility |
| Bookmark in offline-actor gap | `BookmarkIndex` has no entry | Layer-file binary search via trailer GET (gap fallback) |
| Fork at GC'd bookmark | OCC abort on regular-read of parent's `retention_pin_txid` | Retry rejects with `ForkOutOfRetention` |
| Fork chain too deep | Pre-tx depth check | `ForkChainTooDeep` |
| Concurrent fork + delete | OCC on `state` regular reads | Loser fails; `fork` returns `BranchNotReachable` if loser |
| Parent branch deleted while child alive | `delete_branch` rejects with refcount > 0 | `BranchPinned` |
| Actor delete with 70k+ S3 objects | Workflow-level delete (gasoline job) | Tombstone marker in FDB + async batch DeleteObjects |
| Frozen sibling branch's history GC'd despite descendant pin | Descendant pin (`oldest_descendant_parent_txid`) keeps it pinned | Pin recomputes per pass; no monotonic ratchet |

## Cold-tier client abstraction

For sandbox/dev/test, abstract the cold tier behind a trait:

```rust
pub trait ColdTierClient: Send + Sync {
    async fn put(&self, key: &str, body: Bytes) -> Result<()>;
    async fn put_multipart(&self, key: &str, body: impl Stream<Item = Bytes>) -> Result<()>;
    async fn get(&self, key: &str, range: Option<RangeInclusive<u64>>) -> Result<Bytes>;
    async fn head(&self, key: &str) -> Result<HeadInfo>;
    async fn list(&self, prefix: &str) -> Result<Vec<ListEntry>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn delete_batch(&self, keys: &[String]) -> Result<()>;
}

pub enum ColdTier {
    /// No cold tier. PITR returns `BookmarkExpired` for any txid older than
    /// what hot can serve. Forks work, GC runs on hot only. Default for
    /// `pnpm start` local dev — no minio needed.
    Disabled,
    /// Local filesystem pretending to be S3. ~200 LoC over `tokio::fs`.
    /// Matches the `MemoryStore` pattern from existing fault injection.
    Filesystem(PathBuf),
    /// Production S3.
    S3(S3Config),
    /// Test fault-injection over any underlying client.
    Faulty(Box<dyn ColdTierClient>, FaultInjectionPolicy),
}
```

`ColdTier::Disabled`: cold compactor service is registered but always no-ops (config check at start). Hot tier works; PITR is bounded by hot retention only.

`ColdTier::Filesystem`: integration tests use this. Full feature parity at S3 protocol level except no multipart semantics.

`ColdTier::S3`: production. Backed by `aws-sdk-s3`.

`ColdTier::Faulty`: testing only. Wraps any client; injects timeouts, 5xx, partial writes, slow responses.

## Implementation strategy

### Stage 0: prerequisite

Stateless spec (`sqlite-storage-stateless.md`) is shipped. All work in this spec extends it.

### Stage 1: branch primitive in hot tier

- Add `[BRANCHES]` and `[BR]` subspace prefixes.
- Add `Branch`, `BranchManifest`, `BranchParent`, `BranchState` types in `pump/types.rs`.
- Add `oldest_descendant_parent_txid` atomic-min counter helpers.
- Add `MAX_FORK_DEPTH` enforcement to fork().
- Migrate hot-path key builders to take `branch_id`.
- Wire `ActorDb::new` to resolve `[BRANCHES]/head`.
- Add `COMMITS/{T}` write to commit path.
- Move rolling checksum onto `/META/head`.
- Tests: `tests/branch_keys.rs`, `tests/branch_commit.rs`.

### Stage 2: bookmarks

- Implement 33-char `Bookmark` type (parse/format/round-trip).
- Implement `get_current_bookmark`, `get_bookmark_for_time`, `resolve_bookmark`, `resolve_bookmark_in_branch`.
- Hot `COMMITS/*` range scan helpers.
- Tests: `tests/bookmarks.rs` covering recent, gap, expired, cross-branch failure modes.

### Stage 3: forking

- Implement `fork(ForkPoint)` with OCC on retention_pin_txid.
- Implement `restore_to_bookmark`, `delete_branch` cascade, `swap_head`.
- Add flattened parent ancestry to read path.
- Tests: `tests/fork.rs` covering: chain depth, OCC abort, cascade delete, refcount race.

### Stage 4: cold-tier client abstraction

- Define `ColdTierClient` trait.
- Implement `ColdTier::{Disabled, Filesystem, S3, Faulty}`.
- Wire into `ColdCompactorConfig`.
- Tests: smoke-test each impl with the same trait test suite.

### Stage 5: cold compactor

- New module `cold_compactor/`.
- Layer file builder (lifted from pump/ltx.rs).
- Manifest read/write, BookmarkIndex, BranchColdState (all vbare with schema_version).
- Lease helpers (lifted from hot compactor's lease.rs).
- Pending marker (HWM) helpers.
- Phase A/B/C pass orchestration.
- Tests: `tests/cold_compactor.rs` using `ColdTier::Filesystem` + `tokio::time::pause`.

### Stage 6: GC

- Refcount tracking on fork/delete.
- Pin computation with frozen-branch carve-out.
- S3 prefix walk + DeleteObjects batches.
- Hot COMMITS GC.
- Tests: `tests/gc.rs` covering: descendant pin extension/retreat, frozen-branch retention, tombstone cascade.

### Stage 7: cold-path read with fork warmup

- Cold-tier layer fetch on miss.
- Fork warmup UPS publish + cold compactor handler.
- Tests: `tests/cold_read.rs`, `tests/fork_warmup.rs`.

### Stage 8: burst mode + observability

- Hot quota burst mode wired to S3-degraded gauge.
- All new metrics enumerated above.
- `debug_describe_bookmark` API.
- Daily reconciliation job for `cold_bytes`.
- Tests: failure-injection via `ColdTier::Faulty`.

### Stage 9: integration tests

- End-to-end: commit, fork, commit on child, restore-to-bookmark, verify state.
- "S3 unreachable for 1h" with synthetic clock advance — assert burst mode.
- "Lease lost mid-pass" — assert orphan reconciliation.
- "Manifest write fails after layer upload" — assert orphan sweep.
- "FDB tx-age 5s + S3 p99 5s" — assert Phase A/B/C does not blow tx age.
- "30-day rollover" — synthetic clock + assert layers expire.

## Open questions

- **Image layer freshness vs commit storm.** Verified: the SRS "log >= db size" rule is the right threshold. Implementation detail in Stage 5.
- **Append-only manifest segments.** Future optimization; v1 single-PUT works at < 1M actor scale.
- **`cold_pass_interval_secs` removal.** v1 had this; v2 drops the cron entirely. Confirmed in [Trigger model](#trigger-model-ups-only-no-cron).
- **Bookmark wire format compatibility with CF DO.** v2 explicitly diverges (33 chars vs 66) and documents the divergence. CF DO interop is not a v1 requirement.
- **MAX_FORK_DEPTH = 16.** Reviewed; fits Neon-style typical use (1-3 levels) with headroom. Revisit if customer use cases push deeper.

## Future work

- Multi-region cold tier with replicated manifests.
- Append-only manifest segments at >= 1M actor scale.
- Cold-tier read-through cache layer in front of S3 (per-pod LRU).
- Fork warmup priority queue (warm before background L1+ compaction).
- `restore_to_bookmark_destructive` API (fork + head-swap + freeze-then-tombstone-old in one op).
- Cross-actor branch references (not in v1; would need separate trust model).
- Layer-level zstd compression as a flag bit.

## Files affected

### New modules

- `engine/packages/sqlite-storage/src/pump/branches.rs` — branch CRUD on hot tier.
- `engine/packages/sqlite-storage/src/pump/bookmark.rs` — `Bookmark` parse/format/resolve.
- `engine/packages/sqlite-storage/src/pump/checksum.rs` — CRC-ISO-64 rolling checksum.
- `engine/packages/sqlite-storage/src/pump/ancestry.rs` — flattened parent ancestry resolver + cache.
- `engine/packages/sqlite-storage/src/cold_compactor/mod.rs` — service entry.
- `engine/packages/sqlite-storage/src/cold_compactor/worker.rs` — UPS subscriber + Phase A/B/C orchestrator.
- `engine/packages/sqlite-storage/src/cold_compactor/layer.rs` — LTX V3 layer builder.
- `engine/packages/sqlite-storage/src/cold_compactor/manifest.rs` — `ColdManifest` IO.
- `engine/packages/sqlite-storage/src/cold_compactor/bookmark_index.rs` — sparse index IO.
- `engine/packages/sqlite-storage/src/cold_compactor/state.rs` — `BranchColdState` IO.
- `engine/packages/sqlite-storage/src/cold_compactor/pending.rs` — HWM marker helpers.
- `engine/packages/sqlite-storage/src/cold_compactor/gc.rs` — pin computation + DeleteObjects batches.
- `engine/packages/sqlite-storage/src/cold_compactor/warmup.rs` — fork-warmup handler.
- `engine/packages/sqlite-storage/src/cold_tier/mod.rs` — `ColdTierClient` trait + variants.
- `engine/packages/sqlite-storage/src/cold_tier/s3.rs` — AWS SDK impl.
- `engine/packages/sqlite-storage/src/cold_tier/filesystem.rs` — local-disk impl.
- `engine/packages/sqlite-storage/src/cold_tier/faulty.rs` — fault injection wrapper (test only).
- `engine/packages/sqlite-storage/src/burst_mode.rs` — hot quota burst-mode coordination.

### Modified

- `engine/packages/sqlite-storage/src/pump/keys.rs` — branch-id-prefixed builders.
- `engine/packages/sqlite-storage/src/pump/actor_db.rs` — branch resolution on first request, ancestry cache, cold-tier client handle.
- `engine/packages/sqlite-storage/src/pump/commit.rs` — COMMITS write, rolling checksum on /META/head, per-branch keys.
- `engine/packages/sqlite-storage/src/pump/read.rs` — flattened parent fall-through, cold-tier fetch on miss.
- `engine/packages/sqlite-storage/src/pump/quota.rs` — burst-mode cap.
- `engine/packages/sqlite-storage/src/pump/metrics.rs` — full cold metrics suite.
- `engine/packages/sqlite-storage/src/pump/types.rs` — `Branch`, `BranchManifest`, etc.
- `engine/packages/engine/src/run_config.rs` — register `sqlite_cold_compactor`.
- `engine/packages/pegboard/src/namespace/keys/metric.rs` — add `SqliteColdBytes { actor_name, branch_id }`, `SqliteBranchCount { actor_name }`.
- `engine/CLAUDE.md` — add cold-tier schema migration rule.

### Tests

- `tests/branch_keys.rs`, `tests/branch_commit.rs`, `tests/bookmarks.rs`, `tests/fork.rs`, `tests/cold_compactor.rs`, `tests/cold_read.rs`, `tests/fork_warmup.rs`, `tests/gc.rs`, `tests/burst_mode.rs`, `tests/integration_pitr_fork.rs`.

### Deployment requirements

- S3 bucket lifecycle policy: `AbortIncompleteMultipartUpload` after 1 day.
- S3 bucket access pattern: per-actor IAM scoping (out of scope for this spec; broader system).
- IAM policy must allow `s3:DeleteObjects` (batch) for the cold compactor service principal.
