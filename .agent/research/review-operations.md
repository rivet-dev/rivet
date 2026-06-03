# Operations review: sqlite-pitr-fork.md

Adversarial focus: production failure modes, recovery, debugging, migrations.
Cross-references `sqlite-pitr-fork.md` (PF) and `sqlite-storage-stateless.md` (SS).

## Recovery / failure scenarios

### S3 down for 1 hour
PF "Failure modes" table (line 572-583) handles per-PUT failure but not "S3
unreachable for the full pass duration." The cascade is concrete:

- Cold compactor cannot drain. `cold_drained_txid` does not advance.
- Hot compactor keeps folding DELTA into SHARD as normal, but `META/cold_compact`
  lags. Hot compactor's hot-tier GC of `DELTA/{T}` is gated on
  `T <= cold_drained_txid` (Pass step 8, line 432) — DELTA blobs cannot be
  reclaimed.
- `META/quota` keeps climbing. Hot quota cap = 10 GiB (line 524). For a
  busy actor at 10 MB/s, the hot quota fills in ~17 minutes.
- After the cap, all commits return `SqliteStorageQuotaExceeded` (SS line 152).
  The actor is effectively read-only on its own working set even though cold
  capacity is unbounded.

**Fix.** Distinguish "drain to cold" from "free hot tier" as separate
budgets. Add `hot_drain_only_mode` triggered when cold pass failures > N or
S3 5xx ratio > threshold:
- Hot compactor still folds; hot quota's cap raises by a tunable `hot_burst_multiplier`
  (e.g. 2x) for the duration of cold-tier degradation.
- Emit `sqlite_cold_tier_degraded{actor_id}` gauge so ops sees it.
- After recovery, cold compactor's first pass drains the backlog and the cap
  reverts. Document the burst-mode quota in user-facing limits.
- Alternatively, allow the cold compactor to dump DELTA blobs into a *secondary
  S3 bucket / region* on outage, with manifest reconciliation on recovery.

### S3 slow (p99 = 5s)
PF cold compactor pass procedure (line 414-434) opens a UDB read range then
performs S3 PUTs in steps 4-7, then commits a single FDB tx in step 8.
- FDB tx age limit = 5s (SS line 287, line 391).
- If S3 PUTs serialize in step 4-7 and each takes p99=5s, a multi-shard pass
  blows past tx age. The lease renewal task (SS line 411-419) keeps the lease
  alive but the *underlying transaction* the cold pass is computing against
  is gone.
- Lease margin = 5s (PF line 411 implies same as SS). At p99=5s, every renewal
  is flirting with failure.

**Fix.** Make the pass structure tx-bounded:
1. Phase A (FDB only, <5s): read drain window, snapshot DELTA bytes into
   process memory. Tx commits cleanly.
2. Phase B (S3 only, no FDB tx): upload layer files. No FDB tx age clock
   running. Renewal still ticks.
3. Phase C (FDB only, <5s): single tx writes `META/cold_compact` + clears
   DELTA range + writes manifest pointer.
PF's step 8 conflates "drain DELTAs from FDB" with "advance cold cursor"; they
should be in separate FDB txs separated by a synchronous-PUT-confirmation
barrier.

### Compactor pod loses lease mid-pass (orphan layers)
PF Pass procedure step 6 (line 428) writes the cold manifest *after* uploading
layer files in step 4-5. Failure mode:
- Pod uploads `layers/delta/0000-...-...ltx` to S3.
- Pod loses lease before step 6.
- New pod takes lease, reads `META/cold_compact.cold_drained_txid` — unchanged.
  Re-drains the same window. Writes a new layer at *the same key* (same
  min_txid, max_txid, but maybe different checksum if drain re-read at a
  different `materialized_txid`).
- The first pod's orphan is overwritten if checksum matches, or sits as
  unreferenced garbage if checksum differs (filename includes checksum,
  per line 286).

PF "Failure modes" table line 575 says "rerun from drained_txid is
idempotent" but that is only true if the layer filename is *deterministic*
from `(min_txid, max_txid)` content. With checksum in the filename, two passes
can produce divergent filenames if input bytes differ (e.g. compaction read at
different `materialized_txid`).

