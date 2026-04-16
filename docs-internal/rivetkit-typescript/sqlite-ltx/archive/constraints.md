# SQLite VFS v2 — Constraints

> Canonical source of truth for the load-bearing constraints behind the v2 design. Everything in [`protocol-and-vfs.md`](./protocol-and-vfs.md), [`compaction-design.md`](./compaction-design.md), [`key-decisions.md`](./key-decisions.md), and the workload analyses derives from this. If a constraint here changes, the design has to be re-evaluated. Earlier draft docs are in [`./archive/`](./archive/).
>
> **Status (2026-04-15):** Locked.

---

## The constraints

### C1 — Warm reads must be zero round trips

The dominant case for a Rivet SQLite actor is *"stable working set, repeated queries against it."* That case has to execute at memory speed. The only way to achieve this is for SQLite — including its page cache — to run inside the actor process.

**This rules out, definitively:**
- Engine-hosted SQLite ("Model A"). Every query would pay one RTT to the engine even when the data is hot, violating C1.
- Hybrid designs that put canonical SQLite on the engine side and a read-through cache on the actor side. Cache invalidation across the boundary is its own design hell, and warm reads still trip cache-miss RTTs more often than direct local SQLite would.
- Architectures that intercept above the SQLite pager (parse SQL in the actor, dispatch to remote ops). These bypass SQLite's own page cache and lose the free warm-read property.

### C2 — Writes are the primary optimization target

Within the space allowed by C1, the VFS is designed first and foremost to make writes fast: large atomic-commit envelopes, sharded storage (many pages per KV value), compression, prefetch and preload to keep cold reads tolerable. Reads are a secondary concern *because C1 already handles the warm case for free*.

This is a deliberate inversion of v1's tuning, which paid for cold-read latency with per-page KV keys. v2 trades a slightly more expensive cold read (fetch a shard, slice out the page) for a much cheaper write path.

### C3 — Cold reads pay round trips, and that's acceptable

