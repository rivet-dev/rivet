> **Stale numbers (2026-04-15):** Computed at 2.9 ms local-dev RTT. Per `constraints.md` C6, the production target is ~20 ms. Multiply round-trip-bound numbers by ~7×. Qualitative findings still hold. Recompute pending implementation.

# Workload Analysis — Point Reads and Point Writes

> **Status (2026-04-15):** Design-time analysis. v2 is unimplemented. All numbers below are estimates derived from the v1 code path (`rivetkit-typescript/packages/sqlite-native/src/vfs.rs`), the engine KV implementation (`engine/packages/pegboard/src/actor_kv/mod.rs`), the v2 design (`walkthrough.md` + `design-decisions.md`), and the observed `examples/sqlite-raw/BENCH_RESULTS.md` trace (1 MiB insert = 287 puts, 30 gets, ~856 ms in put time).

This is the hostile case for v2. Log-structured storage with background compaction is textbook-terrible at hot-row OLTP. The value proposition of v2 is large-transaction commits and cold-read prefetch, not sub-millisecond point ops. The goal of this document is to *quantify* how bad — or not — the regression is, and to flag the tuning knobs that mitigate it. Every scenario below assumes a per-op KV round trip of **≈ 2.5 ms** (optimistic, local engine) up to **≈ 5 ms** (remote / loaded). Where a single number is useful I pick the midpoint of 3.5 ms.

---

## Scenario 1 — `UPDATE counter SET val = val + 1 WHERE id = 1` × 1000

The canonical hot-row update. Single row, same page, every iteration. SQLite's own pager serves the read of the counter row from its in-process cache after the first iteration, so the "read phase" of each UPDATE goes nowhere near the VFS. What the VFS *does* see is the write-back of the modified pages at commit.

**Dirty page count per commit.** In practice SQLite dirties:
- Page 1 — the database header, because `change_counter`, `schema cookie`, and `file_change_counter` all live there and SQLite bumps one of them on every write txn.
- The leaf page holding the row — 1 page.
- The root page of the table's B-tree — usually the same page as the leaf for a tiny counter table, so merge to 1 or 2 pages.
- Freelist tracking when the pager allocates a dirty slot — 0 extra for an in-place update.

Call it **3 dirty pages per commit** for the worst case, **2** for the common case. The analysis below uses 3 for conservatism.

### v1 — per-commit cost

Each commit goes through `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE`. `kv_io_file_control` collects the dirty buffer (3 entries), adds the meta key, and issues **one** `kv_put` call with 4 keys. One KV round trip per commit.

- 1 commit: ~3.5 ms.
- 1000 commits: ~**3500 ms** total.
- Total KV page writes: `1000 commits × 3 pages = 3000`. Plus ~1000 meta writes. **~4000 KV key-writes over 1000 commits.**

This is literally the best case for v1: the transaction fits comfortably inside the 128-key / 976 KiB envelope, no journal fallback. v1 behaves well here.

### v2 — per-commit cost (fast path)

Each commit still has 3 dirty pages. The v2 write path:

1. `xWrite` calls buffer pages into the in-memory dirty buffer — free.
2. `COMMIT_ATOMIC_WRITE` encodes the 3 pages as one LTX frame. Three 4 KiB pages LZ4-compress to ~6 KiB of frame body (realistic compression on a dense B-tree leaf is ~2×). Add ~200 bytes for header / index / trailer. The frame is well under the ~1 MiB max value.
3. The VFS issues **one `kv_sqlite_commit`**: `log_writes = [LOG/<txid>/0, LOGIDX/<txid>]`, `meta_write = META`. 3 KV values, ~6 KiB payload, one UDB transaction, one round trip.

- 1 commit: **~3.5 ms**, the same as v1.
- 1000 commits: **~3500 ms**.

So far v2 ties v1 on wall-clock.

### v2 — materializer cost over 1000 commits

Here's where v2 starts to pay a tax. Each commit wrote 3 new LOG/ entries (2 LTX payload keys + 1 META rewrite). In steady state:
- `LOG/` accumulates ~2000 keys across 1000 commits (frame + logidx per txid).
- Background materializer wakes up periodically.

