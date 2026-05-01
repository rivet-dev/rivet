# SQLite concurrency cleanup — recommendation

Summary of the synthesized concurrency design and the next-step options. Source design doc: `.agent/specs/sqlite-concurrency-design.md`.

## Position revision

More conservative than the original cleanup-plan ambition. Three reviews surfaced real blockers; the synthesis adjusted accordingly.

| Concurrency mechanism | Verdict | Why |
|---|---|---|
| Hot compactor lease | **drop** | Efficiency-only. Hot fold tx is fast; FDB serializability catches duplicate-pod races; idempotent SHARD content + `compare_and_clear` PIDX makes parallel folds converge |
| Cold compactor lease | **keep** | Bounds Phase B duration, gates the stale-marker sweep, prevents bandwidth waste from duplicate S3 PUTs on large databases |
| Eviction global lease | **keep** | Prevents N pods scanning `eviction_index` in parallel, which would amplify FDB read load by N |
| OCC fence on `cold_drained_txid` | **drop** | Replaced by Serializable Phase A reads + monotonic-guard write |
| OCC fence on `last_hot_pass_txid` | **drop** | Replaced by single-tx-per-branch eviction with Serializable read |
| "OCC fence" wording on `bk_pin` | **drop (rename only)** | Already FDB native serializability; just stop labeling it OCC |

## Required code changes

Some are pre-existing bug fixes (must land alongside or before the cleanup), not part of the cleanup proper.

### Pre-existing bugs surfaced by the reviews

1. **SHARD chunking** — current code writes single 256 KB FDB values, violating the 100 KB per-value limit. Pre-existing; would break in production at scale.
2. **Hot fold pin-awareness** — `clear_range` on folded DELTAs proceeds without reading `desc_pin`. A concurrent fork can be left without recoverable state for pages written between shard-fold boundaries. Pre-existing race (see correctness review C1, plus the diagram earlier in the design conversation).
3. **`bk_pin` recompute uses plain `set` instead of `byte_min`** at `bookmark.rs:301-302`. Pre-existing race against parallel forks.
4. **Versionstamp simulator stubbed** in `universaldb/src/atomic.rs:20-23`. Tests validating versionstamp ordering silently pass with wrong values on RocksDB / Postgres backends.

### Cleanup-driven changes

5. **Snapshot → Serializable conversions** in load-bearing read paths:
   - Cold Phase A handoff reads (currently Snapshot, in `phase_a.rs:308, 314, 322, 512`)
   - Eviction's per-branch plan reads (currently Snapshot, in `compactor/eviction/mod.rs:269, 563, 595`)
   - GC's pin scans (currently Snapshot, in `gc/mod.rs:118, 128, 138, 174`)
6. **Restructure eviction** to a single FDB tx per branch. Today it's plan-then-clear in two txs; the synthesis recommends merging so FDB serializability replaces the bespoke OCC fence on `last_hot_pass_txid`. Avoids the 10 MB write-set blow-up the FDB review found in the original proposal.
7. **Drop hot lease** (`META/compactor_lease`), renewal task, and cancel-token machinery.
8. **Enriched `pending/{uuid}.marker`** with `pod_id`, `pass_started_at_ms`, `last_phase` so operators have a forensic trail when leases are absent.
9. **`pin_count` cap is already correct** — `bookmark.rs:194` does Serializable read + atomic-add. Original proposal mislabeled the protection mechanism. Document the read-then-write-for-caps pattern in CLAUDE.md.

## Operational chapter (required additions)

- **Metrics:**
  - `eviction_tx_aborts_total`
  - `cold_pass_duplicate_total`
  - `pegboard_exclusivity_violations_total` (catches `/META/head` writes from a non-current `runner_id`)
  - `cold_pass_active_pods` (cardinality-safe at the cluster level, not per-DB)
  - `stale_marker_age_ms` histogram
- **"Compaction stuck" runbook** — use the enriched `pending/{uuid}.marker` as the forensic trail. Lease keys are gone; pod_id + last_phase + pass_started_at_ms tell you who's doing what.
- **Eviction backoff protocol** — explicit exponential up to a max delay, give-up after N retries, alert on backlog growth.
- **Fork retry budget and customer SLA** — bounded retries before surfacing `transaction_too_old` to the caller.
- **Two-phase rollout for revert safety** — code can run with leases or without, switched by config; safe to roll back per-pod if production breaks.
- **Cardinality-safe metric labeling** at 100k+ tenants (cluster-level aggregations, no per-DB labels).

## Open questions deferred

1. Eviction lease TTL vs sweep duration — needs measurement.
2. `MAX_PHASE_B_MS` sizing for large databases — empirical.
3. NATS JetStream redelivery configuration tuning.
4. Hot fold duplicate rate empirical bound — verify the "FDB serializability handles it cheaply" claim under real load.

## Next-step options

### A. Another adversarial review

Run a second-pass review on the new design (`sqlite-concurrency-design.md`) specifically targeting the issues the first round flagged. Verify the fixes actually hold.

### B. Proceed

Update the spec (`sqlite-rough-pitr.md`) to incorporate the design and add Ralph stories for the code changes.

### C. Sit on it

Wait a day, re-read fresh.

## If we go with B — natural story breakdown

Order matters: pre-existing fixes land first (independent value), then the cleanup-specific work, then the spec finalization.

| Order | Story | Type |
|---|---|---|
| 1 | SHARD chunking to fit FDB 100 KB value limit | Pre-existing fix |
| 2 | Hot fold reads `desc_pin` Serializable before DELTA deletion | Pre-existing fix |
| 3 | `bk_pin` recompute uses `byte_min` instead of `set` | Pre-existing fix |
| 4 | Versionstamp simulator implementation in `universaldb` | Pre-existing fix |
| 5 | Snapshot → Serializable conversions (cold Phase A, eviction, GC pin scans) | Cleanup |
| 6 | Restructure eviction to single FDB tx per branch | Cleanup |
| 7 | Drop hot lease + renewal + cancel | Cleanup |
| 8 | Enriched `pending/{uuid}.marker` + forensic metrics | Cleanup |
| 9 | Operational chapter additions to spec + runbook docs | Cleanup |
| 10 | v5 spec revision incorporating all of the above | Cleanup |

## Recommendation

Probably B with story #1–#4 prioritized first since they're pre-existing bugs that need fixing regardless of the cleanup. #5–#10 are the actual concurrency simplification.

Worth running A first if there's any doubt about the synthesis design — the third reviewer's findings (Snapshot vs Serializable, write-set limits) caught real issues the second round of analysis surfaced. A second adversarial pass on the new design is cheap insurance.
