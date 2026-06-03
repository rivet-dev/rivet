# SQLite Storage Optimizations — Tracking File

Open optimization questions for the SQLite storage engine (`engine/packages/sqlite-storage/`). Tracked here so we can evaluate them as a set instead of one-off in conversation.

Each entry includes: what the optimization is, the tradeoff, why it's not in the current spec, and what would make us pull the trigger.

The current architectural baseline is `.agent/specs/sqlite-storage-stateless.md` + `.agent/specs/sqlite-pitr-fork.md`. Constraints floor is `r2-prior-art/.agent/research/sqlite/requirements.md` and `engine/packages/sqlite-storage/CLAUDE.md`.

---

## 1. Per-DELTA index instead of per-page PIDX

**Status:** Proposed. Worth deciding before implementation lands.

### The current shape (what's in the spec)

Every commit dirtying N pages writes N `PIDX/delta/{pgno_be:4} = txid_be:8` entries — one per dirty page. PIDX is the global routing table: for any pgno, look up `PIDX/delta/{pgno}` to find the owning DELTA blob.

Cost: **N FDB key writes per commit**, accumulating against FDB's 5 MB tx-size limit. For a 1000-page commit, ~1000 small writes plus their conflict-range entries.

### The alternative

Replace per-page PIDX with one per-DELTA index entry:

```
DELTA/{T}/{chunk_idx}     → blob bytes (unchanged)
DELTA_INDEX/{T}           → list of pgnos in T (one small value, ~4 bytes × N pages)
```

Writes per commit: **1 DELTA_INDEX entry**, regardless of dirty page count.

The in-RAM PIDX cache still exists, but it's bootstrapped from a range scan over `DELTA_INDEX/*` (returns K small list-values, decode + sort to build the pgno → max_txid map) instead of a range scan over `PIDX/delta/*`.

Cold-conn warmup is roughly the same shape (one FDB range scan, in-memory work to build the map). Hot-path reads after warmup are identical (RAM lookup + DELTA fetch).

### Tradeoffs

| | Per-page PIDX (current) | Per-DELTA index |
|---|---|---|
| Commit writes for N dirty pages | N | 1 |
| FDB tx size on commit | linear in N | constant |
| Cold-conn warmup | 1 range scan, ~N entries | 1 range scan, K list-values; decode + sort |
| Hot read (warm cache) | RAM lookup + DELTA fetch | same |
| Cold read (cache miss, single page) | 1 PIDX point-get + 1 DELTA fetch | range scan DELTA_INDEX + DELTA fetch (worse for selective single-page reads) |
| Compaction PIDX cleanup per folded T | N × `COMPARE_AND_CLEAR(PIDX[pgno], T)` | 1 × `delete DELTA_INDEX/{T}` |
| Stale-routing race | resolved by COMPARE_AND_CLEAR atomic per pgno | implicit: deleting DELTA_INDEX/{T} doesn't claim ownership; newer T's claims survive |
| Compose with `getMappedRange` (#2) | yes | no — the primary range disappears |

### Why this is worth considering

- **Commit-side cost is the original SQLite-on-Rivet pain point.** `r2-prior-art/.agent/specs/sqlite-remote-performance-remediation-plan.md` explicitly cites "1 MiB insert generating hundreds of KV writes" as the disease the existing fast-path tried to cure. Per-DELTA index is the structural cure on the index axis.
- **LiteFS already does this.** LiteFS LTX file's internal page index serves the same purpose as our PIDX, and LiteFS does not maintain a separate global PIDX. The DELTA blob's internal LTX index already names which pgnos live in T; per-DELTA index is "expose that index as a queryable separate KV." Reference: `.agent/research/litefs.md`, sections "LTX format (LiteFS variant)" and "Page index" — LiteFS uses an in-memory `[]Checksum` vector for rolling-checksum maintenance and has no per-page external index.
- **Compaction simplifies.** Hot compactor's "delete N PIDX entries via COMPARE_AND_CLEAR" becomes "delete 1 DELTA_INDEX entry." Race resolution becomes implicit (newer T's claims live in their own DELTA_INDEX entries).

### Why not rush it

