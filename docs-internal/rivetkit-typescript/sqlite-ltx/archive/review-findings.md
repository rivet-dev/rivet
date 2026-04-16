# v2 Design Review Findings

## Critical (must fix before implementation)

1. **C7 "existing engine schema-version flag" does not exist.** The constraint says "the v2 engine already has a schema-version mechanism that routes between v1 and v2 actor implementations." No such mechanism exists in the engine. The `SQLITE_SCHEMA_VERSION = 0x01` byte at `rivetkit-typescript/packages/sqlite-native/src/kv.rs:14` is a constant embedded in the *actor-side* key encoding, not an engine-side routing flag. The engine (`engine/packages/pegboard/src/actor_kv/`) has zero awareness of SQLite schema versions. `protocol-and-vfs.md` Section 1 says "dispatch between the two happens at the engine schema-version flag (per C7)" and `walkthrough.md` Chapter 11 says "dispatch happens at actor open time by reading the schema-version byte of the first key in the actor's KV subspace." These are two different mechanisms, and neither actually exists today. The protocol-and-vfs.md description (new `actor_sqlite/` subsystem with new runner-protocol ops routed by the engine) is architecturally sound, but C7 as written is factually wrong about an existing mechanism. **Fix:** Rewrite C7 to say "v2 dispatch uses the runner-protocol schema version (v7 vs v8 ops)" or "dispatch uses a new per-actor config flag." Delete all references to "the existing engine schema-version flag" since there is no such flag.
   - constraints.md C7
   - protocol-and-vfs.md Section 1 paragraph 2
   - walkthrough.md Chapter 11 paragraph on dispatch
   - key-decisions.md does not reference a specific mechanism (OK)
   - design-decisions.md 1.5 says "reading the version byte of the first key" -- contradicts protocol-and-vfs.md which says engine-level dispatch

2. **`antiox::sync::mpsc` is a TypeScript library, not a Rust crate.** `compaction-design.md` Section 6.1 specifies `antiox::sync::mpsc::UnboundedChannel<Id>` as the Rust scheduler queue, and `protocol-and-vfs.md` Section 4.6 repeats this. `antiox` is a TypeScript concurrency library per CLAUDE.md ("Use `antiox` for TypeScript concurrency primitives"). It has no Rust equivalent. The engine is written in Rust. **Fix:** Replace with `tokio::sync::mpsc::UnboundedSender` / `UnboundedReceiver` (already in the workspace dependency tree via `tokio`).
   - compaction-design.md Section 6.1 line 329
   - protocol-and-vfs.md Section 4.6 line 879

3. **`litetx` crate does not exist on crates.io.** The design references `litetx` (crates.io, Apache-2.0) in at least 6 places across key-decisions.md, compaction-design.md, and protocol-and-vfs.md. Searching the workspace `Cargo.toml` finds no `litetx` dependency. The Fly.io LTX format is documented in the `litefs` and `litestream` ecosystems but the Go reference implementation was never published as a standalone Rust crate. The only Rust LTX code in the open-source ecosystem is a partial implementation in `litefs-go`. **Fix:** Either (a) write a minimal Rust LTX encoder/decoder (compaction-design.md Section 8.3 already estimates ~200 lines), (b) find the actual crate name if one exists, or (c) fork the Go implementation. This is a P0 risk because the entire format layer depends on it. Mark the crate as "to be written" rather than "to be imported."
   - key-decisions.md Section "LTX as the on-disk format" line 35
   - compaction-design.md Section 0 line 19, Section 4.2 lines 203-208, Section 8.3
   - protocol-and-vfs.md Section 4.1 line 39

