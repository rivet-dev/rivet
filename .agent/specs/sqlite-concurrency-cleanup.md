# SQLite Storage Concurrency Cleanup

A plan to simplify the concurrency model in `engine/packages/sqlite-storage/` by removing leases and bespoke OCC fences, relying on FDB-native serializability + idempotent/monotonic writes + pegboard exclusivity + NATS queue groups.

## Goal

Replace this stack:

- Hot compactor lease (per-database, with TTL/renewal/cancel)
- Cold compactor lease (per-database, same machinery)
- Eviction compactor global lease
- Bespoke OCC fence on `cold_drained_txid` (cold Phase A → Phase C)
- "OCC fence" wording on `last_hot_pass_txid` (eviction-vs-hot)
- "OCC fence" wording on `bk_pin` (fork-vs-GC, bookmark-vs-GC)

with this simpler stack:

- **Pegboard exclusivity** — single writer per database (already the only mechanism guaranteeing this; nothing changes)
- **FDB native serializability** — read-write conflict detection on every tx (already in FDB; we just stop layering bespoke names on top)
- **Idempotent + monotonic writes** — keys designed so that parallel commits converge to the same correct state
- **NATS queue groups** — first-line trigger dedup for compactors
- **Stale-marker sweep** — S3 orphan cleanup for any source of partial writes

## Why this works

The bespoke OCC fences and leases were written assuming we needed explicit fence keys for each cross-tx invariant. Closer analysis shows:

1. Most "OCC fences" in the spec are actually FDB native serializability under a different name (fork's `bk_pin` read, eviction's `last_hot_pass_txid` read — both happen inside one FDB tx).
2. Cold compactor's multi-tx structure (Phase A → Phase B → Phase C across S3 PUTs) is the only place where FDB serializability alone is insufficient — but every Phase C write is monotonic or idempotent, so parallel passes converge correctly.
3. Hot compactor is also multi-tx (one shard per fold tx for multi-shard databases), but every write is idempotent (deterministic SHARD content, COMPARE_AND_CLEAR PIDX, idempotent clear_range on DELTAs).
4. Leases were defending against duplicate work (efficiency), not corruption.

For typical workloads, NATS queue group routing already deduplicates compactor triggers ~99% of the time; the rare duplicate is bounded waste, not corruption.

## Current operations and their conflict landscape

### Commit (single writer per DB via pegboard)

Reads `/META/head`. Writes DELTA, PIDX, `/META/head`, COMMITS, VTX, `/META/quota`.

Conflicts: none expected — pegboard exclusivity prevents two commits on same DB. If pegboard ever leaks (bug), FDB serializability catches it via `/META/head` read-write conflict.

### Hot compactor (multi-tx, one fold per shard)

Reads SHARD versions, DELTA range, `/META/head`, `/META/compact`. Writes new `SHARD/{shard_id}/{as_of_txid}` (chunked across multiple FDB rows), clear DELTAs, COMPARE_AND_CLEAR PIDX, `/META/manifest/last_hot_pass_txid`.

Per-shard fold = one FDB tx. Multi-shard pass = multiple txs in series.

Conflicts: parallel hot folds on the same shard write the same content (deterministic). FDB serializability detects key collisions. Parallel folds on different shards don't conflict.

### Cold compactor (multi-tx Phase A → Phase B → Phase C)

Phase A (FDB tx): snapshot reads, writes `/META/cold_compact.in_flight_uuid`.
Phase B (no FDB): S3 PUTs (image, delta, pin, manifest chunks, catalog snapshot, pending marker).
Phase C (FDB tx): writes `cold_drained_txid`, transitions BOOKMARK pinned status, clears `in_flight_uuid`.

Conflicts: parallel cold passes converge on `cold_drained_txid` via monotonic write (FDB serializability detects); BOOKMARK transitions are idempotent (Pending → Ready); S3 PUTs are idempotent (deterministic filenames).

### Eviction compactor (single-tx per database)

Reads `/META/manifest/*`, BRANCHES counters, SHARD versions. Writes clear of old SHARD versions, clear of evictable DELTAs, removes from eviction_index.

Refactor: plan + commit fit in one FDB tx per database. FDB native serializability catches hot-pass interleaving via read-write conflict on `last_hot_pass_txid` or SHARD keys.

### Fork (single-tx)

Reads source BRANCHES record, source `bk_pin`, VTX, COMMITS. Writes new BRANCHES record, `head_at_fork`, refcount atomics, `desc_pin` atomic-min, NSCAT entry.

Conflicts: GC concurrently advances `bk_pin` → FDB read-write conflict, fork tx aborts and retries.

### Pinned bookmark create (single-tx)

Reads `pin_count`, BRANCHES record. Writes BOOKMARK record, `bk_pin` atomic-min, `pin_count` atomic-add.

