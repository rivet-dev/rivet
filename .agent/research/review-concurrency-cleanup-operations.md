# Operational Review: SQLite Concurrency Cleanup

Adversarial review of `/home/nathan/r2/.agent/specs/sqlite-concurrency-cleanup.md` from a production-operations angle. Targets oncall debuggability, deployment safety, multi-pod scaling, alert fidelity, and forensic capability.

---

## Critical operational risks

### C1. No unbounded-eviction-retry budget under sustained hot pressure

**Scenario.** A high-write tenant commits at ~1ms cadence. Every commit advances `last_hot_pass_txid` (or, more precisely, every hot pass does). Eviction's per-DB tx reads `last_hot_pass_txid`, plans, then commits. Under sustained hot bursts the eviction tx aborts on every retry. Quota in FDB grows; quota check eventually returns `SqliteStorageQuotaExceeded` to the user.

**What goes wrong.** The proposal says "may need to back off eviction during hot bursts" but specifies no backoff protocol, no maximum retry budget, no escape hatch (e.g. lower SHARD_RETENTION_MARGIN, force-evict on quota pressure, take a transient lease). Today's global eviction lease at least serializes the work and lets one pod make steady progress; with N pods all retrying the same DB, they collectively make negative progress (N retry storms vs 1 retry storm).

**Missing.** Concrete backoff schedule, retry budget per DB per sweep, alarm threshold on `eviction_tx_aborts_total{db_id}`, and a documented "what to do when eviction is stuck" runbook entry that is not "wait for quota to fail." Possibly a degraded-mode hint key like `/META/eviction_paused_until_ms` set by a control-plane operator.

### C2. Pegboard exclusivity violations have no operational backstop

**Scenario.** Pegboard has historically had bugs and lost-timeout windows where two actors both believe they are the live writer. Today, even if pegboard leaks, the FDB lease layer sometimes catches it (one of the two writers fails to take the lease). After the cleanup, FDB serializability "catches" divergent commits via `/META/head` read-write conflict. One commit silently aborts and the user sees a transient error.

**What goes wrong.** The user-visible failure mode is the wrong one. The losing writer has no way to know its work was orphaned because of an exclusivity leak rather than ordinary contention. The proposal does not specify how to surface "this looked like an exclusivity violation, not normal contention." `pegboard exclusivity is the right layer to enforce this` (proposal §7) is not an operational answer; it's an architectural one.

**Missing.** A debug-mode or release-mode metric that distinguishes "FDB serializability aborted a `/META/head` write because two pods both held the actor" from generic contention. Cheapest version: keep a `runner_id` field in `/META/head` writes (debug builds; or release behind a flag) so the abort log line says "writer X expected head from writer X, found writer Y." Without that, exclusivity bugs are forensically invisible.

### C3. "Compaction stuck" debug story has no mechanism

**Scenario.** Customer reports their DB is not making progress (cold-tier lag growing, `head_txid - cold_drained_txid` rising). Oncall checks. Today: read `/META/cold_lease`, see who holds it, when it was renewed; if expired, look at the holder pod's logs. After the cleanup: there is no answer to "who is currently compacting this DB?"

**What goes wrong.** The runbook entry "check the lease" disappears. The proposal mentions `cold_pass_active_pods{db_id}` as a gauge "alert if > 1 sustained" but does not specify how each pod publishes itself into the gauge, what the source of truth is, or how an oncall reads it as a structured query (e.g. PromQL by `db_id`). High-cardinality `db_id` labels in Prometheus are also a known cardinality bomb across thousands of tenants.

**Missing.**
- A non-Prometheus structured-log convention: every pass start/finish emits `db_id`, `pod_id`, `pass_uuid`, `phase`, with stable field names that are queryable in the log search tool.
- A "who is doing what right now" admin endpoint or debug query: scan `pending/{uuid}.marker` for the DB across S3, list active passes by their durable side effects (the marker IS the breadcrumb — say so explicitly).
- An explicit replacement runbook entry for "stalled DB" that walks the operator through: check head/cold-drained lag, check active markers, check NATS queue depth on `cold_compactor`, check FDB conflict-resolver metrics.

---

## Major operational concerns

### M1. Multi-pod sweep amplification

**Scenario.** 10 compactor pods. Each independently scans `eviction_index` to find candidate DBs. 10× the FDB read load on the global index, with a small write fan-out (only one wins each tx). At 100k tenants this is meaningful FDB conflict-resolver and read traffic.

**What goes wrong.** The global eviction lease today serializes the scan to one pod. Removing it without specifying "one pod sweeps at a time via NATS queue group" creates 10× scan load. The spec says "multiple pods do not need to parallelize across databases" (former invariant) but the cleanup proposal does not replace this with anything. Open question 3 acknowledges this and is "future tuning concerns" — that is not an operational answer.

