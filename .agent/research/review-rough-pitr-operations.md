# Operations review: sqlite-rough-pitr.md

Adversarial review focused on production failure, recovery, monitoring, and migration. Cites by section number.

## Recovery / failure scenarios

### S3 down for 1 hour (charter Q1) — §12.2, §16

Spec is silent on what happens to commits during a sustained S3 outage on Tier 1+ namespaces. Reading the pieces:

- Hot compactor on Tier 1+ "deletes folded DELTAs only when `max_folded_txid <= cold_drained_txid`" (§12.1, line 785). With cold stuck, every fold accumulates SHARD versions in FDB without ever reclaiming DELTAs.
- `BranchManifest.cold_drained_txid` (§6, line 302) freezes; eviction predicate (§12.3) requires `cold_drained_txid >= max_folded_txid` so eviction also stalls.
- Quota enforcement at commit (§16, "Eviction compactor lags badly" → `SqliteStorageQuotaExceeded`) is the only backpressure. Per-actor quota is the *only* fence. If actors are within quota, FDB grows unbounded for an hour. Cluster-level FDB pressure is not modeled.
- No spec for "S3 burst tolerance" or a global FDB cold-tier-lag kill switch.

User-visible failure: per-actor quota rejection only, no namespace-aware degradation. A noisy actor that was within quota when S3 was healthy now hits quota and writes start failing with `SqliteStorageQuotaExceeded`, which is misleading because the issue is operator-side.

**Fix:** Define a global cold-tier lag SLO. Add `sqlite_cold_lag_seconds_total` metric + a global FDB pressure gauge (sum of `head_txid - cold_drained_txid` across active branches). Add a "degraded mode" flag in cold compactor that, if S3 is failing for more than threshold, returns a distinct `SqliteStorageColdTierUnavailable` error to commits in Tier 1+ rather than letting per-actor quota reject. Document the RPO degradation explicitly.

### S3 slow (p99 = 5s) — §12.2

Phase A is bounded by FDB tx-age (5s). Phase B is "S3 only, no FDB tx" so it is not directly bounded. Phase C is FDB tx with regular-read OCC fence on `cold_drained_txid` (lines 815-823).

Risk: Phase A snapshot-reads SHARDs, DELTAs, and COMMITS. For a heavily-loaded branch, that read alone (lines 800-805) can run multiple seconds. Then Phase A *also* writes the pending marker to S3 *inside the FDB tx* per line 806: "Write pending/{uuid}.marker to S3 as a HWM. Tx commits." That means an S3 PUT is in the FDB tx-age budget.

If S3 PUT p99 is 5s, Phase A blows tx-age routinely. Phase A retries are silent (no metric called out).

**Fix:** Move the pending marker S3 PUT to *before* the Phase A FDB tx commits (write marker, then commit FDB tx that records the marker UUID into `META/cold_compact.in_flight_uuid`). Or split: write marker in a separate pre-Phase A step. Add `sqlite_cold_pass_phase_a_tx_age_seconds` histogram and an alert at 80% of the 5s budget.

Phase C also "renews cold_lease" (line 816) inside the tx. Lease renewal is a write op; one extra write inside a possibly-long Phase C tx erodes the budget. Renewal should be in the local-timer task, not in-tx.

### Cold compactor pod loses lease mid-pass (charter Q3) — §12.2, §16

Trace pod A → pod B handoff:

1. Pod A: takes `cold_lease`, writes `pending/{uuid_A}.marker` (Phase A), uploads layer files (Phase B), dies before `cold_manifest.bare` is written and before Phase C commits.
2. Lease TTL expires.
3. Pod B: takes `cold_lease` for the same branch.
4. Pod B starts Phase A. Reads `META/cold_compact.cold_drained_txid`; same as pod A read. Plans the same drainable range.
5. Pod B writes `pending/{uuid_B}.marker`.
6. Pod B re-reads pending markers in Phase B (line 813): "Re-read pending markers; clean up orphaned ones older than `STALE_MARKER_AGE_MS`."

