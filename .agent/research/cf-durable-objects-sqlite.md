# Cloudflare Durable Objects SQLite — PITR + Bookmarks + Forking

Research compiled 2026-04-29. Sources are public Cloudflare docs, the September 2024 SRS launch blog post, and third-party architecture write-ups. The user-mentioned "SLS" appears to be a near-miss for the actual name **SRS** (Storage Relay Service); no Cloudflare reference to "SLS" was found.

## Sources

- [Zero-latency SQLite storage in every Durable Object — blog.cloudflare.com](https://blog.cloudflare.com/sqlite-in-durable-objects/) — primary architecture article, introduces SRS, WAL log shipping, 5-follower replication, snapshot rule, 30-day PITR.
- [SQLite-backed Durable Object Storage — developers.cloudflare.com](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/) — canonical reference for the PITR API surface (`getCurrentBookmark`, `getBookmarkForTime`, `onNextSessionRestoreBookmark`) and the bookmark string shape.
- [Access Durable Objects Storage — developers.cloudflare.com](https://developers.cloudflare.com/durable-objects/best-practices/access-durable-objects-storage/) — confirms 30-day PITR window and that PITR covers both SQL data and KV `put()` data.
- [Time Travel and backups — Cloudflare D1 docs](https://developers.cloudflare.com/d1/reference/time-travel/) — D1's user-facing surface for the same SRS-backed PITR; confirms bookmarks are deterministic from Unix timestamps and lexicographically sortable, and that restore is destructive in-place but undoable.
- [Restore D1 Database to a bookmark or point in time — Cloudflare API](https://developers.cloudflare.com/api/python/resources/d1/subresources/database/subresources/time_travel/methods/restore/) — D1's REST/CLI surface for restore.
- [Chapter 12: D1: SQLite at the Edge — architectingoncloudflare.com](https://architectingoncloudflare.com/chapter-12/) — third-party summary; confirms WAL is stored alongside the database, free vs paid retention (7d / 30d), and that long-term archive uses R2 export via Workflows (not SRS).
- [Durable Objects in Dynamic Workers (Facets) — blog.cloudflare.com](https://blog.cloudflare.com/durable-object-facets-dynamic-workers/) — facets get *separate* SQLite databases packaged inside one DO; relevant to forking discussion (it is *not* a fork, it is per-tenant isolation).
- [SQLite WAL spec — sqlite.org](https://sqlite.org/wal.html) — background on the WAL frame format SRS ships.

## Architecture overview

```
   Worker / Application
          |
          v
   ctx.storage / SQL API   (synchronous from app perspective)
          |
          v
   SQLite (in-process)  --- writes --->  WAL file (*.db-wal)
          |                                  |
          |                                  v
          |                       SRS leader (same machine as the DO)
          |                                  |
          |   intercepts via SQLite VFS      |
          |                                  v
          |     (1) replicate WAL frames synchronously to 5 followers
          |          (3-of-5 quorum gate ack to the application)
          |     (2) after batch window: upload to object storage (R2)
          |     (3) periodically upload a full snapshot when log >= db size
          |
          v
   Local disk cache  <----- restore by replaying log from snapshot -----+
                                                                        |
                                                              R2 (object storage)
                                                              - snapshots
                                                              - WAL batches
                                                              - retained 30d for PITR
```

Per [the SRS launch post](https://blog.cloudflare.com/sqlite-in-durable-objects/):
"Local disk is fast and randomly-accessible, but expensive and prone to disk failures. Object storage (like R2) is cheap and durable, but much slower than local disk and not designed for database-like access patterns."

SRS hooks SQLite's VFS to observe WAL frames as they are appended. The local disk holds the live SQLite database (hot path); R2 holds the durable archive of WAL batches and periodic snapshots.

## Storage tiering (hot vs cold)

| Tier | Backing | Contents | Purpose |
| --- | --- | --- | --- |
| Hot | Local disk on the DO host machine | Live SQLite DB + WAL | Synchronous reads/writes; "zero-latency" path |
| Replication buffer | Local disk on each of 5 followers in distinct datacenters | Recent WAL frames awaiting "persisted" notification | Durability before a WAL batch reaches R2; also failover if leader unreachable |
| Cold / archive | Object storage (R2) | Compacted WAL batch objects + periodic full-database snapshots | Long-term durability and PITR replay source |

When does data move:

- **Local -> followers:** synchronously, on every SQLite commit. Quorum (3/5) acks before the write is ack'd to the app.
- **Local/followers -> R2:** in batches. Per the blog: "SRS batches changes over a period of up to 10 seconds, or up to 16 MB worth, whichever happens first, then uploads the whole batch as a single object."
- **Snapshots -> R2:** "SRS will decide to upload a snapshot any time that the total size of logs since the last snapshot exceeds the size of the database itself." This caps full-DB reconstruction cost at "no more than twice the size of the database."
- **Retention:** WAL batches and snapshots are not physically deleted on checkpoint. Per the blog: "SRS merely marks them for deletion 30 days later. In the meantime, if a point-in-time recovery is requested, the data is still there to work from." (Free-plan D1 is 7 days; Paid D1 and Durable Objects are 30 days per the [D1 Time Travel doc](https://developers.cloudflare.com/d1/reference/time-travel/).)

Long-term archives beyond 30 days are explicitly *out of band*: per [Chapter 12 of Architecting on Cloudflare](https://architectingoncloudflare.com/chapter-12/), users wanting longer retention export to R2 themselves via Workflows. SRS does not internally tier to a deeper cold store.

## Bookmarks

Identifier shape (verbatim from [the Storage API doc](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/)):

> "A bookmark is a mostly alphanumeric string like `0000007b-0000b26e-00001538-0c3e87bb37b3db5cc52eedb93cd3b96b`. Bookmarks are designed to be lexically comparable: a bookmark representing an earlier point in time compares less than one representing a later point."

Decoded structure (inferred from format and the D1 docs; not officially documented):

- Four hex-padded fields plus a trailing 16-byte hash-like value.
- The first three look like ascending sequence/log-position counters (e.g. epoch, batch index, frame index).
- The trailing token appears to be a content-addressable identity / nonce so two databases never collide.
- Cloudflare states explicitly that bookmarks are *deterministically derived from Unix timestamps* on the D1 side: "Bookmarks can be derived from a Unix timestamp ... and conversion between a specific timestamp and a bookmark is deterministic (stable)" ([D1 Time Travel](https://developers.cloudflare.com/d1/reference/time-travel/)).

Generation:

- **Automatic, continuous.** Every committed transaction advances the bookmark space, because every commit is a WAL frame in the shipped log. No "snapshot bookmark" ceremony is required.
- A specific bookmark for the current instant is fetched via `getCurrentBookmark()`.
- Bookmarks for past wall-clock times are computed via `getBookmarkForTime(t)` (Date or epoch-ms; D1 also accepts RFC3339).

API surface (Durable Objects, JS):

```ts
ctx.storage.getCurrentBookmark(): Promise<string>
ctx.storage.getBookmarkForTime(timestamp: number | Date): Promise<string>
ctx.storage.onNextSessionRestoreBookmark(bookmark: string): Promise<string>
```

`onNextSessionRestoreBookmark` returns a bookmark for the instant *just before* the planned restore — that return value is the "undo" pointer if you need to revert the revert.

API surface (D1, CLI / REST):

- `wrangler d1 time-travel info <db> [--timestamp ...]`
- `wrangler d1 time-travel restore <db> --bookmark ... | --timestamp ...`
- REST: `POST /accounts/{id}/d1/database/{db}/time_travel/restore`

Retention:

- Durable Objects: 30 days.
- D1: 30 days on Workers Paid, 7 days on Workers Free.
- No documented mechanism inside SRS to extend retention; long retention requires app-level export to R2.

## Point-in-time recovery

Granularity:

- **Effectively continuous** in Durable Objects: any wall-clock instant in the last 30 days resolves to the closest bookmark. Per the blog: "we can restore to any point in time by replaying the change log from the last snapshot."
- D1's CLI surface advertises **minute-level** granularity for the timestamp form (per [Time Travel docs](https://developers.cloudflare.com/d1/reference/time-travel/)). The DO API does not document this floor; for direct bookmark restore the granularity is per-commit (a bookmark advances every WAL frame group).

Mechanism:

- Restore by **log replay from the last full snapshot** up to the target bookmark. This is identical to ordinary cold-start hydration except the replay stops early at the target bookmark instead of at the head of the log.
- Restore is **destructive in place**: existing post-restore state is overwritten when the DO next boots. Per [D1 Time Travel](https://developers.cloudflare.com/d1/reference/time-travel/): "queries in flight will be cancelled, and an error returned to the client."
- Restore is **undoable**: `onNextSessionRestoreBookmark` returns a bookmark from the moment before the restart, and the pre-restore log frames are not deleted. Per D1 docs: "Restoring a database to a specific bookmark does not remove or delete older bookmarks."
- The restore is gated to the *next* session — `onNextSessionRestoreBookmark` schedules the restore, and the application typically calls `ctx.abort()` to force the restart.

Window: 30 days for Durable Objects (and Workers Paid D1); 7 days for Workers Free D1. Bookmarks older than the window become invalid.

API:

```ts
const before = await ctx.storage.getCurrentBookmark();
const target = await ctx.storage.getBookmarkForTime(new Date(Date.now() - 5 * 60_000));
const undoBookmark = await ctx.storage.onNextSessionRestoreBookmark(target);
ctx.abort(); // forces restart, which does the restore on next boot
// later, to revert: onNextSessionRestoreBookmark(undoBookmark) + abort()
```

## Forking

**Not directly supported.** No Cloudflare API exposes "clone this Durable Object's storage to a new id" or "branch this database at a bookmark." Searches across the [DO Storage API doc](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/), the [llms-full.txt dump](https://developers.cloudflare.com/durable-objects/llms-full.txt), and the [SRS launch post](https://blog.cloudflare.com/sqlite-in-durable-objects/) returned zero references to fork/clone/branch/copy of a SQLite-backed DO.

The architecturally adjacent feature is **Durable Object Facets** ([Facets blog](https://blog.cloudflare.com/durable-object-facets-dynamic-workers/)): one supervisor DO can host multiple named "facets," each of which gets *its own independent SQLite database* stored together in the same overall DO. Facets allow per-tenant isolated databases inside a shared object, but they are created empty and there is no documented "facet from snapshot of facet X" mechanism.

Mechanism (if you wanted to imitate forking on top of SRS):

- The data primitive is already present — a snapshot at bookmark B plus the log from B onward is a complete reconstructable state. Conceptually a "fork" is "spin up a new actor whose initial hydrate uses snapshot S and replays the WAL up to bookmark B."
- Cloudflare has not exposed this. Whether they implement it internally is not publicly documented.

API: none publicly exposed. Application-level workarounds use `sql.exec(...)` to dump rows and re-`put` them into a new DO id, which is not COW and not bookmark-precise.

Limitations: even if a fork API existed, the SRS log is keyed per database identity (the trailing hash-token in the bookmark looks like a database/installation id). Forks would need a new identity *and* either a copy of the snapshot in R2 or a shared snapshot reference with separate forward-log streams.

## Compaction / retention

- **Checkpoints (in SQLite):** SRS still issues SQLite WAL checkpoints to keep the live DB file current. Per the blog, when checkpointed, "from time to time the database is 'checkpointed', merging the changes back into the main database file."
- **Snapshot trigger (in SRS):** uploaded to R2 when accumulated log size since last snapshot >= current DB size. Caps reconstruction download at <= 2x DB size.
- **Logical deletion of old log/snapshot objects in R2:** marked for deletion 30 days after they would otherwise be obsolete (i.e. after a newer snapshot supersedes them). This is what makes 30-day PITR work despite snapshots existing.
- **No explicit "compact range" API.** Compaction is implicit in the snapshot rule. A new full snapshot makes older logs eligible for the 30-day deletion timer.

## Open questions / things that are not public

The following details could not be confirmed from public sources and would need a Cloudflare contact, code-leak, or empirical reverse-engineering:

- **Exact bookmark field encoding.** The four-segment shape (`0000007b-0000b26e-00001538-<hash>`) is documented as opaque. The mapping from Unix timestamp -> bookmark is stated to be deterministic but the formula is not published.
- **Granularity floor for DO PITR.** D1's UI floor is per-minute; the DO docs imply per-commit. Whether SRS internally rounds, or how it picks "the bookmark closest to time t," is not documented.
- **Snapshot format on R2.** The blog refers to "an object" per batch and "a snapshot," but does not specify whether snapshots are full SQLite DB files, page-level deltas, or a custom format.
- **Whether R2 truly is the cold store, or whether SRS uses an internal object store.** Cloudflare's blog uses R2 as the example but does not commit to it being the literal backend.
- **Forking.** No public API, no public statement about whether one is on the roadmap.
- **Patent coverage.** Searches for Kenton Varda / Cloudflare patents on SRS specifically returned no direct hits; the blog post is the canonical public reference. There are general Cloudflare patents around DO orchestration but none publicly tied to SRS by name.
- **Follower geographic policy.** "Five followers in different physical data centers" is stated; the selection algorithm (latency, region, customer locality) is not.
- **Read-replica path.** D1 has read replication ([D1 read replication blog](https://blog.cloudflare.com/d1-read-replication-beta/)) but the DO post does not say whether SRS itself plays a role or whether read replicas are layered on top. Out of scope here.

## Direct quotes from authoritative sources

From [Zero-latency SQLite storage in every Durable Object](https://blog.cloudflare.com/sqlite-in-durable-objects/):

> "For SQLite-backed Durable Objects, we have completely replaced the persistence layer with a new system built from scratch, called Storage Relay Service, or SRS."

> "SRS has already been powering D1 for over a year, and can now be used more directly by applications through Durable Objects."

> "Local disk is fast and randomly-accessible, but expensive and prone to disk failures. Object storage (like R2) is cheap and durable, but much slower than local disk and not designed for database-like access patterns."

> "SRS records a log of changes, and uploads those."

> "SRS always configures SQLite to use WAL mode. In this mode, any changes made to the database are first written to a separate log file."

> "SRS monitors changes to the WAL file (by hooking SQLite's VFS to intercept file writes) to discover the changes being made to the database, and uploads those to object storage."

> "SRS batches changes over a period of up to 10 seconds, or up to 16 MB worth, whichever happens first, then uploads the whole batch as a single object."

> "SRS will decide to upload a snapshot any time that the total size of logs since the last snapshot exceeds the size of the database itself."

> "the total amount of data that SRS must download to reconstruct a database is limited to no more than twice the size of the database."

> "Every time SQLite commits a transaction, SRS will immediately forward the change log to five 'follower' machines across our network."

> "When a follower receives a change, it temporarily stores it in a buffer on local disk, and then awaits further instructions."

> "if the follower never receives the persisted notification, then, after some timeout, the follower itself will upload the change to object storage."

> "Each of a database's five followers is located in a different physical data center."

> "if we can't reach the DO's host, we can instead try to contact its followers. If we can contact at least three of the five followers, and tell them to stop confirming writes for the unreachable DO instance, then we know that instance is unable to confirm any more writes going forward."

> "Since SRS stores a complete log of changes made to the database, we can restore to any point in time by replaying the change log from the last snapshot."

> "SRS merely marks them for deletion 30 days later. In the meantime, if a point-in-time recovery is requested, the data is still there to work from."

From [SQLite-backed Durable Object Storage](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/):

> "A bookmark is a mostly alphanumeric string like `0000007b-0000b26e-00001538-0c3e87bb37b3db5cc52eedb93cd3b96b`. Bookmarks are designed to be lexically comparable: a bookmark representing an earlier point in time compares less than one representing a later point."

> "Returns a bookmark representing the current point in time in the object's history." (`getCurrentBookmark`)

> "Returns a bookmark representing approximately the given point in time, which must be within the last 30 days." (`getBookmarkForTime`)

> "Configures the Durable Object so that the next time it restarts, it should restore its storage to exactly match what the storage contained at the given bookmark." (`onNextSessionRestoreBookmark`)

> "These methods apply to the entire SQLite database contents, including both the object's stored SQL data and stored key-value data."

From [Time Travel and backups (D1)](https://developers.cloudflare.com/d1/reference/time-travel/):

> "Bookmarks are lexicographically sortable. Sorting orders a list of bookmarks from oldest-to-newest."

> "Bookmarks can be derived from a Unix timestamp (seconds since Jan 1st, 1970), and conversion between a specific timestamp and a bookmark is deterministic (stable)."

> "Restoring a database to a specific point-in-time is a destructive operation, and overwrites the database in place. However, the restore operation will return a bookmark that allows you to undo and revert the database."

> "Restoring a database to a specific bookmark does not remove or delete older bookmarks."

> "Bookmarks older than 30 days are invalid and cannot be used as a restore point."

From [Chapter 12: D1: SQLite at the Edge](https://architectingoncloudflare.com/chapter-12/):

> "D1 stores a write-ahead log (WAL) of all changes alongside your database, recording every modification before it's applied."

> "Changes are indexed by bookmarks; opaque identifiers corresponding to specific points in time. Restoration reconstructs database state by replaying changes up to the specified bookmark."

> Retention: "30 days on paid plans; 7 days on free tier. Longer-term archival uses R2 export via Workflows."