Conflicts: same shape as fork.

### GC pass (per-branch batched txs)

Each batch tx: reads pins (refcount, desc_pin, bk_pin, root_versionstamp), computes floor, deletes batch of keys below floor. Across batches, no fence — each tx re-reads pins.

Conflicts: fork or pinned bookmark advances pin between batches → next batch sees new pin and recomputes floor (does less work).

## Idempotency / monotonicity invariants that MUST be preserved

These are the correctness preconditions for the no-lease model. Any future write to one of these keys must preserve the property:

| Key | Required property |
|---|---|
| `SHARD/{shard_id}/{as_of_txid}` content | Deterministic from inputs (parent shard + folded deltas) |
| `cold_drained_txid` | Monotonic — never regresses |
| `BOOKMARK/.../pinned.status` | Pending → Ready (one-way) |
| `desc_pin`, `bk_pin` atomic-min | Commutative by FDB op |
| `refcount` atomic-add | Commutative by FDB op |
| PIDX clears | COMPARE_AND_CLEAR (conditional) |
| DELTA blob writes | Append-only at unique txids |
| S3 layer / manifest / pin / catalog snapshot files | Deterministic filenames, deterministic content |
| `pending/{uuid}.marker` | Self-describing; lists files for orphan cleanup |
| `/META/head_at_fork` | Written once on fork, cleared on first commit |
| `/META/head` | Single writer per DB (pegboard) |

Document each in code comments and `engine/packages/sqlite-storage/CLAUDE.md`.

## Multi-tx operations: recovery without fences

| Op | Steps | If interrupted | Recovery |
|---|---|---|---|
| Hot fold (multi-shard) | One fold tx per shard, in series | Some shards folded, others not | Next hot pass picks up remaining shards |
| Cold pass | Phase A (FDB) → Phase B (S3) → Phase C (FDB) | Phase B partial: marker in S3, some layers uploaded, no FDB commit | Stale-marker sweep on next pass deletes orphan files |
| Cold pass | Phase C interrupted between writes | Partial FDB commit not possible (atomic tx) | Same as Phase B interruption |
| GC per-branch | Batched delete txs | Some keys deleted, others not | Next GC pass continues |

## S3-side races (no S3 transactions)

| Race | Outcome | Mitigation |
|---|---|---|
| Two passes rewrite `cold_manifest/index.bare` | Last-writer-wins; loser may overwrite with older view | Index is a hint, not source of truth — readers can reconstruct by listing `chunks/` |
| Two passes write `pointer_snapshot/{pass_vs}.bare` | Different versionstamps, both kept | DR readers prefer newest |
| Two passes write same image layer | Identical content via deterministic naming | None needed (idempotent overwrite) |
| Stale-marker sweep race | Idempotent deletes | None needed |
| Two passes upload pin layer for same bookmark | Same content (deterministic from versionstamp) | None needed |

The flag: **`cold_manifest/index.bare` is non-transactionally consistent with the chunk files it references**. A chunk can land while the index isn't yet updated. Readers must tolerate this by listing `chunks/` directly when the index is missing or out of sync. Document this contract.

## NATS reliability assumptions

| Concern | Mitigation |
|---|---|
| Queue group redelivery → duplicate cold trigger | Idempotent + monotonic writes tolerate it |
| Trigger lost (no consumer alive when published) | `cold_max_silence_ms` force-publish from hot pass |
| Multiple hot pods both force-publish → duplicate triggers | Same as redelivery |

We do **not** depend on exactly-once NATS delivery. The trigger is a hint; correctness comes from FDB.

## Edge cases worth flagging

1. **Eviction-vs-hot retry rate.** On busy databases, hot pass commits frequently and eviction's per-DB tx may abort and retry many times due to read-write conflict on `last_hot_pass_txid`. Worth metrics: `eviction_tx_aborts_total`. If it goes high, may need to back off eviction during hot bursts.

2. **Concurrent forks of the same source.** Two parallel `fork_database(D, V)` calls are safe via atomic-min/atomic-add commutativity, but they'll write two different new database records. Caller must distinguish via the returned `new_database_id`.

3. **Fork at the GC frontier.** Race: GC has read pin, computed floor, is about to delete; concurrently fork advances `desc_pin` to a value at or above the floor. Without explicit fence, GC's tx aborts on read-write conflict on `desc_pin` and retries. New floor sees fork's pin; deletion adjusts.