Race: pod A's marker and orphan layer files can be younger than `STALE_MARKER_AGE_MS` when pod B starts. Pod B will *re-upload over* pod A's layer files (idempotent — line 38, line 947), then on its own next-pass cleanup, sees the older `uuid_A` marker as stale and deletes it. The orphan layer files are also overwritten by pod B's same-keyed PUT (filenames omit content checksum per §3 line 38).

**But:** what if pod A uploaded layers under a slightly different planned set? E.g., pod A plans shards [10, 20, 30] but dies after only [10, 20] uploaded. Pod B picks a fresh range and plans [10, 20, 30, 40]. Layers for shard 30, 40 do not collide with pod A's; pod A's shard 10 + 20 were re-uploaded by B. But pod A's marker UUID is now still in S3, pointing at... no specific layers — the marker is just a UUID, not a manifest of which layers. Cleanup of pod A's orphans relies entirely on overwrite-by-same-key. If pod A computed a different `as_of_txid` for a shard than pod B (e.g., a hot pass landed in between, advancing `last_hot_pass_txid`), pod A's orphan layer at the older `as_of_txid` is **not overwritten**. Spec does not have a sweep that lists all `image/{prefix}/` and deletes layers not referenced in the latest `cold_manifest.bare`.

**Fix:** Make the pending marker self-describing — include the planned object keys. On stale-marker cleanup, *also* delete the listed object keys (idempotent if already overwritten). Add a periodic full sweep that lists S3 objects and reconciles against `cold_manifest.bare` to catch leaked layers from version-mismatched plans. Add `sqlite_cold_orphan_layers_cleaned_total` metric.

### Eviction during active session (charter Q4) — §12.3

Spec: eviction reads `last_access_ts` and skips if `now - last_access_ts < HOT_CACHE_WINDOW_MS` (line 860). `last_access_ts` is updated lazily by `ActorDb` on read/write activity (line 850) and is "best-effort; not load-bearing for correctness."