4. **`protocol-and-vfs.md` and `design-decisions.md` define two completely different protocol shapes.** `protocol-and-vfs.md` Section 2 defines a clean `sqlite_*` op family with BARE schema types (`SqliteTakeoverRequest`, `SqliteGetPagesRequest`, `SqliteCommitRequest`, etc.) where the engine owns the storage layout. `design-decisions.md` Section 2.2 defines a different protocol (`KvSqliteCommit`, `KvSqliteCommitStage`, `KvSqliteMaterialize`, `KvSqlitePreload`, `KvSqliteTakeover`) where the *actor* encodes LTX, constructs `LOG/` keys, and sends raw key-value writes. These are fundamentally incompatible architectures: one puts the LTX encoding in the engine, the other puts it in the actor. `protocol-and-vfs.md` is the newer document (status: draft, complete). **Fix:** Mark the `design-decisions.md` Section 2.2 protocol sketch as **superseded** by `protocol-and-vfs.md` Section 2. Add a note at the top of design-decisions.md Section 2 saying "This section describes an earlier protocol sketch. See protocol-and-vfs.md Section 2 for the current design."
   - design-decisions.md Section 2.2 (the `KvSqliteCommit` etc. structs)
   - design-decisions.md Section 2.4 (the `SqliteKv` trait extensions)
   - design-decisions.md Section 3 action items reference "implement the engine-side handlers per Section 2.2"

5. **`test-architecture.md` is designed against the superseded protocol.** The `MemoryKv` driver, `SqliteKv` trait extensions (Section 3.5), and the harness are all built around the old `KvSqliteCommitOp` / `KvSqliteMaterializeOp` shapes from `design-decisions.md` Section 2.2. They assume the actor builds LOG/PAGE keys and sends raw KV writes, which contradicts `protocol-and-vfs.md` where the engine owns storage. The test architecture needs a rewrite to test against the `SqliteV2Protocol` trait from `protocol-and-vfs.md` Section 3.1, not the `SqliteKv` trait extensions from the old design. **Fix:** Rewrite `test-architecture.md` Section 3.5 and all test-case assumptions to align with `protocol-and-vfs.md`.
   - test-architecture.md Section 3.5 (entire `SqliteKv` trait extension)
   - test-architecture.md Section 5 Tier B tests reference `dirty_pgnos_in_log` (removed in new design)

6. **Storage prefix inconsistency: `0x02` vs `0x10`.** `protocol-and-vfs.md` Section 1 says "v2 uses a disjoint prefix (proposed `0x10`)" while `compaction-design.md` Section 1 says "Schema-version byte `0x02` prefixes everything." `protocol-and-vfs.md` Section 4.1 also uses `0x02`. These are different bytes. **Fix:** Pick one and make it consistent. `0x02` (schema version 2) is more natural. Update `protocol-and-vfs.md` Section 1 to say `0x02` instead of `0x10`.
   - protocol-and-vfs.md Section 1 line 13: says `0x10`
   - protocol-and-vfs.md Section 4.1 line 755: says `0x02`
   - compaction-design.md Section 1 line 25: says `0x02`

## Important (design needs adjustment)

7. **Compaction CAS checks `generation` but not `materialized_txid`.** `protocol-and-vfs.md` Section 4.5 says "Compaction's UDB tx CAS-checks `(generation, materialized_txid)`" but the actual pseudocode in `compaction-design.md` Section 4.2 line 172 only checks `generation`: `if head.generation != expected_generation { return Err(FenceMismatch); }`. There is no CAS on `materialized_txid`. The Section 5.1 discussion says "both CAS on `generation`" with no mention of `materialized_txid` CAS in the compaction path. The compaction pseudocode does advance `materialized_txid` in step 10, but a concurrent compaction for a different shard could have already advanced it. Without a CAS, two concurrent shard compactions could race on `materialized_txid`. This is partially mitigated by the `in_flight` serialization per actor, but the docs should be consistent about what is CAS-checked.
   - protocol-and-vfs.md Section 4.5 line 866
   - compaction-design.md Section 4.2 line 172

8. **`B_soft` is 16 MiB in trigger policy but 100 MiB in back-pressure.** `compaction-design.md` Section 2.1 defines `B_soft = 16 MiB` as the trigger threshold. But Section 5.4 line 314 says "`> B_soft = 100 MiB` -> succeed but return `compaction_pressure`." These are the same variable name with different values. Either they are two different thresholds that need different names, or one is wrong.
   - compaction-design.md Section 2.1 line 49: `B_soft = 16 MiB`
   - compaction-design.md Section 5.4 line 314: `B_soft = 100 MiB`

