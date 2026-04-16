# SQLite VFS v2 — Parameters to Tune

Things we need to measure and tune empirically before and after launch. Each parameter has a working default, a hypothesis about the right range, and what measurement would settle it.

## Storage layout

- **`S` (shard size, pages per shard):** Default 64 (~256 KiB raw, ~128 KiB compressed). Trade: larger shards = fewer per-key overhead entries but more bytes transferred per cold read (overfetch). Smaller shards = more per-key overhead but less overfetch. Measure: cold-read latency vs. write throughput across S = {16, 32, 64, 128, 256}. The right S minimizes total round-trip-weighted cost for the dominant workload mix.
- **SQLite page size:** Default 4096. SQLite supports 512–65536. Larger pages = fewer pages per DB = fewer PIDX entries = fewer compaction passes, but more write amplification per row update (overwrite 16 KiB to change 1 byte). 4096 is SQLite's default and matches KV billing chunks. Probably don't change unless benchmarks strongly motivate it.
- **v2 prefix byte:** Proposed `0x10`. Must be disjoint from v1's `0x08`. No performance implication, just a correctness guard.

## Actor-side VFS

- **Page cache capacity:** Default 50,000 pages (~200 MiB). Trade: bigger = more warm reads (C1 payoff), higher memory per actor. Smaller = more cold reads (RTT cost under C6). Measure: cache hit rate and memory pressure across {5k, 10k, 25k, 50k, 100k} pages for representative workloads. The right number depends on typical working-set size vs. actor density per host.
- **Prefetch depth:** Default 16 (same as mvSQLite). Trade: deeper = more pages fetched per RTT on sequential scans, but more wasted bandwidth on random access. Measure: prefetch hit rate and overfetch ratio across {4, 8, 16, 32, 64}. Sequential-heavy workloads want higher; random-heavy want lower.
- **Max pages per commit stage (slow path chunk size):** Default 4,000. Trade: larger chunks = fewer RTTs on the slow path, but each chunk is a bigger network transfer. Constrained by the ~9 MiB per-op envelope. Measure: slow-path commit latency across {1000, 2000, 4000, 8000}.

## Compaction

- **`N_count` (delta count threshold):** Default 64. Number of unfolded deltas before compaction triggers. Lower = more compaction CPU, fewer cold-read penalty from delta scans. Higher = less CPU, more delta scan cost. Measure: compaction CPU overhead vs. cold-read latency across {16, 32, 64, 128, 256}.
- **`B_soft` (delta byte threshold):** Default 16 MiB compressed. Soft trigger for compaction. Measure: storage amplification ratio at different thresholds.
- **`B_hard` (back-pressure threshold):** Default 200 MiB compressed. Hard limit — engine refuses new commits until compaction drains below this. Trade: higher = more tolerance for write bursts, more storage consumed. Lower = tighter write latency ceiling but potential for write stalls. Measure: write stall frequency and duration under sustained write pressure at {50, 100, 200, 500} MiB.
- **`T_idle` (idle timer):** Default 5 s. How long to wait after the last write before triggering an idle-compaction pass. Lower = more responsive compaction, more CPU on lightly-loaded actors. Higher = less CPU, more delta accumulation. Probably fine at 5 s.
- **`shards_per_batch` (fairness budget):** Default 8. Max shards a single actor's compaction can process before yielding to the scheduler queue. Trade: higher = faster compaction for hot actors but starves other actors. Lower = fairer but slower individual compaction. Measure: tail latency for a cold actor's compaction when co-hosted with a noisy hot actor.
- **Compaction worker pool size:** Default `max(2, num_cpus / 2)`. Trade: more workers = higher compaction throughput, more CPU contention with the engine's other work. Measure: compaction lag under sustained write pressure at different pool sizes.

## Protocol

- **Max commit payload (fast-path envelope):** Default ~9 MiB. Constrained by UDB's 5 s transaction timeout — the write has to complete within the timeout including network time. Measure: actual UDB tx latency for 1, 5, 9, 15 MiB writes across the postgres and rocksdb drivers.
- **Max get_pages response size:** No current limit. Could become a problem if prefetch returns hundreds of pages and the response is 50+ MiB. Measure: response deserialization time and memory pressure at high prefetch depths. Probably add a limit once benchmarks reveal the inflection point.

## Preload

- **Default preload hint set:** Currently empty (user-configured). Should we preload page 1 unconditionally? First N pages (schema/index roots)? Measure: cold-start latency with and without default preload at {0, 1, 100, 500, 1000} pages.
- **`max_total_bytes` for preload:** Default TBD. Safety bound on the preload response. Too low = actor has to page-fault on frequently-needed pages. Too high = preload response is a multi-MiB blob that wastes bandwidth if most pages aren't needed. Measure: preload hit rate (fraction of preloaded pages that are actually read within the first 5 s of actor lifetime).

## How to tune

The plan is:
1. Ship with the defaults listed above.
2. Add metrics instrumentation (cache hit rate, prefetch hit rate, compaction lag, cold-start latency, write stall count, storage amplification ratio) from day 1.
3. Run the `examples/sqlite-raw` bench (extended with a v2 mode) against real engine instances at realistic RTT.
4. Sweep each parameter independently while holding the others at defaults. Identify knee points.
5. Set production defaults based on the sweep results.
6. Expose the parameters as per-actor config so power users can tune for their workload.