**Missing.** Mandate that eviction sweep itself is gated by a NATS queue-group trigger published on a fixed cadence (e.g. one global UPS topic, one consumer wins). Otherwise capacity planning has to assume N-pod amplification.

### M2. Migration order during rolling deploy is unspecified

**Scenario.** Rolling deploy of N pods. During the window, M pods have lease+OCC code, K pods have the new lease-free code. Old pods take leases; new pods don't. Old pods still write `/META/cold_lease` keys; new pods ignore them. An old pod takes the lease, crashes, lease key persists. New pod starts a pass alongside, ignoring the lease. Both pods now run the same cold pass concurrently, which (per the proposal's invariants) is safe but not what was tested.

**What goes wrong.** The transitional behavior is a third concurrency model that was never designed. Subtle interactions: the old pod might write `last_hot_pass_txid` with OCC semantics while new pods do not; old pods might OCC-fence on `cold_drained_txid` while new pods only do monotonic-skip; an old pod can starve under retry while new pods make progress (or vice versa).

**Missing.** Explicit two-phase migration:
1. Deploy code that respects existing leases (writes them, reads them) but is also correct without them. Run this version everywhere.
2. Deploy code that stops writing leases. Old keys garbage-collect over TTL.
Without this, the cleanup PR is single-step and rolling-deploy behavior is undefined.

### M3. Rollback removes durable side effects

**Scenario.** Cleanup ships, and a week later we discover a contention pathology not caught in test. We need to revert. But by then, no pod has been writing `/META/compactor_lease`, `/META/cold_lease`, or `CMPC/lease_global/*` keys for a week. Reverting the code reintroduces the lease take/renew but the live keys are absent (already cleared by cleanup PR or never written).

**What goes wrong.** The reverted lease-take logic finds no lease, takes one, proceeds. That's actually fine — leases are TTL-based and self-healing. So this concern is small. **However**, if the cleanup PR also adds a one-time migration that DELETES old lease keys at startup (to clean up orphans), revert is still fine because they're already gone. Unless the cleanup PR adds a schema-version bump that older code refuses to read.

**Missing.** Spec should commit to "no schema version bump, no destructive migration, no key-format change" and explicitly state revert is safe. Also explicit: if a hotfix is needed before full revert, manually re-introducing a lease key by hand should still work as a kill-switch.

### M4. Customer-facing fork SLA under contention

**Scenario.** A customer hits `fork_database` during a busy GC sweep. The fork tx aborts on `desc_pin` read-write conflict. FDB native retry retries N times. After N retries (default 5? 100? unspecified) the call returns an error. What does the user see?

**What goes wrong.** The proposal says "FDB serializability handles the GC race" but does not specify (a) the FDB retry budget, (b) the user-facing error code if retries are exhausted, (c) the SLA for fork latency under contention, (d) whether the call hangs or returns. Today there's still no explicit lease on fork, but there are explicit OCC reads — so the failure mode is at least named (`ForkOutOfRetention`, `ForkChainTooDeep`). After the cleanup, generic FDB tx-aborted is harder to map to a user error.

**Missing.** Explicit retry budget, explicit timeout, explicit error code for "fork could not commit due to sustained contention." Without these, the user-facing error is a generic 500 and customer support is blind.

### M5. Stale-marker accumulation sizing

**Scenario.** NATS redelivery rate climbs to 5% (cluster network instability, NATS rebalance, etc.). Each cold pass that retries leaves a `pending/{uuid}.marker` until the next pass cleans it up. Across 100k DBs, 5% redelivery means ~5000 stale markers in flight at any moment. S3 LIST cost for stale-marker sweep grows with marker count.

**What goes wrong.** `STALE_MARKER_AGE_MS` default isn't specified in the cleanup spec. If it's set conservatively (say 1 hour), cleanup is slow and markers accumulate; if aggressive (say 5 min), in-flight long passes get their markers deleted out from under them. Either way, S3 LIST cost is now driven by failure rate, not workload — hard to capacity-plan.

**Missing.** Sizing analysis: for a target redelivery rate, what's the steady-state marker count? What's the S3 LIST cost ceiling? Add `cold_pending_markers_total` gauge with a clear alert threshold.

### M6. Multi-tenant cost amplification from duplicate work

**Scenario.** 100k DBs × 5% NATS redelivery × ~10 cold passes/day each = 50k duplicate passes/day. Each duplicate pass is a full S3 PUT cycle (image + delta + manifest). At an average pass of 10MB and S3 PUT pricing of $0.005/1000, the duplicate cost is small ($2.50/day) but the egress and FDB read cost (re-running Phase A) is non-trivial. At higher redelivery rates (10%) and bigger DBs (1GB), this scales.