The materializer merges by "latest wins." Over the 1000 commits, **pages 1, root, and leaf were each rewritten 1000 times** but the final value is only 3 pages. With latest-wins merge:

- If the materializer runs **once** at the end: 3 PAGE/ writes + 1 META update + range-delete of 2000 LOG/ keys. **1 extra round trip.**
- If it runs on a per-pass budget (e.g., every 6 commits as Chapter 12 of `walkthrough.md` suggests): 1000/6 ≈ **167 extra round trips over the session**, each merging 6 txids worth of frames into ≤ 3 distinct pages. That's ~167 × 3.5 ms = **~585 ms of materializer work** running concurrently with (and therefore competing with) the 3500 ms of writer work.

**Total page-writes to KV over 1000 commits:**
- 1000 LTX frames into LOG/ (at ~6 KiB each) = 1000 writes.
- 1000 LOGIDX/ writes.
- 1000 META rewrites.
- Materializer: 167 passes × (3 PAGE/ writes + META + range-delete) ≈ 167 × 4 writes ≈ 668.
- **Grand total: ~3668 KV key-writes for 1000 logical UPDATEs**, compared to ~4000 on v1. Roughly even on write count but nearly 2× worse on *payload bytes* because LOG/ is a write-once log that then gets copied into PAGE/.

### Net comparison

| | v1 | v2 (fast path + materializer every 6 txids) |
|---|---|---|
| Writer wall clock | ~3500 ms | ~3500 ms |
| Writer round trips | 1000 | 1000 |
| Background round trips (same actor) | 0 | ~167 |
| KV payload bytes written | ~12 MB (3 × 4 KiB × 1000) | ~18 MB (LZ4-compressed LTX ×1000 + PAGE rewrites ×167) |
| Storage amplification peak | 1× | 2× (LOG/ + PAGE/ coexist for the latest pass) |
| Writer tail latency | 3.5 ms | 3.5 ms + possible materializer contention (2–5 ms extra if both run simultaneously over shared KV bandwidth) |

**Honest verdict:** v2 does not win here. It does not regress *wall clock* for the writer, but it doubles payload bytes, adds background work that competes for the same KV pipe, and slightly increases peak storage. The one benefit — latest-wins merging — saves us from a naive "1000 materializer rewrites per hot page" pathology but does not help against v1, which just wrote each page once in place.

**Recommended tuning:** For actors identified as hot-row-update-heavy, set the materializer lag target aggressively high (e.g., merge on idle only, or every 50+ commits) so the background cost batches up instead of interleaving with the write path. If the actor never reads, the materializer could be deferred entirely until the log approaches its back-pressure bound. Expose a config knob `sqlite.materializer.min_pass_txids` (default 6, raise to 50 for point-write workloads).

---

## Scenario 2 — `SELECT * FROM users WHERE id = ?` × 1000 with random id

Index seek, working set larger than the 5,000-page LRU. The B-tree descent on the `users_id` index is typically 3 pages (root → internal → leaf), plus the row data page in the table B-tree, totaling **~4 pages per query**. B-trees hit the root and upper internal pages for every query, so those pages are warm after the first handful of queries. The *leaf* pages and the *data* pages are what turn over.

Assume the table is 100k rows at ~128 bytes average = 12.8 MB ≈ 3,200 data pages. The index leaves are similar scale (assume 800 pages). With a 5,000-page LRU, **the entire working set fits** for this exact table shape. So this scenario needs to be evaluated at **two** densities: (A) working set fits, and (B) working set exceeds LRU (e.g., 1M rows).

### Case A — working set fits in LRU (100k rows)

After the first ~100 queries warm the cache, every subsequent query is a cache hit for all 4 pages. Random access still trains the predictor poorly but it doesn't matter because the cache absorbs everything.

