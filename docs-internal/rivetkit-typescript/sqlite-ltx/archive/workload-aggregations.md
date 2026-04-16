> **Stale numbers (2026-04-15):** Computed at 2.9 ms local-dev RTT. Per `constraints.md` C6, the production target is ~20 ms. Multiply round-trip-bound numbers by ~7×. Qualitative findings still hold. Recompute pending implementation.

# SQLite VFS v2 — Workload Analysis: Aggregations

Companion to [`walkthrough.md`](./walkthrough.md) and [`design-decisions.md`](./design-decisions.md). Evaluates v2 against read-heavy aggregation workloads.

> **Status (2026-04-15):** Parallel sub-agent output. Quantitative, not adversarial. Numbers use `~2.9 ms` per round trip (the median of the 1 MiB `get` trace in `examples/sqlite-raw/BENCH_RESULTS.md`: `63.1 ms / 30 gets ≈ 2.1 ms` traced, but the caller-observed wall time is closer to `~2.9 ms` once client stitching is counted, per the parent agent's instruction).

---

## Preliminaries and shared assumptions

Before scenario-by-scenario analysis, two baseline facts both paths depend on.

**v1's startup preload is not what you think it is.** `engine/packages/pegboard/src/actor_kv/preload.rs:53` (`batch_preload`) ships at most `pegboard.preload_max_total_bytes` (default **1 MiB**, per `engine/packages/config/src/config/pegboard.rs:319-320`) of KV entries alongside the actor start command. The sub-request for SQLite is `partial: true` and uses the actor's registered prefix. That means v1 actors *do* get a preload, but it is **bounded by bytes, not by page numbers**, and the entries that land are the first N pages in key order until the byte budget is exhausted. At 1 MiB with ~4.1 KiB per preloaded chunk (4 KiB page + a few bytes of metadata + tuple framing), that's **~240 pages** delivered on the start command, covering pgno 1..240. Every page beyond that is a live `kv_get` at SQLite open time.

**v1 read cache is opt-in and off by default.** `vfs.rs:104` (`read_cache_enabled`) gates it behind `RIVETKIT_SQLITE_NATIVE_READ_CACHE=1`. Production actors run without it, so cold pages beyond the 240-page preload cost one round trip each. The scenarios below assume v1 runs with the read cache *enabled* (generous to v1) but note where the default-off behavior changes the numbers.

**v2 prefetch depth.** The parent agent's instruction fixes `PREFETCH_DEPTH = 16` (mvSQLite default). For sequential scans this means one `kv_sqlite_get`-style call fetches the target page plus 16 predicted pages per round trip. The bigram/stride predictor will converge on +1 stride within ~3 reads once it sees a sequential pattern.

**v2 preload hints.** The scenario-5 preload hint is "first 1000 pages" which at ~9 MiB per `kv_sqlite_preload` envelope fits comfortably — 1000 pages × 4 KiB raw = 4 MiB, and uncompressed pages fit in one preload round trip.

**RTT latency.** 2.9 ms/RTT, matching the prompt. For sequences that fit in one UDB transaction but return large payloads (e.g. the preload), the round-trip is effectively the same — UDB's single-transaction read of 1000 keys is close in wall time to a single-key read because the dominant cost is the network hop and the transaction setup, not the range scan.

---

## Scenario 1: `SELECT COUNT(*) FROM big_table` (no covering index)

Model: 100 MiB table, 4 KiB pages → **25,000 leaf pages** in the data B-tree. SQLite walks every leaf, calling `xRead(pgno=L, 4096, offset=(L-1)*4096)` in ascending pgno order (interior-then-leaf traversal, but the big cost is the leaves). The traversal touches a handful of interior pages (log_480(25000) ≈ 2 levels × few pages) — rounding error relative to 25,000 leaves.

### v1 round trips

- **Preload covers pgno 1..240.** First 240 leaf accesses are served from the startup preload map (`vfs.rs:274-297`). Zero round trips.
- **Remaining 24,760 leaves.** Each `xRead` becomes one `kv_get([PAGE/L])`. SQLite's page cache (separate from the VFS cache) holds pages across the query, but during a first pass over 25,000 leaves the SQLite pager cache (default 2,000 pages = ~8 MiB) is smaller than the working set, so pages get evicted and re-read is rare.
- **With VFS read cache on:** still one round trip per uncached leaf, because v1 has no prefetch. The read cache only helps on re-reads.
- **Interior pages:** 3–5 round trips for the B-tree spine, amortized into the total.
- **Total:** `24,760 ± 5 ≈ 24,760` round trips.
- **Latency:** `24,760 × 2.9 ms ≈ 72 seconds`.

### v2 with prefetch

- **Preload on startup:** META + page 1 + LOGIDX/ scan. Irrelevant to the scan body.
- **First leaf read** (pgno 2 or wherever the B-tree root points): cache miss, predictor has no history, issues a single-page `kv_get`. One round trip. The predictor records (delta +1, stride +1 candidate).
- **Second leaf read:** stride detector suggests +1, Markov sees delta=+1. Predictor emits 16 predictions `[L+1, L+2, ..., L+16]`. VFS issues one batched `kv_get` of 17 keys (target + 16 predictions). After this: pages L..L+16 are in cache.
- **Steady state:** every 17th leaf read is a miss, triggers a 17-key `kv_get`, fills the cache with the next stride. The other 16 reads are cache hits.
- **Round trip count:** `⌈24,999 / 17⌉ ≈ 1,471` round trips for the scan body, plus 1–2 warmup round trips. Call it **~1,472**.
- **Latency:** `1,472 × 2.9 ms ≈ 4.3 seconds`.
- **Speedup vs v1:** `72s / 4.3s ≈ 17×`. This is essentially the prefetch multiplier `17/1 = 17`, which is expected when the predictor is in its best case.

### v2 with preload hints ("first 1000 pages")

- **Preload delivers pages 1..1000** via one `kv_sqlite_preload` call. The caller issues that at startup alongside META — ~1 round trip folded into boot. Cost: the preload call itself, ~2.9 ms (already in the boot budget).
- **Scan body:** pages 1..1000 served from preload (zero round trips). Pages 1001..25,000 served by prefetch in stride-16 batches: `⌈24,000 / 17⌉ ≈ 1,412` round trips.
- **Latency:** `1,412 × 2.9 ms ≈ 4.1 seconds`. ~5% better than v2 without hints. The hint barely matters for a 25k-page scan because 1000 pages is 4% of the work.

### Prefetch stride detector honest evaluation

Sequential table scans are the **best case** for the mvSQLite predictor. The bigram chain on deltas immediately sees `+1, +1, +1...` and emits stride-based predictions with high confidence. There is no need to rely on Markov probabilities — the stride detector carries the workload.

### v2 failure modes

1. **Predictor warmup.** The first 2–3 reads issue single-page gets because the predictor is empty. Negligible at 25k pages but worth noting for short scans.
2. **Page cache pressure.** 25,000 pages × 4 KiB = 100 MiB. Default cache of 1,000–5,000 pages evicts aggressively during the scan. That's fine for a single pass (we never re-read) but ruinous for scenario 5.
3. **Log tail check.** If any of the 25,000 pages are in `dirty_pgnos_in_log`, each one is an extra round trip to pull the LTX frame. For a read-only workload this is zero; for a workload mid-write it could add a handful of extra round trips.

**Verdict: v2 wins by ~17×.** Preload hints add only marginal value.

---

## Scenario 2: `SELECT AVG(amount) FROM transactions WHERE created_at > ?` (index on `created_at`)

Model: indexed range scan. Approximately **5,000 index leaf pages** walked sequentially (the index slice) + **50,000 data page dereferences** in *random* order (one heap fetch per qualifying row).

### v1 round trips

- **Index slice walk:** 5,000 leaf pages sequentially. With ~240 covered by preload, **4,760 round trips** for the index walk.
- **Data page dereferences:** 50,000 random heap fetches. SQLite does not batch these. Each is one `kv_get`. The preload and SQLite pager cache catch some (maybe 240 + a few hundred from locality if `created_at` correlates with heap layout), but for a truly random access pattern assume ~50,000 round trips.
- **Total:** `4,760 + 50,000 ≈ 54,760` round trips.
- **Latency:** `54,760 × 2.9 ms ≈ 159 seconds` (≈ 2.6 minutes).

### v2 with prefetch

- **Index slice (sequential):** 5,000 leaves at stride +1. Predictor catches on in 2 reads. `⌈5,000 / 17⌉ ≈ 295` round trips.
- **Heap dereferences (random):** this is the **mixed case** the prompt flags. The predictor sees a pattern like `[index_leaf, heap_page_X, index_leaf, heap_page_Y, ...]`. The delta sequence is noisy: `(+1, −Δi, +1, −Δj, ...)`. The bigram Markov chain will not find a confident bigram because heap pages are effectively random per row (no clustering unless `created_at` correlates with primary key order). The stride detector will not detect a stride because successive heap pages are uncorrelated.
- **Outcome:** predictor issues mostly single-page `kv_get`s for heap pages, with occasional lucky multi-page bursts if consecutive rows happen to land on the same heap page (which the SQLite pager cache would also catch). Call heap prefetch multiplier ~1.1× on average.
- **Heap round trips:** `~50,000 / 1.1 ≈ 45,500` round trips.
- **Total:** `295 + 45,500 ≈ 45,800` round trips.
- **Latency:** `45,800 × 2.9 ms ≈ 133 seconds` (≈ 2.2 minutes).
- **Speedup vs v1:** `159 / 133 ≈ 1.2×`. Marginal. Index walk is 17× faster but it was a small fraction of the total work.

### v2 with preload hints

- "First 1000 pages" preload will only help if `created_at` is recent and the recent data lives at low pgnos. In the common append-only OLTP pattern, recent rows are at **high** pgnos, so preload hints targeting pgno 1..1000 are **worthless** here.
- **Actionable preload hint would be:** "the index root + first 100 pages of the `created_at` index" plus "heap pages 50000..55000" (the range where recent rows live). v2's preload protocol supports ranges, so this is expressible, but requires the application to know where recent rows physically land.
- **If the user supplies the right hints:** cover the entire index slice (5,000 pages) in one preload round trip. Heap fetches still random. Total ≈ `1 + 45,500 ≈ 45,501`. Virtually identical to plain v2.

### Prefetch honest evaluation

**The predictor is mostly wasted** on index-then-heap dereference patterns. mvSQLite's `docs/prefetch.md` explicitly lists "B-tree point lookups followed by heap fetches" as the weak case. The stride detector fails (heap pages are random). The Markov bigram fails (delta from leaf to heap is noisy and non-repeating). What little win v2 has over v1 comes from the index walk being fast, and the index walk is only 10% of the total work.

### v2 failure modes

1. **Cache pressure from prefetch on the index side.** 16 extra index pages fetched per round trip means the LRU cache evicts useful heap pages that might have been hit on a second pass. For a single-pass scenario this is fine; for scenario 5 it's a concern.
2. **Wrong preload hints hurt.** Preloading a fixed pgno range that doesn't overlap the query's access pattern wastes the 1–2 MiB envelope and evicts nothing useful (the preload just sits in cache unused). This is cheap at warm-up but real on cache-constrained actors.
3. **The `dirty_pgnos_in_log` lookup runs 55,000 times.** Each check is a hashmap lookup — a few hundred nanoseconds. Not a round trip, but worth measuring; if the log tail has a thousand pages and the HashMap is not tuned, it's a ~0.5 second CPU tax per scan.

**Verdict: v2 wins by only 1.2×.** The index-then-heap pattern is the workload where the LTX/v2 redesign most obviously underdelivers. This is the scenario to highlight when setting expectations.

**Open question (v2 gap):** SQLite has no way to tell the VFS "I'm about to read 50,000 heap pages with the following pgnos" — the pgnos are only known after the index scan produces the rowids. A hypothetical `xFileControl` extension that lets SQLite pass a batch of upcoming pgnos to the VFS would turn this scenario into 50,000 / 128 ≈ 391 round trips instead of 45,500. That does not exist today and would need SQLite-side work to enable.

---

## Scenario 3: `SELECT COUNT(*) FROM big_table` with a covering index

Model: pure index scan, ~**2,000 index leaf pages**, sequential. No heap fetches at all (the index covers everything needed). This is the **pure sequential case**.

### v1 round trips

- Preload covers ~240. Remaining `1,760 × 1 RTT = 1,760` round trips.
- **Latency:** `1,760 × 2.9 ms ≈ 5.1 seconds`.

### v2 with prefetch

- Predictor immediately detects stride +1. Steady state: 17-key batches.
- `⌈2,000 / 17⌉ + warmup ≈ 120` round trips.
- **Latency:** `120 × 2.9 ms ≈ 350 ms`.
- **Speedup vs v1:** `5.1s / 0.35s ≈ 14.6×`.

### v2 with preload hints

- "First 1000 pages" has a decent chance of covering the entire index if the index is small and lives at low pgnos. If it does: `1,000` pages served from preload, `1,000` from prefetch in 59 round trips. Latency `60 × 2.9 ms ≈ 174 ms`, so ~**30×** faster than v1. If the index lives at high pgnos, hints don't help and we're back to the ~350 ms baseline.

### Prefetch stride detector honest evaluation

**Perfect case.** Identical reasoning to scenario 1 but with a shorter scan. The predictor warmup is a larger fraction of the total (2 warmup reads out of 120 = 1.7% vs 0.1% in scenario 1), but still trivial.

### v2 failure modes

- **Over-prefetch at the tail.** Once we're within 16 pages of the end of the index, the last prefetch batch fetches pages past the index, wasting ~40 KiB of KV bandwidth and polluting the cache with unrelated pages. Trivial at 2,000 pages; worth a stride-aware boundary check in the predictor.

**Verdict: v2 wins by 14–30×.** Strong case.

---

## Scenario 4: `SELECT category, SUM(price) FROM products GROUP BY category`

Model: full table scan with hash aggregation. ~**10,000 pages**, sequential. Same shape as scenario 1 but half the size.

### v1 round trips

- `10,000 − 240 = 9,760` round trips.
- **Latency:** `9,760 × 2.9 ms ≈ 28.3 seconds`.

### v2 with prefetch

- `⌈10,000 / 17⌉ + warmup ≈ 590` round trips.
- **Latency:** `590 × 2.9 ms ≈ 1.7 seconds`.
- **Speedup:** `28.3 / 1.7 ≈ 16.6×`.

### v2 with preload hints

- "First 1000 pages" covers 10% of the scan. Latency saving ~100 × 2.9 ms ≈ 290 ms. Effectively ~1.4 seconds. ~20× over v1.

### Prefetch honest evaluation

Full sequential scan, same as scenario 1 but shorter. The hash aggregation happens in the SQLite layer and does not interact with the VFS — memory, not disk. The predictor carries the scan cleanly.

### v2 failure modes

- None beyond those in scenario 1.

**Verdict: v2 wins by ~16–20×.** Strong case.

---

## Scenario 5: Dashboard with 5 aggregations every minute

Model: the actor runs the same 5 queries once per minute. Queries are mixes of scenarios 1–4. Key question: **does the cache carry warm data across queries within a minute, and across minutes of the dashboard loop?**

### v1 behavior

- **Preload is one-shot** (`startup_preload` in `vfs.rs:191`). It is populated on actor start and mutated by subsequent puts/deletes. It does not grow to absorb hot data that wasn't in the original preload. After the first query, pages read from KV **are not cached** unless `RIVETKIT_SQLITE_NATIVE_READ_CACHE=1` is set.
- **With read cache enabled:** the SQLite pager cache (SQLite-side, not VFS) holds ~2000 pages. The VFS read cache is unbounded and holds every page ever read. After run 1 of the dashboard, the union of all touched pages is in the VFS read cache.
- **Total pages touched in one run:** scenario 1 (25k) + scenario 2 (5k index + 50k heap, but heap is random) + scenario 3 (2k) + scenario 4 (10k) ≈ 92,000 unique pages (assuming minimal overlap). 92,000 × 4 KiB = 368 MiB.
- **Run 1 latency (with read cache):** same as cold scenarios = `72 + 159 + 5 + 28 ≈ 264 seconds`. Disaster.
- **Run 2+ latency (with read cache, if RAM permits):** all pages in the VFS read cache. Every read is a hashmap lookup. Near-zero round trips. But: 368 MiB in RAM per actor is well beyond the hinted 5,000-page (20 MiB) cache budget and will OOM small actors.

### v2 behavior

- **v2 cache is bounded LRU (default 5,000 pages = 20 MiB).** It cannot hold 92,000 pages. During run 1, the cache evicts aggressively. By the start of run 2, the cache contains the **last-touched** 5,000 pages — whatever was tail of scenario 4. Run 2 must re-fetch everything else.
- **Run 1 cost with prefetch:** ~`1,472 + 45,800 + 120 + 590 ≈ 47,982` round trips. Latency `~139 seconds`. Dominated entirely by scenario 2 (the index+heap pattern that prefetch cannot help).
- **Run 2 cost with prefetch:** scenario 1 starts with 5,000 cached pages that happen to be from scenario 4's tail — they almost certainly do not overlap the big_table scan. Cache is useless here. Run 2 ≈ run 1.
- **Over 60 minutes (60 runs):** `60 × 139s = 8,340 seconds = 139 minutes`. Every minute of actor uptime spends 2.3 minutes on dashboard work. **The dashboard cannot complete in one minute** — it's missing the deadline by 2.3×.

### v2 with preload hints

- Preload "first 1000 pages" helps a tiny bit in scenarios 1, 3, 4. Doesn't help scenario 2. Expected saving: `~3 seconds` per run. Still doesn't meet the 1-minute deadline.
- **A smarter preload hint** for a dashboard actor: enumerate the hot index roots, the covering index range, and maybe the tail of the transactions heap if `created_at` correlates with pgno order. This is the workload where preload hints finally earn their weight — **but it requires the application to tell v2 what matters**, and that's a developer-experience problem, not a v2-protocol problem.

### v2-specific failure modes in scenario 5

1. **Cache thrash.** 5,000 × 4 KiB = 20 MiB is too small for a 92k-page working set. The LRU throws away everything useful between queries. **Recommendation:** bump default cache to 50,000 pages (200 MiB) for dashboard-shaped actors, configurable per actor.
2. **Materializer interference.** If writes land concurrently with the dashboard (unlikely for a pure dashboard but possible), the materializer is burning round trips the dashboard could use. Since it runs serially against the same KV channel, its round trips add directly to wall time. For a 60-second dashboard window, a 10-round-trip materializer pass is a 3% tax — fine.
3. **Log tail checks.** `dirty_pgnos_in_log` is checked for every one of ~92,000 page reads per run. If the log is large this is noticeable CPU. Not round trips, but latency.
4. **Preload bloat.** If a user preloads the wrong 1000 pages (say they preload the old `created_at` range), those pages are loaded once, never evicted until pressure, and every scan of another table evicts them the first time it touches the LRU. Mostly harmless — one round trip of preload wasted — but it creates a false sense of optimization.

### Honest verdict for scenario 5

**v2 is not meaningfully better than v1 here.** Both have an RTT-bound read path for ~45k random heap pages per run (scenario 2 dominates). Neither fits in a 1-minute budget. The only way to make this work is to restructure the workload: add a covering index on `(created_at, amount)` so scenario 2 becomes scenario 3. That's a schema change, not a VFS change.

**If the dashboard is just 1 + 3 + 4 (no random heap deref):** scenarios 1 + 3 + 4 combined = `(1472 + 120 + 590) × 2.9 ms ≈ 6.3 seconds per run`. Easily fits in 60 seconds. The cache would still thrash across runs but the per-run cost is tolerable. **v2 wins 40× over v1 in this restricted version.**

**If run 2+ within a minute reuses the cache:** scenario 4's 10,000 touched pages are partially in cache from run 1's tail. In practice maybe 20-30% of scenario 4 pages are hot on run 2, saving ~1.3 round trips × 2.9 ms ≈ 400 ms. Real but small.

---

## Recommendations

Concrete v2 tuning knobs for aggregation-heavy actors, in order of impact.

1. **Ship a large LRU cache for analytical actors.** Default of 5,000 pages is fine for point-query OLTP; dashboards need 50,000–100,000 pages (200–400 MiB). Make this a per-actor config. Track cache hit rate in VFS metrics so operators can see when they need to bump it.

2. **Preload the first 1–2 MiB of the SQLite header + schema pages.** These are always read on every query and live at low pgnos. The proposed v2 preload already does this (page 1); extend it to pages 1..500 by default for any actor with a nontrivial schema. Cost: one preload round trip of 2 MiB at boot.

3. **Expose prefetch depth as a config.** `PREFETCH_DEPTH = 16` is fine for mixed workloads but aggregation-heavy actors should run at 32 or 64. The tradeoff is bandwidth waste on non-sequential patterns, which the predictor's confidence threshold is supposed to gate — verify in benchmarks.

4. **Log `prefetch_hit_rate` and `prefetch_overfetch` metrics.** The predictor is the core of v2's read-side win and we have no way to see if it's actually working in production. Every fetched-but-never-read page is a cache line evicted for nothing.

5. **Add a VFS-level directive for "bulk pgno hints."** This is the scenario-2 gap. After SQLite performs the index scan and has the list of rowids, a smart query executor could tell the VFS "the next 50,000 reads will be at these pgnos" and let the VFS batch them. SQLite itself has no such API; this requires a custom SQL function or prepared-statement wrapper exposed to RivetKit users. Tag this as an **open question**.

6. **Back-pressure the materializer during long reads.** If the actor has been executing a read-only query for >1 second and the LOG/ size is below 50 MB, pause the materializer. Its round trips compete with the reader on the same KV channel. This is a latency optimization, not a correctness concern.

7. **Document that index-then-heap aggregations are slow.** Users should be told that `SELECT ... FROM t WHERE indexed_col > ?` across a large random heap region is the **worst case** for v2. The fix is always "add a covering index," and we should say that in the limits doc.

8. **Keep the `partial: true` preload path.** v1 already ships 1 MiB of SQLite pages on start. v2 should do **at least** as well at the same size budget, and ideally ship the preload via `kv_sqlite_preload` with richer hints (pages + LOGIDX + META) in one round trip. The current walkthrough's `kv_sqlite_preload` sketch covers this; make sure the implementation doesn't regress below v1's 1 MiB default.

### Open questions / missing v2 features

- **No SQL-level pgno-batch hint** (scenario 2 blocker). Would need either a custom pragma or a query-time extension. Parking as a v2.1 design item.
- **No hot-range reload after runtime change.** If the actor notices "scenario 4 is running for the 10th time and the cache isn't holding," it cannot re-preload. The hint API is config-time-only per the design doc, and a runtime hint would be valuable for dashboards. Parking as a v2.1 design item.
- **No prefetch-bypass for small covered indexes.** If the predictor sees a 2,000-page scan, it might as well prefetch the entire table at once — `⌈2000/128⌉ = 16` round trips — rather than use stride-16 for 120 round trips. Dynamic sizing of prefetch depth based on observed scan length is a future optimization.
- **No cross-query learning.** The predictor resets at transaction end per mvSQLite's design. For a dashboard that runs the same 5 queries forever, a persistent predictor state across transactions would be a ~10% win. Small but worth noting.

### Scenarios where v2 is no better than v1

- **Scenario 2 (index + random heap deref).** v2 is 1.2× faster, which is not worth the design cost unless the rest of the workload is sequential-dominated. Both are RTT-bound at ~2.6 minutes for 50k random fetches. The only fix is a covering index at the application layer.
- **Scenario 5 run 2+ with random access pattern.** v1 with read cache *enabled* (non-default) holds the entire working set in memory after run 1 and finishes run 2 in near-zero time, at the cost of OOMing any actor with a >400 MiB working set. v2's bounded LRU is the sane tradeoff, but it sacrifices hot-set reuse for memory discipline. Calling it a "win" depends on what you're optimizing for.

---

## Scenario round-trip summary table

| Scenario | Pages touched | v1 RTTs | v1 latency | v2 RTTs (prefetch) | v2 latency | v2 speedup |
|---|---|---|---|---|---|---|
| 1. `COUNT(*)` no index | 25,000 | 24,760 | 72 s | 1,472 | 4.3 s | 17× |
| 2. Indexed range + heap | 55,000 (mixed) | 54,760 | 159 s | 45,800 | 133 s | 1.2× |
| 3. `COUNT(*)` covering index | 2,000 | 1,760 | 5.1 s | 120 | 0.35 s | 14.6× |
| 4. `GROUP BY` full scan | 10,000 | 9,760 | 28.3 s | 590 | 1.7 s | 16.6× |
| 5. Dashboard (5 queries/min) | ~92,000 | ~92,000 | 264 s/run | ~48,000 | 139 s/run | 1.9× |

Latencies use 2.9 ms/RTT per prompt. v1 numbers include the 240-page preload. v2 numbers use `PREFETCH_DEPTH = 16` and a 5,000-page LRU cache. "v2 latency" assumes the predictor warmup cost is negligible.
