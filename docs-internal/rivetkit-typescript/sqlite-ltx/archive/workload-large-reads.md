> **Stale numbers (2026-04-15):** Computed at 2.9 ms local-dev RTT. Per `constraints.md` C6, the production target is ~20 ms. Multiply round-trip-bound numbers by ~7×. Qualitative findings still hold. Recompute pending implementation.

# Workload Analysis: Large Reads

Status: Draft 2026-04-15. Companion to [`walkthrough.md`](./walkthrough.md) and [`design-decisions.md`](./design-decisions.md). Numbers are reasoned estimates, not measurements. Confirm with a bench before committing to the tuning constants in the Recommendations section.

## Assumptions used throughout

Carried through every scenario so the per-section bullets stay short.

- **Round-trip cost.** `2.9 ms` per engine KV round trip in local dev (bench data). `~3 ms` for one `kv_get`, regardless of whether it carries 1 or 128 keys, because the latency is dominated by the engine tunnel, not the UDB read itself. A handful of extra us per key for the FDB range reads. Treat `3 ms / RTT` as the budget.
- **Page size.** SQLite `PRAGMA page_size = 4096` (v1 and v2). `1 MiB of table data = 256 pages`.
- **SQLite pager cache.** Neither v1 nor v2 overrides `PRAGMA cache_size`, so SQLite's default pager cache is **2000 pages (≈8 MiB) per connection**. This is SQLite's own in-process page cache, sitting *above* the VFS. It absorbs repeated reads for anything that fits in 8 MiB. The VFS only sees the reads that miss this cache.
- **VFS read cache.** v1's `read_cache` is opt-in via `RIVETKIT_SQLITE_NATIVE_READ_CACHE` (`vfs.rs:57`). Default is off. v2's LRU cache is always-on, default 5000 pages = 20 MiB. For head-to-head fairness I assume **v1 without the opt-in cache** (the shipping default), and I call out the opt-in variant when it matters.
- **SQLite xRead granularity.** SQLite's pager asks the VFS for one 4 KiB slice per page miss. It never asks for a 64 KiB stripe. That means v1's per-read batch opportunity is **zero pages** unless something else widens it. v1's only RTT amortization today is the startup preload (not relevant to mid-query reads).
- **Data-page-to-index-page ratio for a B-tree scan.** For a 1M-row table with small rows, the leaf pages dominate: a SQLite table with ~50 B rows and 4 KiB pages fits roughly 70–80 rows/leaf, so ~13k leaf pages, plus ~200 internal pages for the B-tree itself. For a 50 MB table (12800 pages) the internal nodes are ~1–2% of the data volume. I simplify this to "roughly all pages are leaves" for round-trip arithmetic.
- **LZ4 on SQLite pages.** Published numbers (mvSQLite notes, LiteFS measurements on real workloads, recent SQLite-WASM bench data) land 4 KiB B-tree leaf pages at **2.0–3.0× compression** with LZ4 block mode. I use **2.2×** (1.8 KiB avg per page) for arithmetic. Realistically variable: nearly-empty pages compress harder, TEXT-heavy overflow pages compress worse. The exact ratio is an open question in `design-decisions.md §5`.
- **v2 prefetch depth.** The mvSQLite port defaults to **8 predicted pages per read** on a hot stride, with the `predictor.multi_predict` batch going to the engine as one `kv_get` carrying `[target, ...predictions]` = **up to 9 keys per RTT** during a sequential scan.
- **Materializer state.** Scenarios 1–4 assume the materializer has caught up before the read starts. The 4-layer read path short-circuits on step 4 (PAGE/) in the steady state. I flag the case where the log layer matters.

---

## Scenario 1 — Full table scan that fits in memory but not in the VFS cache

`SELECT * FROM users` against a 1M-row table (~50 MB on disk). Roughly 12,800 pages counting table B-tree leaves plus a small number of internal pages and index root pages.

