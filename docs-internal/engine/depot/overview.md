# Depot SQLite Overview

High-level map of how Rivet stores, reads, compacts, forks, and time-travels per-actor
SQLite databases. **UniversalDB ("UDB")** is the source of truth; there is no local SQLite
file.

This doc stays high level and links out for detail:

- Exact key/byte layout: [storage-structure.md](storage-structure.md)
- Design rationale: [constraints-and-design-decisions.md](constraints-and-design-decisions.md)
- Component ownership: [components.md](components.md)
- Native↔wasm VFS parity rules: [sqlite-vfs.md](../sqlite-vfs.md)
- Comparison to other PITR systems: [comparison-to-other-systems.md](comparison-to-other-systems.md)

## Design constraints

These shape everything below; full statements in
[constraints-and-design-decisions.md](constraints-and-design-decisions.md) and the
`engine/packages/depot/CLAUDE.md` "Hard Constraints" section.

- **Single writer per database.** Pegboard guarantees at most one actor instance touches a
  database's storage at a time.
  - Storage does no multi-writer conflict resolution.
  - A generation + head fence guards the brief failover window.
- **No local SQLite files, ever.** The VFS speaks to Depot; Depot speaks to UDB. Nothing on
  disk or tmpfs.
- **Lazy reads.** Pages are fetched on demand, never bulk-preloaded. Forks copy no data.
- **Per-commit granularity.** PITR targets commits/versionstamps, not individual WAL frames.
- **Branches are immutable.**
  - A database id *is* its database-branch id; a bucket id *is* its bucket-branch id.
  - Rollback is engine-owned (fork + swap the pointer).
- **Persisted records use vbare**, except atomic counters and simple indexes (e.g. `VTX`).

## Glossary

### Layers & components

- **VFS** — SQLite's pluggable I/O layer. Our VFS replaces file I/O with calls to Depot.
- **depot-client** — the crate that implements the VFS and the transport to Depot.
- **depot** — the storage engine: owns branches, the read/write/compaction logic over UDB.
- **conveyer** — Depot's commit/read path (the data-plane code in `depot/src/conveyer/`).
- **pegboard-envoy** — the engine-side service that hosts an actor's storage access and
  validates it at the trust boundary.
- **envoy** — the actor↔engine bridge; "inline" vs "remote" SQLite are two modes of it.

### Storage primitives

- **page** — fixed-size (4 KiB) unit SQLite reads/writes. See the pages primer below.
- **delta** — one commit's changed pages, encoded as an LTX blob under `DELTA`.
- **LTX** — the on-disk delta/shard blob format (LTX V3).
- **PIDX** — page-index: maps a page number to the `DELTA` txid that currently owns it.
- **shard** — a 64-page group; a *shard version* is the full page set of that group as-of a
  txid, written during compaction.

### Commits & retention

- **txid** — per-branch monotonic commit counter; the newest is the **head**.
- **versionstamp** — UDB's 16-byte globally-ordered commit token (see primer below).
- **head** — the newest committed txid on a branch.
- **hot watermark** — the txid up to which deltas have been folded into shards; the
  retention/GC frontier.

### Branching & history

- **bucket / database** — a bucket groups databases; a Rivet Actor's SQLite database *is* a
  `database_id`. Forks operate on either.
- **branch** — an immutable version of a database or bucket, with a parent link.
- **pointer / catalog** — `DBPTR`/`BUCKET_PTR` map ids→current branch; `BUCKET_CATALOG`
  records database membership (lazily inherited across bucket forks).
- **pin** — a `DB_PIN` row at a concrete `(txid, versionstamp)` that keeps history alive.
  Restore points (bookmarks), database forks, and bucket forks are all pins.
- **PITR** — point-in-time recovery: periodic auto-pins so you can restore to a past time.

## UDB data structure (reference)

Sketch only; exact bytes in [storage-structure.md](storage-structure.md). The sections below
recall the slice of this layout they touch, so you don't have to scroll back here.