9. **`SqliteCommitTooLarge` response has no protocol-level way for the actor to avoid the round trip.** `protocol-and-vfs.md` Section 2.2 Op 3 says the engine checks `MAX_DELTA_BYTES` and returns `SqliteCommitTooLarge`. The actor-side VFS code in Section 3.4 always tries the fast path first, eats the rejection, then retries with the slow path. This wastes one full RTT (20 ms) on every large commit. The actor could avoid this by pre-computing the compressed size locally, but the doc never specifies `MAX_DELTA_BYTES` or gives the actor a way to know it. **Fix:** Either (a) include `max_delta_bytes` in the `SqliteMeta` returned by `sqlite_takeover` so the actor can pre-check, or (b) document that the wasted RTT is acceptable and explain why.
   - protocol-and-vfs.md Section 2.2 Op 3 line 182
   - protocol-and-vfs.md Section 3.4 lines 614-630

10. **No `STAGE/` cleanup in compaction.** `protocol-and-vfs.md` Section 2.2 Op 4 says staged chunks are stored under `STAGE/<stage_id>/<chunk_idx>`. `protocol-and-vfs.md` Section 4.7 recovery scan handles `DELTA/` orphans (txid > head) and `DELTAREF/` leaks, but there is no mention of scanning `STAGE/` entries. If the actor crashes after staging but before finalizing, `STAGE/` keys accumulate as permanent garbage. `compaction-design.md` Section 6.4 mentions "STAGE/" but only in the context of deleting DELTAREF and STAGE entries for orphan deltas from crashed slow-path commits, not standalone STAGE orphans.
    - protocol-and-vfs.md Section 4.7 lines 889-895
    - compaction-design.md Section 6.4 lines 350-361

11. **`dirty_pgnos_in_log` still referenced in multiple places despite being removed.** `protocol-and-vfs.md` Section 3.3 explicitly says "The `dirty_pgnos_in_log` map is gone." But `walkthrough.md` Chapters 6, 7, 9 still describe it as a live component of the read path and the materializer. `workload-aggregations.md` Scenario 2 failure mode 3 says "The `dirty_pgnos_in_log` lookup runs 55,000 times." `workload-point-ops.md` Scenario 1 references it. These are stale references from the older draft. **Fix:** Add a note to walkthrough.md that this concept is superseded, and update the workload docs if they are used for planning.
    - walkthrough.md Chapter 6 line 309
    - walkthrough.md Chapter 9 line 430
    - workload-aggregations.md Scenario 2 failure mode 3 line 103
    - workload-point-ops.md Scenario 1 line 60

12. **`kv_sqlite_materialize` op is in `design-decisions.md` but absent from `protocol-and-vfs.md`.** In the new architecture, compaction runs engine-side and there is no `materialize` op. But `design-decisions.md` Section 2.2, `walkthrough.md` Chapters 5, 9, 12, and `test-architecture.md` Section 3.5 all reference `kv_sqlite_materialize` as an actor-to-engine call. This is the single largest semantic difference between the old and new designs. **Fix:** Mark `kv_sqlite_materialize` as **dropped** in `design-decisions.md` and add it to the "dropped" list in Section 3.

## Clarifications needed (ambiguous specs)

13. **Initial META values on first-ever `sqlite_takeover`.** `protocol-and-vfs.md` Section 2.2 Op 1 says "the engine creates the initial META and DBHead." What are the initial values? Implied from context: `head_txid = 0`, `next_txid = 1`, `materialized_txid = 0`, `db_size_pages = 0`, `page_size = 4096`, `generation = 1`, `creation_ts_ms = now()`. But `page_size` is never negotiated. What if the actor wants 8192-byte pages? The protocol has no field for this. **Fix:** Explicitly list the initial values. Decide whether `page_size` is fixed at 4096 or negotiable.

14. **What does `sqlite_get_pages` return for `pgno = 0`?** SQLite uses 1-indexed page numbers. Page 0 is never a valid request. The spec says `bytes: absent if pgno > db_size_pages`, but doesn't cover pgno = 0. **Fix:** Document that pgno = 0 is invalid and the engine returns an error (or omits it from the response).