**Fix options:**
- Drop checksum from filename. Use `delta/{min_txid}-{max_txid}.ltx`.
  Resolves overwrite vs orphan ambiguity. Checksum still lives in
  `LayerEntry.checksum` for read verification.
- Add an explicit orphan sweep: at start of each pass, list
  `branches/{branch_id}/layers/` and remove any object not referenced by the
  current `ColdManifest`.
- Adopt LiteFS's HWM pattern: write a small `pending/` marker before the
  actual layer; on recovery, sweep `pending/` markers older than N minutes.

### S3 partial-write (multipart PUT race)
PF line 4-5 of pass step 4: "single-PUT for layers <= 16 MiB; multipart for
larger." Multipart PUT is not atomic — it succeeds only when CompleteMultipartUpload
runs. If the pod dies after one chunk but before CompleteMultipartUpload, S3
holds an in-progress multipart upload that bills storage and never completes.

PF spec doesn't mention CompleteMultipartUpload abort. AWS recommends an
S3 lifecycle rule to abort incomplete multipart uploads after N days; the spec
should require this rule on the bucket and document it.

**Fix.**
- Document an S3 lifecycle policy (`AbortIncompleteMultipartUpload` after 1d)
  as a deployment requirement.
- Add `sqlite_cold_compactor_multipart_uploads_in_flight` gauge.
- Verify completion explicitly: after CompleteMultipartUpload, HEAD the object
  and check ETag/size before writing the manifest.

### FDB commits manifest update but S3 PUT raced (orphan layer)
PF doesn't specify the order between "write S3 layer" and "rewrite cold
manifest" beyond pass step 6 (manifest after layer). Two failure modes:
- **S3 PUT first, then FDB / manifest:** layer uploads but manifest rewrite
  fails. Orphan layer sits in S3 unreferenced. Cold bytes accounting drifts
  upward forever.
- **FDB first, then S3:** spec doesn't do this, but if it did, manifest would
  reference a non-existent S3 object — read-time `NoSuchKey` errors that are
  hard to diagnose.

**Fix.** Two-phase commit with reconciliation:
1. Write S3 layer files at terminal keys.
2. Write `ColdManifest.v1.vbare` (S3 single-PUT) referencing them.
3. Update `META/cold_compact.cold_drained_txid` in FDB.
- The invariant becomes "cold_drained_txid in FDB is the high-water mark of
  the manifest; if FDB has a value but the manifest does not yet contain
  layers up to that txid, panic."
- Add a startup reconciliation that lists `branches/{branch_id}/layers/`,
  diffs against manifest, and either deletes or re-references orphans.
  Run this at the start of every cold pass after lease acquisition.

## Atomicity / idempotency holes

- **`COMMITS/{T}` GC vs PITR.** PF line 431: "delete COMMITS/{T} for T <
  retention_pin_txid." But the `BookmarkIndex` (line 340) is the *cold*
  successor for that data, written by the *same* pass. If cold pass writes
  `BookmarkIndex` at line 7 then crashes before line 8 (FDB updates), the
  next pass re-runs, bookmark index re-rewritten — that's fine. But if it
  crashes *after* clearing `COMMITS/{T}` but *before* finishing the
  `BookmarkIndex` write, lookup of bookmarks in the deleted hot-tier window
  has no source. Spec assumes both happen in step 8's FDB tx, but the
  `BookmarkIndex` write is to S3 (step 7).
  **Fix.** Strict ordering: write `BookmarkIndex` to S3, HEAD-verify it, *then*
  clear `COMMITS/{T}` in FDB tx. Make this an explicit invariant in the
  Pass procedure.

