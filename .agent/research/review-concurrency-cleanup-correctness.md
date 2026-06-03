# Adversarial Review: SQLite Storage Concurrency Cleanup

Target: `.agent/specs/sqlite-concurrency-cleanup.md` (lease/OCC removal proposal).

## Critical issues (data corruption / silent wrong reads)

### C1. Hot fold deletes a DELTA still required by a fork descendant after eviction

The proposal cites "PIDX clears use COMPARE_AND_CLEAR" and "DELTA clear_range is idempotent." But fork-vs-hot-fold is *not* protected by either.

Trace:
1. Hot pass on database `D` reads `head_txid = 100`, plans to fold `DELTA[1..100]` into `SHARD/{s}/100`.
2. Pod B's `fork_database(D, V_T=50)` commits between hot pass tx-A (snapshot) and tx-B (write). Fork advances `desc_pin` on `D` to versionstamp `V_50`.
3. Hot pass tx-B writes `SHARD/{s}/100` and `clear_range(DELTA[1..100])`. Hot pass does *not* read `desc_pin`, so FDB does not detect a conflict (no read-write conflict on `desc_pin`).
4. Eviction pass sees `desc_pin = V_50`, refuses to evict `SHARD/{s}/100`. Good.
5. Fork descendant reads pgno P at versionstamp V_50. PIDX miss, falls through to `SHARD`. Reverse-range scan picks largest `as_of_txid <= 50`. If no `SHARD/{s}/<=50` exists (the only fold so far is `SHARD/{s}/100`), it falls through to DELTA. But `DELTA[1..100]` was just cleared. PIDX has no owner, so falls through to SHARD. SHARD `as_of=100 > 50`, rejected by cap. **Page returns as zero (unallocated) but should be the page written at txid 30.**

Proposal's claim: "deterministic SHARD content + idempotent clear_range". But the bug is not non-determinism - it is that the hot pass *deletes data the fork still depends on*, because hot pass takes no read-conflict on `desc_pin`. The pre-existing `MAX_SHARD_VERSIONS_PER_SHARD` "if every version is pinned, return ShardVersionCapExhausted" partially addresses this, but only when the pin already exists at hot-pass start. A pin appearing concurrently is invisible.

The lease did not directly address this either, but the leased model serializes a hot pass with the eviction logic that re-checks pins; the proposal's "single-tx eviction" preserves the eviction read-write conflict but introduces no new fence on hot-pass DELTA deletion. The CLAUDE.md invariant `Hot compactor enforces MAX_SHARD_VERSIONS_PER_SHARD` checks pins at write time, but it does not check that `desc_pin` ≤ the lower bound of the DELTA range being cleared.

### C2. Cold pass observability gap: SHARD evicted before cold uploads it