15. **Stage ID generation.** `protocol-and-vfs.md` Section 2.2 Op 4 says "`stage_id` (a random u64)." The VFS pseudocode in Section 3.4 line 631 calls `generate_stage_id()` without defining it. What if two slow-path commits on the same actor use the same random stage_id? The probability is ~1/2^64, which is negligible, but the spec should say "collision is fatal, use a cryptographic RNG" or "collision is a soft error, retry with a new stage_id." **Fix:** Add a one-liner about the generation strategy and collision handling.

16. **Encoding of `SqlitePageBytes` on the wire.** The spec says "uncompressed when sent over the wire" but the engine "compresses on the way to UDB." Is this LZ4 compression? Is the LZ4 frame the same format as the LTX page body? Or is the engine free to use any compression? This matters for the `litetx` crate dependency. **Fix:** Specify: "The engine encodes dirty pages into an LZ4-compressed LTX blob for storage. The wire format between actor and engine carries raw 4 KiB pages."

17. **What happens if `sqlite_takeover` succeeds but `sqlite_preload` fails?** `protocol-and-vfs.md` Section 3.2 treats preload failure as `VfsError::FenceMismatchOnPreload`. But a network error during preload is not a fence mismatch. The actor has already bumped the generation but has no warm cache. Is this recoverable? Can the actor retry the preload? **Fix:** Add a recovery path: if preload fails with a non-fence error, the actor should retry the preload (not the takeover, since generation is already bumped).

18. **What happens to writes outside an atomic-write window?** `protocol-and-vfs.md` Section 3.4 `x_write_v2` has a comment "Outside an atomic-write window. SQLite is doing direct page writes... We still buffer -- the next sync will commit a single-page tx." But `xSync` is a no-op (Section 3.5). So when do these buffered writes actually commit? There is no trigger. If SQLite writes outside the atomic window and then reads the same page, the dirty buffer serves it, but it is never persisted. **Fix:** Specify the commit trigger for non-atomic writes, or document that they are silently dropped (and explain why that is safe for the cases SQLite uses them).

## Inconsistencies between docs

19. **Walkthrough says preload is 1 RTT; protocol-and-vfs says cold start is 2 RTTs.** `walkthrough.md` Chapter 7 says "one KV round trip in the common case" for cold start. `protocol-and-vfs.md` Section 3.2 says "Total cost of cold start: 2 round trips (takeover + preload)." These are the same operation; the difference is that the walkthrough predates the takeover-as-a-separate-op design. `key-decisions.md` Section "Preload" says "cold start is 2 RTTs total" (consistent with protocol-and-vfs.md). **Winner:** `protocol-and-vfs.md` and `key-decisions.md` (2 RTTs).
    - walkthrough.md Chapter 7 line 344
    - walkthrough.md Chapter 12 line 497: says "one round trip, ~50 ms" for preload + "Another ~5 ms" for recovery, inconsistent with the 2-RTT number

20. **Walkthrough recovery order is inverted.** `walkthrough.md` Chapter 8 says: (1) preload, (2) takeover. `protocol-and-vfs.md` Section 3.2 says: (1) takeover, (2) preload. Takeover must come first (it bumps the generation and fences out old actors). Preloading before takeover risks reading stale data from a concurrent actor. **Winner:** `protocol-and-vfs.md`.
    - walkthrough.md Chapter 8 line 392: preload first, then takeover
    - walkthrough.md Chapter 12 line 496: preload first, then recovery

21. **Walkthrough describes a 4-layer read path; protocol-and-vfs describes 3 layers.** `walkthrough.md` Chapter 6 has: (1) page cache, (2) write buffer, (3) unmaterialized log (`dirty_pgnos_in_log`), (4) materialized PAGE/. `protocol-and-vfs.md` Section 3.3 has: (1) write buffer, (2) page cache, (3) engine fetch. The order of write buffer vs page cache is also swapped. **Winner:** `protocol-and-vfs.md` (the `dirty_pgnos_in_log` layer is gone because compaction is engine-side).
    - walkthrough.md Chapter 6 lines 296-329
    - protocol-and-vfs.md Section 3.3 lines 438-543