```text
BR/{database_branch_id}/     # per-database-branch data (one subtree per branch)
  META/head                  # current head (txid, db_size, checksum)
  META/quota                 # storage accounting (atomic i64)
  COMMITS/{txid}             # commit metadata (wall clock, versionstamp, size)
  VTX/{versionstamp}         # versionstamp -> txid
  DELTA/{txid}/{chunk}       # LTX delta blob (the commit's changed pages)
  PIDX/{pgno}                # pgno -> owning delta txid
  SHARD/{shard_id}/{as_of}   # compacted full-shard snapshot at txid `as_of`
  PITR_INTERVAL/{bucket_ms}  # one representative commit per time bucket
  CMP/root, CMP/stage/...    # compaction watermark + staged output
DBPTR/{bucket_branch}/{name}/cur   # database name -> current database branch
BUCKET_PTR/{bucket}/cur             # bucket -> current bucket branch
BUCKET_CATALOG/...                  # database membership (inherited on fork)
BRANCHES/..., BUCKET_BRANCH/...     # immutable branch records + parent links
DB_PIN/{database_branch}/...        # pins: restore points, db forks, bucket forks
RESTORE_POINT/...                   # user restore-point tokens
```

## Workflows

Compaction, GC, and retention run as Gasoline workflows — one set per database branch, not yet
enabled in the production registry:

- **`DbManagerWorkflow`** (the **manager**) — the authority: owns compaction state and
  plans/dispatches every publish and delete; the companions only do what it authorizes.
- **`DbHotCompacterWorkflow`** (the **hot-compacter**) — stages compacted `SHARD` output for an
  install and reports back.
- **`DbReclaimerWorkflow`** (the **reclaimer**) — runs GC: deletes manager-authorized rows and
  stale staged output.

Later sections use the short names (**manager**, **hot-compacter**, **reclaimer**) for these.

The commit path is not a workflow — it runs inline in the conveyer transaction. Workflows own
only the background compaction/GC lifecycle.

## Database pages (primer)

- SQLite stores a database as a flat array of fixed-size **pages** (4 KiB here).
- Every read or write is page-granular; the page is the unit Depot stores and versions.
- We do not re-explain the page format here — see the SQLite reference:
  <https://www.sqlite.org/fileformat2.html>.

## SQLite VFS and depot client

**What a VFS is.** SQLite delegates all file I/O (open, read, write, sync, lock, size) to a
pluggable VFS. Ours (`depot-client/src/vfs.rs`) replaces "read/write a file on disk" with
"read/write pages through Depot." There is no backing file — the *no local files* invariant.

**Only two operations cross the boundary.** Despite implementing the full VFS surface,
exactly two calls reach Depot:

- **`get_pages`** — page reads. On an `xRead` cache miss the VFS requests the missing page
  numbers (lazy: only what's touched).
- **`commit`** — page writes. SQLite runs in batch-atomic mode; dirty pages are buffered in
  memory and flushed as one delta on `xFileControl(COMMIT_ATOMIC_WRITE)` (or `xSync` for
  non-atomic flushes).

**Lock callbacks are no-ops** — single-writer is enforced by Pegboard exclusivity plus
fencing, not by SQLite's lock state machine.

**Sequence (query → pages):**

1. `SQL → SQLite → xRead(pgno)`.
2. Cache miss → `get_pages(pgnos)`.
3. Depot resolves from PIDX/DELTA/SHARD.
4. Pages returned → cached → SQLite continues.
5. Writes mirror this: buffered `xWrite`s → `commit(dirty_pages)` at the atomic-write
   boundary.

**Inline vs remote (envoy).** Two independent axes:

- *Where SQLite runs:*
  - **LocalNative** (common): SQLite + VFS run in the actor process; the two page ops are
    tunneled over the envoy websocket to pegboard-envoy, which calls Depot against UDB.
  - **RemoteEnvoy**: the actor ships whole SQL strings to pegboard-envoy, which runs SQLite
    there with an embedded (in-process) Depot transport straight to UDB.
- *How the VFS reaches Depot:*
  - **embedded** (`depot-client-embedded`): calls the Depot `Db` directly in-process (used by
    pegboard-envoy's remote-SQL executor and the depot CLI).
  - **websocket** (`EnvoySqliteTransport`): marshals the two ops over the envoy tunnel.

Either way, pegboard-envoy is the trust boundary: it validates namespace, actor existence,
and generation before any request reaches Depot.

**Fencing on read & write.** Every op carries `(generation, expected_head_txid)`:

- pegboard-envoy CAS-checks the generation against UDB.
- Depot checks `expected_head_txid` against the branch head inside the same serializable
  transaction and raises `HeadFenceMismatch` on a mismatch (`conveyer/read.rs`,
  `conveyer/commit/apply.rs`).
- This catches the rare two-instances-writing case during actor failover.

## How pages are stored and read

(Forks are deferred to [Forking & pinning](#forking--pinning); this section assumes a single
linear branch.)

**Data structure (the keys a read touches):**

```text
PIDX/{pgno}                # pgno -> owning DELTA txid
DELTA/{txid}/{chunk}       # the owning commit's changed pages (LTX)
SHARD/{shard_id}/{as_of}   # compacted full-shard snapshot at txid `as_of`
```

- A commit writes its changed pages as an **LTX delta** under `DELTA/{txid}` (plus `COMMITS`
  and `VTX` rows). Deltas are append-only.
- **PIDX** maps each page number to the `DELTA` txid that last wrote it.
- **Shards** are compacted full snapshots of a 64-page group as-of a txid (built later by
  compaction). A read uses them when the owning delta has been reclaimed.

**LTX (the delta/shard blob format) primer.** Both `DELTA` and `SHARD` blobs are LTX V3 (the
LiteFS "lite transaction" format; V3 is our variant). One blob stores:

- **A set of pages** — each as `(pgno, page bytes)`, plus a small header (page size, the txid
  range it covers, db page count after the commit). A delta holds one commit's changed pages; a
  shard holds a full 64-page group folded as-of a txid.
- **A page index** mapping `pgno → (offset, size)` in the blob, so the blob is
  **frame-addressable**: a reader parses just the header + index and decompresses only the one
  page it needs, never the whole blob. This is what keeps lazy `get_pages` cheap.

More on the format: the upstream LTX spec is <https://github.com/superfly/ltx> (see its "File
Format" section); our V3 byte layout lives in `conveyer/ltx.rs`.

**Read path** — for each requested page:

1. Consult `PIDX/{pgno}` for the owning txid, then load that `DELTA` and return the page.
2. If the delta is absent ([reclaimed](#gc)), fall back to a shard. Scan
   `SHARD/{shard_id}/{as_of}` for the newest `as_of` at or below the read cap.
   - The shard id is pure arithmetic from the page number: `shard_id = pgno / 64`
     (`SHARD_SIZE`), so shard `N` owns pages `N*64 .. N*64+63`.
3. If neither a delta nor a shard provides an in-range page, it reads back as **zeros**. That
   is a legitimate gap — a page below the database size that was never written, or one absent
   from its covering shard — and SQLite expects zeros there.
4. If `PIDX` named a delta that's gone *and* no shard covers the page, the required content is
   unrecoverable, so the read raises a storage error (`ShardCoverageMissing`) rather than
   zero-filling.

## Committing SQLite pages (conveyer)

**Data structure (the keys a commit writes):**

```text
COMMITS/{txid}             # commit metadata (wall clock, versionstamp, size)
VTX/{versionstamp}         # versionstamp -> txid
DELTA/{txid}/{chunk}       # the dirty pages, as LTX
PIDX/{pgno}                # repointed to this txid for each changed page
META/head, META/quota      # advanced
```

A commit runs the conveyer commit path in one UDB transaction (`conveyer/commit/apply.rs`):

1. Read `META/head` serializably and **fence** on `expected_head_txid` (reject on mismatch).
2. Encode the dirty pages as an LTX `DELTA` (chunked) and write `COMMITS`, `VTX`, `DELTA`,
   `PIDX`.
3. Advance `META/head` and update `META/quota`.
4. Wake workflow compaction (a throttled signal) when delta lag crosses a threshold.

The commit path **only records new history** — it never publishes shards or deletes anything.
Shards and deletion are compaction's job.

## Compacting deltas to shards

### Why compaction is needed

`PIDX` makes a *current* read cheap — it routes each page straight to its one owning delta, no
replay. But two costs grow with every commit:

- **Deltas accumulate without bound.** A delta can't be dropped while it still holds the only
  copy of some page, so raw history grows forever.
- **Point-in-time reads get expensive.** A read pinned to an earlier point in history (a fork's
  view of its parent, or a PITR/restore target — explained under [PITR](#pitr)) can't use
  `PIDX`; it walks the delta chain backward to that point, which is O(deltas).

Compaction fixes both: it folds deltas into full **shard snapshots** at the points reads can
land on, so an as-of read is a single shard fetch, and the folded deltas become reclaimable
(see [GC](#gc)).

### The compaction process

Compaction is a two-phase commit — plan and stage first, then install atomically. The three
phases:

1. **Plan** (manager) — scan the batch and decide the work.
2. **Stage** (hot-compacter) — build and stage the shard snapshots.
3. **Install** (manager) — atomically promote them live and advance the watermark.

**Plan.** Scan the batch range once and decide the work: which deltas exist, and the **coverage
points** to snapshot — the txids a later read can be anchored to, so each must stay readable
after the deltas below it are reclaimed.

```text
commits, deltas, pidx = scan(hot_watermark+1 ..= head)   # single range scan

coverage_points = { head }                       # always
for pin in pins:                                 # db/bucket fork, restore point
    if pin.txid in batch:
        add pin.txid
for rep in pitr_reps(commits):                   # commits bucketed by wall-clock
    add rep.txid
```

**Stage.** Loop the coverage points, folding the in-memory deltas (no re-scan). At each point,
snapshot every shard that changed — overlaying the folded pages on the shard's previous
version — and write it as a *pending* blob.

```text
for as_of in sorted(coverage_points):
    # fold in-memory deltas <= as_of, grouped by shard. only changed shards
    # appear; newest write per page wins; truncate-aware (shrunk pages dropped).
    pages_by_shard = fold(deltas where txid <= as_of)

    for (shard_id, pages) in pages_by_shard:
        base = prev_shard_version(shard_id, as_of)   # newest snapshot, or empty
        blob = encode(base overlaid with pages)      # complete 64-page snapshot
        stage(SHARD/{shard_id}/{as_of} = blob)       # pending, not yet live
```

**Install.** In one atomic transaction, revalidate the plan, promote the staged blobs to live
`SHARD` rows, and advance the watermark.

```text
if plan_fingerprint_changed():
    abort_and_replan()
promote staged SHARD blobs -> live SHARD rows    # publish
hot_watermark = head                             # advance the frontier
```

- **Truncate-aware fold:** a page removed by a later truncate is dropped, not resurrected, so an
  as-of read never sees pages a shrink had already freed.
- **Atomicity:** the publish and the watermark advance are in the *same* transaction, so "below
  the watermark = covered" is always true — never a window where the watermark moved but the
  shards are missing.
- **Authority:** the **manager** owns all publish/delete; the companions only stage or delete
  what it authorizes.

Garbage collection of the now-redundant deltas and superseded shards is handled by reclaim —
see [GC](#gc).

## GC

Reclaim is the unified collector. Once the watermark passes a txid, that txid's deltas are
redundant (the covered points have shards), so reclaim collects them. In one pass it:

1. deletes `DELTA` rows at or below the watermark (see below),
2. deletes `COMMITS`/`VTX` below the watermark, except a **keep-set** (see below),
3. clears **stale `PIDX`** (see below),
4. deletes **superseded `SHARD` versions** (see below), and
5. drops **expired `PITR_INTERVAL` rows** (see below).

It is inert while the watermark is 0.

**Delta retention.** Retain a `DELTA` if and only if its txid is above the hot watermark.
Everything at or below is reclaimable with no per-shard proof, because the install published
shard coverage for every covered point in the same transaction that advanced the watermark.
This simple rule is sound *only because forks are constrained to covered points* (see
[Forking & pinning](#forking--pinning)): the alignment fence makes every reachable read cap a
covered point or the head, so a reclaimed below-watermark delta can never be the only source for
a read. Drop either half — the rule or the fork constraint — and the other breaks.

**Keep-set.** Below the watermark, `COMMITS`/`VTX` are normally collected — but a commit must
stay readable if something still points at it. The keep-set is exactly those survivors: the
txids referenced by a **pin** (`DB_PIN`: database fork, bucket fork, or restore point) or a
**retained PITR interval representative**. Everything else below the watermark is provably
unreachable and is collected.

**Stale `PIDX`.** A `PIDX` entry is stale once its owning txid is at or below the watermark: the
delta it names has been folded into a shard (and may already be deleted), leaving the entry a
dangling routing hint. Reclaim clears it with compare-and-clear; reads already fall back to the
shard in the meantime.

**Superseded `SHARD` versions.** A shard version is superseded once no covered txid reads
through it — it is not the newest version at or below any covered point, and not above the
watermark. Reads resolve "newest version at or below the cap" and every reachable cap is a
covered point or the head, so such a version is unreachable and is deleted.

**Expired `PITR_INTERVAL` rows.** Each interval representative carries a retention TTL
(`expires_at_ms`). Once it passes, the row is reclaimable, and it is deleted in the same pass
that drops it from coverage — so the `COMMITS`/`VTX` it was keeping never lose their last
reference before being collected.

## Forking & pinning

**Versionstamps & VTX (primer).** Forks, pins, and PITR are all defined *as of a versionstamp*,
never a txid:

- **Why not txid:** a txid is a *per-branch* counter, so it can't order or compare commits
  across branches.
- **versionstamp** — UDB's 16-byte, globally-ordered commit token, assigned at commit time. It
  gives a total order over every commit on every branch *without a clock*.
- Every commit writes one (recorded as `VTX/{versionstamp}` → txid) because any commit might
  later become a fork/pin/PITR target, and the global order has to be fixed at commit time.
- `VTX` is the reverse index that resolves a target versionstamp back to its branch txid.
- The whole alignment/retention fence compares versionstamps (not wall-clock times) to decide
  what a read can reach.

**Data structure (forks + pins):**

```text
BRANCHES/{id}              # immutable branch record: parent + parent_versionstamp
DBPTR/{bucket_branch}/{name}/cur, BUCKET_PTR/{bucket}/cur   # id -> current branch
BUCKET_CATALOG/...         # database membership (inherited lazily on bucket fork)
DB_PIN/{database_branch}/… # pins: restore points, db forks, bucket forks
```

**Forking is effectively free.**

- A fork is just a new immutable `BRANCHES` record with a parent link and the fork
  versionstamp; **no data is copied**.
- All the real work happens in compaction (which stages shard coverage at fork/pin points) and
  the read path (which walks branch ancestry).

**Where you can fork from.** Only:

- a point **above the watermark** (its deltas still exist; the new pin makes the next
  compaction stage coverage for it), or
- an **already-covered point** (the watermark, a PITR interval representative, or an existing
  pin).
- A caller-supplied versionstamp between covered points is **snapped down** to the newest
  covered point at or below it.
- This constraint is exactly what makes GC sound.

**Reads with forks.** A read resolves across the branch and its ancestors:

1. Start at the branch. For each `parent` link (`BRANCHES` carries `parent` +
   `parent_versionstamp`), include the ancestor **capped** at the versionstamp it was forked at.
2. Resolve each requested page per source — PIDX/DELTA/SHARD, identical to the basic read path.
3. The most-specific branch that has the page wins.

**Pins are one unified thing.**

- Creating a restore point (a "bookmark"), a database fork, or a bucket fork all write a
  `DB_PIN` row holding a concrete `(txid, versionstamp)`.
- Both the compaction coverage-staging and the reclaim keep-set read all `DB_PIN` rows, so a
  pin both gets shard coverage staged at its txid and is kept by GC.
- There is no separate "bookmark" store — a bookmark *is* a pin of kind `RestorePoint`.

**The three pin shapes:**

- **Database fork.** Fork one database at a versionstamp → a new database branch with a parent
  link and a `DatabaseFork` pin on the source. (An actor's database is a `database_id`, so
  forking an actor's DB is a database fork.)
- **Bucket fork.** Forks a whole bucket metadata-only (catalog is inherited, not copied).
  - A database inherited through the fork is **materialized lazily** on first access.
  - Its first read/write derives a capped database fork at the fork point, so reads freeze at
    the fork and writes build on the inherited state instead of leaking into the source.
- **Restore / rollback.** Reuse the same primitive: (1) resolve a snapshot selector to a
  covered `(txid, versionstamp)`, (2) fork there, and (3) for rollback, move the engine-owned
  pointer to the new branch.

## PITR

**Data structure:**

```text
PITR_INTERVAL/{bucket_ms}  # one representative commit (txid, versionstamp) per time bucket
```

**PITR coverage is just periodic auto-pins.**

- During compaction we bucket the batch's commits by wall-clock time (default 5-minute
  intervals).
- We record one representative commit per bucket as a `PITR_INTERVAL` row holding its
  `(txid, versionstamp)`.

**Why you can't restore to an arbitrary past point.**

- We deliberately do *not* keep every delta — reclaim deletes them once the watermark passes.
- So only covered points survive, and a timestamp restore **floors** to the nearest interval
  representative at or before your timestamp.
- To restore to an *exact* point, create a restore point (bookmark) while that point is still
  reachable; that pin then survives reclaim.

**Reading at a past point (the point-in-time read).** Restoring or forking to a past point
gives you a branch whose reads are *as of* that point: each page resolves to the newest write
at or below the target — the same capped ancestry read described under
[Forking & pinning](#forking--pinning). With a shard published at that covered point the read is
a single fetch; without one it would have to walk the delta chain backward to the target. That
walk is the cost compaction exists to remove, and why every restorable point is a covered
point with a shard.

**PITR is just a fork.** An `AtTimestamp` restore:

1. resolves the timestamp through the `PITR_INTERVAL` rows to a representative's
   `(txid, versionstamp)`,
2. forks there, and
3. for an in-place restore, moves the engine-owned pointer to the new branch.

Same fork primitive, same alignment rules — the only difference is how the target point is
chosen.