Cold reads (cache misses) cannot be made zero-RTT because the data has to come from somewhere. The design optimizes them — sharding amortizes per-key overhead, prefetch coalesces sequential access, preload hints warm the cache on cold start — but doesn't try to drive them to zero. Workloads that can't tolerate cold-read latency (large random-access working sets that don't fit in cache) are out of scope and would need a separate runtime.

### C4 — No local disk; KV is the only durable store

Every byte of state lives in the actor's KV subspace. Page caches and dirty buffers are ephemeral and disappear with the actor process. Recovery on restart comes from KV, not from any local file.

### C5 — Single writer per actor (with fencing for the failover window)

Rivet schedules at most one actor process at a time. The engine's runner-id check is best-effort and has a brief window during runner reallocation where two processes can both believe they own the actor. v2 defends against this with **generation-token fencing**: every commit op carries `(generation, expected_head_txid)` and the engine performs a CAS, failing closed on mismatch. On startup, every new actor bumps the generation. This makes the head-pointer commit pattern safe under concurrent writers.

### C6 — Round-trip latency between the VFS and the KV is high (~20 ms typical)

The design assumes ~20 ms per round trip from the VFS to the engine actor KV. This is the load-bearing parameter for sizing batches, choosing between local cache and remote fetch, and deciding when to pay CPU to save round trips.

If production turns out to be lower, the design still works (just less critically). If it turns out to be higher, the design's value goes up — every architectural decision that saves a round trip pays back proportionally.

### C7 — Dispatch between v1 and v2 VFS uses the existing engine schema-version flag

The v2 engine already has a schema-version mechanism that routes between v1 and v2 actor implementations. v2 SQLite VFS piggybacks on it — no new dispatch byte, no probing keys, no separate version tag in the SQLite subspace. Whatever the engine says about an actor's schema version is what determines which VFS implementation it gets.

### C8 — Breaking API compatibility for SQLite v2 is acceptable

v1 actors stay on v1 forever, v2 actors are a new world. There is no Drizzle compatibility shim, no v1 trait surface to preserve, no on-disk format compatibility to maintain. v2 can change:
- The wire format on the runner protocol (add new ops freely)
- The on-disk KV layout (sharded, compressed, indexed however we want)
- The Rust-side `SqliteKv` trait surface (new methods, new error variants)
- The user-facing JS/TS SQL API surface, if it makes the design materially better

The only thing v2 cannot do is corrupt v1 actors' data. Since dispatch happens at the engine schema-version level (C7), there is no shared key space and no risk of cross-contamination.

---

## What the constraints rule out, definitively

| Idea | Ruled out by |
|---|---|
| Engine-hosted SQLite (Model A — engine runs SQLite, actor sends SQL strings) | C1 (warm reads would always be ≥1 RTT) |
| Hybrid local + remote SQLite (Model C — engine canonical, actor cache) | C1 (cache invalidation across the boundary, warm-read regressions) |
| Per-page KV layout for v2 (one KV key per SQLite page) | C2+C6 (per-key overhead × 20 ms RTT × number of pages = unacceptable cold-read and bulk-write cost) |
| v1→v2 migration | C7+C8 (the engine schema-version flag separates them; no migration needed because they don't coexist for the same actor) |
| Drizzle compatibility shim or any v1 API preservation | C8 |
| LTX rolling checksum maintenance | (implicit) v2 does not replicate or use third-party LTX tooling, so the integrity guarantee is provided by SQLite + UDB byte fidelity |
| Materializing LTX log entries into per-page KV keys (the original v2 LOG → PAGE materializer) | C2 (it pays LTX encoding cost on the way in and per-page cost on the way out, capturing neither benefit) |

---

## What the constraints imply about the math

The existing benchmark in `examples/sqlite-raw/BENCH_RESULTS.md` was captured at ~2.9 ms RTT (local dev). Under C6's 20 ms RTT assumption, every v1 cold-path number scales by roughly 7×:

| Workload | v1 @ 3 ms (today's bench) | v1 @ 20 ms (production target) | v2-shards @ 20 ms |
|---|---|---|---|
| 1 MiB insert | 832 ms (287 RTTs) | **~5.7 s** | ~3 RTTs × 20 ms = **60 ms** |
| 10 MiB insert | 9438 ms (~2k RTTs) | **~65 s** | ~5 RTTs × 20 ms = **100 ms** |
| 100-page cold read | ~290 ms (100 RTTs) | **~2 s** | ~2 RTTs × 20 ms = **40 ms** |
| Warm read of cached page | ~5 µs (0 RTT) | ~5 µs | ~5 µs |

Speedups of 50×–650× on the cases v2 actually targets. **Under C6, v1 is borderline unusable for any non-trivial write workload, and v2 is not optional — it is the only way Rivet SQLite can serve serious workloads at the production RTT.**

---

## Architectural decision: which layout

The design space inside the C1+C2+C6 envelope is a small set:

| Option | Layout | Pros | Cons |
|---|---|---|---|
| **A** | Per-key (v1 today) | Simple. Reads are 1 RTT per page on miss. | Per-key overhead × 20 ms RTT = catastrophic for any cold workload. |
| **B** | Sharded raw bytes (~64 pages per KV value, raw concatenation) | ~1000× per-key overhead reduction. Simple format. | Every commit must read-modify-write affected shards. Small commits pay full shard cost. No compression. |
| **C** | Sharded LTX (LZ4 inside each shard) | Same per-key win as B + ~2× compression on shards. | Same RMW-per-commit problem as B. CPU cost on read decompression (small). |
| **D** | Sharded LTX + delta log (DELTA tier of small recent LTX files, SHARD tier of larger compacted LTX files) | Small commits land in DELTA in 1 RTT with no shard rewrite. Background compaction folds DELTA → SHARD. Best write latency for both small and large commits. | Most machinery: in-memory delta page index, background compaction task, fencing-protected materialize op. |

**Recommendation: Option D (sharded LTX + delta log).**

Reasoning, walked carefully through realistic workloads at C6 = 20 ms RTT. (An earlier version of this section overstated D's win on large commits; the numbers below are the honest comparison.)

Assumed shard size: ~64 pages = ~256 KiB raw, ~128 KiB LZ4-compressed. Assumed envelope: ~9 MiB per `kv_sqlite_*` op.

A small 4-page commit (the dominant OLTP case):
- Option B (raw shards): cold path is 1 RTT shard read + 1 RTT shard write = **40 ms**, ships 256 KiB. Warm shard: 1 RTT (write only) = 20 ms, ships 256 KiB.
- Option C (LTX shards): same as B with compression. **40 ms** cold, 20 ms warm. Ships 128 KiB.
- Option D (delta log): write delta directly, no shard read needed. **20 ms**, ships ~8 KiB.

A 5,000-page commit (~80 shards affected, ~10 MiB compressed total):
- Option B: 2 RTTs to read 80 shards (envelope-split) + 2 RTTs to write them back = **80 ms**, ships ~20 MiB total.
- Option C: same RTT pattern as B with compression. **80 ms**, ships ~10 MiB.
- Option D: encode all 5,000 pages as one LTX delta (~10 MiB compressed) and write it. Doesn't fit one envelope, so 2 RTTs = **40 ms**, ships ~10 MiB.

A hot-page rewrite (same 4 pages updated 100 times):
- Option B: 100 × (read shard + write shard) = ~200 RTTs (or ~100 with caching), ships ~25 MiB. ~4 s.
- Option C: same RTT pattern, ships ~12.5 MiB compressed. ~4 s.
- Option D: 100 × 1 RTT delta append + 1 × compaction RTT, ships ~1 MiB. ~2 s. **2× RTT win, ~25× bandwidth win.**

A cold single-page read:
- Option B: 1 RTT for whole shard = **20 ms**, ~256 KiB transfer.
- Option C: 1 RTT for compressed shard = **20 ms**, ~128 KiB transfer, ~250 µs decompress.
- Option D: 1 RTT for shard or delta = **20 ms**, ~128 KiB transfer.
- Tie.

A warm cache hit (any option): **0 RTT**. The point of C1.

| Workload | A (per-key v1) | B (raw shards) | C (LTX shards) | **D (LTX + delta)** |
|---|---|---|---|---|
| 4-page commit, cold shard | 1 RTT (20 ms) | 2 RTT (40 ms), 256 KiB | 2 RTT (40 ms), 128 KiB | **1 RTT (20 ms), 8 KiB** |
| 4-page commit, hot shard | 1 RTT (20 ms) | 1 RTT (20 ms), 256 KiB | 1 RTT (20 ms), 128 KiB | **1 RTT (20 ms), 8 KiB** |
| 5,000-page commit | ~2k RTT journal-fallback (40 s) | 4 RTT (80 ms), 20 MiB | 4 RTT (80 ms), 10 MiB | **2 RTT (40 ms), 10 MiB** |
| Hot 4-page rewrite × 100 | 100 RTT (2 s), 100 raw page writes | ~200 RTT (4 s), 25 MiB | ~200 RTT (4 s), 12 MiB | **~100 RTT (2 s), 1 MiB** |
| Cold single-page read | 1 RTT, 4 KiB | 1 RTT, 256 KiB | 1 RTT, 128 KiB | **1 RTT, 128 KiB** |
| Warm read | 0 | 0 | 0 | **0** |

**Option D wins or ties on every workload class.** The biggest wins are:
1. **Small commits**: 2× RTT win + 32× bandwidth win over B/C, because writes don't have to read the shard first.
2. **Hot-page rewrites**: 2× RTT + ~25× bandwidth win, because deltas are tiny and compaction folds them lazily.
3. **Cold-shard commits of any size**: D skips the read-then-write penalty B/C must pay.

D's win on the 5,000-page case is a smaller 2× factor (40 ms vs 80 ms) because both options are envelope-bound, not RTT-bound, at that size. The dramatic D advantage is at the *small* end of the commit-size distribution, not the large end.

Where D barely wins:
- Large commits when affected shards are already hot in cache: ~tie on RTT, D wins on bandwidth only.
- Cold single-page reads: tie. Both fetch one shard.

The cost of D over B/C is implementation complexity:
- An in-memory `dirty_pgnos_in_log` map (or equivalent) so reads know which pages live in deltas.
- A background compaction task that merges deltas into shards.
- A fencing-protected atomic op that writes a new shard + deletes folded deltas + advances META.
- Recovery logic for orphan deltas after a crash.

These are real but well-understood. The earlier adversarial review identified a handful of correctness hazards in the original design's analogous machinery; the shard+delta variant avoids most of them because the compaction unit is one shard key (not a multi-key range delete + per-page write sequence).

### LZ4 compression: in or out?

Independently of the layout choice, we have to pick whether the bytes inside each shard or delta are LZ4-compressed (LTX style) or raw. **Recommendation: in.** LZ4 decompression is ~1 GB/s — a 256 KiB shard decompresses in ~250 µs, completely hidden by the 20 ms RTT. The compression saves ~50% on bytes shipped per cold read and ~50% on KV storage cost. Net positive at 20 ms RTT.

If the implementation cost ever becomes a problem, ship D-with-raw-bytes first and add LZ4 in a follow-up. The format is internal to v2; we can change it freely.

---

## What stays open

These are not constraint questions but they affect implementation tuning. None of them block the architecture decision above:

- **Shard size.** ~64 pages (~256 KiB raw, ~128 KiB compressed) is the working assumption. Trade is per-shard fetch cost vs. per-shard read overfetch on point lookups. Needs measurement.
- **Default page cache size.** mvSQLite uses 5,000 pages (~20 MiB). The workload analyses suggest 50,000 pages (~200 MiB) for analytical actors. Probably make it per-actor configurable with a sensible default; pick the default empirically.
- **Compaction trigger.** Time-based, delta-count-based, or delta-size-based? Probably delta-size-based with an upper bound on delta count.
- **Compaction concurrency.** Always-on background task vs. on-idle vs. only-when-pressured. Each has tail-latency implications.
- **Preload hint API surface.** Config-time list, runtime mutable, or both? Per-key, per-range, or tagged?
- **20 ms RTT — typical or worst case?** If typical, v2 is the highest-priority project on the team. If worst case (most users <5 ms, a few cross-region), the urgency is "design for it" rather than "ship yesterday." The architecture is the same either way.

---

## Update log

- **2026-04-15** — Initial constraint set locked. C0–C8 defined. Architecture decision: Option D (sharded LTX + delta log).