- **v1:** 4 KV round trips per query on the first pass (v1 does not batch the 4 pages into one call because `xRead` is called separately for each page inside the B-tree walk). The optional `READ_CACHE_ENV_VAR` read-cache, when enabled, caches pages across reads and drops this to ~0 after warmup. Without the read cache, **v1 issues ~4 round trips per query forever**.
- **v2:** same 4 pages, but the LRU page cache is always on. After warmup, ~0 round trips per query. During warmup, the prefetch predictor speculatively batches together the 4 pages of the descent *if it can predict them*, which it probably can't on the first hit because B-tree descents are data-dependent (the child page to fetch depends on what was in the parent page, which you haven't read yet). In practice, v2 makes **1 round trip per B-tree level** that isn't cached, so ~4 round trips in the first cold pass, then 0.

Net for Case A: v2 is strictly better than v1 (forever-on LRU) but only meaningfully better than v1-with-read-cache-enabled during the first handful of queries.

- 1000 queries, v1 without read cache: 4000 round trips, ~14,000 ms.
- 1000 queries, v1 with read cache: ~400 round trips (cold warmup), ~1,400 ms.
- 1000 queries, v2: ~100 round trips (shorter cold warmup because LRU is permanently on), ~350 ms.

### Case B — working set exceeds LRU (1M rows, ~32k data pages)

Now the random accesses keep churning the cache. Each query still needs 3 index pages (root + 2 internal, usually warm) + 1 leaf (sometimes warm) + 1 data page (almost always cold).

- **v1:** ~2 round trips per query average (leaf sometimes warm, data page cold). 2000 round trips for 1000 queries, ~7000 ms.
- **v2:** Same structural 2 round trips per query. The prefetch predictor does *not* help here because the access pattern is genuinely random — page N being accessed says nothing about page N+1. mvSQLite's predictor is a bigram Markov chain, which only captures *spatial* or *transition* locality. Random-`id` OLTP has neither. Predictions emit below threshold, the predictor noops, v2 behaves like "v1 with an always-on cache."

**Cache hit rate estimate (Case B):**
- Index root + 2 internal levels (~30 pages total with a fat B-tree): always hot → hit rate 100%.
- Index leaf (~800 pages): ~16% hit rate for a 5,000-page LRU (but 5,000 − 30 − 30 ≈ 4,900 slots is way more than the 830 leaf pages, so actually 100%).
- Data pages (32,000 pages vs. ~4,000 remaining LRU slots): **~12.5% hit rate** (independent random sampling with replacement).

Effective round trips per query in Case B with v2 LRU: 0 (index) + 0.88 (data page). **~880 round trips over 1000 queries, ~3,080 ms.**

### v2 wins by a fixed constant

v2 wins Case A handily over v1-without-read-cache (~40×) and wins Case B modestly (~2.3×). The headline for this scenario is **v2 always has the LRU on, so it is strictly ≥ v1-without-read-cache and roughly comparable to v1-with-read-cache**. The unmaterialized-log layer is not exercised at all by this scenario (there are no writes), so the 4-layer lookup collapses to two effective layers: LRU and PAGE/.

**Materializer interaction:** none. The workload is read-only; LOG/ is empty.

**Recommended tuning:** Bump the LRU cache up for read-heavy actors. The default 5,000 pages is 20 MB per actor. If the host has room, raise to 20,000 pages (80 MB) to capture the hot data pages of a 1M-row table. Also: ship the index root/upper-internal pages as `preload.hints` so the very first query doesn't eat 3 cold reads on the descent.

---

## Scenario 3 — `INSERT INTO logs VALUES (...)` × 10,000 in 10,000 separate transactions

Append-only. Each transaction touches:
- The root of the `logs` table (updated because child pointer changes) — 1 page.
- The current rightmost leaf page of the B-tree (where the new row is appended) — 1 page.
- Page 1 header — 1 page.

That's **3 dirty pages per commit**, same as Scenario 1, and because `rowid` auto-increments, the same leaf page is hot until it fills up (~200 rows for small rows, then a new leaf is allocated). So the dirty set is "header + root + (current leaf)" with current-leaf rotating every ~200 commits.

### v1 — per-commit

- 1 commit = 1 round trip (3 keys + meta = 4 keys, well under 128).
- 10,000 commits = **10,000 round trips = ~35 seconds** of writer time.
- KV writes = 30,000 page writes + 10,000 meta writes ≈ **40,000 key-writes**.

### v2 — per-commit