Race: actor A is being read by an envoy. Eviction runs concurrently. Eviction reads `last_access_ts` at T0. Envoy update arrives at T1 > T0 *after* eviction's read but *before* eviction's clear_range (Phase C-equivalent; spec doesn't define one for eviction). Eviction clears, envoy then reads page → FDB miss → cold tier fall-through (§10 lines 741-749). The user sees an extra-latency read but no error.

But: if the cold manifest has not yet been updated to reflect that this shard is in S3 (cold pass landed but cold_manifest write failed and got retried), eviction's predicate `cold_drained_txid >= max_folded_txid` was based on a stale read. The cold_manifest in S3 may not yet list the layer.

**Real concern:** eviction does not coordinate with active envoys via lease/pin. The "still hot" gate is purely a timestamp heuristic. If `last_access_ts` is not updated atomically with the read (and §6 line 309 says "not load-bearing for correctness"), eviction can race.

The cold-tier read fall-through path (§10 lines 741-749, §22 stage 8) is implementation-staged at *Stage 8* — last. Until Stage 8 ships, eviction landing on a still-hot actor causes a read failure with no fallback.

**Fix:** Document eviction-vs-active-read race semantics explicitly. Either (a) extend `BranchCounters` with an `active_session_pin` atomic-min-versionstamp updated at envoy WS open and decremented at close, eviction reads regular and aborts on conflict, or (b) accept the race but specify that the cold-tier read fall-through is mandatory before eviction can ship to production. Add ordering: Stage 7 (eviction) must not deploy before Stage 8 (cold fall-through) on Tier 1+ namespaces.

### Tier 0 → Tier 1 transition during active commits (charter Q5) — §14

Spec line 907: "The first `fork_*` or `create_pinned_bookmark` call discovers `tier == T0` on the namespace branch, atomically promotes to T1 in the same tx. Concurrent first-fork calls race on the same `set_if_equal(tier, T0, T1)` op."

Gap: in-flight commits on actors in the namespace at the moment of transition. A commit running in parallel with the fork tx does not see the fork's tier change until it re-reads the namespace branch record. The commit (§11) does not read the namespace branch record at all — it reads APTR + actor branch. The actor branch record carries `tier: Tier` (§5.4 line 179) which was T0 when the actor was created.

After fork: the *namespace* branch has tier=T1, but the actor branch records still have `tier: T0`. The hot compactor (§12.1 "Tier-aware DELTA deletion") reads actor-branch tier? Spec doesn't say which tier check it uses. If it uses actor-branch tier, the actor stays Tier 0 forever (deletes folded DELTAs unconditionally) even though the namespace fork is Tier 1.

This breaks fork descendants: the parent's old DELTAs got deleted under Tier 0 rules, so the fork at versionstamp V has no DELTA to read.

**Fix:** Tier transition must also rewrite each actor branch's `tier` field, OR the hot compactor must read the namespace branch tier (one extra FDB read per pass; acceptable since hot pass is rare). OR change schema so `tier` lives only on the namespace branch and actor branch reads through it. The third option is cleanest. Add a regression test: T0 commit landing during T0→T1 transition must not delete DELTAs the fork depends on.

### Versioned SHARD eviction races compaction (charter Q6) — §12.1, §12.3

Hot pass produces a new SHARD at `as_of_txid = max_folded_txid` (§5.2 line 122). Eviction predicate gates on "newer SHARD version exists" (§12.3 line 859). Hot pass commits SHARD-write *and* deletes folded DELTAs (Tier 0) *and* updates `META/compact.materialized_txid` in one tx (inherited from stateless spec).

Hot pass mid-pass holds a snapshot read on a SHARD version. Eviction passes are separate tx with separate lease; they do not coordinate with hot snapshot. Eviction reads regular on `desc_pin` and `bk_pin` so OCC fences against fork; but eviction's clear_range on the older SHARD version is in eviction's tx, not hot's.

If hot's snapshot was reading the *older* SHARD version (because the new SHARD is being produced), eviction can clear that older SHARD between hot's read and hot's commit. Hot's commit doesn't write the cleared SHARD back; it writes the new one. Result: temporary phantom miss for any other reader (e.g., a fork descendant reading at an old versionstamp) between eviction's clear and the new SHARD being durable.

But hot's tx is OCC; eviction clearing the same key would cause hot to abort if hot had a read conflict on it. Snapshot reads do *not* take read conflict ranges (per FDB semantics + CLAUDE.md line 29). So hot does not abort; hot commits a new SHARD; eviction commits the clear of the old SHARD. Outcome OK *for hot* but a fork descendant pinned at the older version sees a brief window where neither old nor new is queryable (descendant reads at versionstamp before new SHARD's `as_of_txid`).

The desc_pin / bk_pin gate is supposed to prevent eviction of pinned versions. But what if the pin is set by a fork that lands *between* eviction's pin-read and eviction's clear? Spec says "Eviction reads `desc_pin` regular (not snapshot); fork's atomic-min triggers OCC abort on eviction tx" (§15 line 937). Good. So fork landing post-pin-read aborts eviction. Confirmed safe for the fork scenario.

**Fix:** No correctness fix needed for fork case. For the snapshot-vs-clear race on hot, document it explicitly: hot's snapshot of an older SHARD version is meaningful only as input to fold; the older SHARD version is overwritten by the new SHARD's pages anyway, so loss of the old SHARD between snapshot and commit is benign. Add a debug-build assertion that hot's `materialized_txid` advances strictly after the new SHARD write commits.

## Atomicity / idempotency holes

### Pinned bookmark synchronous PUT failure — §16

Spec line 954: "Pinned bookmark S3 PUT fails → `create_pinned_bookmark` returns the error; pin not persisted." Good.

But: `restore_to_bookmark` (§8.4 lines 645-650) calls `create_pinned_bookmark` *before* `rollback_actor`. If the pinned PUT succeeds but the subsequent `rollback_actor` fails (FDB tx error, e.g., conflict), the pin is now persisted with no rollback having happened. The pin's bk_pin atomic-min has advanced; GC retention is now extended unnecessarily. There is no rollback of the pinned bookmark.