22. **Workload analyses use 2.9 ms RTT, not the C6 20 ms RTT.** All three workload docs (`workload-large-reads.md`, `workload-aggregations.md`, `workload-point-ops.md`) compute speedup ratios at 2.9 ms RTT. `constraints.md` locked C6 at 20 ms. `protocol-and-vfs.md` Section 5 acknowledges "the current workload-*.md files were computed at 2.9 ms RTT and need a recompute pass." The speedup ratios are valid (RTT cancels in ratio), but the absolute latency numbers are ~7x too optimistic for production. This is already tracked as a separate task but should be flagged prominently.
    - workload-large-reads.md line 9: "2.9 ms per engine KV round trip"
    - workload-aggregations.md line 5: "~2.9 ms"
    - workload-point-ops.md line 5: "~2.5 ms per round trip"

23. **`PREFETCH_DEPTH` is 8 in key-decisions.md but 16 in workload-aggregations.md.** `key-decisions.md` Section "Preload" and `protocol-and-vfs.md` Section 3.6 say default prefetch depth is 16. `workload-large-reads.md` uses `PREFETCH_DEPTH = 8`. `workload-aggregations.md` uses `PREFETCH_DEPTH = 16`. **Fix:** Harmonize. The workload analyses should use the same default as the VFS config (16 per protocol-and-vfs.md Section 3.6).
    - key-decisions.md line 69: "~9 MiB envelope" (implies larger depth)
    - protocol-and-vfs.md Section 3.6 line 718: `prefetch_depth: usize, // default 16`
    - workload-large-reads.md line 16: "8 predicted pages per read"

24. **Cache size default: 5,000 vs 50,000.** `protocol-and-vfs.md` Section 3.1 line 335 and Section 3.6 line 717 say `cache_capacity_pages: usize, // default 50_000 (200 MiB)`. `design-decisions.md` Section 5 line 245 says "mvSQLite uses 5,000 pages." `workload-large-reads.md` recommends 10,000. `workload-aggregations.md` recommends 50,000 for analytical actors. The protocol-and-vfs.md default of 50,000 (200 MiB per actor) is aggressive for actor density. **Fix:** Decide on the actual shipping default and make it consistent. 5,000 (20 MiB) as default with configurable up to 50,000 seems like the consensus.
    - protocol-and-vfs.md Section 3.1 line 335: "50k pages = 200 MiB"
    - protocol-and-vfs.md Section 3.6 line 717: "default 50_000"
    - design-decisions.md Section 5 line 245: "5,000 pages"
    - workload-large-reads.md Recommendations: "10,000 pages (40 MiB)"

## Math to verify

25. **"~1000x per-key overhead reduction" from sharding.** `constraints.md` and `key-decisions.md` claim this. The math: v1 has 1 KV key per page. v2 has 1 KV key per 64 pages. That is a 64x reduction in key count, not 1000x. The "1000x" claim likely factors in the per-key overhead (metadata row, tuple encoding, chunking at 10 KB per chunk per `mod.rs:26`). For a 4 KiB page, v1 stores it as 1 chunk (4 KiB < 10 KB). For a 256 KiB shard, v2 stores it as ~26 chunks. So the actual key-level overhead ratio is 64 pages / 1 shard key, but the per-key metadata overhead is amortized over 64 pages vs 1. The "1000x" number needs a walk-through of the actual metadata cost per key to validate. It is plausible if UDB's per-key overhead is ~16x the page size, but that needs to be stated explicitly.
    - constraints.md line 96: "Roughly a 1000x reduction"
    - key-decisions.md line 16