**v1 behavior.**
- SQLite's pager cache holds the first 2000 pages. Everything past that page-faults *and* evicts earlier leaves because the scan is strictly forward-walking. By the end, the pager cache contains the last 2000 pages scanned. (LRU + linear walk = no reuse.)
- 12,800 unique leaf pages × 1 `xRead` each at the VFS boundary = **12,800 `xRead` calls**. Each call is one 4 KiB chunk.
- v1 has no prefetch. Each `xRead` misses the VFS read cache (disabled by default; and even enabled it's empty on a cold scan) → one `batch_get` with `[PAGE/<pgno>]` carrying **1 key**.
- **Round-trip count: ~12,800.** Aggregate latency: `12,800 × 3 ms ≈ 38.4 s`.
- If `RIVETKIT_SQLITE_NATIVE_READ_CACHE=1` is set, a repeat of the same scan is free-ish (bounded by the unbounded HashMap's memory footprint of ~60 MB). Cold scan is still ~38 s.
- This is the shape of the pathology the user sees in production on any meaningfully-sized table.

**v2 behavior.**
- Same 12,800 `xRead` calls from SQLite. The v2 read path:
  1. **Layer 1 (LRU cache)** — empty on cold start, fills as we go. At the 5,000-page default, by the time we're past page 5,000, the early pages are evicted; scan does not reuse.
  2. **Layer 2 (write buffer)** — empty for a read-only query.
  3. **Layer 3 (unmaterialized log)** — assumed empty (materializer caught up). Zero cost.
  4. **Layer 4 (materialized PAGE/)** — this is where the reads go.
- The prefetch predictor observes a stride of +1 after the first couple of reads, then emits `PREFETCH_DEPTH` = 8 predictions per call. Each fourth layer 4 lookup becomes one `kv_sqlite_preload` (or a fat `kv_get` via the existing path) carrying **9 keys** (target + 8 predictions). Cache hits for the 8 predicted pages on the next 8 reads cost zero RTT.
- Effective RTT rate: `12,800 / 9 ≈ 1,422` round trips.
- **Round-trip count: ~1,422.** Aggregate latency: `1,422 × 3 ms ≈ 4.3 s`. **~9× speedup over v1.**
- If the cache is bumped to 2,000 pages (so it equals the SQLite pager cache, removing the last-pages-eviction problem), the *first* scan is unchanged; the speedup comes from Scenario 4.

**Cache effectiveness.** For a strict forward scan longer than the cache, neither v1's opt-in 60 MB HashMap nor v2's 20 MB LRU provides value *during* the scan. The cache only helps *after* the scan, and only if the same pages are touched again (Scenario 4). The thing that helps mid-scan is the prefetch batch.

**Prefetch predictor rating: (a) very effective.** This is the canonical case the stride detector was built for. A +1 stride on page numbers gets the highest-confidence bin in the mvSQLite predictor. Expected effectiveness: **8–16× RTT reduction** depending on `PREFETCH_DEPTH`.

**Breakeven vs v1.** v2 is never slower on this scenario. Even with prefetch disabled, v2 pays the same 12,800 RTTs as v1. With prefetch depth = 2 it breaks even immediately.

---

## Scenario 2 — Cold-start read of a working set

Actor boots. Immediately runs `SELECT * FROM users WHERE region = 'us-east'`. Returns 100k rows. The index on `region` has ~300 internal pages and scans match ~800 index leaf pages. Each matched row is then dereferenced to a data page; with a non-covering index and 100k matched rows averaging ~10 rows per data page, we fetch **~10,000 data pages**, mostly in table-rowid order (SQLite groups fetches by page when it can, but a random-ish order is more realistic for an index that does not cluster rows).

**v1 behavior.**
- Cold start: META + a bounded preload of recently-touched pages (may or may not include anything from the users table). Realistic: 1 RTT for startup plus ~1 RTT of preload bodies.
- Query execution:
  1. Walk B-tree root → index root → index leaves. **~800 index leaves + ~3–5 root/internal pages ≈ 805 `xRead` calls**, each its own 1-key `batch_get`.
  2. Dereference ~10,000 data pages, each its own 1-key `batch_get`.
- **Round-trip count: 2 (boot) + 805 + 10,000 ≈ 10,807.** Aggregate latency: `10,807 × 3 ms ≈ 32.4 s`.
- Most data-page reads are "clustered-ish random" — not strictly +1 stride. Any v1 read cache the user had enabled doesn't help on a genuinely cold start.

**v2 behavior.**
- Cold start path: one `kv_sqlite_preload` op that fetches META + page 1 + LOGIDX scan. If the user has declared preload hints covering the users table's root pages and/or the `region` index root, those come in the same RTT. **1 RTT for startup.**
- Query execution, two sub-phases:
  1. **Index scan** (~805 pages). The first few reads train the predictor to stride +1. After warmup, each RTT carries 1 target + 8 predictions = 9 keys. `805 / 9 ≈ 90` RTTs.
  2. **Data-page dereferences** (~10,000 pages). This is the hard case: data pages are visited in an order that is *correlated* with rowid-order but is not +1 stride because the `region='us-east'` rows are scattered. The stride detector stalls. The Markov bigram helps **only** if particular (Δ=+k) pairs recur, which is workload-dependent. Realistic estimate: **3× RTT reduction** (3 pages/call average) for data-page reads. `10,000 / 3 ≈ 3,333` RTTs.
- **Round-trip count: 1 + 90 + 3,333 ≈ 3,424.** Aggregate latency: `3,424 × 3 ms ≈ 10.3 s`. **~3× speedup over v1**, limited by the data-page phase.
- If preload hints include the `(region, rowid)` index leaf pages for the relevant region (user knows their hot partitions), the index-scan sub-phase collapses to `ceil(index_leaf_bytes / ~1 MiB) ≈ 1–2` RTTs, saving ~270 ms. Not the bottleneck.

**v2 design gap this scenario exposes.** The data-page dereference phase is the real cost and the prefetch predictor is only partly helpful. v2 has no way today to tell the engine "give me all data pages that are referenced from this set of index leaf entries" — it would need either:
- A much deeper prefetch window (but then we fetch pages we don't need, wasting payload budget), or
- A new "dereference me" hint in `kv_sqlite_preload` that takes a list of pgnos and fetches them in one fat batch.

The second is conceptually a generalization of preload hints from "load on startup" to "load in the middle of a query, SQLite-agnostic." It would collapse 3,333 RTTs to `10,000 / 512 ≈ 20` RTTs (at the `~512` key/op envelope). **Total becomes 1 + 90 + 20 ≈ 111 RTTs = 333 ms. Flag this as an open question.**

**Cache effectiveness.** Cold run has zero cache warmth. The 8 MiB SQLite pager cache holds about 2000 of the 10,000 data pages at the end; the v2 5000-page LRU cache can hold half the data pages plus the index leaves. A repeat of the same query with the same region reuses everything that fits.

**Prefetch predictor rating: (b) somewhat effective.** Great for the index-walk sub-phase (stride +1). Middling for the data-page sub-phase (Markov bigram on deltas that happen to recur). This is the scenario where the predictor's honest limit shows up.

**Breakeven.** v2 beats v1 even without the predictor, because preload folds startup into 1 RTT. The margin widens as prefetch works.

---

## Scenario 3 — Large index range scan with prefetching opportunity

`SELECT * FROM events WHERE ts BETWEEN a AND b ORDER BY ts` against a time-indexed table. Scans a B-tree index top-down. The index walk is strictly sequential over ~2,000 index leaves (say 10 MB of index data). Each index entry points to a data page; the data-page visit order is *mostly sequential by rowid* because `events` is an append-only table where ts and rowid are strongly correlated. Call it 20,000 data pages, in mostly-sequential order with small skips.

**v1 behavior.**
- Index walk: ~2,000 `xRead` calls × 1 key each = **2,000 RTTs**.
- Data-page fetches: ~20,000 `xRead` calls × 1 key each = **20,000 RTTs**.
- Total: **~22,000 RTTs**. Aggregate latency: `22,000 × 3 ms ≈ 66 s`.
- This is the worst-case observable regression for reporting queries on v1.

**v2 behavior.**
- Index walk: stride +1, prefetch depth 8. `2,000 / 9 ≈ 222 RTTs`.
- Data-page fetches: mostly +1 stride (since rows are clustered by ts ≈ rowid order) with occasional small skips. The stride detector holds most of the time; the Markov bigram fills in the skips. Realistic prefetch effectiveness: **7 pages/RTT** average (one stride miss every 8–9 predictions). `20,000 / 7 ≈ 2,857 RTTs`.
- Total: **~3,080 RTTs**. Aggregate latency: `3,080 × 3 ms ≈ 9.2 s`. **~7× speedup over v1.**

**Cache effectiveness.** The 5000-page LRU cache holds ~20 MiB. This query's working set is ~90 MiB. No reuse *within* the scan. If the scan is repeated (dashboard refreshes), the tail of the cache retains the last ~5,000 pages. Scenario 4 covers this.

**Prefetch predictor rating: (a) very effective.** This is the single most predictor-friendly real-world workload. The paper mvSQLite wrote about the predictor used exactly this shape as the motivating example.

**Breakeven.** v2 is unambiguously better. Even without the predictor, v2 is tied with v1; with the predictor, it's 6–8×.

**v2 design note for this workload.** The fat-batch `kv_sqlite_preload` op envelope is the binding constraint once prefetch is saturated. At ~512 keys/call and ~8 pages prefetched per call, we're only using 9 of 512 slots per call. If the predictor could emit wider predictions (e.g., "the next 100 pages" on a saturated stride), we'd collapse the 2,857 data-page RTTs to `20,000 / 100 = 200 RTTs`. That is a 14× additional speedup on top of the predictor. **Variable prefetch depth on stride saturation is a concrete tunable and a v2-shipping candidate.**

---

## Scenario 4 — Repeated full table scans

A reporting dashboard polls the same query every minute. Assume it's the Scenario 1 query (full scan of 12,800 pages).

**v1 behavior.**
- **Default (read cache off):** every scan re-fetches all 12,800 pages. Each scan is 38.4 s. There is no reuse.
- **Opt-in read cache on:** first scan is 38.4 s (all pages fetched, all inserted into the unbounded HashMap). Cache is now ~60 MB on the first scan. Subsequent scans read from the cache → 0 RTTs → limited only by SQLite execution time (probably 1–3 s CPU). Memory grows with the working set; the cache does not evict.
- **Caveat:** the opt-in path holds all pages in per-file-state HashMaps with no bound. A 10 GiB DB would bust the actor memory. This is not shippable as an "always on" v1 mode.

**v2 behavior.**
- First scan: 4.3 s (from Scenario 1). 5000 pages of the 12,800 fit in the LRU cache; the final 5,000 pages are what's resident afterward (LRU order).
- Second scan: SQLite asks for pages 1, 2, 3, ... in order. Pages 1–7,800 are NOT in the cache (evicted by the forward walk). Pages 7,801–12,800 ARE in the cache, and the scan hits them free.
  - Pages 1–7,800 go through layer 4 with prefetch: `7,800 / 9 ≈ 867 RTTs`.
  - Pages 7,801–12,800 are all cache hits: 0 RTTs.
  - Total: 867 RTTs × 3 ms = **2.6 s per subsequent scan.**
- Third+ scans: same shape — cache contents are the last 5,000 pages every time. No improvement beyond the first repeat. (A forward-walk scan longer than the LRU converges to "miss the first N−cache_size, hit the last cache_size" steady state.)

**Cache effectiveness.** v2 gets a **one-time 40% speedup per scan** just from the last 5,000 pages being cached, but **does not converge to the opt-in-v1 ideal**. The fundamental issue: an LRU cache with a forward-walk access pattern is degenerate. The cache evicts exactly the pages we're about to need again.

**Fix: MRU cache or predictor-aware eviction.** If the cache evicted *most recently used* instead of *least recently used* during a long sequential scan, the cache would retain the first N pages of each scan and a repeat scan would hit them. mvSQLite notes this tradeoff and leaves it unaddressed; we have the same choice.

**Alternative fix: `kv_sqlite_preload` hint at query time.** If the application tells the actor "I'm about to scan this whole table, please preload pages `[1..12800]`", v2 can issue one (or a few) fat batch reads up front. At 512 keys/op that's `12,800 / 512 = 25 RTTs = 75 ms` to warm the cache, then the scan runs entirely from memory. This requires:
1. The cache to be large enough to hold the full scan (today 5,000, would need 12,800+).
2. A runtime preload API, not just startup-time preload hints.

Both are extensions of current v2 design.

**Prefetch predictor rating: (b) somewhat effective.** Same as Scenario 1 — predictor helps the miss phase, but cache strategy, not prefetch, is the lever that matters for repeated access. I'd argue this is the scenario where v2's design is **weakest** relative to a simple "big opt-in cache" in v1.

**Breakeven.** If the user enables the v1 opt-in read cache and has enough RAM, v1 wins on repeated scans after the first. v2 does not beat v1 here unless you either grow the cache to cover the working set or switch the eviction policy on sequential scans.

---

## Recommendations

Concrete v2 tuning parameters this workload class wants. Everything here is a tunable, not a fixed decision.

**Cache sizing.**
- Raise the default LRU cache size from the mvSQLite-inherited 5,000 pages to **10,000 pages (40 MiB)**. Rationale: our actors typically run one SQLite connection at a time, so the per-actor memory budget can absorb it, and 40 MiB covers most "small reporting database" working sets (Scenarios 1 and 4).
- Make cache size **configurable per-actor** with a sane upper bound (e.g., capped at 100 MiB per actor to keep actor density sane).
- Consider **MRU (most-recently-used) eviction** when the predictor reports a saturated stride, to avoid the "cache evicts what we're about to need" degenerate case in Scenario 4. Revert to LRU when stride confidence drops.

**Prefetch depth.**
- Default **`PREFETCH_DEPTH = 8`** (matches mvSQLite). Good for Scenarios 1 and 3.
- **Allow depth to scale up when the stride detector is saturated.** A saturated +1 stride for 16+ consecutive reads should bump the prefetch envelope to the payload limit: `min(remaining_payload_budget, 256)` pages per call. This is the concrete speedup for large sequential scans (Scenario 3 benefits most).
- Keep the envelope bounded by the `kv_sqlite_preload` 9 MiB / 512-key limit — at 2.2× LZ4 on the wire, 512 pages fits comfortably.

**Preload hints.**
- **Startup hints.** The ones described in the walkthrough. Useful for Scenario 2 (cold start working set) *if* the user knows their schema well enough to declare index roots and hot data ranges.
- **Runtime hints (new capability, currently unspecified).** Expose a per-query API like `c.db.preloadPages(pageno_list)` or `c.db.preloadTableRange(table_name, low, high)`. This is the lever for Scenario 4 (repeated scans) and addresses the Scenario-2 data-page dereference phase by letting the application tell v2 "these are the rows I'll need" before running the query. Implementation: one or a few fat `kv_sqlite_preload` calls before the query runs. Requires the application to reason about its access pattern, which is acceptable for the scenarios where it matters (reporting queries, dashboards).

**Protocol tuning.**
- The 9 MiB / 512-key per-op envelope is right for Scenario 3 where prefetch saturates. Don't shrink it below this.
- Consider a **"scatter-gather read" op** `kv_sqlite_fetch_pages(pgno_list)` distinct from `kv_sqlite_preload`. Same wire shape, but semantically "serve me this list of PAGE/ keys in one RTT." This is what the predictor is already effectively using via `kv_get` today; making it a first-class op lets the engine assume a single-txn snapshot and avoid spurious extra work.

**Open questions / design gaps this workload class flagged.**
- **Data-page dereference from an index scan** (Scenario 2) is the weakest point for the prefetch predictor. The predictor cannot know *in advance* which data pages an index leaf will point to, so it cannot prefetch them ahead of the index walk. The honest fix is an application hint ("after you scan this index range, warm the referenced data pages") or a cross-cut optimization in the VFS that peeks at the index leaf bytes before returning them. Neither is in the current v2 design. **Open question.**
- **MRU vs LRU eviction for sequential scans** (Scenario 4). Without this, v2 cannot beat v1 + opt-in cache on repeated full-table-scan workloads. **Open question**; leaning toward stride-aware MRU.
- **Cache-sizing defaults that match typical actor memory budgets.** Need a survey of actor RAM provisioning before fixing the 10,000-page number. **Open question.**
- **Predictor effectiveness on index → data page dereferences.** No hard numbers — the 3×, 7× estimates above are rule-of-thumb from the mvSQLite docs applied to our expected workloads. Worth a targeted bench. **Verification needed.**
- **Runtime preload hints** are not in the current v2 design. Adding them is a small protocol extension and a medium-sized VFS change. **Recommended for v2.0 if we can afford the scope; otherwise v2.1.**
