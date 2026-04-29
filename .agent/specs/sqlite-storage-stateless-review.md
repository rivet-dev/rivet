# Adversarial Review — Stateless SQLite Storage Spec

Review findings for `sqlite-storage-stateless.md`. Four hostile reviews ran in parallel: correctness, operations, performance, design coherence. Findings synthesized below in priority order.

## Status: spec needs revision before implementation

The spec is bundled into one design but contains two independent changes (stateless protocol + out-of-process janitor). Three concrete design problems are unresolved. Several existing CLAUDE.md invariants are contradicted without justification.

## Critical blockers

### B1. Fence rule change does not actually decouple commit from compaction on FDB

**Status: resolved in revised spec.**

The original framing was "fence on field X vs Y." That framing dissolved when both fences (generation + head_txid) were dropped from the release hot path entirely (pegboard exclusivity is the contract; debug-mode sentinels only). What remains is a pure key-locality problem: commit and compaction both write the same META *key*, regardless of which fields they touch.

The revised spec resolves this by **splitting META into multiple keys**:

```
META             — head_txid, db_size_pages, static fields (commit-owned)
META/compact     — materialized_txid (compaction-owned)
QUOTA            — sqlite_storage_used (FDB atomic counter)
```

- Commits write `META` + `atomic_add(QUOTA, +bytes)` + PIDX + DELTA.
- Compactions write `META/compact` + `atomic_add(QUOTA, -bytes)` + PIDX deletes + DELTA deletes + SHARD writes. Reads `META.head_txid` as snapshot read (no conflict range).
- FDB atomic adds compose without conflict ranges.

Net: commit and compaction never conflict on META or QUOTA. The remaining contention surface is PIDX deletes when compaction folds a pgno that a recent commit also wrote — bounded and small. The revised spec uses a conflict-range read on PIDX during compaction (option 1 in its concurrency section), with per-pgno CAS as a fallback if it ever starves.

Breaking changes are acceptable per the revised spec, so the META split lands cleanly without a migration path.

### B2. Single-shot commit removes the only path for large blobs

**The claim**: "Drop multi-chunk staging; UDB chunks values internally."

**Why it's wrong**: Today's slow path exists specifically because there's an upper bound the fast path can't handle (`SQLITE_MAX_DELTA_BYTES` cutoff plus FDB ~10MB tx-size limit). Engine CLAUDE.md explicitly says "slow-path finalize must accept larger encoded DELTA blobs because UniversalDB chunks logical values internally" — meaning today's slow path is the safety valve. Removing it makes any commit larger than the fast-path cap return `SqliteCommitTooLarge` with no fallback.

**Resolution options**:
- Option (a) — keep multi-chunk staging as a fallback. `pending_stages` stays. Wire protocol unchanged for the slow path. Recommended for v1.
- Option (b) — client-side commit splitting. Client breaks a too-large logical commit into multiple sequential server-side commits. Requires VFS-side coordination so the user-code-level transaction stays atomic. Bigger redesign.

**Severity**: blocker. Functional regression for any actor with non-trivial commits.

### B3. Recovery folded into single takeover tx may exceed FDB's 5s tx limit

**The claim**: "Fold `build_recovery_plan` into the takeover write tx."

**Why it's wrong**: `build_recovery_plan` does three full prefix scans (DELTA, PIDX, SHARD). For actors with millions of accumulated keys after long uptime, scan + deletes + META rewrite blows FDB's tx-age budget. Today's `open()` does the scans *outside* the small atomic write, so it doesn't hit this. The spec hand-waves "same cost, different home" — not actually the same.

**Resolution**: chunked-recovery mode with a "recovery in progress" state in META. Takeover bumps generation atomically and starts orphan cleanup; new envoy serves traffic while a dedicated recovery task (or the janitor) finishes incrementally. The orphan set is defined by `head_txid` and `db_size_pages` frozen at the takeover moment, so correctness holds across multiple txs.

**Severity**: blocker for actors with large key counts.

### B4. Existing CLAUDE.md fence invariant directly contradicted

**Status: resolved in revised spec.**

The CLAUDE.md invariant said: "sqlite-storage compaction must re-read META inside its write transaction and fence on `generation` plus `head_txid`." This existed because compaction and commit both wrote the same META key, and without the fence, compaction could rewind the head.

In the revised spec, compaction does not write `head_txid` at all — that field lives in the `META` key, which is commit-owned. Compaction writes `META/compact` (its own key). The head-rewind risk vanishes because there is no shared write target. The fence is therefore unnecessary in release.