- **Refcount race vs GC pin.** PF line 559 mentions `GC_FORK_MARGIN_TXIDS`
  (default 1024 txids = ~1 minute) but doesn't enforce that `fork()`
  *fail-fast* if `parent_txid < head_txid - GC_FORK_MARGIN_TXIDS`. If the fork
  RPC takes longer than the margin to land the refcount bump, GC could
  delete a layer between fork's read of `parent_txid` and its refcount-add.
  **Fix.** `fork(ForkPoint::Bookmark)` must validate `parent_txid >= retention_pin_txid + safety_margin` *inside* the tx that bumps refcount, not on the
  client side. Otherwise, fail with `ForkOutOfRetention`.

- **`cold_bytes` is best-effort.** PF line 332-336 stores `cold_bytes` in S3
  (`BranchColdState`), updated only by cold compactor passes. If a GC delete
  partially fails (some objects remain) the spec says step "log warning,
  leave the layer" (line 577). `cold_bytes` is then *under*-reported relative
  to actual S3 spend. Billing under-reads; ops can't detect divergence
  without a separate prefix-listing reconciliation.
  **Fix.** Periodic (daily) reconciliation job that lists
  `branches/{branch_id}/layers/`, sums `Content-Length`, and overwrites
  `cold_bytes` with ground truth. Emit `sqlite_cold_bytes_reconciliation_drift`.

## Observability gaps

PF "Metrics" sections only mention adding `SqliteColdBytes` and
`SqliteBranchCount` MetricKey variants (line 530-531). The spec lacks every
operational metric needed to triage the failure modes above:

- `sqlite_cold_compactor_pass_duration_seconds{outcome}` — needed to detect
  S3 slowness.
- `sqlite_cold_compactor_pass_failures_total{stage=upload|manifest|fdb_commit|gc}` —
  needed to localize which stage failed.
- `sqlite_cold_drain_lag_txids{actor_id_bucket}` — `head_txid - cold_drained_txid`.
  The ops-critical metric. Without it, "cold compactor falling behind" is
  undetectable.
- `sqlite_cold_drain_lag_seconds{actor_id_bucket}` — wall-clock equivalent
  via `BookmarkIndex` lookup.
- `sqlite_branch_count_per_actor` (histogram) — needed to detect runaway
  branching (programmatic-fork loops).
- `sqlite_branch_gc_eligible_count{state=Deleted}` — branches awaiting hot
  prefix delete + cold prefix delete.
- `sqlite_s3_request_duration_seconds{op,outcome}` — base S3 latency.
- `sqlite_s3_inflight_multipart_uploads` — incomplete-multipart leakage.
- `sqlite_cold_layer_orphan_count{branch_id}` — listed in S3 but not in
  manifest. Reconciliation pass populates.
- `sqlite_pitr_resolve_duration_seconds{path=hot|cold}` — `get_bookmark_for_time`
  latency by source.
- `sqlite_bookmark_resolve_failures_total{reason=expired|branch_unreachable|gap}`.
- `sqlite_cold_compactor_lease_steals_total{branch_id}` — pod churn signal.
- `sqlite_refcount_drift_total` — debug-only invariant.

**Debug pathway for "PITR returned wrong data."** The spec offers nothing.
Bookmark format (line 79) embeds `(timestamp_ms, txid, branch_id)`. To trace:

1. Decode bookmark → `(ts, txid, branch_id)`.
2. `resolve_bookmark` returns `Position { branch_id, txid, checksum }`.
3. Log this `Position.checksum`.
4. Compare against `COMMITS/{txid}.checksum` (hot) and the matching layer's
   `post_apply_checksum` (cold).

Spec missing:
- A `debug_describe_bookmark(b)` operator-only API that returns the resolution
  trail: branch chain walked, checksum at each step, layer files used.
- Structured logs at PITR resolution time including `bookmark, resolved_branch_id,
  resolved_txid, resolved_checksum, source_layer_keys`. Without this, the
  "wrong data" complaint is unprovable.

**Branch enumeration.** `list_branches() -> Vec<Branch>` (line 493) is
unbounded. An actor with 1M forks blows the FDB tx. Spec doesn't address.
**Fix.** Replace with `list_branches(cursor, limit) -> (Vec<Branch>, NextCursor)`.
Add `count_branches() -> u64` via an atomic counter on
`[BRANCHES]/list/count` for cheap quota checks. Add per-actor branch-count
cap (e.g. 1024) enforced at `fork()`.

