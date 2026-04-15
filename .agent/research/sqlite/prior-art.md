# Remote-SQLite Prior Art: Architecture Comparison and Re-Architecture Proposal

## Status

 Draft for review. Produced under US-024 after the adversarial review of the
 earlier optimization spec killed most of the incremental proposals.

 This revision corrects an overstatement in the first draft. The first pass
 only covered the "local SQLite file + downstream replication" family
 (LiteFS, Durable Objects SQLite, libSQL). There is also a real
 "VFS-native/page-store" family that matters for Rivet: mvSQLite, dqlite,
 Litestream VFS, sql.js-httpvfs, and absurd-sql. Some of those are genuinely
 optimized. Some are only optimized for read-mostly or local-browser workloads.

## TL;DR

There are **two** relevant architecture families:

1. **Local-file + downstream log replication**. LiteFS, Cloudflare Durable
   Objects SQLite, libSQL/Turso, and dqlite all run SQLite against a local file
   or local file image and replicate transaction/WAL state behind it. Reads are
   local on the hot path.
2. **VFS-native/page-store systems**. mvSQLite, Litestream VFS,
   sql.js-httpvfs, and absurd-sql intercept page I/O directly. The good ones
   are only good because they add aggressive batching, conflict tracking,
   prediction, caching, and in some cases hydration to a local file.

The earlier draft was right about the practical disease in the current Rivet
benchmark, but wrong about exclusivity. Rivet is **not** the only system that
treats remote storage as the first-class page source. It is, however, a
relatively naive version of that idea today: `xRead` still devolves to one
remote `kv_get` per page miss, and `xWrite` only recently stopped paying that
shape on the write path.

The corrected high-level takeaway is:

- If Rivet can tolerate a local per-actor file plus rehydration on migration,
  the local-file family is still the cleanest end state.
- If Rivet must keep a remote authoritative page store, the real reference is
  **mvSQLite-shaped**, not "more `kv_get` micro-optimizations."

## How the three prior-art systems actually work

Full detailed research notes are captured in the three agent reports referenced
below. This section is the short architectural summary of each.

### LiteFS (Fly.io)

- **Storage.** A FUSE passthrough filesystem sits in front of a normal on-disk
  SQLite database. Under its mount point you see a real `database`, `journal`,
  `wal`, `shm`, and an `ltx/` directory of captured transaction files.
- **Writes.** FUSE tracks every dirty page during a transaction. At commit,
  LiteFS assembles a single **LTX file** (header + sorted dirty-page block +
  trailer with pre/post-apply CRC64 checksums) representing exactly that
  transaction. One fsync on the LTX, one atomic rename, one fsync of the `ltx`
  directory, and the commit is durable. The LTX is then broadcast to replicas
  over a persistent HTTP chunked-transfer stream.
- **Reads.** `DatabaseHandle.Read` on both primary and replicas just calls
  `f.ReadAt` on the local file. There is **no** remote page fetch path. SQLite
  does normal `pread` on a real file, and FUSE stays out of the way.
- **Durability.** Local fsync is the durability boundary. Replication is
  asynchronous — a catastrophic primary death can lose recent committed txns.
  Sync replication is on the roadmap, not shipped.
- **Bottleneck.** FUSE interposition caps writes at roughly 100 transactions
  per second per node. A future "libSQL/Virtual WAL style" implementation is
  planned to avoid FUSE.
- **Unit of replication.** A whole SQLite transaction, shipped as one LTX byte
  stream. A 10 MiB insert is one LTX file, one HTTP chunked-transfer response.

### Cloudflare Durable Objects SQLite

- **Storage.** SQLite runs as an in-process library on the same thread as the
  Worker. The database file lives on the host machine's local SSD.
- **Writes.** `ctx.storage.sql.exec` is a direct library call, not an RPC. A
  shim called Storage Relay Service (SRS) hooks SQLite's VFS and watches the
  WAL. On commit, SRS **synchronously** ships the WAL delta to five follower
  machines in nearby data centers and blocks acknowledgment until 3 of 5 ack.
  In parallel, WAL batches are asynchronously uploaded to object storage every
  16 MB or 10 seconds, plus periodic full snapshots bounding replay to at most
  2x the DB size.
- **Reads.** Always local SQLite against the host's own file. Followers exist
  for durability and failover, not read serving.
- **Durability boundary.** Commit returns only after 3-of-5 follower acks. The
  application does not block on this — workerd's Output Gate holds the
  outbound HTTP response until confirmed, so requests feel synchronous without
  stalling the JS event loop.
- **Movement.** DOs do not live-migrate today; instance location is fixed at
  creation. Failover spawns a new instance, reconstructing the DB from the
  latest object-storage snapshot plus WAL batches.
- **Limits.** 10 GB per DO, 2 MB per row, 100 KB per SQL statement. SQLite is
  pinned in WAL mode by SRS.

### libSQL / sqld / Turso embedded replicas

- **Storage.** libSQL is a C-level fork of SQLite with a pluggable Virtual WAL
  (`libsql_wal_methods_*`). sqld (the server) runs real SQLite connections
  against real on-disk files and plugs in `ReplicationLoggerWalWrapper` for
  replica streaming and optionally `BottomlessWalWrapper` for S3 backup. Turso
  Cloud's diskless variant splits the DB into 128 KB segments and ships the
  current WAL generation to S3 Express One Zone.