26. **"~9 MiB envelope" byte budget.** The protocol says the envelope is ~9 MiB. With 4096-byte pages and 2x LZ4 compression, that is ~4500 compressed pages or ~9000 raw pages. `protocol-and-vfs.md` Section 2.2 Op 3 says "roughly 4,500-5,000 raw pages" which is correct for the compressed case. But the framing overhead (BARE serialization of `SqliteDirtyPage` list, per-page `pgno` field) is not zero. Each `SqliteDirtyPage` has a 4-byte pgno + ~4096 bytes raw. At 5000 pages that is 5000 * 4100 = ~20 MiB uncompressed, which does not fit in 9 MiB. The spec says pages are "uncompressed when sent over the wire," so the 9 MiB envelope must hold raw pages. 9 MiB / 4100 bytes per page = ~2300 pages, not 4500. **Fix:** Clarify whether the 9 MiB envelope carries compressed or raw pages. If raw, the fast-path threshold is ~2300 pages. If compressed, the actor must compress before sending (but the spec says the engine compresses).
    - protocol-and-vfs.md Section 2.1 line 74: "uncompressed when sent over the wire"
    - protocol-and-vfs.md Section 2.2 Op 3 line 154: "~9 MiB compressed LTX after framing"
    - These contradict: either the wire carries uncompressed pages (fitting ~2300 in 9 MiB) or compressed LTX (fitting ~4500).

27. **Compaction pass cost "~5 ms, ~700 us CPU, ~22 hot actors per core."** The math: 1000 commits/sec * 10 dirty pages = 10,000 dirty pages/sec. At 64 pages per shard, that is ~156 shards dirtied per second. Compaction fires every 64 commits (every 64 ms). Per trigger: identify which shards have unfolded deltas. The doc says "~4 shards affected per trigger." At 10 dirty pages per commit and 64 commits, that is 640 dirty pages across ~640/64 = 10 shards (not 4). The "~200 distinct after hot-page overlap" assumption implies 68% overlap, which is workload-dependent. The 4-shard number is plausible for a hot-row workload but not for a uniform distribution. **Verdict:** The numbers are internally consistent for the assumed workload but the assumption should be stated more clearly.
    - compaction-design.md Section 7 lines 369-375

28. **Page index memory cost "~10 KiB per actor."** `compaction-design.md` Section 3.2: "640 pages per actor x 16 bytes = ~10 KiB." But `scc::HashMap` overhead is ~48 bytes per entry per Section 8.1. At 640 entries * 48 bytes = 30 KiB, not 10 KiB. The 10 KiB figure uses a 16-byte per-entry estimate (pgno + txid) which is the payload, not the total including `scc` overhead. **Fix:** Use the 48 bytes/entry figure for the memory budget calculation. At 640 entries * 48 bytes = ~30 KiB per actor, 10,000 actors = ~300 MiB (which Section 8.1 actually acknowledges, contradicting Section 3.2).
    - compaction-design.md Section 3.2 line 97: "16 bytes (pgno + txid + scc overhead) = ~10 KiB"
    - compaction-design.md Section 8.1 line 393: "~48 bytes/entry overhead... ~300 MiB"

## Unstated dependencies and changes to existing code

29. **Runner-protocol v8.bare must be created, not just v7.bare referenced.** `protocol-and-vfs.md` Section 2 says "proposed: v8." The existing code has `PROTOCOL_MK2_VERSION: u16 = 7` at `engine/packages/runner-protocol/src/lib.rs:12`. Per CLAUDE.md, both `PROTOCOL_MK2_VERSION` in Rust and `PROTOCOL_VERSION` in TypeScript must be bumped together. The doc does not call out the TypeScript-side bump at `rivetkit-typescript/packages/engine-runner/src/mod.ts`. **Fix:** Add to the implementation plan: create `v8.bare`, bump both version constants, update `versioned.rs` to handle v7-to-v8 bridging.

30. **Engine WebSocket handler needs new dispatch arms.** `engine/packages/pegboard-runner/src/ws_to_tunnel_task.rs:230` dispatches KV ops via `req.data` match. The new `sqlite_*` ops need new match arms here (or in a parallel dispatch path). The docs mention this implicitly in "new engine-side subsystem" but never identify the specific file or the dispatch code that needs to change.