## Disaster scenarios

### S3 IS the backup
PF line 8 says "S3 is the cold tier; FoundationDB is the hot tier" and
implies cold-tier bytes contain the historical record. PF goal 1 says "Any
committed transaction in the last 30 days is recoverable." But the *most
recent* `(head_txid - cold_drained_txid)` window of commits lives ONLY in
FDB. If FDB loses a backup window's worth of data (replica failure during
maintenance, unprovable but real for any DB), the hot DELTA between
`cold_drained_txid` and `head_txid` is unrecoverable.
- 1-hour cold pass cadence (line 10) means the recovery point objective for
  S3 disaster scenarios is **up to 1 hour of writes lost**. Spec doesn't
  state this RPO. CF DO ships every 10s/16MiB (per cf-durable-objects-sqlite.md
  line 184).

**Fix.**
- Document RPO explicitly: "in-flight commits between cold passes are
  FDB-durable but not S3-durable; cold-pass cadence is the RPO."
- Optional: add a *fast cold drain* mode that writes raw per-commit DELTAs to
  S3 inline with commit (CF-DO style 10s/16MiB), separate from the L0+
  compaction. Trade off cost-per-commit vs RPO.
- Multi-region cold tier (PF "Future work" line 657) becomes a hard
  requirement for any branch tagged "production" or above a tier threshold.

### `cold_drained_txid = 0` zeroed by bug
PF line 333: `cold_drained_txid: u64`. If a code path zeroes this, the next
pass re-drains the entire history.
- Re-drain is "idempotent" only at the (min_txid, max_txid) layer-key level.
  Filenames include checksums (line 286), so re-derivation should match if
  inputs are the same. But if `materialized_txid` advanced (DELTAs were
  GC'd), the input set is *smaller*, and the re-derived layer file has a
  different checksum → different filename → orphan duplicates.
- Quota counter (`cold_bytes`) double-counts after re-drain unless GC
  catches up.

**Fix.**
- Make `cold_drained_txid` monotonic at the FDB-write level: use atomic-MAX
  or fail the write if `new < current`. UDB doesn't natively expose MAX
  but can with a CAS loop. Even simpler: make
  `META/cold_compact` write conditional on the *prior* `cold_drained_txid`
  being read in the same tx (regular read takes conflict range; OCC aborts
  any racer).
- Periodic invariant check: compare `cold_drained_txid` against the
  manifest's `max(layer.max_txid)`; if they diverge by more than expected,
  panic in debug, alert in prod.

### Actor delete cleanliness
PF line 463-468: "schedule cold-tier delete for next pass:
`actors/{actor_id}/branches/{branch_id}/*` S3 prefix delete." This is
completely vague.
- For an actor with 100 branches and 720 L3 layers each, that's 72,000+
  S3 objects.
- Sync delete on the destroy path blocks pegboard's actor-destroy tx.
- Async delete via cold compactor: spec doesn't define a "tombstone" mechanism
  to mark which actors await deletion.
- Race: if pegboard destroys actor A and actor A's id is reused (it isn't,
  by UUID convention), but the prefix is still being cleaned, a new actor
  collides with stale layers. Acceptable under UUID, document.

**Fix.**
- Add `[BRANCHES]/list/{branch_id}.state = Tombstoned` separate from
  `Deleted`. Tombstoned branches are visible to GC but not to read paths.
- Cold compactor walks tombstones at top of each pass, batch-deletes S3
  objects (~1k per delete request via `DeleteObjects`), updates state on
  completion.
- Make actor-delete a workflow-level operation (gasoline workflow with
  explicit progress) so it survives pod restarts. Pegboard's destroy
  tx just inserts a tombstone — fast, atomic, contained.

### 30-day window strictness
PF line 451 says GC runs hourly; layers expire 30 days old. PF section 9
acknowledges the up-to-1-hour slip but doesn't document it. CF DO uses
"marked for deletion 30 days later" (cf-durable-objects-sqlite.md prior art)
which is similarly soft.

