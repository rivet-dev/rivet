# SQLite VFS v2 — End-to-End Walkthrough

A book-length tour of how the v2 SQLite VFS works. Read this first if you're new to the project.

> **Read [`constraints.md`](./constraints.md) first.** Everything in this walkthrough derives from the C0–C8 constraint set there. The narrative below describes one specific architecture (Option D: sharded LTX + delta log) chosen against those constraints. If the constraints change, the walkthrough has to be revisited.

Companion documents:
- [`constraints.md`](./constraints.md) — the locked constraint set and architectural decision rationale.
- [`design-decisions.md`](./design-decisions.md) — running log of corrections, action items, and the protocol sketch.

> **Status (2026-04-15):** Design phase. v2 has not been implemented. v1 is what ships today. This walkthrough describes the *intended* v2 behavior under Option D.

> **Note:** Earlier sections of this walkthrough still describe a `LOG/` + `PAGE/<pgno>` layout from an earlier draft. That layout has been superseded by the sharded `SHARD/` + `DELTA/` layout per `constraints.md` Option D. The chapter structure and concepts (atomic head pointer, fencing, materialization, prefetch, preload, recovery) carry over almost unchanged — just substitute "shard" for "PAGE" and "delta" for "LOG" while reading. A full rewrite of the chapters against Option D is pending.

---

## Chapter 1 — Why we need a new VFS at all

Every SQLite database is, at its core, a numbered array of fixed-size pages. SQLite's own engine doesn't care where those pages physically live. It calls a small set of C functions — read this page, write this page, tell me the file size — and lets the host environment supply the implementation. That set of functions is called a **VFS** (Virtual File System).

The standard VFS reads and writes pages to a real file on a real disk. Ours can't do that. Rivet actors run in environments with no useful local disk: the actor process can be killed and rescheduled at any time, and any local file would vanish with it. The only durable storage we have is the actor's **KV subspace**, a key-value store backed by UDB (Rivet's wrapper around the postgres or rocksdb drivers).

So our VFS has to translate SQLite's "read page 1234" into "fetch the right byte range from the KV store, somehow." The current implementation (we'll call it **v1**) does the most obvious thing: it gives every SQLite page its own KV key. Page 1234 lives at key `(SQLITE, MAIN, 1234)`. To read it, you do one KV get. To write it, you do one KV put.

That works, but it's slow. Two facts make v1 painful:

1. **A KV round trip is much slower than a disk read.** On a real disk, reading 50 random pages is maybe 5 milliseconds. Through the KV path it's 50 round trips to the engine, each one involving serialization, network, and an actual UDB transaction. We need to amortize KV calls across many pages.

2. **The engine puts a per-call ceiling on how much one KV put can carry: 128 keys, 976 KiB total per put, 128 KiB per individual value.** SQLite advertises atomic batched writes via `SQLITE_IOCAP_BATCH_ATOMIC`, and our v1 VFS handles the `BEGIN/COMMIT_ATOMIC_WRITE` callbacks. When a transaction touches more than 128 dirty pages, the v1 VFS returns `SQLITE_IOERR` from `COMMIT_ATOMIC_WRITE`. SQLite **catches that error and falls back to its rollback-journal path** — issuing dozens to hundreds of small writes one at a time. The transaction still succeeds; it just takes 3–10× longer.

Our goal for v2 is to fix both: make cold reads cheap by batching, and let large transactions commit through one fast path instead of falling back to the slow journal path. v2 is **purely a performance optimization** — there is no correctness gap in v1, just a cliff in throughput when transactions exceed the per-call envelope.

The constraints we have to honor:
- No local disk. Every byte of state has to be in the KV store or in actor RAM.
- One writer per actor. Rivet schedules at most one actor process at a time, but the engine layer does *not* fence concurrent writers strongly today, so v2 has to add its own fencing (see Chapter 5).
- KV limits as exposed by the engine layer today. We are free to add new KV ops with different limits — see [`design-decisions.md`](./design-decisions.md) for the proposed `kv_sqlite_*` op family.
- The on-the-wire runner protocol can be extended (new ops in a new schema version) but existing ops can't be mutated.

That's the playing field. Let's build something on it.

---

## Chapter 2 — Meet the cast

Before we describe v2, you need a mental model of three things: SQLite's VFS interface, the actor KV, and the LTX file format. They are the three pieces v2 stitches together.