**What goes wrong.** The proposal calls duplicate work "bounded waste" without sizing it. No metric in the proposal lets ops detect "duplicate cost is now 30% of cold-tier spend."

**Missing.** Add `cold_pass_duplicate_bytes_total` (PUT bytes that landed because of a duplicate pass) and `cold_pass_duplicate_phase_a_reads_total`. Both are required for cost attribution.

---

## Minor operational concerns

### m1. Manual operator interventions disappear

Today, an operator can manually clear a stuck `/META/compactor_lease` key via UDB tooling to force a pass to retry on a healthy pod. After the cleanup there is no such key to manipulate. The cleanup spec gives no replacement (e.g. a "force re-trigger via UPS publish" admin command). For every "the system is wedged, fix it now" incident, operators need a manual lever.

### m2. Forensics rely entirely on logs

After the cleanup, the only durable record of "pod X processed pass Y at time T" is the structured log emitted by pod X. If logs are dropped (Loki backpressure, pod evicted before flush), the trail is gone. Today, the lease key is durable in FDB. The proposal should require: every pass writes its `pod_id` into the `pending/{uuid}.marker` body so post-incident forensics can read S3 to reconstruct attribution even if logs are lost.

### m3. Alert thresholds need empirical baselines

`eviction_tx_aborts_total` is suggested but no threshold. Under healthy multi-pod operation, what's the normal abort rate? Without a baseline, every operator either sets the threshold too loose (alert fatigue) or too tight (real problems missed). The cleanup PR should ship default alert configs along with the metrics.

---

## Validation: where the new model is genuinely easier

- **No lease renewal task** means no class of bugs from renewal-misses-deadline-due-to-tokio-starvation. This is a real ops win; lease-renewal latency was a documented flake source in `engine/CLAUDE.md`.
- **Single FDB tx for eviction** simplifies the failure mode: either the tx commits or it doesn't. Today's plan-then-fence has a window where the plan is computed against stale state and the fence catches it; reasoning about "is my plan stale?" goes away.
- **No lease TTL window** means failed pods don't cause `TTL` waits before failover. Today, a pod death during a cold pass burns a 30s lease TTL before another pod can pick up. Under the cleanup, the next NATS redelivery picks up immediately.
- **Fewer keys** means the FDB schema is smaller. Smaller key space = faster scans, faster backups, less to reason about.

---

## Required additions to the cleanup plan

These should be **mandatory** acceptance criteria for landing the cleanup PR, not "future tuning":

1. **Sweep coordination story.** Either NATS-queue-group-gate the eviction sweep, or document why N-pod amplification is acceptable (with a concrete max-pods cap).
2. **Per-pass forensic trail.** `pending/{uuid}.marker` body must include `pod_id`, `pass_started_at_ms`, `last_phase`. Logs alone are not durable enough.
3. **Stuck-DB runbook.** Replace the "read the lease" debug step with concrete substitutes: query for marker objects, query for FDB lag, query for NATS queue depth, query for active passes via structured logs. Ship this in `docs-internal/engine/sqlite/operations.md` (or equivalent).
4. **Eviction backoff protocol.** Define what happens when `eviction_tx_aborts_total{db_id}` exceeds a threshold. Default behavior should be "skip this DB this sweep, retry on next sweep" rather than "spin retrying."
5. **Fork retry budget.** FDB tx retry count for `fork_database` and a user-facing error code for "fork could not commit due to contention."
6. **Migration ordering.** Two-phase deploy: first deploy reads-but-doesn't-write leases; then deploy stops-writing leases. Both must be correct standalone.
7. **Rollback safety statement.** Explicit "no schema version bump, lease keys may be re-introduced by reverting the PR" commitment.
8. **Alert defaults.** Ship default alert configs for `eviction_tx_aborts_total`, `cold_pass_duplicate_total`, `cold_pending_markers_total`, `sqlite_cold_lag_versionstamps`. Do not leave thresholds for operators to discover.
9. **Cardinality plan.** `cold_pass_active_pods{db_id}` is unworkable at 100k tenants. Specify aggregation (`{namespace}` or `{tenant_tier}`) or replace with a non-Prometheus sink.
10. **Exclusivity-leak detection.** Debug-mode `runner_id` field in `/META/head` writes so FDB serializability aborts can be classified as "exclusivity leak" vs "ordinary contention."

---

## Bottom line

The cleanup is architecturally sound but operationally under-specified. The current spec is a refactor with no operations chapter. Production operability after this change depends on five things the spec does not deliver: a sweep-coordination story, a forensic trail, a stuck-DB runbook, an eviction backoff protocol, and a migration plan. Without those, the next P1 incident on a busy DB will land with the oncall having no leases to grep, no lease-holder logs to read, and no documented runbook for the new world.