The CLAUDE.md note will need updating once the revised spec lands. Action item: when the META split is implemented, update the relevant `engine/CLAUDE.md` `## SQLite storage tests` bullet to describe the new key layout instead of the fence rule.

### B5. v1→v2 migration path is broken

`engine/packages/pegboard/src/actor_sqlite.rs:131-138` calls `prepare_v1_migration` → `open` → `commit_stage_begin/stage/finalize`. The spec deletes all four downstream methods. Migration path silently breaks. Spec doesn't mention v1 migration at all.

**Resolution**: either preserve the migration code path with the multi-chunk methods, or design a single-shot migration commit.

**Severity**: blocker.

## Major issues — fix in spec

### M1. The "v3.bare" deliverable is mislabeled

There is no separately-versioned `ToRivetSqlite` schema crate. The relevant ops live in `engine/sdks/schemas/envoy-protocol/v2.bare`. The actual change is a VBARE bump on envoy-protocol. Runner-protocol does NOT change. Spec mislabels deliverable and confuses readers about which protocol crate is affected.

### M2. VBARE migration story missing entirely

Per `engine/CLAUDE.md` "VBARE migrations": every variant must be reconstructed field-by-field, never byte-passthrough. Spec doesn't specify v2→v3 converter for `ToRivetSqliteRequestData` / `ToEnvoySqliteResponseData`, doesn't mention `versioned.rs`. Need explicit converter implementation guidance.

### M3. Trust-boundary regression: client claims actor_id in every request

Today, `open()` ties `(actor_id, generation)` to a specific WS connection at start. After: any envoy could claim any `actor_id`. The actual binding probably comes from the existing actor-start handshake (the WS is already pinned to one actor), but the spec doesn't say so. Easy fix: state the invariant explicitly. Verify it's actually enforced in pegboard-envoy WS setup.

### M4. NATS publish is not free

`async_nats::Client::publish()` is `async fn` that awaits a bounded mpsc and allocates. Spec says "fire-and-forget after WS response" but doesn't specify mechanism. Without `tokio::spawn` or `try_send`, publish runs inline on the commit response path and can backpressure under NATS slowness. Need to specify: `tokio::spawn(nats.publish(...))` so it never blocks the commit response.

### M5. PIDX cache cold-start does duplicated scan after takeover

Recovery's `live_pidx` BTreeMap (built during takeover-tx) is thrown away after commit; the next `get_pages` then does its own PIDX prefix scan. Two scans where today there's one (today's `open()` preloads the cache from recovery's scan). Resolution: takeover should ship the recovery's PIDX result to the new envoy via some path (e.g., a pre-warmed META hint, or the next `get_pages` accepts a "skip cache load" flag). Or accept the cost — it's one extra prefix scan per takeover, not per request.

### M6. Lifecycle wiring is unaddressed

`open()`/`close()`/`force_close()` are called from `actor_lifecycle.rs:189-201,237-250` on actor lifecycle events, not WS frames. Spec says "ws_to_tunnel_task" without specifying:
- Which lifecycle call sites are deleted vs retained
- How `page_indices` is evicted on actor stop
- How dev-mode (single-process) handles the janitor (in-process tokio task variant?)

### M7. Janitor pod death loses the trigger forever

No durability, no acks. "Next commit republishes" is only true for actively-writing actors. An actor that hits threshold and goes idle (e.g., scheduled batch job) has no recovery path until next write. Resolution: either accept this risk (with monitoring on `head_txid - materialized_txid` lag), or add a periodic safety sweep.

### M8. NATS partition / outage scenarios

Spec claims "next commit republishes" makes trigger loss tolerable. But:
- Bursty actors that go idle right after threshold don't retrigger
- Cross-tenant blast radius (one noisy tenant floods queue, blocks others)
- Subject namespace `sqlite.compact` is global — should be per-tenant or have priority

Plus: no monitoring metric exists today for "how many triggers were dropped." Need a Prometheus counter on publish failure.

### M9. Cross-process compaction trigger latency increase

mpsc::send: sub-µs. NATS publish + queue routing + janitor pod scheduling + janitor's META read tx: 10–50ms. For sustained writes, in-flight backlog grows. Document the SLO target.

### M10. Monitoring gaps

Spec requires zero metrics. Operability needs:
- Per-actor `head_txid - materialized_txid` lag histogram
- NATS publish success/drop counter
- Compaction conflict-abort rate
- Takeover recovery duration histogram
- Janitor pod queue-group consumer count
- Compaction worker concurrency