- **Protocol.** Hrana is **SQL-level**, not page-level. Clients send
  `execute`/`batch` requests over WebSocket or HTTP containing SQL text and
  typed values. The server runs them against its local SQLite and returns
  rows. No page data crosses Hrana.
- **Replication.** WAL frames (24-byte header + 4 KiB page body, chained via
  rolling CRC-64) are streamed over gRPC from primary to replicas. Replicas
  poll the primary for new frames and apply them through the pluggable WAL.
  Bottomless batches and uploads frames to S3 asynchronously.
- **Embedded replicas.** A client-side libSQL file on local disk plus a sync
  URL. The embedded replica fetches frames from the primary via HTTP and
  reconstructs a real SQLite file locally. Reads execute against the local
  file; writes are forwarded to the primary.
- **Durability boundary.** Self-hosted sqld: local fsync. Bottomless: local
  fsync + async S3 upload. Turso Cloud diskless: ~6.4 ms commit because each
  commit is one S3 Express PUT. All three only commit after the frame is
  durable in whatever the configured log store is.

### One-line summary of the first family

**Local SQLite against a real file. The remote layer is a durable WAL-frame
log sitting behind it. Reads never hit the network.** All three differ only in
what the log store is (local disk for LiteFS, followers + object storage for
DO, gRPC stream + S3 for libSQL).

## The VFS-native family we missed in the first draft

The systems above are not the whole market. There is a second family that
really does use VFS interception or a page/block-backed filesystem layer as the
core data path.

### mvSQLite

- **Storage model.** mvSQLite is "Distributed, MVCC SQLite that runs on top of
  FoundationDB" and integrates with SQLite as a custom VFS layer. It keeps the
  authoritative database in a distributed page store, not in one local file.
- **Why it matters.** This is the closest serious prior art to Rivet's current
  direction. It proves that "remote authoritative page store" can work, but
  only with much richer machinery than today's pegboard KV path.
- **Read path.** mvSQLite does not issue one network round-trip per page miss.
  It has a per-connection prefetch predictor that combines a Markov table, a
  stride detector, and a recent-history ring buffer. On a miss it can fetch the
  requested page plus predicted pages in one `read_many` call.
- **Write path.** Conflict detection is page-level, not namespace-level. The
  PLCC path tracks read sets and page versions so transactions touching
  different pages can commit concurrently without distributed locks.
- **Large commits.** mvSQLite has a separate multi-phase path for large write
  sets and even experimental commit groups to batch writes across databases into
  one FoundationDB commit.
- **What to steal.** If Rivet keeps a remote page store, mvSQLite is the gold
  standard for the data plane: `read_many`, predictive prefetch, page-versioned
  MVCC, idempotent commit handling, and a protocol that is page-aware instead
  of bolting `kv_get` onto `xRead`.

### dqlite

- **Storage model.** dqlite configures SQLite to use a custom VFS that stores
  the database file image, WAL, and WAL-index in process memory instead of on
  disk. The durable state is the Raft log, not the SQLite files.
- **Write path.** When SQLite commits, dqlite intercepts the WAL append,
  encodes the resulting page updates into a Raft log entry, waits for quorum,
  then updates the in-memory WAL image and replies success.
- **Read path.** Reads are local memory lookups and `memcpy`, not remote page
  fetches. The network is on the replication/consensus path, not on each read.
- **Why it matters.** dqlite proves a second thing besides the local-file
  family: VFS interception can still be excellent when the remote/durable layer
  is transaction-shaped rather than page-fetch-shaped. It looks closer to
  "local image + replicated log" than to Rivet's current page-over-KV design.

### Litestream VFS

- **Storage model.** Litestream VFS serves SQLite directly from a replica chain
  of snapshots and LTX files stored in object storage. It builds an in-memory
  page index and fetches pages on demand.
- **Read path.** The VFS indexes page numbers to byte offsets inside LTX files,
  caches hot pages in an LRU cache, and maintains separate main and pending page
  indexes so read transactions get a stable snapshot while polling continues in
  the background.
- **Write path.** In write mode it uses a local write buffer, tracks dirty
  pages, packages them into a new LTX file on sync, and performs optimistic
  conflict detection against the remote txid before upload.
- **Hydration.** Litestream can stream-compact the replica into a local
  hydrated SQLite file while continuing to serve reads. After hydration, reads
  move from remote LTX blobs to the local file.
- **Why it matters.** Litestream VFS is not a strong write path for multi-writer
  OLTP, but it is extremely relevant for Rivet's read path. It shows three
  tactics Rivet lacks today: page indexes, transaction-aware dual indexes for
  snapshot isolation, and transparent hydration to local disk.

### sql.js-httpvfs

- **Storage model.** A read-only browser-side virtual filesystem backed by HTTP
  range requests against a static SQLite file.
- **Read path.** It uses "virtual read heads" that grow request sizes during
  sequential access, making sequential scans logarithmic in request count
  instead of one request per page.
- **Why it matters.** This is pure read-path prior art. It shows that even when
  the network remains the backing store, naive per-page reads are optional.
  Prefetch and range coalescing matter a lot.

### absurd-sql

- **Storage model.** A filesystem backend for sql.js that stores SQLite pages
  in small blocks inside IndexedDB. This is not distributed, but it is a very
  direct example of "SQLite on top of a slower block store."