31. **`EnvoyHandle` napi bindings need new methods.** The `EnvoyV2` impl in `protocol-and-vfs.md` Section 3.1 line 366 "delegates to napi methods on `EnvoyHandle`." The existing napi surface at `rivetkit-typescript/packages/rivetkit-native/src/database.rs` exposes `EnvoyKv` methods for `batch_get/put/delete`. New methods for the 6 `sqlite_*` ops must be added. This is acknowledged in `design-decisions.md` Section 3 action item "Wire napi bindings" but not in `protocol-and-vfs.md`.

32. **Actor-side runtime initialization needs a v1/v2 branch.** The actor startup code that registers the VFS and opens the SQLite connection needs to choose between `vfs.rs` (v1) and `vfs_v2.rs` (v2). This dispatch logic is not specified anywhere. Where does it live? In the TypeScript runner? In the Rust native module? The walkthrough says "by reading the schema-version byte" but protocol-and-vfs.md says the engine schema-version flag. Neither identifies the actual code location that makes the decision.

## Cross-referenced open questions

33. **`MutationType::Add` re-read semantics.** Flagged as open in `compaction-design.md` Section 8.1 and Section 4.4. Verified: the UDB `tx_ops.rs` implementation at lines 102-158 does apply pending `atomic_op` operations when a subsequent `get` on the same key runs within the same transaction. The read-after-atomic-op pattern works correctly. **This open question can be closed.**

34. **Shard size = 64.** Open in `constraints.md`, `compaction-design.md` Section 10. Still open, needs measurement.

35. **Compaction trigger thresholds.** Open in `compaction-design.md` Section 10. Still open, needs measurement.

36. **`litetx` crate API audit.** Open in `compaction-design.md` Section 8.3 and Section 10. As noted in Critical item 3, the crate may not exist. This should be elevated from "open question" to "blocking dependency."

37. **Default page cache size.** Open in `constraints.md`, `design-decisions.md` Section 5, and `workload-large-reads.md` Recommendations. Conflicting recommendations across docs (see Inconsistency 24). Needs a single decision.

38. **Hard back-pressure interaction with actor SQLite layer.** Open in `compaction-design.md` Section 10. Not covered anywhere else. When the engine returns `KvSqliteCompactionBackpressure`, what does the VFS do? `protocol-and-vfs.md` Section 2.2 does not include this error variant in the `SqliteCommitResponse` union. **Fix:** Either add it as a response variant or document that back-pressure is handled by the engine refusing new commits with a retryable error.

## Things the docs got right

- **SQLite in the actor process (C1 satisfaction)**: Well-argued with concrete alternatives ruled out. The three-model comparison table is clear.
- **Sharded storage + delta log (Option D)**: The constraints-to-architecture derivation in `constraints.md` is rigorous and honest about where D barely wins vs B/C.
- **Generation-token fencing (C5)**: The adversarial review findings in `design-decisions.md` Section 1.4 correctly identify the runner-id gap at `ws_to_tunnel_task.rs:205-220` and the solution is sound.
- **Compaction in the engine**: The 8x RTT savings argument is well-reasoned and the per-shard pass design correctly bounds transaction size under the 5s UDB timeout.
- **Crash safety analysis**: The compaction pass idempotency argument in `compaction-design.md` Section 4.5 is correct -- UDB transaction semantics guarantee all-or-nothing.
- **Dropping the LTX rolling checksum**: Well-justified in `design-decisions.md` Section 1.2. UDB + SQLite already provide integrity.
- **No v1-to-v2 migration**: Clean separation. C7+C8 combined make this the right call.
- **Atomic-write ROLLBACK is local-only**: `protocol-and-vfs.md` Section 3.4 correctly identifies that nothing needs to go to the engine on rollback, eliminating a class of race conditions.
- **UDB `atomic_op` + same-tx re-read**: Verified in code. The implementation at `engine/packages/universaldb/src/tx_ops.rs` correctly applies pending atomic ops to subsequent reads within the same transaction.
- **Five-second UDB timeout correctly identified**: `transaction.rs:18` confirms `TXN_TIMEOUT = Duration::from_secs(5)` and the design correctly uses this as the binding constraint.
- **Workload analyses are honest about where v2 doesn't win**: The `workload-point-ops.md` "honest bottom line" and `workload-aggregations.md` Scenario 2 (1.2x) are refreshingly candid.
