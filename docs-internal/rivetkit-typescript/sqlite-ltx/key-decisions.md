# SQLite VFS v2 — Key Decisions

The load-bearing constraints are C1 (warm reads must be zero RTT), C2 (writes are the primary optimization target), and C6 (~20 ms RTT between VFS and KV). Every decision below is the answer to "what is the right shape given those three?" This doc summarizes each decision, the alternative we rejected, and the workload it pays off on.

## SQLite runs in the actor process, not the engine

- Decision: the SQLite engine and its page cache live inside the actor process. The actor-side VFS is what talks to the engine.
- Why: C1. Warm reads are the dominant case for Rivet actors; they have to hit RAM, not the network.
- Alternative: Model A (engine-hosted SQLite, actor sends SQL strings). Also ruled out: Model C (canonical SQLite on engine, read-through cache on actor).
- When the alternative wins: never, given Rivet's workloads. It would only win if cold reads dominate and the working set is much larger than any reasonable cache, which is out of scope per C3.
- Concrete payoff: warm cache hit = 0 RTT. Under Model A every query pays 1 RTT (20 ms) even for data that never changed.

## Sharded storage instead of one KV key per page

- Decision: store pages in shards of ~64 pages (~256 KiB raw, ~128 KiB compressed) per KV value. v1's per-key layout is gone.
- Why: per-key engine overhead (metadata row, tuple encoding, UDB-internal chunking) is paid once per shard instead of once per page. Roughly a 1000× reduction in per-key work on any cold read or bulk write.
- Alternative: per-key (v1's current layout).
- When per-key wins: never at C6. Every cold read pays 1 RTT times the number of pages touched.
- Concrete payoff: a 100-page cold read is 100 RTTs × 20 ms = 2 s on v1 versus ~2 RTTs × 20 ms = 40 ms on v2. A 1 MiB bulk insert drops from ~5.7 s (287 RTTs) to ~60 ms (~3 RTTs).
- Note: this win is separate from the LTX/LZ4 compression win below. Sharding is the dominant factor (~1000×); compression is secondary (~2×).

## Delta log on top of shards

- Decision: two tiers — DELTA (small recent LTX files written directly on commit) and SHARD (larger compacted LTX files produced by background compaction).
- Why: writes don't pay read-modify-write on shards. A small commit lands in the DELTA tier in 1 RTT with no shard read, instead of 2 RTTs (read shard, write shard) on a shard-only layout.
- Alternative: Option C from `constraints.md`, shards-only with no delta log.
- When shards-only wins: read-mostly workloads. The only difference between C and D is the write side, so a pure reader workload is a tie.
- Why we picked delta log under C2+C6: eliminating the read half of every small commit is the single most impactful write optimization. The concrete numbers from the architecture table:
  - 4-page cold-shard commit: shards-only is 2 RTT (40 ms) and 256 KiB shipped; delta log is 1 RTT (20 ms) and 8 KiB shipped. 2× RTT win, 32× bandwidth win.
  - Hot-page rewrite × 100: shards-only is ~200 RTTs and ~25 MiB; delta log is ~100 RTTs and ~1 MiB. 2× RTT win, ~25× bandwidth win.
- Cost: background compaction, an orphan-delta cleanup path, and fencing (covered below).

## LTX as the on-disk format inside shards and deltas

- Decision: both DELTA and SHARD KV values are LTX-framed (LZ4-compressed pages, varint page index, sparse page support). Use the existing Rust `litetx` crate.
- Why: LZ4 is ~50% bandwidth and storage savings on typical B-tree pages, and decompression cost (~250 µs per 256 KiB shard at ~1 GB/s) is completely hidden by the 20 ms RTT.
- Alternative: raw concatenated pages inside shards.
- When raw wins: only if compression CPU mattered, which it doesn't at 20 ms RTT.
- Why we picked LTX: free 2× density win on top of sharding, with a mature Rust crate and no real implementation cost. LTX's rolling checksum is explicitly dropped (we don't replicate; UDB + SQLite already guarantee byte fidelity).
- Note: LTX is a separable choice from sharding. Sharding gives ~1000×; LTX gives ~2× on top.