- **Performance model.** The project explicitly leans on SQLite's own page
  cache and page-size tuning. The author calls out that SQLite's default 2 MB
  page cache and larger page sizes are part of why the approach works.
- **Why it matters.** absurd-sql is the local-browser version of the same
  lesson: block-level indirection only becomes tolerable when you let SQLite's
  cache and larger I/O units do real work. Rivet currently leaves both on the
  table.

## Side-by-side comparison

| Dimension | LiteFS | DO SQLite | libSQL (sqld+bottomless) | Rivet (current) |
|---|---|---|---|---|
| Authoritative local file | Yes (real SQLite file) | Yes (real SQLite file) | Yes (real SQLite file) | **No** |
| Reads hit the network | No | No | No (replica is local file) | **Every page** |
| Unit shipped remotely | LTX file per txn | WAL delta per commit | WAL frames (batched) | Per-page KV entry |
| Remote protocol | HTTP chunked LTX | Internal RPC to followers + object-store PUT | gRPC frame stream + S3 | WebSocket + per-page KV ops |
| Commit durability | Local fsync (async replica) | 3-of-5 follower ack (sync) | Local fsync (or S3 PUT) | Per-page KV commit |
| Read serving | Local SQLite on file | In-process SQLite on file | Local SQLite on file | VFS callbacks → remote KV |
| Cold start | Rebuild from LTX catch-up | Snapshot + WAL replay from object store | Frame replay from primary/S3 | Fetch pages on demand |
| Locks active instance to host | Consul lease | Fixed at creation | Single-primary sqld | **No** (portable via KV) |
| Journal mode | DELETE or WAL | WAL (forced) | WAL (pluggable) | DELETE |
| Bulk-insert network cost | 1 HTTP stream | 1 follower fan-out | 1 Hrana request | **~2500 KV writes** |
| Bulk-verify network cost | 0 network ops | 0 network ops | 0 network ops | **~2500 KV reads** |

The bottom two rows are the entire story of the current Rivet benchmark:
~900 ms insert because we pay per-page on write, ~5000 ms verify because we pay
per-page on read. The VFS-native systems that do use remote storage avoid this
exact cost profile by batching reads, predicting future reads, hydrating to a
local file, or keeping the network off the hot path entirely.

## Side-by-side comparison of the VFS-native systems

| Dimension | mvSQLite | dqlite | Litestream VFS | sql.js-httpvfs | absurd-sql | Rivet (current) |
|---|---|---|---|---|---|---|
| Authoritative store | FoundationDB-backed page store | Raft log + in-memory file images | Snapshot + LTX files in object storage | Static SQLite file over HTTP | IndexedDB blocks | Pegboard actor KV pages |
| Primary target | Distributed read/write DB | HA replicated SQL service | Read replicas, light single-writer sync | Read-only static datasets | Local persistent web apps | Portable actors |
| Read miss unit | Batched `read_many` + predicted pages | Local memory | Indexed page fetch from LTX/object store | HTTP ranges with virtual read heads | Local block read from IndexedDB | One `kv_get` per page miss |
| Read isolation | MVCC + page-version tracking | Leader/follower state machine | Main/pending page indexes per txn | Read-only | Single-worker/local locking | SQLite pager only |
| Write commit unit | FDB transaction, multi-phase for large writes | Raft log entry from captured WAL append | LTX upload on sync interval | None | Small block writes to IndexedDB | Batched page writes, still page-shaped store |
| Conflict model | Page-level OCC (PLCC) | Single leader + quorum | Optimistic single-writer conflict detection | None | Local browser coordination | Per-file fencing, no page-level MVCC |
| Hot-path network on reads | On miss, but amortized and predicted | No | Yes until hydrated/cache hit | Yes, but range-coalesced | No | Yes, one miss at a time |
| What it proves | Remote page store can work if the protocol is rich enough | VFS can feed a replicated log instead of raw file I/O | Read-path remote VFS can be civilized | Sequential remote scans do not need per-page RTTs | Slow block stores can be rescued by SQLite cache | Current protocol is missing the winning pieces |

## The actual Rivet architecture

From the adversarial review and the code in `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` and `engine/packages/pegboard/src/actor_kv/mod.rs`:

- The VFS does not own a local SQLite file. The only SQLite file that exists is
  the VFS's in-process view; every `xRead`/`xWrite` is serviced by the KV
  bridge.
- Pages are encoded as 4 KiB KV entries keyed by `(file_tag, chunk_index)` in
  the pegboard actor-KV subspace, with a per-key `EntryMetadataKey` carrying
  `(version, update_ts)`.
- A "fast path" (US-008 through US-014) batches dirty pages at xSync boundaries
  into one `sqlite_write_batch` request per file-tag, fenced against stale
  replay. That collapses the per-page write chatter on the commit side.
- The read side still runs one `kv_get` per SQLite `xRead` callback. A
  transaction-local dirty buffer exists and is already promoted to an opt-in
  `read_cache` on flush, but the gate defaults off, so verify is effectively
  uncached.

**Why Rivet built it this way.** Actors are portable: they can be killed and
respawned on any pegboard node. The KV store is the only piece of state that
follows an actor across node boundaries. Pinning the actor's SQLite file to a
specific host's disk (the way LiteFS, DO, and libSQL all do) would break actor
mobility.