4. **Cold pass commits `cold_drained_txid` with stale plan.** If pod A's Phase A reads `cold_drained_txid = X` and pod B commits Phase C advancing it to X' between A's Phase A and A's Phase C, then A's Phase C tries to read `cold_drained_txid` (sees X') and write its own value Y. Three sub-cases:
   - Y > X': monotonic, A wins. But A's plan was based on X; A may upload a slightly stale set of layers — they're idempotent overwrites of B's layers, so no harm.
   - Y == X': no-op for A; FDB serializability via read-write may abort A; safe.
   - Y < X': must not happen if `cold_drained_txid` is monotonic by construction. A's Phase C should compute Y from A's Phase A snapshot; if A's plan says Y < X', A should detect this via "the snapshot I read is now stale" and abort gracefully. **This requires Phase C to compare the new `cold_drained_txid` it wants to write against the current value and skip if not strictly greater.**

5. **Lease-style same-role assumption.** Without leases, two pods may run the same compactor on the same DB simultaneously. We rely on FDB serializability + idempotent writes for correctness. The cost is bounded duplicate work + bandwidth.

6. **Pin recompute under concurrent bookmark delete.** When a pinned bookmark is deleted, GC eventually recomputes the per-branch pin floor. If a new bookmark is created at the moment of recompute, the recompute might miss it. Mitigation: pin recompute reads all bookmark pins in one FDB tx; if a new bookmark commits in parallel, FDB serializability detects.

7. **Pegboard rollback during in-flight commit.** Engine flips `actor → database_id` mapping; pegboard exclusivity ensures the actor's writer is revoked before a new database receives commits. If pegboard ever fails this guarantee, FDB serializability on `/META/head` of the old database catches the stale write — but pegboard exclusivity is the right layer to enforce this.

## Changes to the spec

Edit `.agent/specs/sqlite-rough-pitr.md`:

- Add a `## Revision: post-concurrency-cleanup v5` block at top.
- §6 FDB schema: drop `META/compactor_lease`, `META/cold_lease`, `CMPC/lease_global/{kind}`.
- §8 The two operations: drop "OCC fence on bk_pin" wording from `derive_branch_at`. Just describe the read inline; serializability does it.
- §9 Bookmarks: drop OCC wording from pinned bookmark creation.
- §12.1 Hot compactor: drop the lease section. Describe the multi-tx (per-shard) structure explicitly. State idempotency invariants.
- §12.2 Cold compactor: drop the lease section. Drop the OCC fence on `cold_drained_txid`. Add: Phase C writes `cold_drained_txid` only if strictly greater than currently observed value (monotonic guard, single-tx serializability).
- §12.3 Eviction compactor: drop the global lease. Make plan + commit one FDB tx per database. Drop the OCC fence wording on `last_hot_pass_txid`.
- §13 GC: confirm per-branch batched txs read pins each batch (no cross-batch fence).
- §15 Concurrency invariants table: replace lease/fence rows with idempotency / monotonicity rows.
- §16 Failure modes: replace lease-loss rows with stale-marker recovery rows.
- §17 Hot-path latency: drop the +1 read for OCC reads. Update RTT counts.
- §20 Metrics: drop lease metrics. Add `eviction_tx_aborts_total`, `cold_pass_duplicate_total`, etc.
- §21 Tests: add the test scenarios listed below.
- §22 Implementation strategy: drop lease-related stages.
- §27 Divergences from prior art: add an entry on lease-free design.

## Changes to code (Ralph story to add)

Add `US-068` (or next available ID) at priority `14.7` (slots in after `US-067` rollback removal):

**Title:** `Remove leases + bespoke OCC fences from sqlite-storage; rely on FDB native serializability + idempotent writes`

**Description:** Remove hot lease (`META/compactor_lease`), cold lease (`META/cold_lease`), eviction global lease (`CMPC/lease_global/*`), and lease renewal task / cancel-token machinery. Drop the bespoke OCC fence on `cold_drained_txid` from cold compactor Phase C. Refactor eviction's plan + commit into a single FDB tx per database, dropping the explicit `last_hot_pass_txid` OCC fence. Stop labeling `bk_pin` reads in fork/bookmark creation as "OCC fences" — they're plain serializable reads. Add explicit monotonic-guard to cold compactor's `cold_drained_txid` write (skip if not strictly greater). Document idempotency invariants in code comments + CLAUDE.md.

**Acceptance criteria:**
- All lease key builders + lease helper code removed from `engine/packages/sqlite-storage/`
- Renewal task + cancel-token machinery removed
- `META/compactor_lease`, `META/cold_lease`, `CMPC/lease_global/*` no longer written
- Cold compactor Phase C writes `cold_drained_txid` only if strictly greater (monotonic guard)
- Eviction compactor plan + commit is one FDB tx per database
- `bk_pin` checks in fork + bookmark creation use plain FDB tx serializability (no special fence wording in code)
- Code comments + `engine/packages/sqlite-storage/CLAUDE.md` document idempotency invariants for: SHARD content determinism, `cold_drained_txid` monotonicity, BOOKMARK status monotonicity, atomic-min/atomic-add commutativity, COMPARE_AND_CLEAR for PIDX
- All existing tests pass
- New tests added (see below)
- `cargo check -p sqlite-storage` passes
- `cargo test -p sqlite-storage` passes
- Typecheck passes

