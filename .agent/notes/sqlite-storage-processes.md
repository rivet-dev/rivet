# SQLite storage processes — plain English walkthrough

A jargon-free explanation of every process the storage layer runs. Reflects the v4 spec (rough PITR + fork-only, no rollback at storage).

## Concepts you need first

- A **database** is a SQLite-like database. It stores data in fixed-size 4 KB **pages** (page #1, page #2, ...).
- A **commit** is one transaction's worth of changes — it produces a small bundle of "here are the pages that changed in this transaction" called a **delta**.
- Every commit gets a **transaction id** (sequential per database) and a **versionstamp** (globally unique sequence number across the whole cluster).
- Pages are grouped into **shards** for storage efficiency. Pages 1–1000 in shard 0, pages 1001–2000 in shard 1, etc.
- We have two storage tiers: a **fast tier** (FoundationDB, transactional key-value store) and a **cold tier** (S3, dumb object storage).
- A **branch** is an immutable history record. Forks produce new branches.
- A **pin** is a marker that says "don't delete history older than this point."

---

## Read — "give me these pages"

Someone wants pages 47, 102, and 998 from database D.

1. Look up D's metadata to find the **head** — the latest committed transaction id.
2. For each page:
   1. Check the **page index** — a map from "page number" to "which transaction last wrote this page." If page 47 was last written in commit 1234, it tells us so.
   2. Fetch the **delta** for commit 1234 and pull out page 47.
   3. If the page index says nothing about this page, no recent commit changed it. Fall back to the **shard** that owns this page — a folded-up bundle of pages from many older commits, kept in the fast tier.
   4. If the shard isn't in the fast tier either (because cleanup pushed it to cold storage), reach into S3 and fetch it.
3. If D is a fork and we still didn't find the page, walk up to the parent database and try again — capped at the moment the fork was made (so we never see post-fork changes in the parent).
4. Return the assembled pages.

The first time a database connection makes a read, it walks the parent chain once and caches the result. Subsequent reads skip the walk.

## Write — "commit these page changes"

A commit on D wants to change pages 47 and 102. Fast and entirely in the fast tier:

1. Look up the head, increment to get the new transaction id (e.g. 1234 → 1235).
2. Bundle pages 47 and 102 into a **delta** for transaction 1235 and write it.
3. Update the page index: "page 47 → commit 1235", "page 102 → commit 1235".
4. Update the head metadata: "head is now 1235."
5. Write a **commit row** recording wall-clock time, the unique versionstamp, the database size, and a checksum.
6. Write a **versionstamp-to-transaction-id index entry** so we can later look up "what transaction was at this versionstamp?" without scanning every commit.
7. If we've accumulated a lot of deltas since the last hot compaction, send a notification to wake up the hot compactor.

This is one transaction. It stays small on purpose.

## Hot compact — "fold deltas into shards so reads stay fast"

The problem: every commit writes a delta. After a thousand commits, a single page might be touched in hundreds of small deltas. Reads would chase a long chain.

Solution: periodically the **hot compactor** wakes up for one database and:

1. Takes all the deltas since the last compaction.
2. For each shard, computes "what does the shard look like after applying all those deltas?"
3. Writes the result as a new **versioned shard**: shard 0 at transaction id 1500, for example. The previous version of shard 0 stays for now.
4. Records that everything up to transaction 1500 has been folded.

Why "versioned" shards? A reader at a past versionstamp (e.g. a fork) might still need the old contents. Multiple versions of the same shard coexist; reads pick the right version based on the transaction id they're reading at.

After this, the deltas that got folded can be deleted — but only if cold storage already has them durably (see cold compaction). Otherwise they stay in the fast tier until cold compaction catches up.

## Cold compact — "move data to cheap storage"

The problem: the fast tier is expensive. Old data should live in cheap object storage. Also S3 is the only thing that survives if the fast tier blows up.

The **cold compactor** uploads stuff to S3 in three phases, because S3 uploads can take seconds and the fast-tier transactions need to stay short.

**Phase A — plan the work.** Briefly inside a fast-tier transaction: figure out which shard versions and which deltas haven't been uploaded yet. Write a "work-in-progress marker" with a unique ID. Then leave the transaction.

**Phase B — upload to S3.** Outside any fast-tier transaction, upload:

- One file per shard version (an "image layer")
- One file per delta range (a "delta layer")
- For any pinned bookmarks waiting to be materialized: a full database snapshot at that point (a "pin layer")
- A small manifest file listing what was just uploaded
- A "catalog snapshot" recording who-belongs-where (database → namespace mappings) so the system can rebuild metadata if the fast tier blows up

This is the slow part. Can take seconds. Fine because nothing is holding a database transaction open.

**Phase C — record the work as official.** Briefly inside another fast-tier transaction: bump the "everything up to here is durable in S3" cursor. Mark any pinned bookmarks as Ready. Clear the work-in-progress marker.

If anything crashes mid-Phase B, the next pass sees the stale marker and reuses or overwrites the partial uploads — they're idempotent.

## Cleanup — three things, all distinct

### 1. Eviction (fast tier → empty)

Once shard version X for database D is durably in S3, AND nothing is actively using that version, AND it's older than the hot-cache window: delete it from the fast tier. The data still exists in S3; reads on it now go to S3.

The **eviction compactor** sweeps periodically. It reads an "access timestamp" the database keeps so it doesn't evict something that was just touched.

### 2. Garbage collection (delete data nothing depends on)

Each branch tracks the oldest **pin** it has — the oldest point in time anything still cares about. Pins come from three sources:

- **Reference count > 0** — something still references this branch (a fork descends from it, or its database hasn't been deleted yet).
- **Descendant pin** — a fork descends from this branch at some point; we can't delete anything older than that fork point.
- **Bookmark pin** — a user-created pinned bookmark holds retention here.

GC computes the *minimum* of the three. Anything older than that minimum is deletable: old commit rows, old delta files, old versioned shards in the fast tier (if not already evicted), old image/delta/pin layers in S3.

### 3. Universal hot-tier retention floor

Even if nothing else is holding retention, the system keeps recent commit history in the fast tier for a configurable window (e.g. 7 days). This bounds how long the fast tier holds raw commit metadata before it gets folded or shipped to cold.

## Forking — two flavors, both metadata-only

### Fork a database

Caller asks "fork database D at versionstamp V" and gets back a new database id D'.

In one fast-tier transaction:

1. Read D's branch record. Check that V isn't older than D's pin.
2. Look up "what was D's head at versionstamp V?" via the versionstamp-to-transaction-id index.
3. Read the commit row at that transaction to get D's database size and checksum at V.
4. Write a new branch record for D': parent = D, parent-versionstamp = V, root-versionstamp = V.
5. Write a "synthetic head" record for D' so a reader knows what state the fork starts at, before D' has done any of its own commits.
6. Bump reference counts: D's count up (D' references it), D' starts at 1.
7. Set a descendant pin on D at V so GC can't delete anything older than V from D.
8. Record D' as a member of its target namespace (so list_databases can find it).

That's it. No data copy. D is unaffected. D' starts life with no commits of its own; reads fall through to D for any page D' hasn't yet written.

### Fork a namespace

Caller asks "fork namespace N at versionstamp V" and gets back a new namespace id N'.

Same shape:

1. Write a new namespace branch record: parent = N, parent-versionstamp = V.
2. Bump reference counts and set descendant pin on N.
3. **No per-database work.** N' starts with empty membership.

The trick: when something later asks "what databases are in N'?", it walks up the parent chain. N' is empty → check N → find all of N's databases at versionstamp V. Lazy inheritance. Pays nothing at fork time.

## Deleting

### Delete a database

Decrement its reference count. If it hits zero AND no fork descends from it AND no bookmark pins it, GC will eventually delete every byte: branch record, commit history, page index, deltas, shard versions, S3 layers, S3 manifests, S3 branch record.

If anything still holds it, it stays alive — possibly forever, until the holder releases. This is by design: deleting a database that someone forked from would corrupt the fork.

### Delete a namespace

Same logic. The namespace's reference count drops; if zero and no descendants and no pins, the namespace is gone.

### Delete a bookmark

If pinned, this is what releases the retention. The next GC pass recomputes the database's pin and may free a chunk of history.

## Bookmarks

### Resolve a bookmark — "what state was this database in at this point?"

A bookmark is a small string encoding `{wall_clock_time}-{transaction_id}`. To resolve:

1. Look up the transaction's versionstamp via the versionstamp-to-transaction-id index.
2. Find the nearest preserved commit. If it's still in the fast tier, return its versionstamp. If it's only in cold storage, look in the cold manifest.
3. If the bookmark is older than the database's pin, return "expired."
4. If the database descends through other branches, the resolution walks parent chains.

### Create a pinned bookmark — "guarantee I can recover this exact point"

1. Synchronously: write the bookmark record (status = Pending), bump the database's bookmark pin so GC can't delete this point, send a notification to the cold compactor.
2. Asynchronously: cold compactor on its next pass takes a full snapshot of the database at this point, uploads it as a "pin layer" in S3, marks the bookmark Ready.

The pin holds retention immediately; the snapshot upload happens in the background.

## Minor periodic processes

- **Stale work-marker cleanup** — every cold pass also lists S3 for old work-in-progress markers (from crashed prior passes), deletes the orphaned uploaded files they listed, then deletes the marker.
- **Catalog snapshot upload** — every cold pass writes a small file recording "as of this moment, database X belongs to namespace Y, etc." This is the catalog used to reconstruct who-belongs-where if the fast tier blows up and we need to rebuild from S3.
- **Lease renewals** — both the hot compactor and cold compactor hold leases (timed locks) so two pods don't compact the same database at once. A background timer renews the lease every few seconds.
- **Cache touches** — every read or write bumps a "last-accessed" timestamp on the database (throttled to once per minute) so eviction knows what's hot vs cold.