That constraint is real and none of the three local-file systems solves it
directly:

- **LiteFS** pins the primary via a Consul lease and fails over with a
  ~10-second TTL. Writes stop during failover.
- **DO** pins the instance at creation and does not live-migrate at all.
- **libSQL Turso Cloud** has a single primary; embedded replicas can read but
  must forward writes back to the primary.

None of them has "actor can start anywhere, any time, with fresh state on
whatever node gets it." That is a harder problem than what the local-file trio
tackles. The VFS-native trio is closer:

- **mvSQLite** solves portability by putting the hard part into a distributed
  page store plus a richer protocol and concurrency model.
- **Litestream VFS** solves only the read-mostly replica version of the problem.
- **dqlite** solves HA via leader-based replication, not arbitrary mobility.

Rivet's page-over-KV design was an honest attempt to solve the harder problem,
but it currently pays for it on every single byte of I/O because the protocol
is much closer to raw `kv_get`/`kv_put` than to mvSQLite.

## What this says about the current optimization direction

The previous spec (`sqlite-remote-performance-remediation-plan.md`) took the
architecture as given and tried to reduce waste inside it:

- Coalesce dirty pages into one batched commit (US-005 through US-014).
- Add a page-store fast path on the server (US-010, US-011).
- Measure everything (US-001 through US-004).

That was correct work for the constraint "cannot change the storage layout or
the mobility model." It bought us a 10× insert improvement and it uncovered
that the read side is now the bottleneck, and that every remaining
write-side win is single-digit percent because the server-side fast path is
already at 96 ms for 10 MiB. The adversarial review then killed the
follow-on proposals (page bundling, packed keys, zstd, per-page metadata
dedup, read-ahead prefetch, sqlite_read_batch, multi-tag sqlite_commit) because
they either duplicate existing work, violate the hard migration constraint, or
attack the wrong layer.

**The incremental path has hit a wall.** The adversarial-review quick wins
(US-020 and US-021 — flip the read cache default, bump `PRAGMA cache_size`)
still apply and should ship. But they are not a long-term architecture; they
are a papercut fix for the symptom.

The corrected structural answer is not one single answer. There are now two
serious branches:

1. **Get a real SQLite file back onto local disk** and replicate a log behind
   it. This matches the local-file family and still looks like the cleanest
   end state if pegboard local storage is acceptable.
2. **If local files are politically or operationally off the table, rebuild the
   remote page-store design to be mvSQLite-shaped instead of `kv_get`-shaped.**
   That means accepting a much larger rewrite: batched reads, predictive
   prefetch, page-versioned MVCC, richer retry semantics, and likely a new
   server-side data plane rather than generic actor KV calls.

## Re-architecture options

Five directions are on the table. I rank them by fit with Rivet's constraints.

### Option A: Local file + KV-backed WAL frame log (LiteFS-shaped)