- 1 commit = 1 `kv_sqlite_commit` fast path. 3 pages in an LTX frame ≈ 6 KiB compressed.
- 10,000 commits = **10,000 round trips = ~35 seconds** of writer time.
- KV writes: 10,000 LOG frames + 10,000 LOGIDX + 10,000 META = **30,000 key-writes** for the writer.

### Materializer cost

The materializer sees 10,000 LOG entries. If it runs once at the end, it needs to merge them into **~50 distinct PAGE/ writes** (the current leaf rotates 10,000 / 200 ≈ 50 times over the run). Plus page 1 (1 page) and the root (1 page). That's ~52 PAGE/ writes plus META and the LOG range-delete.

- If materializer runs once: 52 PAGE/ writes + 1 META + 1 range-delete = **1 big `kv_sqlite_materialize` call** (52 KV values, well under the 512-key / 9 MiB limit). ~1 round trip. **~52 amortized materializer writes vs. 10,000 LOG writes gives a 192× write-amplification *savings* on hot pages.**
- If materializer runs every 200 commits: ~50 passes, each merging 200 LOG entries into ~3 PAGE/ writes. 50 × 3 = 150 PAGE/ writes + 50 META. **50 extra round trips** for the materializer background, but still only ~200 total KV writes for pages.

**Net v2 writes: ~30,000 LOG + 150–200 materializer = ~30,200.**

**Net v1 writes: ~40,000.**

This is one of the rare point-op scenarios where v2 *wins* on write count, because the materializer's latest-wins merge collapses the 10,000 rewrites of the root page down to ~50. v1 just blindly rewrites the root 10,000 times because it has no log layer to deduplicate against.

**Wall clock is still dominated by the 10,000 synchronous commit round trips**, which is the same on both systems. The materializer runs concurrently and adds ~175 ms (50 passes × 3.5 ms) of background work, spread over the 35 seconds of writer time — 0.5% overhead.

**Materializer interaction:** neutral. Background work is small relative to foreground.

**Recommended tuning:** Default config works. Optionally, batch the materializer more aggressively (every 500 txids instead of 200) to cut background round trips further, since the workload can absorb up to ~2 MB of unmaterialized log without hitting the quota.

---

## Scenario 4 — `BEGIN; INSERT × 10,000; COMMIT` (one transaction)

Now we force all the inserts into one SQLite transaction. The dirty set grows as inserts pile up. With ~200 rows per leaf and 10,000 inserts, we dirty ~50 new leaves plus the root (probably promoted to a 2-level B-tree ~internal + 50 leaves = 51), plus page 1, plus the freelist / pointer-map overhead (~5 more pages). Call it **~57 dirty pages** total at commit time, maybe 70 with some freelist churn.

But here's the subtlety: **SQLite doesn't hold all dirty pages in its own pager forever.** The pager cache has a bound (default 2000 pages for `PRAGMA cache_size = -2000`), and when it fills, SQLite *spills* dirty pages early. In practice, for 10,000 inserts into a small table, the dirty set at commit time stays bounded around 50–70 pages because B-tree leaves keep getting reused as they fill and the next row goes to a fresh leaf.

### v1 — slow-path regime

- 57 pages fits in 128-key envelope. **v1 hits the fast path, 1 round trip.** v1 wins this case comfortably: a single 57-page commit is ~3.5 ms + the serialization overhead of 57 pages (~230 KiB payload, within envelope).
- Wall clock: ~5 ms commit + whatever SQLite took for the inserts themselves (in-memory pager, fast).

### v2 — fast path still applies

- 57 pages × ~2 KiB LZ4 = ~115 KiB of frame body. One `kv_sqlite_commit` fast path. **1 round trip.**
- Wall clock: ~5 ms commit. Same as v1.

So neither hits the slow path for 10,000 *small* inserts. **The question in the scenario description — "this forces the slow-path on v2 because 10,000 inserts dirty more pages than fit in one envelope" — is only true if inserts are individually large.** For 1 KiB row payloads, 10,000 inserts × 1 KiB = 10 MiB, which even after LZ4 at 2× is still 5 MiB, comfortably over the 1 MiB single-value cap and into the ~9 MiB payload ceiling. At that size:

- v1: **journal fallback.** 10 MiB across 128-key batches × 976 KiB caps = ~11 `kv_put` calls minimum, probably ~20 with journal-header rewrites, plus the page writes themselves. This is the BENCH_RESULTS 287-puts case scaled up 10×. Conservative estimate: **50–300 round trips, ~200–1100 ms.**
- v2: slow path. Phase 1 stages the LTX frames across ~2–3 `kv_sqlite_commit_stage` calls, then 1 `kv_sqlite_commit` Phase 2. **3–4 round trips, ~10–15 ms.**

### Net

For a truly giant single-transaction insert (the case where dirty pages blow out the envelope), v2 is **~20–100× faster** than v1. This is exactly where v2 was designed to win. But it's not really a "point op" — it's a bulk insert masquerading as one, and point-op analysis doesn't capture it.

For the "normal" case (10k small inserts = 57 dirty pages), v1 and v2 are identical (1 round trip each).

**Materializer interaction:** One big LTX entry lands. Materializer wakes up, processes one txid, merges 57 pages into 57 PAGE/ writes. One `kv_sqlite_materialize` call. Background cost ~5 ms, overlaps with the SQLite statement that follows.

**Recommended tuning:** None. Fast path on both cases. The only v2-specific knob is "what triggers Phase 2" and the default of "envelope full" is correct.

---

## Scenario 5 — Mixed: 100 reads + 10 writes per second sustained

A realistic production load. 100 point reads (4 pages each, Scenario 2 shape) + 10 point writes (3 pages each, Scenario 1 shape) per second, sustained for N seconds. Per second:

- Reads: 100 × 4 = 400 page touches, ~80% LRU hit after warmup → ~80 round trips/sec.
- Writes: 10 × 1 commit round trip = 10 round trips/sec.
- **Foreground: ~90 KV round trips/sec, ~315 ms wall-clock cost, ~31% of the 1000 ms second.**

### v1 vs. v2 for the foreground

Foreground is essentially identical. Writes: 10 × 3.5 ms = 35 ms. Reads: 80 × 3.5 ms = 280 ms. Same on both. v2's LRU is always on, which we already established is a wash vs. v1-with-read-cache-enabled.

### Materializer interference (v2 only)

This is where v2 adds a new failure mode.

- 10 writes/sec × 3 dirty pages × merge dedup → ~2 new pages/sec materializer work.
- Materializer wakes up every ~6 txids → runs every 600 ms, each pass merging ~6 txids into ~6 pages plus META. 1 round trip per pass.
- Background: **~1.67 round trips/sec**, ~6 ms of KV bandwidth.

That's nothing in aggregate, but it **shares the same KV channel as the foreground reads**. During a materializer pass, a concurrent cache-miss read on the same UDB shard waits. Tail latency for reads effectively becomes `max(foreground_read_rtt, materializer_pass_rtt)` ≈ `max(3.5, 3.5+3.5) = 7 ms` at p99. So the read p99 doubles.

### Cache contention

The materializer inserts freshly-materialized pages into the LRU (per Chapter 9 of `walkthrough.md`: "atomically update the in-memory page cache AND remove the merged pgnos from dirty_pgnos_in_log"). If those pages were already hot in the LRU, it's a wash. If they were cold (the workload's writes touch different pages than its reads), the materializer is evicting useful reads from the cache to make room for pages that will never be read again. This is the "log-structured eats my cache" failure mode.

For this workload, the writes target the counter/row update path and the reads target random B-tree lookups, so the overlap is modest. Estimate: **2–5% of LRU slots polluted by materializer** at steady state. Not catastrophic, but not free.

### Write tail latency

- p50 writer: 3.5 ms (fast path, 1 round trip).
- p99 writer: ~7 ms if the writer lands mid-materializer-pass and has to wait for the same UDB shard. Not dramatic but real.

**Compared to v1:** v1 has zero materializer, so p99 writer is just the tail of the KV round trip itself. **v1 wins on p99 write latency by ~2×.**

### Recommendations