### SQLite's VFS, in plain terms

SQLite's view of a database is "a file you can read bytes out of and write bytes into." When you run a query, SQLite figures out which 4 KiB chunks of that file it needs. For each chunk, it calls the VFS:

- `xRead(file, buffer, offset, length)` — fill the buffer from this offset.
- `xWrite(file, buffer, offset, length)` — store these bytes at this offset.
- `xTruncate(file, length)` — the file is now this many bytes long.
- `xFileSize(file, *out)` — how big is the file?
- `xLock`, `xUnlock` — coordinate concurrent access (we no-op these because we have one writer).
- `xFileControl(...)` — a grab-bag escape hatch SQLite uses for special commands. The important one for us is `BEGIN_ATOMIC_WRITE` / `COMMIT_ATOMIC_WRITE`, which is SQLite saying *"I'm about to do a batch of writes; treat them as one atomic unit."*

SQLite also opens *several* files for one logical database: the main DB file, plus optionally a rollback journal, a WAL file, and a shared-memory file. Our VFS sees them all and tags each one (`FILE_TAG_MAIN`, `FILE_TAG_JOURNAL`, etc.) so we know which is which.

The SQLite engine doesn't know or care what's behind these calls. As far as it's concerned, the VFS could be a real disk, a network filesystem, or a pile of Rust code talking to a remote KV store. That's our opening.

### The actor KV

Inside the engine, every actor gets its own private slice of UDB called a **subspace**. From the actor's side, the API is small:

- `kv_get(keys)` — fetch a list of keys, get back values.
- `kv_put(keys, values)` — write a batch of key/value pairs, atomically.
- `kv_delete(keys)` — delete a list of keys.
- `kv_delete_range(start, end)` — delete every key in a range.
- `kv_list(prefix or range)` — scan keys in order.

Each `kv_put` becomes exactly one UDB transaction inside the engine. That's the unit of atomicity we have to work with: anything that needs to commit together has to fit in one `kv_put` call, or it has to be split using a clever protocol.

Limits we must respect today, repeated for emphasis: **128 keys, 976 KiB, 128 KiB per value, 2 KiB per key.** The only enforced UDB-level constraint is the **5-second transaction timeout** — there is no FDB-style 10 MB transaction-size limit in our actual postgres/rocksdb drivers, only the timeout.

For v2, we will add a new SQLite-dedicated op family (`kv_sqlite_commit`, `kv_sqlite_materialize`, `kv_sqlite_preload`) with much larger envelopes and a fencing token in the request. See [`design-decisions.md`](./design-decisions.md) for the protocol sketch.

### LTX, the file format

LTX is a binary format invented at Fly.io for shipping SQLite changes around. It is *just a file format* — there's no storage system or runtime attached to it. An LTX file describes "a set of pages that were modified by a transaction," and that's it. You can think of it as a tiny self-contained patch:

```
[ Header: 100 bytes ]
  - the page size of the database
  - how big the database is (in pages) AFTER applying this patch
  - a transaction ID range
  - a checksum of the database BEFORE this patch (we set this to zero)
  - a timestamp

[ Page block: variable ]
  for each modified page, in ascending page-number order:
    - 6-byte page header (page number + flags)
    - 4-byte size of the compressed data
    - LZ4-compressed page bytes

[ Page index: variable ]
  - varint-encoded map of (page number → byte offset within file)

[ Trailer: 16 bytes ]
  - a checksum of the database AFTER this patch (we set this to zero)
  - a checksum of the LTX file itself (we set this to zero)
```

Three things make LTX useful for us:

1. **It compresses pages with LZ4.** A 4 KiB SQLite page typically ends up around 1–2 KiB after LZ4. That's a 2–4× density win.
2. **It packs many pages into one self-verifying blob.** We can write the blob into one KV value (or split it across a few KV values when it's too big).
3. **The format already exists with mature encoders/decoders.** The `litetx` Rust crate (Apache-2.0) is on crates.io. We don't have to invent or port the encoder.

LTX is not magical. It does not store pages, it does not apply itself, it does not know about KV. It is a serialization format. We will use it as the wire format for our **write-ahead log**, and we will write our own code to interpret and apply it.

**One thing we explicitly drop: the rolling PostApplyChecksum.** LTX's checksum is a running CRC64 maintained by XOR-ing new page bytes in and old page bytes out. It exists so LiteFS replicas can verify they're in sync. We don't do replication, SQLite has its own page integrity, and UDB guarantees byte fidelity. We zero out the checksum bytes and skip the rolling-state machinery entirely.

---

## Chapter 3 — The big idea: two forms of storage living side by side

The central insight of v2 is that **a database needs two simultaneous representations** in our KV store:

1. A **materialized form**, where each page is its own KV key, addressable in O(1). This is what reads ultimately come from.
2. A **log form**, where each transaction is a packed LTX blob. This is what writes go into first.

Why both? Because they optimize for opposite things.

The materialized form is fast to read but expensive to write large transactions into: writing 500 dirty pages means at least 500 KV writes spread across multiple round trips, with no atomicity across the boundary.

The log form is fast and atomic to write — even huge transactions land as one logical entry — but it's expensive to read from, because you'd have to scan it to find the latest version of any given page.

By keeping both, v2 gets the best of each: writes go into the log first (cheap, atomic at any size), and a background process moves pages from the log into the materialized form (so reads stay fast). The system is briefly redundant — newly-written data lives in *both* places until the materializer catches up — but that's the price of the trade.

Here's the layout, scoped under each actor's subspace, prefixed with a **schema version byte (`0x02`)** so v1 and v2 actors never share keys:

```
v2/META                      → DBHead (one small struct, ~80 bytes)
v2/PAGE  / pgno_be32         → 4 KiB page bytes  ← the materialized form
v2/LOG   / txid_be64 / frame_be16 → LTX frame bytes ← the log form
v2/LOGIDX/ txid_be64         → LTX header + page index (small)
```

`META` is the single source of truth. It records which transaction ID is the latest *committed* state, which is the latest *materialized* state, the database size in pages, and a few other fields:

```rust
struct DBHead {
    schema_version:    u32,    // 2
    generation:        u64,    // fencing token — incremented on each runner takeover
    db_size_pages:     u32,    // SQLite "Commit" — file size in pages
    page_size:         u32,    // 4096
    head_txid:         u64,    // last committed LTX txid
    materialized_txid: u64,    // largest txid fully merged into PAGE/
    log_min_txid:      u64,    // oldest LTX still in LOG/
    next_txid:         u64,    // monotonic counter — never reuses
    creation_ts_ms:    i64,
}
```

The invariant is simple: **a transaction is committed if and only if `META.head_txid` references it.** Everything else is bookkeeping. We will return to this when we discuss atomicity in Chapter 5.

`LOGIDX/<txid>` is a small auxiliary entry that holds just the LTX header and page index — no page bodies. It exists so that the VFS can quickly answer "which pages are dirty in unmaterialized transactions?" without fetching gigabytes of LTX frames. We'll see it in action in the read path and in startup.

---

## Chapter 4 — Writing a page

Let's trace what happens when an actor runs `UPDATE users SET balance = balance + 100 WHERE id = 42`.

SQLite parses the SQL, plans the query, and figures out it needs to update one row. That row lives on, say, page 73 of the users table. SQLite fetches page 73 (we'll cover reads in Chapter 6), modifies the row in its in-memory copy, and now needs to write page 73 back. It also needs to update an index, which dirties page 102. And it touches the database header on page 1.

So SQLite has three dirty pages: 1, 73, and 102. It opens what it calls a **batch atomic write window** and starts calling our VFS:

```
xFileControl(BEGIN_ATOMIC_WRITE)
xWrite(page 1, ...)
xWrite(page 73, ...)
xWrite(page 102, ...)
xFileControl(COMMIT_ATOMIC_WRITE)
```

Why does SQLite use this special window instead of just calling `xWrite` three times directly? Because we told it to. When the VFS was registered, we set the flag `SQLITE_IOCAP_BATCH_ATOMIC` in our device characteristics. That tells SQLite: *"I support atomic batched writes. If you tell me a group of writes goes together, I will commit them as a unit. You can skip writing the rollback journal."*

Inside `xWrite`, our VFS does the simplest thing: it stuffs each page into an in-memory `BTreeMap<u32, Vec<u8>>` called the **dirty buffer**. No KV calls happen during `xWrite`. We're just collecting.

When `COMMIT_ATOMIC_WRITE` arrives, the real work begins. The VFS now has to take the dirty buffer and turn it into a durable, atomic commit. There are two paths it can take, depending on size.

### The fast path: one round trip

Most transactions are small — a handful of pages. For these, the VFS encodes the dirty buffer as a single LTX **frame**. A frame is just a chunk of an LTX file: a header, a sequence of LZ4-compressed pages, an index, and a trailer. Three pages of a typical SQLite database might compress to about 6 KiB.

Now the VFS allocates `new_txid` from `head.next_txid` (a durable monotonic counter — never reused even after a crash) and computes:
- `new_head = DBHead { head_txid: new_txid, next_txid: new_txid + 1, db_size_pages: ..., generation: head.generation, ... }`

And issues **one** `kv_sqlite_commit` op:

```
kv_sqlite_commit(actor, generation = head.generation, expected_head_txid = head.head_txid,
    log_writes = [(LOG/<new_txid>/0, frame_bytes), (LOGIDX/<new_txid>, idx_bytes)],
    meta_write = encoded_new_head,
)
```

The op is implemented engine-side as a single UDB transaction that does:
1. CAS check: read META, verify `generation` and `head_txid` match the expected values. If not, fail with `KvSqliteFenceMismatch` and the writer must abort.
2. Range-delete `LOG/<new_txid>/0..` to clear any orphans from a previous crashed attempt at this same `next_txid` (defensive — should never trigger because `next_txid` is monotonic).
3. Write all log_writes.
4. Write the new META.
5. Commit.

Either all of that lands or none of it does. The transaction is committed the instant the engine acknowledges this op. Total cost: one KV round trip.

### The slow path: multi-phase commit

What about a transaction that dirties 5,000 pages? Even at 2 KiB compressed per page, that's 10 MB. There's no way that fits in one `kv_sqlite_commit` call. The VFS has to split it.

Here's the sequence:

```
1. Encode dirty buffer as N LTX frames, each sized to fit comfortably in
   one kv_sqlite_commit envelope.

2. PHASE 1 — stage the frames in LOG/, but DO NOT touch META yet.
   For each batch of frames that fits in one kv_sqlite_commit_stage:
       kv_sqlite_commit_stage(actor, generation, txid = new_txid,
           writes = [LOG/<new_txid>/0, LOG/<new_txid>/1, ...])

   Each of these is its own UDB transaction. They are NOT atomic with
   respect to each other. If we crash halfway through, only some frames
   are written. The first stage call also issues a defensive
   range-delete of LOG/<new_txid>/* to wipe any orphans.

3. PHASE 2 — flip the head pointer.
   kv_sqlite_commit(actor, generation, expected_head_txid = head.head_txid,
       log_writes = [(LOGIDX/<new_txid>, idx_bytes)],
       meta_write = encoded_new_head)

   THIS is the commit. The instant this op returns, the transaction is
   durable and visible.
```

The key insight is **nobody can see the LOG/<new_txid> entries until META points to them.** Phase 1 writes are addressed under a `new_txid` that is greater than `head.head_txid`. The read path and the recovery path both ignore txids beyond `head.head_txid`. So Phase 1 is invisible until Phase 2 lands.

If we crash during Phase 1, those frames become orphans. The next actor startup will notice them and clean them up (Chapter 8).

If we crash during Phase 2, Phase 2 is one engine op which is one UDB transaction. It either commits or it doesn't. There's no half-state.

After the commit returns, the VFS does one more bookkeeping step: it updates its in-memory page cache and `dirty_pgnos_in_log` map *atomically together* with the success acknowledgement. (See Chapter 5 for why "atomically together" matters.)

---

## Chapter 5 — Atomicity, fencing, and the head pointer

It's worth pausing on the atomicity argument because it's the load-bearing claim of the whole design. And it has to defend against more failure modes than I originally thought.

### The basic head pointer pattern

A reader determines what's committed by reading `META.head_txid`. Anything with txid ≤ `head.head_txid` is committed. Anything with txid > `head.head_txid` is not — it's either uncommitted in-flight data (Phase 1 in progress) or junk from a crashed transaction (Phase 1 succeeded, Phase 2 didn't).

So there are exactly three possible outcomes for any given commit attempt:

1. **Phase 1 not yet finished, or Phase 2 not yet started.** META still points at the old head. Readers see the old database. The new frames sit in LOG/ but are unreachable.
2. **Phase 2 in flight.** From the engine perspective, the op either succeeds atomically or fails atomically. There is no observable midpoint.
3. **Phase 2 succeeded.** META now points at the new head. The frames in LOG/ are now reachable. Readers see the new database.

This is the same trick that gives a journaling filesystem its atomicity: the journal commit record is the single small write that flips the world.

### Why we need fencing tokens

The basic pattern above is correct **if there is at most one writer.** Rivet runs one actor process per actor at a time, but the engine's actor-to-runner ownership check happens in a *separate* UDB transaction from each `kv_put`. There is a brief window during runner reallocation where two processes can both believe they own the actor. Without explicit fencing, both can issue commits that interleave on `LOG/<txid>/` keys and corrupt the database.

v2 fixes this with two mechanisms:

1. **A `generation` field in META.** Incremented every time the actor is reallocated to a new runner. The new runner reads META, sees the old generation, increments it, writes it back as part of its first action.
2. **CAS on every commit.** Every `kv_sqlite_commit` op carries `(expected_generation, expected_head_txid)`. The engine-side op reads META, verifies both fields match, and aborts if they don't. An old runner whose generation is stale cannot commit.

This makes the head pointer pattern robust under concurrent writers. The engine layer enforces the fencing; the VFS just supplies the expected values.

### Why we need a monotonic txid counter

The naive "next txid is `head.head_txid + 1`" allocation is broken because crashed attempts leave orphan LOG/ frames that a new attempt can collide with. Specifically: if attempt A crashes after writing `LOG/12/0..2`, and attempt B then computes `new_txid = head.head_txid + 1 = 12`, B's Phase 1 only `clear_subspace_range`s the keys it writes itself. Stale frames from A persist alongside B's frames at the same txid, and the materializer will eventually decode garbage.

v2 fixes this by storing `next_txid` in META as a strictly monotonic counter. Each commit advances it. Crashed attempts leak under their own unique txid which is then never reused. Recovery cleans them up by listing `LOG/` for txids > `head.head_txid` and deleting them. There is no collision possible.

### Why the materializer needs combined writes

The materializer reads LOG/ frames, merges them by latest-txid-wins, and writes the result into PAGE/. The naive "kv_put PAGE/+META, then kv_delete_range LOG/" sequence has a dangerous middle window: between the two ops, the in-memory `dirty_pgnos_in_log` map can be in any state, and a concurrent reader can either see stale PAGE bytes (because the map still says "go to LOG" but LOG is gone) or stale LOG bytes (because the map hasn't been updated yet). No ordering of these three updates is safe.

v2 fixes this with a third dedicated op: `kv_sqlite_materialize(actor, generation, expected_head_txid, page_writes, range_deletes, meta_write)`. The engine implements this as one UDB transaction that does all three things at once. The actor-side `dirty_pgnos_in_log` update happens after the op succeeds, inside the same critical section as the page cache update.

---

## Chapter 6 — Reading a page

Now the other direction: SQLite calls `xRead(pgno=73, ...)`. What does the VFS do?

It's a four-level lookup, fastest first:

```
1. Page cache?
   The VFS keeps an LRU cache of recent pages in actor RAM.
   If the page is here, return it. Zero round trips.

2. Write buffer?
   If we're in the middle of an open SQLite transaction and we've already
   dirtied this page, the freshest version is in the dirty buffer.
   Return it. Zero round trips.
   (Note: SQLite's own pager also caches dirty pages and usually
   intercepts the read before it reaches the VFS. This layer is a safety
   net for any case where SQLite does send a dirty-page read through.)

3. Unmaterialized log?
   Some recent committed transactions may be in LOG/ but not yet copied
   to PAGE/. The VFS keeps a small in-memory map called dirty_pgnos_in_log
   that maps (page number → (txid, frame_idx)) for the most recent log
   entry containing that page. If page 73 is in this map, fetch the LTX
   frame from LOG/<txid>/<frame_idx>, decompress it, extract the page,
   populate the cache. One round trip.

   Important: if the LOG frame is missing (because the materializer
   raced and deleted it), retry the lookup against fresh state — by
   that point, the materialize op will have updated the in-memory map
   to remove this entry, so the retry falls through to step 4.

4. Materialized PAGE/.
   This is the common case. Fetch PAGE/<pgno> from the KV store.
   But before issuing the kv_get, run the page number through a prefetch
   predictor that suggests other pages we are likely to need next.
   Issue ONE kv_get with the target page plus the predicted ones.
   Populate the cache for everything that comes back.
   One round trip per *prefetch group*, not per page.
```

The prefetch predictor is the same idea mvSQLite uses. It watches access patterns: if you just read page 5, then 8, then 11, it learns the stride is +3 and predicts 14 next. It's about 1.5 KB of state, runs in nanoseconds, and turns a sequential scan from "one round trip per page" into "one round trip per N pages." For random-access workloads (B-tree seeks) it doesn't help much — the parallel sub-agents are evaluating exactly how much.

### Isolation guarantees inside a transaction

A common worry: *what if SQLite reads a page that's currently dirty in the active transaction?*

Answer: **SQLite's own pager handles this before the read reaches our VFS.** Inside an open transaction, when SQLite needs to read a page it has already modified, it serves the read from its in-process page cache. Our `xRead` callback is only called for pages SQLite hasn't already pulled in. With `locking_mode=EXCLUSIVE` and one connection per actor, there is no concurrent reader who could observe an in-flight transaction.

So v2 has the **exact same isolation semantics as native SQLite** — it's the same SQLite engine making the calls, we're just changing where the bytes physically live underneath.

---

## Chapter 7 — Cold startup

When Rivet starts an actor, the VFS has to get from "nothing in memory" to "ready to serve queries" as fast as possible. v2 startup is designed to be **one** KV round trip in the common case, by leveraging the engine's existing `batch_preload` primitive (`actor_kv/preload.rs:53`):

```
Round trip 1: kv_sqlite_preload(
    get_keys = [v2/META, v2/PAGE/1],
    prefix_scans = [v2/LOGIDX/]  -- bounded
)

  In one UDB transaction:
  - Reads META and page 1 (the SQLite header).
  - Range-scans all LOGIDX/ entries (small — header + page index per txid).
  - Optionally also scans a configurable extra set of "warm" keys
    that the user has flagged as preload-on-startup (see "Preload
    hints" below).

  After this returns, the actor:
  - Knows head, materialized_txid, db_size_pages.
  - Has page 1 in its cache.
  - Has built dirty_pgnos_in_log from the LOGIDX entries.
  - Has whatever the user preloaded warm in cache.
```

That's it. A 10 GB database with millions of pages opens in one round trip, because we never load the bulk of the data — we just learn where to find it.

### Preload hints

v2 makes preload a first-class feature. The actor can declare, at registration time, a list of:
- **Specific keys** to preload (e.g., the root pages of frequently-queried tables).
- **Page ranges** to preload (e.g., "the first 100 pages of the database" — likely to contain hot schema/index roots).
- **Tagged ranges** that the application can hint into (e.g., "the materialized view I always read first").

The `kv_sqlite_preload` op takes all of these in one call and returns everything in a single UDB transaction. This is dramatically better than v1's bounded-byte preload because the user can target *specific* pages they know they'll need rather than relying on the engine to guess.

Preload also matters for **testing**: see [`design-decisions.md`](./design-decisions.md) for how the test harness uses preload to set up deterministic page state for unit tests.

---

## Chapter 8 — Crash recovery

Actors die. The whole point of Rivet's architecture is that any actor can be killed at any moment and rescheduled. So our recovery story has to be airtight.

The good news is that crash recovery in v2 is simple, because every committed transaction has a single observable instant (the META update in Phase 2). If the actor died before that instant for transaction T, then T did not commit, and any partial work for T is junk to be cleaned up.

The recovery routine on startup, after `kv_sqlite_preload`:

```
1. Read META → head (already done by preload).
2. The new actor immediately calls kv_sqlite_takeover(generation = head.generation + 1)
   which CASes META to bump the generation. This fences off any old runner
   that might still be alive.
3. List LOGIDX entries with txid > head.head_txid via kv_list.
   These are orphans: Phase 1 succeeded for some transaction whose
   Phase 2 never ran.
4. For each orphan txid:
     kv_delete_range(LOG/<txid>/0, LOG/<txid+1>/0)
     kv_delete(LOGIDX/<txid>)
5. Done. The actor is now in a consistent state.
```

A subtle point: orphan deletion is **idempotent**. If recovery itself crashes halfway through, the next attempt picks up where the last one left off, because "list LOGIDX entries with txid > head.head_txid" still returns the remaining orphans.

What if the previous actor was still running mid-Phase-1 when we took over? Its next `kv_sqlite_commit_stage` will fail the generation CAS (because we bumped it in step 2), so it cannot complete its commit. Its in-flight LOG/ writes become orphans that we (or the next startup) will clean up.

---

## Chapter 9 — The background materializer

If we only ever wrote into LOG/, the log would grow forever. The materializer's job is to fold LOG entries into PAGE/ entries and prune the log.

The materializer is a background task running inside the actor. It wakes up when the log has unmaterialized work and does this:

```
1. Read materialized_txid and head.head_txid from in-memory mirror.
2. Pick a budget: at most B pages or T transactions per pass.
3. Fetch the LTX frames for the next batch of txids.
4. Decode them. Merge by "latest txid wins" — strict txid order, never
   skipping.
5. Issue ONE kv_sqlite_materialize call:
     - page_writes: the merged pages
     - range_deletes: LOG/ and LOGIDX/ entries for the merged txids
     - meta_write: new head with advanced materialized_txid
   Engine commits all three in one UDB transaction.
6. After the op returns, atomically update the in-memory page cache
   AND remove the merged pgnos from dirty_pgnos_in_log.
```

The materializer is **asynchronous with respect to writes** but **bounded in lag**. If it falls too far behind — say, more than a configurable threshold — the writer can throttle or block until it catches up. We don't want LOG/ to consume the actor's whole 10 GiB quota.

A subtle benefit of merging by "latest wins": if a hot page gets written 100 times in 100 different transactions, the materializer ends up writing it to PAGE/ once, not 100 times. So even though we have a log layer, the steady-state write amplification on hot pages is much closer to 1× than to N×.

---

## Chapter 10 — How we lie to SQLite, and why it works

We've established that v2 wants SQLite to:
- Group all writes for a transaction into one batch (so we can serialize them as one LTX entry).

We get this by setting these pragmas when we open the connection (these are unchanged from v1):

```sql
PRAGMA page_size      = 4096;
PRAGMA journal_mode   = DELETE;     -- KEEP. Do not change to MEMORY.
PRAGMA synchronous    = NORMAL;     -- KEEP. Do not change to OFF.
PRAGMA temp_store     = MEMORY;
PRAGMA auto_vacuum    = NONE;
PRAGMA locking_mode   = EXCLUSIVE;
```

Plus the device-characteristics flag `SQLITE_IOCAP_BATCH_ATOMIC`, which tells SQLite "I can atomically commit a batch of writes; you don't need to journal first."

**An earlier draft of this doc proposed `journal_mode = MEMORY` and `synchronous = OFF`. We reverted that.** Per a SQLite forum thread, that combination has had bugs where writes leak outside the batch atomic group, and we don't have empirical evidence today that `IOCAP_BATCH_ATOMIC` actually elides journal writes for our workload — the bench shows a 1 MiB insert producing 287 puts, which is consistent with the journal-fallback path being taken. Until we can confirm the elision is happening, we keep the safe pragma defaults from v1 and let the journal fallback live.

The performance win of v2 does not depend on those pragmas. It depends on the LTX-framed log replacing the journal fallback path. When SQLite *does* take the atomic-write path, our handler builds an LTX frame and writes it through `kv_sqlite_commit`. When SQLite falls back to the journal path (for transactions exceeding the pager cache, schema changes, etc.), v1 behavior remains — slow but correct.

---

## Chapter 11 — Edge cases and gotchas

A few things that aren't quite as smooth as the main story.

### The lock page

SQLite has a quirk: the page that contains the byte at offset 1 GiB (page number `1073741824 / page_size + 1` = 262,145 for 4 KiB pages) is reserved as the "lock page" and never contains data. Our LTX encoder has to skip it. The `litetx` crate handles this for us if we use its built-in helpers.

### Page 1 special-case

Page 1 is the SQLite header page. It contains the schema cookie, file format version, and a few counters that SQLite uses to invalidate its in-memory state when the database changes externally. v2 always preloads page 1 on startup so SQLite can open the connection.

### VACUUM

VACUUM rewrites the entire database into a temp file inside one transaction. SQLite opens that temp file with a name our `resolve_file_tag` doesn't recognize today. **VACUUM is unsupported in v2.0.** If a user runs it, they get an error. Future v2.x can grow temp-file handling if there's demand. (`auto_vacuum = NONE` is in our pragma defaults so SQLite won't try to do it automatically.)

### `dirty_pgnos_in_log` size bounds

The map can grow if the materializer falls behind. We bound it implicitly by bounding the LOG itself (Chapter 9). If we hit the bound, the writer back-pressures.

### v1 ↔ v2 separation

There is no migration. v1 actors stay v1 forever. v2 actors start v2 and stay v2 forever. The dispatch happens at actor open time by reading the schema-version byte of the first key in the actor's KV subspace. If the actor's subspace is empty (brand new actor), dispatch is by config — new actors created after a flag-flip get v2.

This means: if we ever want to move an existing v1 actor to v2, the user has to do it themselves (export, recreate, import). We're not building automation for it.

---

## Chapter 12 — A day in the life

Let's tie it all together with a concrete walkthrough.

### Morning: actor boot

The Rivet engine schedules our actor. The actor process starts. The SQLite connection opens, which triggers VFS registration. The VFS:

1. `kv_sqlite_preload(get_keys=[v2/META, v2/PAGE/1], prefix_scans=[v2/LOGIDX/])` — one round trip, ~50 ms.
2. The recovery routine bumps `generation`, lists orphan LOGIDX, deletes any. (No orphans on a clean restart.) Another ~5 ms.

SQLite is now ready to serve queries.

### Mid-morning: a small read

`SELECT * FROM users WHERE id = 42`. SQLite walks the B-tree on the users table, calling `xRead` for each page. The first read is a cache miss; the prefetch predictor has nothing yet. The next few reads start training the predictor. Total: 2–3 round trips, ~30 ms cold, 0 ms warm.

### Late morning: a small write

`UPDATE users SET balance = balance + 100 WHERE id = 42`. SQLite dirties 4 pages and calls `BEGIN/COMMIT_ATOMIC_WRITE`. The VFS encodes them as one LTX frame and issues one `kv_sqlite_commit`. One round trip, fully atomic.

### Afternoon: the materializer wakes up

After 6 commits accumulate, the materializer issues one `kv_sqlite_materialize` call merging them into PAGE/. One UDB transaction, one round trip.

### Evening: a giant write

A CSV importer ingests 100,000 rows, dirtying 8,000 pages. The VFS encodes them as ~50 LTX frames. Phase 1 stages them across ~8 `kv_sqlite_commit_stage` calls (8 round trips). Phase 2 flips META in one `kv_sqlite_commit` (1 round trip). Total: 9 round trips for an 8,000-page commit. Compare to v1, which would fall back to the journal-mode path with hundreds of small writes.

### Night: a crash

The actor dies between Phase 1's 6th and 7th stage call for some unrelated reason. Six frames sit in `LOG/1044/{0..5}` but META still points at 1043.

Rivet reschedules the actor. It boots:

1. `kv_sqlite_preload` reads META (head_txid=1043), LOGIDX (no entries for 1044 because Phase 2 never ran), page 1.
2. Recovery bumps generation, lists LOGIDX for txid > 1043. None found. But: it also lists `LOG/` for txid > 1043 to catch the orphans. Finds `LOG/1044/0..5`. Deletes them.
3. The actor is back in a consistent state. The application code that issued the giant write will get an error from its previous SQLite call (the connection died), and is responsible for retrying if it wants to.

---

## Where to go next

That's the end-to-end picture. Companion docs in this folder:

- [`design-decisions.md`](./design-decisions.md) — corrections to earlier drafts, action items, fixes for adversarial review findings, the full `kv_sqlite_*` op family sketch.
- (To come) `workload-analysis.md` — quantitative comparison of v1 and v2 across large reads, aggregations, and point reads/writes. Generated by parallel sub-agents.
- (To come) `test-architecture.md` — virtual KV driver design, preload-aware test harness, deterministic test fixtures. Generated by parallel sub-agent.
- (To come) `kv-protocol-extensions.md` — the new `kv_sqlite_*` runner-protocol ops and their engine-side implementations.