**Shape.** Each actor, on the pegboard node currently running it, holds a real
SQLite file on local disk (or per-actor tmpfs). SQLite runs in the usual WAL
or DELETE mode against that file. A thin VFS shim (or a FUSE-free equivalent
like libSQL's pluggable WAL) captures dirty pages at commit and ships them
*downstream* to the KV layer as a single WAL-frame blob or LTX-style
transaction file.

The KV layer stops being a page store and becomes a **write-ahead log store**:

- Key: `(actor_id, file_tag, frame_no)`
- Value: a frame or small range of frames (4–128 frames per value is a tuning
  knob, not a correctness question)
- Append-only on the hot path
- Compaction merges adjacent frame ranges into checkpointed snapshots

**Writes.** SQLite writes into the local file through its real WAL. At
commit, the shim reads the new WAL frames and writes them to KV as one frame
blob. The commit returns success only after the KV write acks durably. This
is very close to LiteFS's `CommitJournal` but with KV replacing the local
`ltx/` directory.

**Reads.** SQLite reads from the local file directly. Zero network ops. The
current 5000 ms verify-scan drops to effectively free.

**Cold start on a new node.** When an actor is scheduled on a pegboard node
that does not have a local copy of its database, the node reads the KV frame
log and replays it into a fresh local SQLite file (or downloads the latest
snapshot plus the frame tail). Cold start cost scales with database size,
not with the number of prior writes.

**Migration.** An actor moves by:
1. Draining writes on the source node (quiesce the local file, flush the
   last WAL frames to KV).
2. Updating a small KV-level "actor is now at node X" pointer (already part
   of the pegboard actor lifecycle).
3. The target node reads the log / snapshot from KV and rebuilds the local
   file.

This preserves actor mobility. Migration becomes a rehydrate step, which is
exactly how DO failover works today.

**Durability boundary.** Commit = local SQLite fsync + KV log append
committed. Crash on the source node mid-commit: the local file has whatever
SQLite's pager committed; the KV log has whatever was pushed before the
crash. The node that picks up the actor replays the KV log to converge.

**Scope.**
- VFS: replace page-store VFS with a thin WAL-frame captor (similar to what
  libSQL's `libsql_wal_methods` provides out of the box).
- Pegboard: new API shape on the server side — `append_frames(actor_id,
  file_tag, frame_batch, fence)`, `read_log_tail(actor_id, file_tag,
  from_frame)`, `snapshot(actor_id, file_tag, up_to_frame)`. The existing
  `sqlite_write_batch` fast path can be retired in favor of this.
- KV: same subspace, new key layout (frame-log, not page-store). Migration
  needed for existing actor data.
- Pegboard local disk: need a per-actor data directory. Already exists in some
  form for sandbox mounts; needs audit.

**Wins.**
- Reads: per-page network cost → zero.
- Writes: per-page network cost → per-transaction (one KV write containing a
  frame batch).
- Bulk insert: local SQLite WAL speed + one frame-log append (bounded by KV
  commit latency, not bounded by per-page chatter).
- Benchmark ceiling: local SQLite is ~50 ms for 10 MiB. KV log append at a
  single 10 MiB value is bounded by the KV commit path — could plausibly be
  ~100–200 ms. Total insert budget: ~150–250 ms vs today's 900 ms.

**Risks.**
- Pegboard local disk becomes a dependency. If the node loses its disk
  between actor checkpoints, recovery falls back to the last KV snapshot.
- Rehydration on cold start reads more bytes than the current on-demand model
  for actors that only need a small slice of their database. This matters for
  workloads that open a huge DB to touch one row. Mitigation: lazy snapshot
  + per-range lazy frame replay.
- Migration cost: for an existing actor with N pages in the current KV
  layout, a one-shot rewrite of the layout is required. Offline migration
  during actor idle windows is probably fine.
- Need compaction and retention logic on the frame log, otherwise the log
  grows unbounded. LiteFS handles this with LTX merging; we would do the
  same.

### Option B: Embedded replica model with explicit sync points

**Shape.** Similar to Option A, but instead of making commits synchronous with
the KV log append, commits are durable locally and the log append is
asynchronous up to a configurable sync interval (every N ms or N frames).

**Why different from A.** DO ships writes synchronously to 3-of-5 followers
because it owns the hardware. We don't. On our stack, pushing every commit
into the KV layer synchronously is the thing that makes us slow, not the
thing that makes us fast. If commits can be locally durable with async log
shipping, the actor gets local-SQLite speed and the KV layer catches up in
the background.

**Tradeoff.** Violates the "commit is durable after commit returns" contract
unless the local pegboard disk is itself considered durable. That is a real
semantic change and needs an explicit decision from the user. If the local
disk is not trusted (node death = data loss), this option is unsafe. If the
local disk is trusted for the duration between sync checkpoints, this option
is the fastest possible path.

DO effectively picked the "trusted local disk via 3-of-5 replica quorum"
answer. We do not have that infrastructure on pegboard today.

**Recommendation.** Only revisit Option B if Option A proves too slow at the
synchronous commit boundary. Prefer to ship A first.

### Option C: SQL-over-network (Hrana-shaped)

**Shape.** Move away from the VFS layer entirely. Actors talk to their
database by sending SQL statements over the bridge. A server-side SQLite
engine runs those statements against a real local file. This is what libSQL
Hrana does and what DO SQLite effectively does via `ctx.storage.sql.exec`.

**Fit with Rivet.** Could work, but it requires a real server-side SQLite
process (one per actor, or a multiplexed pool), which is more infrastructure
than we have today. Also breaks the "SQLite runs inside the actor process" UX
that is currently the RivetKit model — callers get a `c.db.execute` that
feels local, and moving to SQL-over-network would make every query pay a
network round-trip.

**Recommendation.** Reject as the main direction. The current local-VFS UX
is valuable and Option A preserves it.

### Option D: Keep page-over-KV, add a local caching layer

**Shape.** Do not move the authoritative store. Add a local SQLite file on
the pegboard node as a cache, populated from the KV store on miss and
invalidated via fencing. Writes still go to KV; reads are served from the
local cache with consistency checks.

**Fit.** This is what flipping `RIVETKIT_SQLITE_NATIVE_READ_CACHE` default-on
(US-020) plus bumping `PRAGMA cache_size` (US-021) effectively approximates
at a much smaller scope. It delivers most of the read-side win without
touching the write path or the storage model.

**Recommendation.** Ship US-020 and US-021 as a standalone tactical fix
regardless of which larger direction we pick. Do not treat D as the
long-term answer because the write path is still stuck at the current
~900 ms floor and the actor-boot cost still pays per-page reads for any
page not already in cache.

### Option E: Rebuild the remote page store to look like mvSQLite

**Shape.** Keep the authoritative store remote and portable, but stop using
generic actor-KV reads and writes as the SQLite data plane. Replace that with a
dedicated SQLite page service:

- batched `read_many` or range-read API, not one `kv_get` per page miss;
- page-versioned metadata and read-set tracking for page-level MVCC;
- predictive prefetch on the client;
- idempotent multi-phase commit for large page sets;
- optional local hydration or warm-cache file for hot actors.

**Fit.** This is the only serious "stay remote-first" answer I found. It keeps
actor mobility without pinning a local authoritative file, but it is a much
larger rewrite than Option A.

**Why it is not just US-025 with better caching.** mvSQLite works because the
entire protocol is designed around page-versioned concurrency and batched data
movement. Rivet currently has neither. Bolting prefetch onto today's KV path
would help, but it would still leave the wrong server contract in place.

**Recommendation.** Only choose E if local pegboard files are a hard no. If
you choose it, stop thinking in terms of incremental performance stories and
treat it as a ground-up protocol/storage redesign.

## Recommendation

1. **Ship US-020 and US-021 immediately as tactical fixes.** They are one-line
   changes that drop verify to near-zero on this benchmark and are valid
   regardless of the long-term direction.
2. **If pegboard local disk is acceptable, pick Option A — local SQLite file +
   KV WAL-frame log — as the primary re-architecture direction.** It is still
   the cleanest fit for Rivet's benchmark pain because it deletes network reads
   from the steady state.
3. **If pegboard local disk is not acceptable, stop considering incremental
   page-store tweaks and define a new Option E: mvSQLite-shaped remote page
   store.** That means:
   - batched `sqlite_read_many`/range reads instead of one `kv_get` per miss;
   - predictive prefetch on the client;
   - page-versioned MVCC / read-set conflict tracking on the server;
   - idempotent multi-phase commit protocol for large writes;
   - likely separation of page metadata/version index from page bodies;
   - optional hydration to a local file on warm actors.
4. **Do not pursue Options B, C, or D as the long-term answer.** B is unsafe
   without replicated pegboard disk; C breaks the local-VFS UX; D is a
   symptom fix, not a cure.

If the user confirms Option A, the next deliverables are:

- A dedicated spec at `.agent/specs/sqlite-local-file-wal-log-plan.md`
  covering: on-disk layout per actor, frame format, KV log schema, fencing
  and retention, migration protocol for existing actor data, cold-start
  rehydration, compaction, and failure semantics.
- A set of follow-up stories appended to `scripts/ralph/prd.json` mirroring
  the existing phased rollout style (measure first, land the VFS shim, land
  the KV log, migrate old data, benchmark).
- An explicit decision on whether to adopt libSQL as the SQLite runtime in
  RivetKit (its pluggable-WAL API is exactly the hook Option A needs) or to
  keep stock SQLite and implement the shim ourselves.

The libSQL question is worth flagging early: libSQL's `libsql_wal_methods`
interface was designed for exactly this use case, and adopting it would let
us reuse their WAL frame format and their pluggable-WAL plumbing instead of
re-inventing it. Tradeoffs include a dependency on the libSQL fork, possible
drift from upstream SQLite, and needing to evaluate whether its licensing
and binary size work for us.

## Open questions for the user

1. **Is local pegboard disk available and trustworthy for per-actor SQLite
   files?** Option A depends on this. If every commit has to land in the
   distributed KV store synchronously, the structural win shrinks because the
   commit boundary is still network-bound.
2. **Is actor mobility via cold rehydration acceptable?** Moving an actor
   costs "time to read log/snapshot from KV + apply" on the target node. For
   large DBs this is significant. Current model pays small cost on migration
   but huge cost on every read.
3. **If local disk is rejected, are we actually willing to build mvSQLite-class
   machinery?** This is not "one more fast path." It is a new protocol and
   likely a new storage/index layout.
4. **Adopt libSQL as the SQLite runtime if Option A wins?** Would the team
   accept a fork dependency in exchange for a pre-built pluggable-WAL hook?
5. **Durability semantics.** Is "committed once the local node has fsynced +
   KV log append has acked" the correct bar, or do we need stronger (e.g.
   replicated-to-N-nodes) durability?
6. **Migration story for existing actor data.** Offline one-shot rewrite of
   every existing SQLite file from the current page-store layout into the new
   frame-log layout is the cleanest path. Is downtime for that acceptable?

## Research sources (from the three agent reports)

**LiteFS.**
- `https://github.com/superfly/litefs` — `docs/ARCHITECTURE.md`, `db.go`
  (`CommitJournal`, `ReadDatabaseAt`, `WriteDatabaseAt`, `ApplyLTXNoLock`),
  `http/server.go` (`/stream`, `streamLTX`, `streamLTXSnapshot`),
  `fuse/database_node.go`, `store.go` (`processLTXStreamFrame`).
- `https://github.com/superfly/ltx` — LTX file format spec, `file_spec.go`.
- `https://fly.io/docs/litefs/how-it-works/`, `https://fly.io/docs/litefs/faq/`,
  `https://fly.io/blog/introducing-litefs`.

**Cloudflare Durable Objects SQLite.**
- `https://blog.cloudflare.com/sqlite-in-durable-objects/` — primary
  architecture article (Kenton Varda / Josh Howard, Sep 2024).
- `https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/`
  — Input Gate / Output Gate rationale.
- `https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/`
  — API surface, transactions, PITR bookmarks.
- `https://developers.cloudflare.com/durable-objects/platform/limits/` — 10 GB
  per DO, 2 MB per row, 100 KB per SQL statement.
- `https://developers.cloudflare.com/durable-objects/reference/data-location/`
  — no live migration today, jurisdictions, location hints.
- Third-party commentary (Simon Willison, chenjianyong on `ImplicitTxn` write
  coalescing, Kenton Varda HN comments).

**libSQL / sqld / bottomless / Turso.**
- `https://github.com/tursodatabase/libsql` — project structure, Hrana
  subpackage, bottomless crate, `libsql-server`.
- `https://github.com/tursodatabase/libsql/blob/main/docs/HRANA_3_SPEC.md` —
  Hrana transports and request shapes.
- `https://github.com/tursodatabase/libsql/blob/main/docs/DESIGN.md`,
  `USER_GUIDE.md`, `libsql_extensions.md`.
- `https://deepwiki.com/tursodatabase/libsql/4.2-wal-and-pager-systems`,
  `.../4.3-libsql-extensions`.
- `https://docs.turso.tech/features/embedded-replicas/introduction`,
  `https://docs.turso.tech/sdk/http/reference`.
- Turso blog: `introducing-embedded-replicas`, `turso-offline-sync-public-beta`,
  `turso-cloud-goes-diskless`, `how-does-the-turso-cloud-keep-your-data-durable-and-safe`.
- Community analysis: Canoozie libSQL replication notes, Compiler Alchemy
  "libSQL Diving In".

Full per-agent reports with concrete file:line citations live in the
conversation history for US-024.

**Additional VFS-native sources.**
- `https://github.com/losfair/mvsqlite` — README, `docs/prefetch.md`,
  `docs/commit_analysis.md`, plus the benchmark post
  `https://su3.io/posts/mvsqlite-bench-20220930`.
- `https://canonical.com/dqlite/docs/explanation/replication` and
  `https://documentation.ubuntu.com/lxd/stable-5.21/reference/dqlite-internals/`
  — custom VFS, WAL interception, Raft replication.
- `https://litestream.io/how-it-works/vfs/` — page-indexed read path, dual
  index transaction isolation, write buffer, and hydration.
- `https://github.com/phiresky/sql.js-httpvfs` and
  `https://phiresky.github.io/blog/2021/hosting-sqlite-databases-on-github-pages/`
  — HTTP-range VFS and virtual read-head prefetch.
- `https://github.com/jlongster/absurd-sql` and
  `https://jlongster.com/future-sql-web` — IndexedDB block-store backend,
  SQLite page-cache reliance, page-size tuning, and durability caveats.

**mvSQLite primary sources (verified).**
- `https://github.com/losfair/mvsqlite` — project README. "A layer below
  SQLite, custom VFS layer, all of SQLite's features are available."
  Integration via `LD_PRELOAD libmvsqlite_preload.so` or FUSE.
- `https://github.com/losfair/mvsqlite/wiki/Atomic-commit` — PLCC
  (`PLCC_READ_SET_SIZE_THRESHOLD = 2000`), DLCC, MPC
  (`COMMIT_MULTI_PHASE_THRESHOLD = 1000`), 5-step commit process, page-hash
  validation, last-write-version (LWV) check, changelog-store append,
  interval read `[client-read-version, commit-versionstamp)`.
- `https://github.com/losfair/mvsqlite/wiki/Caveats` — "max transaction size
  in mvsqlite is 50000 pages (~390MiB with 8KiB page size), the time limit
  is 1 hour"; "SQLite does synchronous 'disk' I/O… reads from FoundationDB
  block the SQLite thread."
- `https://github.com/losfair/mvsqlite/wiki/Comparison-with-dqlite-and-rqlite`
  — mvsqlite handles "both replication and sharding", "linearly scalable to
  hundreds of cores" vs "single consensus group" in dqlite/rqlite.
- `https://github.com/losfair/mvsqlite/wiki/YCSB-numbers` — YCSB A-F
  against a 1M-row table, 64 threads, 16 KiB pages on c5.2xlarge. Read
  throughput 1.9k–11.2k ops/sec, update 1.9k ops/sec, insert 0.4k–0.5k ops/sec.
- `https://su3.io/posts/mvsqlite` — "VFS unlock operation as the transaction
  visibility fence"; tracks read set and write set, commit-time version
  comparison, delta-encoded page storage (XOR + zstd).
- `https://su3.io/posts/mvsqlite-2` — page schema
  `(page_number, page_versionstamp) -> page_hash`, content store
  `page_hash -> page_content`, reverse range scan
  `(page_number, 0)..=(page_number, requested_versionstamp)` with limit 1;
  FoundationDB versionstamps are 80-bit monotonic.

## Update: single-writer and no-local-file constraints

**Two hard constraints from the user after the first draft:**

1. **No local SQLite file.** Options A and B (LiteFS-style local file plus KV
   WAL log) are off the table. The system must operate purely through the VFS
   against a remote authoritative store.
2. **Single writer.** Each actor owns its own SQLite database, and only one
   actor writes to a given database at a time. There is no concurrent writer
   problem.

**What this changes about the prior-art analysis.**

Almost every "remote storage" complication in mvSQLite — PLCC, DLCC, MPC,
read-set tracking, page-versioned MVCC, versionstamps, optimistic conflict
retry, content-addressed dedup, changelog-based cache flush — exists to solve
the multi-writer problem. With a single writer per database, all of that
machinery is dead weight. Rivet does not need MVCC. It does not need
commit-time conflict detection. It does not need versionstamps. It only needs
the *data plane* parts of mvSQLite's design: batched reads, predictive
prefetch, and a large client cache keyed by page number.

The real answer under these constraints is simpler than Option E. It is not
"rebuild Rivet as mvSQLite." It is "keep single-writer SQLite-over-KV but
stop paying per-page network cost on the read path."

## Option F: Single-writer in-memory cache with pure-VFS remote store

**Shape.** Keep the existing pegboard KV subspace as the authoritative page
store. Keep the existing fast-path write batching. Add three pieces:

1. **A large client-side page cache** holding every recently-read or
   recently-written page keyed by `(file_tag, chunk_index)`. Owned
   exclusively by the one actor writer. Invalidated only on truncate, never
   on remote update because there is no remote update that the writer did
   not itself issue. The `read_cache` data structure already exists at
   `vfs.rs:1064-1092`; it just needs to be enabled and pre-populated.
2. **Bulk hydration at actor resume.** When an actor is scheduled on a
   pegboard node, the VFS reads its whole SQLite file (or, for large DBs, a
   configurable prefix plus any lazy-loaded pages on miss) into the page
   cache in one parallel batched request before the first SQL statement
   executes. This is the single-writer analogue of Turso embedded-replica
   hydrate-on-open, except it lives in memory instead of on disk.
3. **A `sqlite_read_many` server op plus a VFS-level stride-detecting
   prefetcher.** For actors whose working set doesn't fit, the VFS predicts
   sequential scans (SQLite's most common read pattern) and issues one
   batched range read ahead of the pager. Misses still happen, but they are
   amortized across hundreds of pages per round-trip.

**Writes.** Unchanged. The existing fast-path write batch (US-008 through
US-014) is already correct for single-writer. Dirty pages are buffered
locally and flushed as one commit to KV. Fences are single-writer serial
monotonic, which is what the current system already provides.

**Reads on steady state.** Zero network operations. After hydration every
page is in the client cache, and the SQLite pager cache sits on top. The
current 5000 ms verify becomes ~50 ms of pager+cache lookups.

**Reads on cold start.** One parallel bulk fetch to hydrate. For a 10 MiB DB
at 128 pages per batch with one inflight batch per 10 ms = 20 ms of
hydration. For a 1 GB DB with lazy hydration + prefetch, it is whatever the
working set costs, still far less than 2500 serial 2 ms round-trips.

**Actor mobility.** Same as today. The actor process dies, the cache dies
with it, the KV store retains everything durable. On resume on a new node,
the new VFS instance hydrates from KV and continues. **The in-memory cache
is not a local file. It is a transient process-lifetime cache.** This is
fully compliant with the "pure VFS, no local file" constraint.

**What this does NOT need from mvSQLite:**
- No MVCC. Single writer means no concurrent read-vs-write conflicts.
- No page versioning in KV. Every page key holds exactly the latest version.
- No conflict detection at commit. The writer is the only writer.
- No content-addressed dedup.
- No commit-intent log. The existing fenced fast-path batch is sufficient.
- No 5-step commit. Current 1-step fenced write_batch is the right shape.

**What this DOES need from mvSQLite:**
- Batched page fetch API. mvSQLite serves many pages per round-trip.
- Prefetch prediction at the client. mvSQLite has a speculative `read_many`.
- A large client cache that is consulted before the KV call. Rivet has the
  data structure (`read_cache` in `vfs.rs:1064-1092`), but it is gated off
  and never pre-populated.

**Scope.**
- **Client VFS.** Enable and pre-populate the read cache. Add a stride
  detector. Add a hydration pass in the VFS file-open path that issues one
  bulk `sqlite_read_many` and populates the cache. Bump `PRAGMA cache_size`.
  Additive to the existing VFS; no protocol changes for the write path.
- **Pegboard server.** Add `sqlite_read_many(actor_id, file_tag, ranges)` to
  envoy protocol v3. The server already has the page keys; this is a
  straightforward extension of `actor_kv::get` into a batched page-range
  form. No storage-layout change.
- **Benchmark.** Re-run the sqlite-raw workload after each of the three
  pieces lands.

**Expected wins.**
- Verify: ~5000 ms → ~50 ms (pager cache or VFS cache serves every page).
- Insert: unchanged from today's ~900 ms write-path floor.
- Cold start for a small DB: ~50 ms total (one bulk hydrate) vs today's
  on-demand fetch spread across the first few SQL statements.

**Risks.**
- Memory pressure. Hydrating a whole DB into the cache consumes RAM
  proportional to DB size. Mitigation: budget-capped hydration with lazy
  fall-through for oversized DBs.
- Prefetch mispredictions. A stride detector can over-fetch on random
  workloads. Mitigation: cap predictor aggressiveness, telemetry for miss
  rate, disable predictor on low hit rate.
- Cold-start latency for very large DBs. A 10 GB DB will not fit in memory
  and cannot be eagerly hydrated. Mitigation: lazy hydration + stride
  prefetch, same as mvSQLite.

## Revised recommendation under the new constraints

1. **Ship US-020 and US-021 immediately.** Still the fastest tactical wins
   regardless of direction. They also pave the path for Option F because
   the cache they enable is the cache Option F pre-populates.
2. **Pick Option F as the long-term direction.** It is the only option that
   respects both hard constraints (pure VFS, no local file) and exploits
   the single-writer guarantee to skip mvSQLite-class complexity.
3. **Retire Options A, B, C, D, and E from active consideration.** A and B
   require a local file. C breaks the local-VFS UX. D is a symptom fix. E
   is mvSQLite-shaped, and the mvSQLite machinery is unnecessary when we
   are single-writer.
4. **New follow-up stories US-025 through US-028:** actor-resume hydration,
   `sqlite_read_many` server op, VFS stride prefetch predictor, and the
   Option F design document.