Trace:
1. Cold pass A reads `cold_drained_txid = 80`, snapshots SHARD versions including `SHARD/{s}/100`.
2. Phase B starts uploading `SHARD/{s}/100` to S3 (slow).
3. Concurrent cold pass B reads `cold_drained_txid = 80`, snapshots same SHARD versions.
4. B finishes Phase B faster, runs Phase C: writes `cold_drained_txid = 100`.
5. Eviction pass observes `cold_drained_txid >= 100` and the predicate clears `SHARD/{s}/100` from FDB.
6. A's Phase C runs: reads `cold_drained_txid = 100`, the proposal's "monotonic guard" makes A skip writing 100 (Y == X', no-op). But A's `pending/{uuidA}.marker` is still in S3, and any pin-layer S3 PUTs A made are still in S3.

Sub-bug: A's Phase C *does not flip BOOKMARK pinned status* for the bookmarks A had committed to upload, because A skipped on the monotonic guard. The proposal lists `BOOKMARK/.../pinned.status: Pending → Ready` as monotonic, but A's Phase C never reaches that step. If B's plan included those pins, B did them; if not, the bookmark stays `Pending` forever. The proposal does not specify what happens to the loser cold pass's pin transitions.

### C3. Versioned SHARD reads can race with Phase C transition flip

Cold pass writes `cold_drained_txid` and flips `BOOKMARK pinned.status: Pending → Ready` in the same tx (Phase C). But the eviction predicate keys on `cold_drained_txid >= max_folded_txid`. If a hot fold lands between cold's snapshot and cold's commit and produces `SHARD/{s}/120` while cold thought it was uploading shard at `as_of=100`, two parallel cold passes produce `cold_drained_txid = 100` and `cold_drained_txid = 120`. The 120 wins via monotonic. But the 100-pass's `pending/{uuid}.marker` listed only the layers up through 100; the 120-pass's marker listed layers through 120. Eviction now sees `cold_drained = 120`, evicts `SHARD/{s}/100` *and* `SHARD/{s}/120` from FDB. If only the 100-marker survived stale-marker sweep timing and the 120-pass's chunk index never rewrote `cold_manifest/index.bare` cleanly (S3 race documented in the proposal), the catalog snapshot for the 120-pass may not reference the 120 layer. Reader path: list `chunks/`, finds 100-chunk, no 120-chunk listed, picks `as_of=100` cap on a request at cap=120. **Wrong page values from layer-set inconsistency.**

The proposal accepts "readers must list `chunks/` to recover" as the mitigation, but listing `chunks/` *only* tells you what files exist - it does not tell you what files were planned. If a chunk file was uploaded but the index file was clobbered by an older pass's write, listing `chunks/` recovers the file. But if a chunk upload partially completed (PUT failed mid-S3-multipart, no error surfaced before the index was updated), the file is missing entirely and no future pass touches that prefix until a hot fold makes that range relevant again.

## Major issues (livelock / unbounded resource growth)

### M1. Concurrent forks of same source: pin_count not enforced

The `MAX_PINS_PER_NAMESPACE = 1024` cap (§9, fix #19) is read at pin creation and incremented atomic-add. Two parallel `create_pinned_bookmark` calls near the cap each read `pin_count = 1023`, both decide "ok, room", both atomic-add(+1), end up at `pin_count = 1025`. Atomic-add is commutative but not conditional. This is a real cap violation.

The proposal's idempotency table lists `refcount atomic-add: commutative by FDB op` - true but not the issue. The cap check needs read-write conflict on `pin_count`, not commutative add. Without a fence, two parallel pin creations under the cap can exceed it. The proposal acknowledges atomic-add commutativity but doesn't note that *cap enforcement* requires conditional logic that commutativity defeats.

Original lease-free code might use `atomic-add` plus a pre-read on `pin_count`. Both pods read `pin_count = 1023` in parallel txs, both pass the gate, both commit. FDB serializability *does* see read of `pin_count` plus write of `pin_count`, so one pod's tx aborts on read-write conflict only if both txs read the same key in the same conflict range. Atomic-add does not register a read-conflict in FDB by default. So both succeed.

This was true even with the old lease model (the lease was per-database, not per-namespace), but the cleanup proposal calls out atomic-add as "commutative by FDB op" without flagging the cap-enforcement subtlety.

### M2. Cold pass force-publish duplicate storms

Proposal: "Trigger lost (no consumer alive when published) → `cold_max_silence_ms` force-publish from hot pass" plus "Multiple hot pods both force-publish → duplicate triggers → idempotent + monotonic writes tolerate it."

Each duplicate trigger that survives queue-group dedup runs a full Phase A + Phase B. Phase B uploads megabytes of S3 data per pass per database. A queue-group failure mode where 4 hot pods all force-publish at the same wall-clock and all 4 duplicate triggers slip past dedup yields 4× S3 PUT cost. NATS redelivery on top means *up to N redelivers × N pods*. The proposal handwaves "rare duplicate is bounded waste" - but the bound depends on `STALE_MARKER_AGE_MS` plus `cold_max_silence_ms` ratios, not on NATS itself. Worth a numeric bound.

S3 PUT idempotence does not help the cost - same filename, same bytes, but PUT charges still accrue. `cold_pass_duplicate_total` metric is good but doesn't bound the worst case.

### M3. Loser pass's `pending/{uuid}.marker` orphaned indefinitely

`/META/cold_compact.in_flight_uuid` is last-writer-wins. Pass A writes uuid_A, pass B writes uuid_B. The FDB key holds uuid_B; uuid_A is forgotten by FDB.

Now both pods race Phase B. If A finishes Phase B and gets to Phase C first, A's Phase C tries to clear `in_flight_uuid` - but the key holds uuid_B, not uuid_A. The proposal does not specify whether Phase C reads-then-clears or unconditionally clears. If unconditional clear, A clears uuid_B - now B is "in flight" but FDB has no record. If conditional clear (only if equal to A's uuid), A leaves uuid_B in FDB. Either way, A's `pending/{uuid_A}.marker` in S3 is now orphaned: no FDB key references it.

Stale-marker sweep deletes markers older than `STALE_MARKER_AGE_MS`. So `pending/{uuid_A}.marker` plus its associated layer files survive `STALE_MARKER_AGE_MS` worth of S3 storage cost, then get cleaned up. Bounded but real.

The bigger issue: if A's Phase B uploaded a `pin/{vs}.ltx` for a `Pending` bookmark and then A's Phase C never ran (because A was the loser), the bookmark stays `Pending` forever. The pin layer survives in S3 (orphan) until stale-marker sweep deletes it. Reader path on resolve hits a `Pending` bookmark with no pin layer - resolves how? The proposal does not specify whether `Pending` is a resolvable state. If the cold compactor enqueues bookmark transitions only via the "winner" pass (last-writer-wins on `in_flight_uuid`), and the loser had the only pass that uploaded the pin layer, the user's pinned bookmark is *lost* (status `Pending`, no pin layer, eventually deleted by stale-marker sweep without the bookmark being marked `Failed`).

### M4. Stale-marker sweep delete-vs-active-pass race

Stale-marker sweep deletes markers older than `STALE_MARKER_AGE_MS`. If a Phase B is genuinely slow (large database, slow S3) and exceeds `STALE_MARKER_AGE_MS` *while still running*, a parallel sweep deletes the still-in-flight pass's marker plus its uploaded layers. Active pass continues, finishes Phase B, runs Phase C, but the layers it uploaded earlier are gone. Future readers cannot find those layers; the cold-manifest chunk file may reference deleted layer keys.

The lease-based model bounded a pass's wall-clock by lease-renewal lifetime (30s TTL, 10s renew). Without a lease, there is no upper bound on a pass's duration vs. `STALE_MARKER_AGE_MS`. Unless `STALE_MARKER_AGE_MS` is much greater than max-pass-duration (worst case: cold-stuck S3 region in a multi-GB database), stale-marker sweep can delete in-flight pass output. The proposal does not specify the relationship.

## Minor issues

### m1. `desc_pin` atomic-min: not idempotent across multi-versionstamp forks

Atomic-min IS commutative across same-key writes. But `derive_branch_at` writes the new branch's metadata + atomic-min on parent's `desc_pin`. If two parallel forks at different versionstamps `V_A < V_B` both target the same source, both atomic-min the source's `desc_pin`. End state is `min(V_A, V_B) = V_A`. Both fork records exist with their own `parent_versionstamp`. Reads on each fork resolve with their own cap. Looks fine.

But the GC pin computed for the source includes `desc_pin = V_A`. GC does not delete below V_A. Good. So this is benign - flagging only because the proposal lists `desc_pin atomic-min commutative` without discussing the multi-fork case.

### m2. Eviction-vs-hot abort storm

Proposal §11 acknowledges this as a risk. On a busy database with frequent commits, every hot pass updates `last_hot_pass_txid`, every eviction tx reads it. Eviction will abort frequently. The proposal suggests `eviction_tx_aborts_total` metric and back-off behavior - good - but no concrete back-off algorithm is specified. Without exponential back-off, a hot pod committing every 100ms permanently starves eviction.

### m3. `head_at_fork` cleared on first commit: race under concurrent fork

`head_at_fork` is "cleared on first commit". If a fork is created and a `get_pages` reads `head_at_fork` *while* the first commit is in flight, what does it see? Single-writer-per-database via pegboard means commit and read coexist on the same conn, so the conn-local cache controls. If pegboard exclusivity gaps (M5 below), this race is observable.

## Concerns the proposal addresses correctly

- **SHARD content determinism.** With deterministic shard layout and deterministic delta inputs, two parallel folds on the same `(shard_id, as_of_txid)` produce identical bytes. FDB serializability aborts on the second tx's write to the same key. Sound.
- **PIDX COMPARE_AND_CLEAR.** Stale PIDX clears that race a fresh commit no-op. Sound and well-established.
- **Refcount atomic-add commutativity for fork+delete.** Fork and delete on the same source can commute; final refcount is correct. Sound.
- **DELTA append-only at unique txids.** Different txid → different key. Cannot collide. Sound.

## Open questions the proposal flags but does not resolve

1. **Hot compactor idle wakeup.** Without leases, what tells one pod to do work? Multiple pods receiving the same UPS publication via queue group is OK *for correctness*, but the proposal punts on "is this OK or do we need explicit coordination?" Until a queue-group implementation gives at-most-one delivery (which NATS does best-effort), N pods will race on every trigger.
2. **Cold compactor scheduling fairness.** Punted as "future tuning".
3. **Eviction global sweep coordination.** Multiple pods scanning the same `eviction_index` in parallel: each picks the same DBs. Each runs the predicate. Each tries to clear. FDB serializability aborts all but one - but each pod paid the cost of plan, and the abort storm under M2 is amplified.

## M5. Pegboard exclusivity: load-bearing under-specified

Proposal: "Pegboard exclusivity is the only mechanism guaranteeing single writer per database." But the existing CLAUDE.md says "Pegboard exclusivity (lost-timeout + ping protocol) holds." Lost-timeout is wall-clock. Network partition during host failover can produce two hosts that both think they own an actor. The proposal's debug fence (`#[cfg(debug_assertions)]` only) does not catch this in release.

If pegboard exclusivity has a gap and two writers hit `D`, both increment `head_txid` via `/META/head` read+write. FDB serializability on `/META/head` aborts one. This is the safety net. But neither the lease model nor the proposed lease-free model defends against pegboard exclusivity gap - the existing system always relied on FDB read-write conflict on `/META/head` here. The proposal doesn't make this worse, but its claim "this is what guarantees single-writer" overstates pegboard's role.

## Bottom line

The proposal is mostly sound for the *intended* concurrency surface (parallel cold passes, parallel hot folds on different shards). The critical correctness gaps are:

1. C1: hot fold deletes DELTAs without checking `desc_pin` (predates this cleanup but the cleanup does not fix it).
2. M3: loser pass's pending pin transitions are silently dropped, leaving pinned bookmarks stuck in `Pending` forever.
3. M4: stale-marker sweep can delete an active long-running pass's layers without lease coordination.

Before shipping, the cleanup needs explicit answers to: (a) hot pass DELTA-vs-fork retention (read `desc_pin` in fold tx?), (b) cold-pass loser semantics for pin transitions (Phase C reads in_flight_uuid before flipping, or pin transitions are tracked separately?), (c) `STALE_MARKER_AGE_MS` ≥ max-pass-duration bound, with metric to alarm violations.