- **Selective single-page reads on cold cache get worse.** Per-page PIDX gives a point-get; per-DELTA index requires a range scan. This rarely matters because the cache amortizes cold reads, but pathological workloads that hammer cold conns could feel it.
- **It precludes `getMappedRange` (#2).** If we want server-side indexed-read collapsing, we need per-page PIDX as the primary range.
- **It changes the on-disk layout.** Adopting it post-shipping would require migration. Adopting it pre-implementation is free.

### What would make us pull the trigger

- Confirmation that the typical commit dirty-page count justifies the optimization (instrumentation from `examples/sqlite-raw` benchmark).
- Decision on whether we want `getMappedRange` (#2). Per-DELTA index and `getMappedRange` are mutually exclusive.

### References

- `.agent/research/litefs.md` — LiteFS LTX format + page index design.
- `.agent/research/litestream.md` — Litestream's LTX V3 format spec (we share this format).
- `r2-prior-art/.agent/specs/sqlite-remote-performance-remediation-plan.md` — original commit-amplification analysis.
- `.agent/specs/sqlite-storage-stateless.md` § "On-disk layout" — current per-page PIDX shape.

---

## 2. `getMappedRange` for batched cold-cache reads

**Status:** Proposed; mutually exclusive with #1 if we choose per-DELTA index.

### The current shape

A `get_pages([pgno1, pgno2, ..., pgnoN])` against a cold WS conn:

1. RTT 1: `get_range(PIDX/delta/*)` to populate cache.
2. RTT 2-K (parallel): `get_range(DELTA/{T}/*)` for each unique owning txid.

Total: 1 + K serialized rounds (K is small, typically 1-3).

### The alternative

FoundationDB exposes `getMappedRange` ([wiki](https://github.com/apple/foundationdb/wiki/Everything-about-GetMappedRange)). One round-trip:

- Primary range: `PIDX/delta/{N_start}..{N_end}` (the requested pgnos).
- Mapper template: `prefix/DELTA/{V[0]}/{...}` — for each PIDX result, substitute the value (the txid) into the DELTA key range and fetch.
- Returns: PIDX entries + their corresponding DELTA byte ranges in one batched response.

Effect: arbitrary-batch-size cold reads collapse to **1 RTT**.

### Tradeoffs

| | Without getMappedRange | With getMappedRange |
|---|---|---|
| Cold-cache batched read | 1 + K parallel RTTs | 1 RTT |
| Cold-cache single-page read | 2 RTTs | 1 RTT |
| Warm-cache read | 1 RTT (cache + DELTA fetch) | unchanged (cache hit means no PIDX lookup) |
| Commit cost | unchanged | unchanged |
| FDB API surface | basic | requires UDB wrapper for `get_mapped_range` |
| PIDX value encoding | raw u64 BE (8 bytes) | tuple-encoded (~10 bytes; +2 byte overhead) |

### Why this is worth considering

- Selective batched reads are the Option F `sqlite_read_many` use case. If SQLite issues a 32-page sequential read with no Stride-prefetch warm-up, today's cold path is `1 + ceil(32 / max_keys) = 2-3 RTTs`. With `getMappedRange` it's 1 RTT.
- It's a pure read-path optimization. No commit cost. No correctness implications. Adoption risk is low if UDB exposes it cleanly.
- Server-side mapping reduces client-side coordination work. Less Rust glue, smaller serialization overhead per fetch.

### Why not rush it

- **Mutually exclusive with per-DELTA index (#1).** Mapping requires a primary range to scan; per-DELTA index has no per-page index to map from.
- **UDB needs the wrapper.** FDB has `get_mapped_range` natively (since 7.0), but UniversalDB doesn't expose it today. The stateless spec already lists `COMPARE_AND_CLEAR` and `MutationType::MIN` as UDB additions; this would join that list.
- **Mapper templates are fragile.** Field-index typos aren't caught at compile time. Need a typed wrapper.
- **Single-page reads benefit only slightly** (saves 1 RTT). The bigger wins are on multi-page batches, which are rare in steady state once the cache is warm.

### What would make us pull the trigger

- Decision on #1: if we keep per-page PIDX, this is essentially free perf. If we adopt per-DELTA index, this is off the table.
- Workload measurement: are cold-cache batched reads common enough to matter? Steady-state warm-cache reads don't benefit.

### References

- https://github.com/apple/foundationdb/wiki/Everything-about-GetMappedRange — input spec, mapper template syntax.
- `.agent/specs/sqlite-storage-stateless.md` § "Read path" — current 2-RTT cold-read shape.

---

## 3. Drop per-page metadata writes for SQLite pages

**Status:** Deferred from `sqlite-vfs-single-writer-plan.md`; revisit after instrumentation.

### The current shape

The actor-KV layer writes one `EntryMetadataKey` per value (carrying version string + `update_ts`, ~40 bytes). For SQLite pages, this is dead weight — SQLite owns versioning through its own pager state, and the metadata is never consulted for page reads.

### The alternative

Add a SQLite-only server read path that bypasses `EntryBuilder::build` for SQLite subspaces. Skip per-page metadata writes entirely.

### Why deferred

Per `sqlite-vfs-single-writer-plan.md`:

> `EntryBuilder::build` at `engine/packages/pegboard/src/actor_kv/entry.rs` calls `bail!("no metadata for key")` on missing metadata, and several generic-path consumers (`get`, `inspector`, `delete_all`, quota accounting) walk the SQLite subspace through that builder. Skipping per-page metadata would require a dedicated SQLite-only server read path that bypasses the generic entry builder. That is real work and it is worth doing **only after** US-025 through US-027 prove their wins on the read side.

### Tradeoffs

- Halves KV writes per commit (~100 KB saved per 10 MB commit).
- Requires forking the read path between SQLite and generic actor-KV. Inspector + `get` + `delete_all` + quota accounting all need updates.
- Storage layout becomes split (SQLite-specific subspace vs generic). Migration concern if landed post-shipping.

### What would make us pull the trigger

- Instrumentation from Phase 0 (the perf remediation plan's measurement gate) shows per-page metadata write cost dominating.
- The fork from generic-path consumers is acceptable.

### References

- `r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md` § "One optional data-structure optimization (not in this plan)".

---

## 4. Burst-mode signal derived from FDB lag (vs per-pod 5xx tracker)

**Status:** Adopted in `engine/packages/sqlite-storage/CLAUDE.md`; not yet propagated to `.agent/specs/sqlite-pitr-fork.md`.

### The current spec

`.agent/specs/sqlite-pitr-fork.md` § "Burst mode (S3 outage handling)" specifies:
- Cold compactor tracks per-pod S3 5xx ratio over the last 3 passes.
- Pod-level gauge `sqlite_cold_tier_degraded_pods` flipped when ratio exceeds threshold.
- Hot tier observes the gauge via UPS broadcast or polling.

### The alternative (already in CLAUDE.md)

Burst mode is **derived from FDB-observable state**, not from per-pod tracking:

- Envoy reads `lag = head_txid - cold_drained_txid` on every commit (already part of the META reads it does).
- When `lag > BURST_MODE_LAG_THRESHOLD` (TBD; should be byte-based not txid-based), envoy raises hot quota cap to `SQLITE_HOT_MAX_BYTES * HOT_BURST_MULTIPLIER`.
- Cold compactor pods are stateless w.r.t. burst-mode signaling; they just keep retrying.

### Why this is the right shape

- Eliminates the only stateful coordination on cold compactor pods.
- Threshold is ops-introspectable via FDB lag metrics.
- No UPS broadcast or polling; signal is local to each commit.

### What's left to do

- Update `.agent/specs/sqlite-pitr-fork.md` § "Quota and metering" to match CLAUDE.md.
- Decide threshold: byte-based (`8 GiB of un-drained DELTA`) is more meaningful than txid-based.

### References

- `engine/packages/sqlite-storage/CLAUDE.md` § "Statelessness contract".
- `.agent/research/review-operations.md` — original concern (S3 outage cascading to hot quota wedge).

---

## 5. Debug `validate_quota` cadence — derive from `materialized_txid % N`

**Status:** Adopted in CLAUDE.md; trivial, no spec change needed.

Replace per-pod `scc::HashMap<actor_id, u32>` pass-count tracker with deterministic `materialized_txid % quota_validate_every == 0`. Eliminates the second stateful bit on cold compactor pods.

### References

- `engine/packages/sqlite-storage/CLAUDE.md` § "Statelessness contract".

---

## 6. Move PIDX from per-page entries to LTX-internal index reuse

**Status:** Speculative; subsumed by #1 if we go that direction.

### The shape

DELTA blobs already carry an internal LTX index that names which pgnos are in T. In principle we could expose that index as the queryable routing source — fetch DELTA blob headers (not full bodies) on cold-conn warmup, derive PIDX in RAM.

### Why we didn't pursue it standalone

FDB range reads return full key-value pairs, not byte-prefixed values. Fetching just LTX headers requires either:

- Pay full DELTA blob bytes on warmup (huge waste).
- Two-phase: range over DELTA keys, then per-key `get` with byte limit (FDB doesn't expose byte-limited reads natively in most APIs; would require multiple round-trips).

The cleaner solution is to maintain the index as a separate small KV (#1: `DELTA_INDEX/{T}`).

### References

- `.agent/research/litefs.md` § "LTX format" — LiteFS's LTX-internal page index.
- `.agent/research/litestream.md` § "LTX V3 byte layout" — page-block + varint page-index layout.

---

## Decision matrix (high level)

| | Saves writes per commit | Saves cold-cache RTTs | Composable | Implementation cost |
|---|---|---|---|---|
| #1 per-DELTA index | yes (N → 1) | no | excludes #2 | medium (storage layout change pre-ship) |
| #2 getMappedRange | no | yes (K → 1) | excludes #1 | medium (UDB wrapper + mapper templates) |
| #3 drop SQLite metadata | yes (~halves) | no | composable | high (forks generic-path consumers) |
| #4 FDB-lag burst mode | n/a (correctness, not perf) | n/a | composable | trivial (already in CLAUDE.md, spec update pending) |
| #5 deterministic validate cadence | n/a (correctness) | n/a | composable | trivial (already in CLAUDE.md) |

## Cross-cutting considerations

- Both #1 and #2 require changes to UDB (`MutationType::MIN` for #1's `oldest_descendant_parent_txid`; `get_mapped_range` for #2). The stateless spec already requires UDB changes (`COMPARE_AND_CLEAR`); piggybacking is reasonable.
- All optimizations are pre-shipping; storage-layout changes are free now.
- None of these optimizations addresses the *read* hot-path under steady-state warm cache — that's Option F territory (`sqlite_read_many`, stride prefetch, `read_cache` defaults, `PRAGMA cache_size`). Option F is orthogonal and complementary.

## Where this slots in the implementation order

Follow `.agent/specs/sqlite-storage-stateless.md` Stage breakdown. The choice between #1 and #2 must be made before Stage 2 (greenfield `pump/`) lands, because both touch the on-disk key layout.

The recommendation today (subject to instrumentation): **adopt #1 (per-DELTA index)** because commit-side cost is the historically-cited pain and warm-cache reads don't benefit from #2 anyway. Defer #2 until measured workloads justify it. Defer #3 indefinitely until measured.

## Related research

- `.agent/research/litefs.md` — LiteFS architecture, LTX format, in-memory checksum vector.
- `.agent/research/litestream.md` — Litestream V3 LTX format details.
- `.agent/research/cf-durable-objects-sqlite.md` — SRS architecture.
- `.agent/research/turso-libsql.md` — bottomless replication.
- `.agent/research/neon-postgres.md` — pageserver layer model.
- `.agent/research/review-architecture.md`, `.agent/research/review-performance.md`, `.agent/research/review-operations.md` — adversarial review of the PITR/fork v1 spec.

## Related specs

- `.agent/specs/sqlite-storage-stateless.md` — base storage architecture (the floor for these optimizations).
- `.agent/specs/sqlite-pitr-fork.md` — PITR + forking v2 spec.
- `r2-prior-art/.agent/specs/sqlite-remote-performance-remediation-plan.md` — Phase 0 instrumentation plan + write-side fast path.
- `r2-prior-art/.agent/specs/sqlite-vfs-single-writer-plan.md` — Option F (read-side hydration + prefetch + `sqlite_read_many`).
- `r2-prior-art/.agent/research/sqlite/requirements.md` — binding constraints (single-writer, no local files, lazy reads).
- `engine/packages/sqlite-storage/CLAUDE.md` — package-level architectural invariants.

---

## Things to research later

These are open research threads, not active proposals. Pull them into the active list above (with a status, tradeoffs, references) once we sit down with FDB docs / source / instrumentation.

### R-1. FDB indexed-read primitives beyond `getMappedRange`

What we know: `getMappedRange` exists (since FDB 7.0) and is a candidate for #2 above.

What to look into:
- Are there other batched / mapped / indexed read APIs in FDB we haven't surfaced (e.g. `getRangeAndFlatMap`, prefix-mapped variants, anything client-side that batches differently)?
- What's the input-token grammar for the mapper template across FDB versions? Any constraints on tuple encoding we'd hit.
- Concurrency / staleness semantics — does `getMappedRange` give the same isolation as a regular `getRange`?
- Cost model: is there a hidden per-row server-side overhead that erodes the win at high fanout?
- Track the wiki link: https://github.com/apple/foundationdb/wiki/Everything-about-GetMappedRange.
- Any blog / talk material from FoundationDB Summit on indexed reads.

### R-2. Exact KV op count per operation, and assert it in code

For each public storage operation (`commit`, `get_pages`, `fork`, `restore_to_bookmark`, hot compactor pass, cold compactor Phase A/B/C), we should know **exactly** how many UDB reads, writes, atomic ops, and clears it issues, in steady state and on the cold path.

Today the spec asserts hot-path counts informally ("1 RTT for commit"). Research and document:

- Exact read/write/atomic-op count per operation, with conditions (cold cache, warm cache, first commit, etc.).
- Add inline comments in the implementation at the call sites, in the form `// UDB ops: 1 read (META/head), 1 write (META/head), 1 atomic_add (META/quota), N writes (PIDX/...)` so future code review can verify against the documented expectation.
- Add a debug-only `OpCounter` in `ActorDb` that increments per UDB op, asserted against the documented expectation in tests.
- Compare against the prior-art systems' equivalents for sanity: LiteFS commit = 1 LTX file write + 1 fsync; CF DO commit = 5 follower replicates + async R2; libSQL = 1 frame stream append.

This isn't a perf optimization on its own — it's the instrumentation that lets us catch regressions and validate optimization claims (#1, #2).

### R-3. Concurrency-overhead audit

Single-writer per actor (pegboard exclusivity) + single-reader/writer per branch (envoy exclusivity) + lease-based compactor exclusion mean we should NOT need most concurrency machinery. Things to audit:

- Are there unnecessary `Mutex` / `RwLock` / `parking_lot::Mutex` usages in the `ActorDb` that exist defensively but never actually contend? E.g. the rolling checksum cache, PIDX cache, ancestry cache — all have a single writer (the WS conn handler thread) so the Mutex is just CYA.
- Are there UDB conflict-range entries we're taking that don't need to exist because pegboard exclusivity already enforces serialization?
- Are there atomic ops we're using that could be plain reads/writes because there's only one writer?
- Are there `scc::HashMap` / `DashMap` per-bucket lock costs being paid for hash maps that have only one access path?
- Any `tokio::sync::Mutex` in the hot path that could be `parking_lot::Mutex` since there's no `.await` while held?

If the answer to any of these is "yes, we're paying overhead for concurrency that can't actually happen," document it and either remove or justify.

### R-4. Snapshot vs regular read audit

The stateless spec and PITR spec are explicit about which reads must be regular (take conflict ranges, used as OCC fences) vs snapshot (no conflict ranges, just consistency). Audit:

- Every `tx.get(...)` and `tx.get_range(...)` call in the implementation. Tag each with: "snapshot OK because X" or "regular required because Y."
- Particular care on:
  - Cold compactor Phase A reads (must be snapshot for `materialized_txid`, `head_txid`).
  - Cold compactor Phase C `cold_drained_txid` (regular — OCC fence against lease theft).
  - Hot compactor lease-take read (regular).
  - Fork's parent retention_pin_txid read (regular — OCC fence vs GC).
  - GC's read of children (snapshot — pin recomputes per pass).
  - `get_pages` reads (snapshot — single writer, no contention).
  - `commit`'s `META/head` read (regular for `head_txid` OCC, but only debug-mode; release uses snapshot per stateless spec goal 5).
- Ensure any "snapshot" choice actually corresponds to "no fence needed," not "I forgot to think about it."

The stateless spec already has rules for this; the audit is to verify the implementation matches.

### R-5. Adversarial architecture comparison via subagent

Spawn a subagent (or several in parallel) to read our current spec + CLAUDE.md and compare against each prior-art system, flagging optimizations the others use that we don't.

Specifically:

- **LiteFS** — page index is internal to LTX file (matches #1), HTTP/2 stream catch-up, position checksum chain. What else?
- **Litestream** — L0/L1/L2/L3 wall-clock-aligned compaction (we adopted), `Pos{TXID, checksum}` flat positioning (we adopted), single-PUT manifest. What else?
- **mvSQLite** — page hash content store (`page_hash → page_content`) for dedup, predictive prefetch with Markov+stride, multi-database commit groups. We rejected most of this for single-writer, but is there anything we missed?
- **Neon** — layer-file two-tier model (delta + image, we adopted), L0 backpressure, image_creation_threshold = 3, compaction_target_size = 128 MB. What tuning constants did we omit?
- **Cloudflare DO SRS** — output gates (workerd-level coalescing) + 16 MB / 10s batch upload trigger. Does our cold compactor trigger threshold match the cost shape?
- **Turso bottomless** — `.dep` chain, batched compression (zstd/gz). Worth zstd around layer files in our cold tier?

The adversarial agent should produce a markdown report at `.agent/research/architecture-comparison-optimizations.md` listing concrete optimizations the prior-art systems have that we don't, ranked by impact estimate and implementation cost.

This subagent run is the single biggest "things we might have missed" surface area. Do it before locking down implementation details.