## Compaction runs in the engine, not in the actor

- Decision: compaction (fold DELTAs into SHARDs) is an engine-side background task. The actor does not participate.
- Why: compaction reads + merges + writes KV state. In the actor it pays 3+ RTTs per pass (20 ms each). In the engine it pays ~0 because UDB is local.
- Alternative: actor-side materializer (the original v2 draft).
- Why we picked engine-side: at 20 ms RTT, every actor-side compaction pass foreground-blocks the actor for tens of milliseconds. Engine compaction is invisible.
- Bonus: the actor's `dirty_pgnos_in_log` map and an entire read-path layer disappear. The VFS read path collapses from 4 layers to 3 (write buffer, page cache, engine fetch).
- Note: compaction does not need to link SQLite. It is byte-level only (LTX decode, latest-wins merge by pgno, LTX encode).

## SQLite-specific runner-protocol op family, not a reuse of general KV

- Decision: a new `sqlite_*` op family (`takeover`, `get_pages`, `commit`, `commit_stage`, `commit_finalize`, `preload`) in a new runner-protocol schema version. No shared code with `actor_kv`.
- Why: different size envelopes (9 MiB vs 976 KiB), different semantics (atomic compound ops with fencing built in), and different evolutionary pressure (SQLite path will be tuned aggressively; general KV is stable for fairness).
- Alternative: extend the general KV API with new ops.
- Why we picked separation: the two systems share no concepts, no code, and no key namespace. General KV stays bounded for fairness; the SQLite path gets its own (larger) limits.

## Generation-token fencing on every SQLite op

- Decision: every op carries `(generation, expected_head_txid)`. The engine CAS-checks both. Fence mismatch is fatal for the actor.
- Why: the engine's runner-id ownership check runs in a separate UDB transaction from the KV write. During runner reallocation, two actor processes can briefly both believe they own the actor. Without fencing, interleaved commits on the head-pointer pattern corrupt the database.
- Alternative: trust the runner-id check and skip fencing.
- Why we picked fencing: an adversarial review found four correctness bugs that all traced to the missing fence. Fencing fixes them all at the cost of one CAS per op. Takeover bumps the generation on every cold start, so any stale runner's next op fails closed.

## Preload as a first-class op

- Decision: `sqlite_preload` is a hot-path op called immediately after `sqlite_takeover`. It bundles META + a list of warm pages + page ranges into one request.
- Why: cold-start latency at 20 ms RTT is the worst case for any actor that just got rescheduled. One batched preload turns "page-by-page warmup over hundreds of round trips" into 1 RTT.
- Alternative: lazy page fetch on first access.
- Why we picked preload: the workload analyses show cold-start latency is a meaningful fraction of total query time for short-lived actors.
- Concrete payoff: cold start is 2 RTTs total (takeover + preload) regardless of database size.

## How the decisions cash out by workload

- Write-heavy: delta log + sharding + LTX + 9 MiB envelope = ~10× speedup on bulk inserts (1 MiB: 5.7 s → 60 ms), ~2× on hot-row updates (RTT-wise; ~25× on bandwidth), and no regression on tiny commits.
- Read-heavy warm: local SQLite + always-on LRU cache = 0 RTT, identical to native SQLite.
- Read-heavy cold: sharding amortizes ~64 pages per fetch; the prefetch predictor doubles or triples that on sequential scans; preload eliminates first-query warmup. Overall cold-read win is ~5–17× depending on access pattern (full scan: ~9×; time-ordered range scan: ~7×).
- Index-then-heap-deref: the honest case where v2 only wins ~1.2–3×. The predictor can't guess random heap pages from index entries, and without a dereference hint in the protocol the data-page phase stays RTT-bound. This is the one workload the design does not meaningfully accelerate.