- Make the materializer **back-off aware**: if foreground round trip rate exceeds a threshold (e.g., 50/sec over the last 500 ms), pause materializer passes until the load drops. The materializer only needs to run when LOG/ is nearing its quota, not on every idle moment.
- Make materializer cache population **configurable**: either "populate LRU" (default, cheap) or "skip LRU unless already present" (for workloads where writes touch pages unrelated to reads).
- Expose `sqlite.materializer.concurrent_mode` with values `always | on_idle | on_quota_pressure`. Default `on_idle`.

---

## Recommendations

The goal of this section is to let us ship v2 *without* regressing point-op workloads. v2's design is correct for the large-commit and cold-read scenarios; point ops are where it needs the most tuning discipline.

**1. Make the materializer lazy by default.**
`sqlite.materializer.concurrent_mode = on_idle`. Only run a materializer pass when the actor has no in-flight SQLite statements *and* LOG/ has at least ~100 ms of slack before it'd start pressuring the writer. This removes the tail-latency interference from Scenario 5 almost entirely, at the cost of up to ~200 MiB of LOG/ during sustained write bursts (below the 10 GiB quota).

**2. Per-actor tuning for hot-row-update workloads.**
Expose `sqlite.materializer.min_pass_txids` (default 6, raise to 50 or 100 for point-write-heavy actors). Larger pass size means better latest-wins deduplication and fewer materializer round trips. Scenario 1's 1000 hot-row updates collapse from 167 passes × 3 pages to 10 passes × 3 pages (one per 100 commits), dropping background cost from ~585 ms to ~35 ms.

**3. Default LRU up, not the same.**
mvSQLite's 5,000-page default (20 MB) is reasonable for v2. For read-heavy actors, raise to **20,000 pages (80 MB)** so working sets up to ~100k rows fit. This is a config knob, not a hard default, because actor density matters.

**4. Always-on LRU, not opt-in.**
v1 has the read cache behind `RIVETKIT_SQLITE_NATIVE_READ_CACHE`. v2 should *not* make it opt-in. Every scenario above that touched reads benefited from it, and the RAM cost (20 MB per actor at 5,000 pages) is modest. Bake it into the VFS unconditionally.

**5. Preload hints for hot root pages.**
For any table the application queries by primary key, preload the root + first-internal-level pages via `kv_sqlite_preload.hints`. Scenarios 1 and 2 lose 2–3 cold round trips per actor startup without this.

**6. Instrument the materializer's cache effect.**
Track the ratio of (materializer-inserted LRU pages that were subsequently read) / (materializer-inserted LRU pages). If the ratio is <10% in production, flip the default populate-LRU behavior off — the materializer is polluting the cache without helping reads.

**7. Accept that Scenario 1 is a tie, not a win.**
The original performance review's concern is validated: **hot-row OLTP does not benefit from v2.** The LTX log adds overhead (extra LOG writes, extra background materialization) that a point-write workload cannot amortize. The mitigations above keep the regression below ~5% p50 and ~2× p99, but they do not make v2 *faster* than v1 for this shape. **If Rivet's production mix is dominated by hot-row OLTP with no large transactions, v2 is a net loss.** v2 should ship only for actors whose profile includes at least one of: (a) occasional large transactions, (b) cold read-heavy workloads, or (c) working sets significantly larger than the LRU.

**8. Keep v1 alive per the no-migration policy.**
This is already the decision in `design-decisions.md` §1.5. Reinforce it: the point-ops-heavy actor profile is exactly the population that should *stay on v1*. The v1→v2 dispatch should happen at actor registration based on a workload hint (`sqlite.profile = point_ops | mixed | bulk`), not a global flag day.

---

## Honest bottom line

v2 is the right design for Rivet's SQLite. It solves the slow-path journal-fallback cliff (Scenario 4 extreme) and the cold-read prefetch deficit (Scenarios 2A and 2B first-pass). But it does **not** meaningfully improve hot-row OLTP, and in Scenario 5 it introduces a modest (~2×) tail-latency regression under concurrent writes-and-reads due to materializer/foreground contention. The recommendations above keep the regression bounded. They do not eliminate it. Anyone building a counter-service on Rivet should stay on v1.