**Fix:** Either (a) move pin creation *after* successful rollback (loses the "exact undo" guarantee in some failure modes), or (b) add an explicit `delete_pinned_bookmark(...)` to clean up on the failed rollback path. Document the partial-failure semantics. Add a retry helper that on rollback failure deletes the just-created pin.

### Refcount decrement on rollback without subsequent pointer write — §8.3, §8.4

Rollback (lines 555-597, 605-638) is one FDB tx that: (a) calls `derive_branch_at` (which atomic-adds parent refcount +1 + child refcount +1), (b) writes pointer history, (c) atomic-adds old branch refcount -1, (d) writes the new pointer. If any step after (a) fails (commit-time conflict on history key, e.g.), the old branch's refcount is +1 too high (because derive_branch_at incremented it). All-or-nothing semantics in a single FDB tx make this atomic, so a tx-abort cleanly rolls back. OK.

But the spec wraps these in a single `udb.run(|tx| async ...)` closure — UDB's tx auto-retries on conflict. If the closure's atomic mutations are not idempotent under retry, refcount drifts. Spec doesn't comment on retry safety of `atomic_add` inside `udb.run`. FDB atomic mutations *are* tx-scoped (rolled back on abort, re-applied on retry), so this is OK *if* the tx body is purely tx-scoped. As long as the closure does no out-of-band side effects (no S3 PUT, no spawned task), retry is safe. Confirm in code review.

### Actor catalog under namespace fork — §8.1, §8.2 (charter Q8)

Namespace fork (§8.1) does not materialize any actor catalog rows. Reads of `NSBRANCH/{ns_branch_id}/actors/{actor_id}` (§6 line 236) miss on the new branch and "fall through to parent" (§8.1 line 487). But spec does not show parent fall-through code for actor catalog reads. The data model has the key but no read helper documented.

Actor enumeration "list all actors in namespace V" semantics:

- For root namespace: `range_scan` on `NSBRANCH/{root}/actors/`.
- For forked namespace: range scan on the fork's prefix returns only divergent actors; parent's actors are missed.
- Spec hints "lazy parent-chain resolution" (§8.1 line 502, §1 line 7) but no enumeration algorithm.

To enumerate fully: walk parent chain, range_scan each ancestor's `actors/` prefix, deduplicate by actor_id, *and* honor "actor was deleted in fork" semantics (which spec doesn't define — there is no tombstone for actor deletion in a fork).

**Fix:** Define actor-deletion-in-fork explicitly. Add an `actors_tombstone/{actor_id}` key per namespace branch. Define enumeration algorithm: union of ancestors' `actors/` prefixes minus tombstones at any descendant. Add `list_actors(ns_id) -> Vec<ActorId>` to public API and document complexity (O(actors-divergent) per ancestor, capped by `MAX_FORK_DEPTH`).

### Bookmark expiry across rollback (charter Q9) — §9, §16

Spec: ephemeral bookmarks resolve at use time. "If the nearest checkpoint is older than the GC pin, returns `BookmarkExpired`" (§9 line 675). `BookmarkExpired` is a typed error (CLAUDE.md line 86).

But after rollback, the undo bookmark from `restore_to_bookmark` is *pinned* (§8.4 lines 645-650). Pinned bookmarks pin GC; they don't expire by GC. They expire only when explicitly deleted.

So 30-day undo expiry is *not* part of the spec. Pinned bookmarks live until deleted. The user request "30 days" must be a separate billing/lifecycle feature on top of the spec.

If the engine edge has a "30-day undo" policy, it must own the pinned-bookmark deletion. Spec does not describe how a pinned bookmark gets deleted (no API), how the bk_pin atomic-min recomputes after deletion (line 941: "Each bookmark contributes via atomic-min; deletion reads bookmark and runs a per-pass full recompute"), or what happens to a user holding a deleted pinned bookmark.