## Tests required

These tests verify the new concurrency model:

1. **Two parallel hot folds on same DB → identical SHARD content.** Spawn two compactor tasks on the same database. Verify they produce the same SHARD bytes for the same `(shard_id, as_of_txid)`. Verify FDB serializability aborts one if they hit the same key, and the surviving SHARD is correct.

2. **Two parallel cold passes → monotonic `cold_drained_txid`.** Spawn two cold passes that both reach Phase C. Verify the higher `cold_drained_txid` wins and the lower's commit aborts via serializability. Verify idempotent S3 PUTs (no corruption from double-upload).

3. **Eviction during active hot pass → eviction tx aborts and retries.** Sequence: eviction reads `last_hot_pass_txid`, pauses; hot pass commits; eviction commits. Verify eviction's tx aborts (FDB serializability on `last_hot_pass_txid`) and retry succeeds with new state.

4. **Fork during GC pin advancement → fork retries.** Sequence: GC reads pin, pauses; fork advances `desc_pin`; GC commits delete. Verify GC's tx aborts and retries with new pin floor (deletes less).

5. **Pinned bookmark create during GC → bookmark wins.** Same shape as #4.

6. **Stale-marker sweep cleans orphan files from crashed Phase B.** Inject Phase B failure mid-upload. Next pass's stale-marker sweep should delete orphan layers + the marker.

7. **Concurrent forks of same source → both succeed with different IDs.** Spawn two `fork_database(D, V)` calls. Verify both commit, both get distinct new database IDs, source's `desc_pin` is at minimum of the two `at_versionstamp` values, source's `refcount` increased by 2.

8. **Cold pass with stale plan → idempotent or aborts.** Pod A reads `cold_drained_txid = X` in Phase A. Pod B commits Phase C advancing to X'. Pod A's Phase C runs; verify it either skips (X' > A's intended Y) or commits (Y > X'), never regresses.

9. **Hot fold across multi-shard DB → idempotent under interruption.** Start fold of DBs with 5 dirty shards; kill after 2 shards folded. Restart compactor. Verify remaining 3 shards get folded; previous 2 are unchanged.

10. **Pegboard exclusivity verification.** Document and test that a database has at most one writer at a time. Verify that concurrent commits on the same DB hit pegboard's exclusivity (not FDB serializability) — if FDB ever sees two commits, that's a pegboard bug to surface.

## Open questions

1. **Hot compactor idle wakeup.** Without a lease, what tells one pod it should wake to fold? Currently the hot compactor wakes on UPS publication. Without a lease, multiple pods receive the publication via queue group; whichever wakes first does the work. Is this OK or do we need explicit "I'm working on this" coordination?
2. **Cold compactor scheduling fairness.** Without a lease, no per-pod load balancing of cold work. NATS queue group distributes triggers, but if one pod is slow, others may take over. Is this fair or do we want explicit work-stealing?
3. **Eviction global sweep coordination.** Without a global lease, multiple pods may run eviction sweeps simultaneously. Each picks up databases from `eviction_index`. They might both try to evict the same database. FDB serializability handles correctness, but how do we avoid wasted scans?

These don't block the cleanup. Note them as future tuning concerns.

## Risks / things to watch

- **Lost mutual exclusion for "I'm working on this."** Without a lease, debugging "why is this DB not making progress" gets harder — no clear owner. Mitigate with metrics: `cold_pass_active_pods{db_id}` (gauge of how many pods are mid-pass on a DB at any moment). Alert if > 1 sustained.
- **FDB conflict-resolver hammered under high contention.** Eviction-vs-hot may produce abort storms. Mitigate with `eviction_tx_aborts_total` metric and back-off behavior.
- **S3 cost during NATS redelivery storms.** Each redelivery → duplicate pass → duplicate S3 PUTs. Mitigate with metric: `cold_pass_duplicate_total` (passes where final commit conflicted with another).
- **Future code change violates idempotency invariant.** A new write to `cold_drained_txid` that's not monotonic, a new SHARD content that's not deterministic, etc. Mitigate with code-comment documentation + CLAUDE.md + explicit tests in the suite that lock the invariants.

## Summary

Removing leases and OCC fences in favor of FDB-native concurrency + idempotent writes is a real simplification: less code, fewer keys, fewer conceptual mechanisms, same correctness. The cost is increased duplicate work during rare contention bursts — bounded by NATS routing efficiency and idempotent writes.

The cleanup pairs naturally with v4 (rollback removal) and the eviction-pin-removal we discussed. Could be a single v5 spec revision covering all three.