None are in scope. Required for production rollout.

### M11. Open questions that aren't actually open

- "PIDX-key-count optimization": deferred work, not open. Move to non-goals.
- "NATS partition handling acceptable?": already decided "yes". Not open.
- "Periodic safety sweep": decided "not in v1". Move to non-goals.
- "Single-shot commit size limit": this is genuinely B2. Promote to blocker.
- "Janitor lag SLO": genuinely open, must answer before shipping.

## Minor issues — revision suggestions

- **Cross-pod duplicate compaction quota math**: the math (`existing_shard_size`, `compacted_pidx_size`, `deleted_delta_size`) is computed against stale snapshots. Pod B can double-decrement bytes Pod A already decremented. Self-healing eventually but fragile. Worth a closer read of `compaction/shard.rs:288-294`.
- **Fixed-point quota recompute** required on takeover recovery (per CLAUDE.md "META writes need fixed-point sqlite_storage_used recomputation"). Spec says "recompute" without naming the fixed-point requirement.
- **PIDX cache memory bound** unspecified. With ~10K concurrent actors, K=1000 LRU entries × ~16MB each = GB-scale RAM at adversarial actor counts. Need a concrete bound.
- **Memory pressure** on single-shot commit: 8MB blob lives in three live copies (WS frame, dirty_pages Vec, LTX encoder buffer). Acceptable but worth measuring.
- **Process-wide `OnceCell` content** after the change isn't enumerated. SqliteEngine retains `page_indices`, but NATS connection, metrics handles, op_counter all need explicit homes.
- **Inspector protocol interaction** unspecified. Inspector reads SQLite META; with no `open()`, how does inspector access work?
- **Pegboard actor exclusivity** invariant (repo CLAUDE.md) should be explicitly cited as the basis for removing in-process exclusivity check.

## Open design problems (require real design work, not just spec edits)

In order of risk:

1. **Compaction fence decoupling on FDB** (B1). **Resolved.** Revised spec splits META into commit-owned `META`, compaction-owned `META/compact`, and an FDB atomic-counter `QUOTA`. Plus snapshot reads on the compaction side and conflict-range PIDX reads for fold-vs-commit pgno overlap.
2. **Large-commit story** (B2). Option (a) keep slow path as fallback (lower risk, recommended for v1). Option (b) client-side splitting (bigger redesign). Pick one.
3. **Takeover tx size for big-orphan actors** (B3). Need chunked-recovery mode with "recovery in progress" META state.

## Recommended next steps

1. **Split spec into two**:
   - **Spec A — Stateless wire protocol**: Remove `open`/`close`, drop multi-chunk staging only after option (a) is decided, fold recovery into takeover-tx with chunked-recovery mode for large actors. No janitor split. Address B2, B3, B5, M1, M2, M3, M5, M6.
   - **Spec B — Out-of-process janitor**: Move compaction to separate process via NATS. Keep wire protocol unchanged. Address B1, B4, M4, M7, M8, M9, M10.

   These changes are independent and can ship separately. Bundling forces wider blast radius on rollout.

2. **Resolve the three open design problems** (B1, B2, B3) with concrete sketches before either spec is final.

3. **Address contradicted CLAUDE.md invariants** explicitly — either justify the change with a written rationale, or update the invariant.

## Issues considered noise / overblown

- Some compaction-vs-commit data-loss scenarios in correctness review depend on edge cases the existing "compare against all global PIDX refs" rule already protects against.
- Dev-mode in-process variant: easy to handle as a tokio task variant of janitor binary, not a real blocker.
- Three-copy memory pressure on 8MB single-shot commits: fine in practice.
- Cold-start race for new janitor pods joining NATS queue group: real but standard NATS behavior, not specific to this design.

## Files referenced during review

- `/home/nathan/r2/engine/packages/sqlite-storage/src/{engine,open,commit,read,page_index,udb}.rs`
- `/home/nathan/r2/engine/packages/sqlite-storage/src/compaction/{mod,shard,worker}.rs`
- `/home/nathan/r2/engine/packages/pegboard-envoy/src/{actor_lifecycle,sqlite_runtime,ws_to_tunnel_task}.rs`
- `/home/nathan/r2/engine/packages/pegboard/src/actor_sqlite.rs`
- `/home/nathan/r2/engine/sdks/schemas/envoy-protocol/v2.bare`
- `/home/nathan/r2/engine/packages/universaldb/src/tx_ops.rs`
- `/home/nathan/r2/CLAUDE.md`, `/home/nathan/r2/engine/CLAUDE.md`