**Fix:** Add `delete_pinned_bookmark(actor_id, bookmark)` API. Document that resolution of a deleted pinned bookmark returns `BookmarkExpired`. Document the "full recompute on deletion" pass cadence (§15 line 941 says "pass is rare" but doesn't define cadence). Define pinned bookmark TTL (engine-edge concern but storage exposes the primitive).

## Observability gaps (charter Q12)

Metrics defined in §20:

- Fork/rollback counts.
- Bookmark create/resolve counts and latency.
- Cold pass duration (Phase A/B/C labeled).
- Cold layers/bytes uploaded.
- Eviction pass duration, shards/deltas cleared.
- Tier engagement counter.
- Pending marker orphans cleaned.

**Missing:**

- `sqlite_cold_lag_versionstamps` — gauge per branch of `head_versionstamp - cold_drained_versionstamp` (charter Q1).
- `sqlite_cold_pass_phase_a_tx_age_seconds` histogram + budget alert.
- `sqlite_cold_lease_held_seconds` histogram (detect stuck pods).
- `sqlite_eviction_predicate_blocked_total{reason=hot_window|desc_pin|bk_pin|cold_drain}` — why evictions are skipped (debug retention growth).
- `sqlite_s3_request_failures_total{op=put|get|delete}` (S3 quota / 5xx tracking).
- `sqlite_cold_quota_bytes_per_branch` gauge (charter: S3 quota observability).
- `sqlite_branch_refcount_distribution` histogram (detect leaked refcounts).
- `sqlite_orphan_layer_files_total` gauge (post-sweep result).
- `sqlite_fork_depth_distribution` histogram (detect approach to MAX_FORK_DEPTH=16).
- `sqlite_pinned_bookmark_count_per_namespace` gauge (cost / quota).
- `sqlite_pin_recompute_duration_seconds` histogram (the "rare full recompute" of §15 line 941 is unbounded; needs metric).
- `sqlite_namespace_tier{tier}` gauge (per-namespace tier visibility).
- `sqlite_aptr_history_rows_per_actor` gauge (rollback log accumulation).
- Charter Q15-related: no failure-injection metric for "pending marker count older than warn threshold".

**Missing tracing context:** spec has §10 line 727 read path that involves ancestry walk; if `actor_id` and `branch_id` aren't structured fields on every span, debugging fork descendant bugs is impossible.

**Fix:** Add metrics above. Require `actor_id`, `branch_id`, `versionstamp` (where applicable), and `kind` structured fields on all storage-layer tracing per CLAUDE.md "Logging" rule. Add a `debug::trace_get_pages(actor_id, pgnos, at_versionstamp)` API that returns a step-by-step trace (which branch served each page, PIDX vs SHARD vs cold, S3 keys hit) for support-ticket diagnostics.

### Operability: "fork descendant returned wrong data" debug path (charter Q7)

Trace path from user complaint to root cause:

1. User says: actor F (forked from A at V) read returned bytes X; expected bytes Y from parent at V.
2. Engineer: `debug::dump_actor_ancestry(F)` returns `[(F_branch, _), (A_branch, V)]`.
3. Engineer: `debug::dump_branch_pins(A_branch)` checks `desc_pin <= V` and `bk_pin <= V` (i.e., V is preserved).
4. Engineer: needs to know "what page state existed at V". Spec offers no `debug::read_at(branch, versionstamp, pgno)` API. Only `get_pages` on the live actor.
5. Engineer: must manually walk PIDX + DELTA + SHARD on A_branch with a versionstamp cap and parse LTX. No tool.
6. Engineer: needs to also check whether eviction landed and SHARD went to S3. `debug::dump_cold_manifest(A_branch)` lists layers; engineer scans for the (shard_id, max_txid) covering the page-at-V.

Steps 4-6 require code-change-to-debug. No production support workflow.

**Fix:** Add `debug::read_at(actor_id, versionstamp, pgno) -> PageReadTrace { branch_id, source: Pidx|Shard|Cold, owner_txid, layer_object_key }`. Add `debug::diff_branches(branch_a, branch_b, at_versionstamp) -> Vec<(pgno, source_a, source_b)>` for comparing fork-vs-parent at the divergence point. These are debug-only, not on the hot path; add them in Stage 4.

## Disaster scenarios

### FDB region failure (charter Q10) — §16

Spec line 957: "FDB disaster + S3 survival → Reconstruct branch records from `branch_record.bare`; replay images + deltas onto a fresh FDB; lossy: any commits since last cold pass are gone (RPO = FDB durability per binding constraint)."

Subtleties:

- **Tier 0 namespaces:** zero S3 data. Total loss. Not mentioned as a scenario in §16. The whole namespace is gone with zero RPO recovery. User has no warning that Tier 0 = no DR.
- **Pointer state lost.** APTR / NSPTR are FDB-only; S3 has only branch records, not pointers. After replay, every actor's APTR is unknown. Spec hints "branch_record.bare" gets you "far enough to enumerate which images exist; full recovery requires FDB" (§7 line 376), implying you need pointer state from somewhere.
- **Bookmarks index** is in `cold_manifest.bare` (§7 line 348), so pinned bookmark catalog survives. But the actor → current_branch mapping (APTR) does not.

Cannot serve reads from S3 alone:

- Without APTR, you don't know which `actor_branch_id` is current.
- Without the per-branch `META/head`, you don't know `head_txid` / `db_size_pages` / `post_apply_checksum`.
- §7 says branch records "exist so that an FDB disaster + S3 survival can reconstruct branch metadata far enough to enumerate which images exist; full recovery requires FDB."

**Fix:** Define DR posture explicitly. Either:
1. Cold compactor periodically writes a `pointer_snapshot.bare` to S3 with `(actor_id → ActorBranchId)` and `(namespace_id → NamespaceBranchId)` mappings. Adds S3 write traffic; acceptable for Tier 1+.
2. Document that DR = restore FDB from FDB backup + S3 (S3 alone is insufficient).
3. Add a *Tier 0 has no DR* warning to the cost model (§14) so users opting for Tier 0 understand the tradeoff.

Add `sqlite_dr_posture{tier, recoverable_from=s3|fdb_backup_only}` gauge for namespace-level DR observability.

## Schema/migration planning gaps (charter Q11)

§3 line 35: "`schema_version: u32` on every persisted S3 object." §16 line 960: "Cross-pod schema mismatch (rolling deploy) → Reader code retains old-version paths for one full retention window past rollout."

Gaps:

- **No schema for FDB-side keys.** APTR, NSPTR, BRANCHES records are vbare-encoded with `schema_version: u32` (§5.3, §5.4) but rolling-deploy schema mismatch in FDB is not modeled. If pod A is on schema v1 and writes APTR{schema_version:1}, pod B is on v2 and reads expecting v2 fields, vbare decode fails. Spec inherits "vbare migration" rules from engine/CLAUDE.md but does not enumerate which FDB record types need which migration.
- **Mid-rollout writer with old schema.** During rolling deploy, both v1 and v2 pods coexist. v2 writes a record with new fields; v1 pod reads, vbare downgrade path needed. Engine vbare rule says "If bytes did not change, deserialize both versions into the new wrapper variant" — but FDB-side records are not in a `vbare::OwnedVersionedData` enum-of-versions according to the spec; they're flat structs with `schema_version: u32`.
- **Cold compactor "reads old version, writes new version" race:** if a v2 cold compactor has just started rewriting a record and crashes mid-rewrite, the next pass picks up. With idempotent overwrite, the next pass re-rewrites. Safe. But: a v1 reader pod hitting the partial state? If the partial state is "old layer file still present + new layer file present" both keyed by deterministic name = re-upload-safe. If "manifest updated to point at new + layer not yet uploaded"… cold uploads layers *before* manifest (§12.2 Phase B order, lines 809-812), so manifest is the last write. Safe.
- **Tier transition is monotonic** (§2 line 24, §14 line 911); no schema migration for Tier 0 → Tier 1 of *existing* records. The spec says "open question 24.3" for downgrade. But: when Tier 0 → Tier 1 happens, all *existing* commits (made under Tier 0, no S3) are not in S3 yet. New commits land in Tier 1 path. Old commits are still hot-only. Eviction predicate requires `cold_drained_txid >= max_folded_txid`. Old commits never get cold-uploaded retroactively (spec doesn't say). So they pin the hot tier indefinitely.

**Fix:**

- For each FDB-side persisted vbare type, declare which versions are supported simultaneously and an explicit upgrade path. Use `vbare::OwnedVersionedData` enum-of-versions per CLAUDE.md global rule.
- Define Tier 0 → Tier 1 transition as triggering a *backfill* cold pass that uploads existing commits to S3, OR explicitly document the "old commits stay in hot forever" semantics and surface it as `sqlite_pre_tier_transition_pinned_commits_total` for capacity planning.
- Add a `debug::schema_versions_in_use(branch_id) -> SchemaVersionMap` API for rolling-deploy verification.

## Multi-region (charter Q13)

§2 line 21: "Cross-region replication. Future work; called out in section 25." §24.5 lists multi-region cold tier as open. Current single-region failure mode:

- One S3 bucket, one FDB cluster, one region.
- Region outage = total outage. No automatic failover.
- Pinned bookmarks are global if the bucket is, but spec doesn't define cross-region bucket replication semantics.

Multi-region is not promised. Single-region failure is total. Charter Q13 answer: regional outages do NOT cascade to multi-region actors *because there are no multi-region actors*. Future-work flag is honest; users must know.

**Fix:** Add a deployment doc note: "single-region only; regional outage = full data plane outage". Confirm the engine edge surfaces this in tier configuration UI. Add a `sqlite_region` static label on all metrics for future multi-region observability.

## Branch deletion cascade (charter Q14) — not explicitly modeled

Spec has GC (§13) but no `delete_branch` or `delete_namespace` operation. The four operations are fork/rollback × actor/namespace. Deletion is implicit via refcount → zero.

When refcount drops to zero (last pointer + last fork descendant + last bookmark removed), the branch is GC-eligible. Spec line 880-892 describes GC predicate. The cold-pass follow-up sweep deletes S3 objects whose range falls below the pin (§13 line 896).

For "delete a branch with 1000s of S3 objects": GC sweep is incremental, not all-at-once. No "destroy tx" blocks on S3 cleanup. Async leakage is bounded by the sweep cadence. Sync cleanup is not even an option.

But: spec doesn't describe the cleanup ordering. If the sweep deletes some S3 objects and then crashes, the manifest still references them. Next sweep re-runs the predicate; objects already deleted return 404 on the next-pass read of their manifest entry — does the cold compactor handle 404 on a manifest-referenced layer? Not specified.

**Fix:** Define `delete_branch` semantics: refcount drops to zero, branch enters "tombstoned" state with `tombstone_at_ms`. After `TOMBSTONE_GRACE_MS`, the GC sweep is allowed to delete S3 objects. Sweep updates the manifest atomically (S3 PutObject with version) to remove the LayerEntry *before* deleting the layer file, ensuring no manifest reference outlives the layer. Add `sqlite_branch_tombstoned_total` counter.

## Test coverage gaps (charter Q15)

§21 lists tests:

- Fork/rollback per-op.
- Cold compactor Phase A/B/C, OCC fence, pending markers.
- Eviction predicate gates (3 each).
- Tier transition + race.
- Bookmarks (4 paths: ephemeral, pinned, expired, unreachable).
- Schema-version-skew.

**Missing fault-injection scenarios:**

1. **S3 returning 5xx during Phase B.** Does cold pass back off? Does FDB pressure metric fire? No test specified.
2. **S3 latency 5s p99 during Phase A pending-marker PUT.** Phase A tx-age violation. (See §12.2 issue above.)
3. **Concurrent fork + rollback on same actor.** Fork's `derive_branch_at` runs in parallel with rollback's. Spec says "either order is valid" (§16 line 955) but no test asserts it.
4. **Refcount underflow on double-delete.** If a pointer flip is retried after a partial commit, can refcount go negative? Spec relies on FDB tx atomicity but no test forces the retry.
5. **Eviction landing during active read fall-through to cold.** Stage 7 vs Stage 8 ordering issue (above). No test.
6. **Tier 0 namespace receives a fork call concurrent with a commit on a Tier 0 actor.** Tier promotion happens; commit's tier-aware logic. No test.
7. **GC pin recompute under bookmark deletion race.** §15 line 941 "per-pass full recompute"; no test for "deletion + immediate fork at the deleted bookmark's versionstamp".
8. **Pending marker accumulating (10+ stale markers).** Cleanup behavior at scale.
9. **Cold compactor pod loses lease between Phase B and Phase C.** Charter Q3 case. Listed as a failure mode (§16 line 947) but no test specified.
10. **Actor enumeration on a 3-deep namespace fork chain with divergent + deleted actors.** §8.1 lazy parent-chain (charter Q8). Not in test list.
11. **DR replay from S3 alone.** Charter Q10. No test.
12. **`MAX_FORK_DEPTH=16` boundary.** Spec mentions cap; no test for the 17th fork being rejected.
13. **`restore_to_bookmark` rollback failure after pin succeeded** (atomicity issue above). No test.
14. **Bookmark creation/resolution across rollback chain** — rollback creates new branch; does bookmark on old branch still resolve via parent chain?
15. **Quota cap rejection during cold compactor stall.** Charter Q1. No test for "cold-down-1-hour, FDB grows, quota fires".
16. **Eviction with active session pin** (if added per fix above).
17. **Schema version skew within FDB** (only S3 schema-skew is in the test list).

**Fix:** Add the 17 scenarios as test fixtures. Prioritize 1, 2, 3, 5, 9, 11 as hardest production failure modes. Use `MemoryStore::snapshot()` (§21 line 1053) for the crash-recovery tests; use `tokio::time::pause()` for tx-age and lease-expiry tests.

## Stuff that's actually robust

- **OCC fence on `cold_drained_txid`** (§12.2 Phase C, §15 line 935) is tight: cold pass cannot commit if a hot pass advanced the watermark. No margin-based hand-waving.
- **Atomic-min for pin advances** (§15 lines 939, 941) composes correctly without conflict ranges. Fork tx atomic-min on `desc_pin` makes eviction's regular read see the new pin and abort. Solid.
- **Idempotent S3 layer overwrite** (§3 line 38, §16 line 947): re-uploads after lease loss are safe by deterministic key. No checksum-in-filename trap.
- **Lease lifecycle = local timer + cancel token + renewal task** (CLAUDE.md line 30, §12.2). Inherited from stateless spec, well-tested pattern.
- **Phase A/B/C separation** (§12.2 line 826) honors FDB tx-age budget once the pending-marker-PUT-in-tx issue is fixed.
- **Single-writer invariant unchanged** (§2 line 19, §15 line 932). All branching adds new branches (with fresh single-writer pegboard locks); never multi-writes a single branch.
- **`COMPARE_AND_CLEAR` for PIDX** (CLAUDE.md line 31, inherited): commit-vs-compaction races are no-ops on stale entries.
- **Per-conn ancestry cache invalidation via APTR change detection** (§10 line 751): sound — APTR.last_swapped_at_ms compared in-tx with /META/head read; no time-based cache TTL.
- **`MAX_FORK_DEPTH=16`** is a hard cap, not a soft warning. Forces O(1) chain depth bound, makes `resolve_bookmark` worst case (§17 line 981) bounded.
- **vbare for all persisted records except /META/quota** (§3 line 34): forward-compatible schema evolution path. Atomic counter exception is justified.
- **Versioned SHARD design** (§5.2): the "newer SHARD version exists" + 3-gate predicate (§12.3) is a clean correctness invariant for eviction. Eviction does not need a full ancestry walk.
- **Pinned bookmark = synchronous full image PUT** (§17 line 991): caller knows it pays an S3 PUT cost. No surprise.
- **Tier transition is monotonic** (§14): simpler reasoning; revisited under open question 24.3.
- **Fork is single FDB tx, no S3 PUT** (§8.2, §17 line 985). O(1) wall-clock + O(1) byte cost matches the "metadata-only" claim.