**Fix.** Document the SLA: "PITR window is 30 days plus up to one cold-pass
cadence (default 1 hour). Bookmarks may be resolvable for up to 30d 1h
after their commit timestamp." Surface this in user-facing docs.

## Schema/migration planning gaps

- `ColdManifest.schema_version: u32 = 1` (line 305). Spec doesn't define
  *anything* about how v2 lands. Concrete questions:
  - Does the cold compactor read v1 + write v2 once on the next pass after
    a code roll? Atomic rewrite of an entire actor's manifest works for small
    manifests but a 720-layer manifest is ~50 KB; rewrite cost is fine.
  - Backwards-read: does the v2 reader keep v1 read code forever, or only
    until the next pass rewrites? Spec must say.
  - Schema migration of `BookmarkIndex` (no version field at all — line 343).
    Add `schema_version: u32 = 1`.
  - `BranchColdState` (line 327) — same gap.

**Fix.**
- Add `schema_version` to every persisted S3 file.
- Establish a migration rule: "cold compactor reads v_old, writes v_new on
  every pass. v_old reader code must remain in tree for at least 1 retention
  window (30 days) past v_new rollout." Document in CLAUDE.md.
- Use `vbare` versioned schemas per CLAUDE.md line "Always use versioned BARE."

## Testing / failure-injection gaps

PF stage 7 (line 638) lists three integration tests but no S3 fault model.

**Required additions:**
- S3 mock with deterministic injection: `fail_on_op(PutObject, n)`,
  `slow_on_op(PutObject, ms)`, `429_on_op(PutObject, n)`,
  `partial_write(PutObject, after_bytes)`.
- A "S3 unreachable for 1h" test using `tokio::time::pause` + injected
  errors; assert hot quota burst-mode kicks in (when implemented).
- A "lease lost mid-pass" test injecting a renewal failure between layer
  upload and manifest write; assert the next pass detects and reconciles
  orphans.
- A "manifest write fails after layer upload" test; assert orphan sweep on
  next pass.
- A "FDB tx age 5s + S3 p99 5s" test; assert pass structure (Phase A/B/C
  above) does not blow tx age.

## Sandbox / dev environment

PF "Open questions" line 651: "do we want a no-S3 mode? Probably yes; gate
behind a config flag."

**Recommendation:** ship a `cold_tier: ColdTier` enum:
- `ColdTier::Disabled` — cold compactor service is registered but always
  no-ops. PITR returns `BookmarkExpired` for any txid older than what the
  hot tier can serve. Hot quota cap remains 10 GiB. Forks work, GC runs on
  hot tier only. This is the dev-machine default.
- `ColdTier::Filesystem(PathBuf)` — local filesystem pretending to be S3.
  Useful for integration tests and self-hosted deploys without an S3 backend.
  Implements the same `ColdTierClient` trait as the AWS implementation.
- `ColdTier::S3(S3Config)` — production.

Justifications:
- `Disabled` lets `pnpm start`-style local dev avoid spinning up minio.
- `Filesystem` is a 200-loc impl over `tokio::fs` that gets fault injection
  for free (use the existing engine `MemoryStore` pattern).
- The `ColdTierClient` trait abstracts every spec failure mode behind a
  testable seam.

## Stuff that's actually robust

- **Lease decoupling.** Hot and cold compactors holding distinct leases
  (line 411) on disjoint META sub-keys (`META/compact` vs
  `META/cold_compact`) is the right call. The argument in line 540-549 is
  airtight: tx-disjoint windows, atomic-add for refcount, no shared write
  conflict ranges.
- **Refcount + descendant pin.** The dependency-graph GC model (line 9, 451)
  is borrowed cleanly from Neon and avoids the entire "delete a layer that's
  still referenced" class of bug, modulo the `GC_FORK_MARGIN_TXIDS` gap
  noted above.
- **Bookmark format.** Lex-sort = chronological order is a correct primitive,
  and it interops with CF-DO-shape semantics if interop ever matters
  (line 88).

