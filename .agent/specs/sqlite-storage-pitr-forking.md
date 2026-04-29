# SQLite v2 Storage: Point-in-Time Recovery + Forking

This spec extends `.agent/specs/sqlite-storage-stateless.md`. Read that first. The new design here adds two operator-facing features: point-in-time recovery (PITR) and actor forking. Both are layered on top of the stateless storage + standalone compactor described in the base spec.

> **PITR is logical recovery, not infrastructure DR.** This system protects against logical errors (bad commit, accidental delete, corrupt application state) by allowing rollback within a configurable retention window. It is NOT a backup against FoundationDB cluster loss, multi-region failure, or hardware corruption. If FDB itself loses data, all checkpoints are lost too. External backups + object-store tiering (Open Questions) are the eventual DR story.

## Goals

1. **Point-in-time recovery.** Restore an actor's SQLite state to any committed `txid` within a configurable retention window. Granularity = single commit (within retention) or single checkpoint (older).
2. **Forking.** Create a new actor whose initial SQLite state is a copy of an existing actor's state at a specified `txid`.
3. **Bounded storage.** Retention overhead is predictable and configurable, both per-actor and per-namespace.
4. **No hot-path overhead.** `get_pages` and `commit` latency must not change. PITR/fork machinery lives in the compactor and admin pipelines, not on the actor path. The single exception is the restore-in-progress commit guard (see "Concurrency model").
5. **Off by default, opt-in per namespace.** Default `retention_ms = 0` and `allow_pitr = false`. Consumers who do not need PITR pay zero storage overhead.
6. **User-atomic admin ops.** From the user's perspective, `actor.restore(...)` is a single API call. Suspend/resume orchestration is internal.
7. **Survives pod failures.** Long-running ops (restore, fork) are idempotent and resumable across compactor pod failures via persisted operation state.

## Non-goals

- Cross-actor consistent snapshot bundles (multi-actor coordinated point-in-time). Each PITR/fork is per-actor.
- Read-only "time travel" — mounting an actor's SQLite at a prior txid for queries without modifying the head. v1 supports only destructive restore + fork-into-new-actor; read-only mount is future work.
- Continuous external backup to object stores. Out of scope (see Open Questions).
- Cross-region replication. Out of scope.
- Automatic incident-driven rollback. Operator-triggered only.
- Changing the `commit` / `get_pages` wire shape (the base-spec hot path). PITR/fork operations live on a *separate* admin protocol.

## How the stateless base spec breaks PITR

The base-spec compactor folds DELTAs into SHARDs and **deletes the DELTA blobs**. Once compacted, per-commit history is gone — there's no way to reconstruct page state at any prior `txid`.

To support PITR + fork without abandoning the stateless design, this spec adds two on-disk constructs and one hot-path guard:

1. **Checkpoints** — frozen full-state snapshots at specific `txid`s, in their own keyspace. Created periodically by the compactor.
2. **Retention-aware DELTA cleanup** — DELTAs are only deleted by compaction once they are both (a) covered by a newer checkpoint and (b) older than the retention window. Within the window, DELTAs are preserved untouched, giving per-commit restore granularity.
3. **Commit guard against in-flight restore** — commits check whether a restore is in progress and bail. This is the only hot-path overhead introduced (one optional cached read per first commit).

PITR restore re-applies preserved DELTAs against the most recent checkpoint ≤ target. Fork copies the same checkpoint + replays DELTAs into a fresh actor's keyspace.

## Data structures

New per-actor key prefixes (under existing `[0x02][actor_id]`):

```
/CHECKPOINT/{ckp_txid: u64 BE}/META            — vbare blob: { taken_at_ms, head_txid, db_size_pages, byte_count, refcount: u32, pinned_reason: optional<string> }
/CHECKPOINT/{ckp_txid: u64 BE}/SHARD/{shard_id: u32 BE}     — frozen SHARD blob (full copy)
/CHECKPOINT/{ckp_txid: u64 BE}/PIDX/delta/{pgno: u32 BE}    — frozen PIDX entry (only if PIDX still pointed to a DELTA at checkpoint time)
/META/retention                                — vbare blob: RetentionConfig
/META/checkpoints                              — vbare blob: ordered list of { ckp_txid, taken_at_ms, byte_count, refcount }
/DELTA/{txid: u64 BE}/META                     — vbare blob: { taken_at_ms, byte_count, refcount: u32 }
/META/restore_in_progress                      — vbare blob: RestoreMarker (absent when no restore active)
/META/fork_in_progress                         — vbare blob: ForkMarker (absent when no fork active; on the dst actor's prefix)
/META/admin_op/{operation_id: Uuid}            — vbare blob: AdminOpRecord (lifecycle + progress for an in-flight or recently-completed op; TTL-cleaned)
/META/storage_used_live                        — atomic i64 LE counter (live data only)
/META/storage_used_pitr                        — atomic i64 LE counter (PITR overhead: checkpoints + retained DELTAs)
```

`/META/quota` from the base spec **splits** into `/META/storage_used_live` and `/META/storage_used_pitr`. Total bytes still equals `live + pitr`. The split is required so commits can enforce only the live cap (predictable user-visible quota) while PITR overhead is governed by a separate per-namespace budget. Migration: on first read after the split, sum existing `/META/quota` into `/META/storage_used_live` and zero `/META/storage_used_pitr`.

`/DELTA/{T}/META` adds a `refcount` field beyond the original spec's `taken_at_ms` + `byte_count`. The refcount pins individual DELTAs while a fork is replaying them (fixes correctness issue C3 from review).

`RetentionConfig` (vbare):

```rust
pub struct RetentionConfig {
    pub retention_ms: u64,             // 0 = PITR disabled; default 0
    pub checkpoint_interval_ms: u64,   // default 3_600_000 (1h)
    pub max_checkpoints: u32,          // default 25 (24h retention + 1 safety)
}
```

`/META/checkpoints` stays under the 16 KiB FDB single-value chunk threshold at default retention. Refcount is mirrored here (also stored authoritative on `/CHECKPOINT/{T}/META.refcount`) so a single read of `/META/checkpoints` powers `DescribeRetention` without scanning every checkpoint. Updated atomically with refcount changes (see "Refcount semantics" below).

`AdminOpRecord` (vbare):

```rust
pub struct AdminOpRecord {
    pub operation_id: Uuid,
    pub op_kind: OpKind,                  // Restore | Fork | Other
    pub actor_id: String,                 // src for fork; subject for restore
    pub created_at_ms: i64,
    pub last_progress_at_ms: i64,
    pub status: OpStatus,                 // Pending | InProgress | Completed | Failed | Orphaned
    pub holder_id: Option<NodeId>,        // pod currently working it; None when terminal
    pub progress: Option<OpProgress>,
    pub result: Option<OpResult>,
    pub audit: AuditFields,               // caller_id, request_origin_ts_ms, namespace_id
}

pub struct OpProgress {
    pub step: String,                     // human-readable
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub started_at_ms: i64,
    pub eta_ms: Option<i64>,
    pub current_tx_index: u32,
    pub total_tx_count: u32,
}
```

`AdminOpRecord` lives in UDB (NOT in-memory on a pod) so an HTTP poll endpoint always has the source of truth, even after the working pod dies. TTL: 24h after a terminal status. Cleanup is folded into the compactor's existing per-actor pass.

## Checkpoint creation (compactor responsibility)

Triggered from a regular compaction pass when `now - latest_checkpoint.taken_at_ms >= checkpoint_interval_ms`, OR no checkpoint exists yet AND retention is enabled.

**Critical sequencing fix (review M3):** the txid the checkpoint is labeled with is `head_txid_observed_at_plan_phase`, not the live head at write-phase time. A new commit between plan and write phases must NOT shift the checkpoint's claimed point.

```
Compaction pass with checkpoint:
1. Plan phase (snapshot reads):
   - Read /META/head — capture ckp_txid_candidate = head.head_txid (use this exact value for the rest of the pass).
   - Read /META/compact, /META/checkpoints, /META/retention.
   - Identify deltas to fold; classify each as fold-only or fold-and-may-delete (retention math).
2. Write phase (regular reads in a fresh tx, lease-protected):
   a. Fold deltas into SHARDs (existing base-spec behavior).
   b. COMPARE_AND_CLEAR PIDX entries for folded pages.
   c. Update /META/compact.materialized_txid.
   d. atomic_add /META/storage_used_live (-bytes_freed_live).
   e. If checkpoint_due AND quota check passes (see "Quota accounting"):
      - Multi-tx phase under the existing lease (separate sequenced txs):
        * For each /SHARD/{id}: read, write to /CHECKPOINT/{ckp_txid_candidate}/SHARD/{id}.
        * For each /PIDX/delta/{pgno} present: write to /CHECKPOINT/{ckp_txid_candidate}/PIDX/delta/{pgno}.
      - Final tx (atomic): write /CHECKPOINT/{ckp_txid_candidate}/META, update /META/checkpoints, atomic_add /META/storage_used_pitr (+checkpoint_bytes).
3. Old-checkpoint cleanup: any /CHECKPOINT/{T} where T < (now - retention_ms_in_txid_terms) AND refcount == 0 AND T is not the latest checkpoint → delete (multi-tx); atomic_add /META/storage_used_pitr (-bytes).
```

**Quota check at checkpoint creation** (review #8): if `(storage_used_live + storage_used_pitr + estimated_checkpoint_bytes) > namespace.pitr_max_bytes_per_actor`, skip the checkpoint and increment `sqlite_checkpoint_skipped_quota_total{actor_namespace}`. Operator gets an alert; fix is to lower retention or raise the namespace budget.

`max_concurrent_checkpoints` is a separate `CompactorConfig` knob (default 16, lower than `max_concurrent_workers = 64`) since checkpoints are 10-100× heavier than regular compactions.

### Retention-aware DELTA cleanup (replaces base spec's "delete folded DELTAs")

```
DELTA T may be deleted iff:
    T <= latest_checkpoint.txid
    AND
    DELTA[T].taken_at_ms < (now - retention_ms)
    AND
    DELTA[T].refcount == 0
```

The refcount clause (review C3) ensures an in-flight fork that needs to replay a delta keeps it alive even past retention.

If `retention_ms == 0` the time clause collapses to "always", behavior matches the base spec exactly.

The compactor reads `/DELTA/{T}/META` during plan phase to evaluate retention.

### Refcount semantics (review C3, M2, M4)

Two separate refcounts:
- **Checkpoint refcount** at `/CHECKPOINT/{T}/META.refcount` and mirrored in `/META/checkpoints[i].refcount`.
- **Delta refcount** at `/DELTA/{T}/META.refcount`.

Both are pinned by an in-flight fork. They are released by the fork-completion path (success or aborted-with-cleanup).

Mandatory tx sequencing for refcount mutations:

1. Refcount increment is in its own committed tx, before the lease that protected the read of the candidate ckp/deltas is released. Sequence: `Tx A: atomic_add(+1) on every pinned key; commit. Tx B: release lease; commit.`
2. Refcount decrement is in its own committed tx after work using the pinned object completes.
3. Decrement-on-abort uses the same separate-tx pattern; never combined with read-then-conditional-decrement in one tx (atomic-add visibility within a tx is undefined in this codebase's UDB).

Auto-recovery for leaked refcounts (review #3): the compactor scans `/CHECKPOINT/*/META.refcount` and `/DELTA/*/META.refcount` once per pass. Any refcount > 0 with no live `/META/admin_op/{id}` referencing the actor is a leak. After `lease_ttl_ms × 10` of staying leaked, the compactor logs `sqlite_checkpoint_refcount_leak_total` AND **resets to 0**. The first-class admin op `ClearRefcount(actor_id, ckp_txid)` (or `(actor_id, delta_txid)`) is exposed for manual operator recovery.

## Restore procedure

User-facing flow (atomic):

```
api-public POST /actors/{id}/sqlite/restore { target: RestoreTarget, mode: RestoreMode } →
{ operation_id: Uuid, status: "pending" }
```

User polls `GET /actors/{id}/sqlite/operations/{operation_id}` (or subscribes via SSE). Op state is persisted in UDB so polling works regardless of which pod is handling the work.

Internally the api-public handler:
1. Authorizes the caller (see "Authorization chain").
2. Allocates `operation_id`; writes `AdminOpRecord{ status: Pending }` at `/META/admin_op/{id}`.
3. Calls `pegboard.suspend(actor_id, reason="sqlite_restore", op_id)` and waits for confirmation. Pegboard sends "going away" to all envoys; envoys close client WSes with code `1012` reason `actor.restore_in_progress`.
4. Publishes `SqliteOpSubject::Restore` to UPS.
5. Returns to caller with operation_id; HTTP request does NOT block on the op.

The compactor (one of the queue group):

```
1. Take /META/compactor_lease for actor_id.
2. atomic update /META/admin_op/{id}.status = InProgress; record holder_id.
3. Read /META/head; /META/checkpoints; /META/retention.
4. Resolve RestoreTarget:
   - Txid(t)             → t
   - TimestampMs(ts)     → max{ T | DELTA[T].taken_at_ms <= ts AND T reachable } OR latest checkpoint <= ts
   - LatestCheckpoint    → max{ ckp.txid }
   - CheckpointTxid(t)   → t (validated as exact ckp)
5. Validate target_txid:
   - target_txid <= head.head_txid
   - reachable: matches some checkpoint ckp_txid OR every DELTA in (ckp.txid, target_txid] still exists
   - If unreachable → AdminOpRecord.result = Failed{InvalidRestorePoint, reachable_hints}; release lease; return.
6. If mode == DryRun: AdminOpRecord.result = Ok{DryRunRestore{ckp_used, deltas_to_replay, estimated_bytes}}; release lease; return.
7. Tx 0 (the SAME tx as the first destructive write):
   - Write /META/restore_in_progress = RestoreMarker { target_txid, ckp_txid, started_at_ms, last_completed_step: Started, holder_id, op_id }.
   - clear_range /SHARD/* and /PIDX/delta/* (Tx 0 IS Tx 1 from the prior draft — they MUST be the same tx).
8. Tx 1..N: copy /CHECKPOINT/{ckp.txid}/SHARD/* into /SHARD/* (paginate into multiple txs). Update marker.last_completed_step = CheckpointCopied at end.
9. Tx N+1..M: copy /CHECKPOINT/{ckp.txid}/PIDX/delta/* into /PIDX/delta/* (paginate). Update marker.
10. For each delta T in (ckp.txid, target_txid]: replay into /SHARD/* + /PIDX/delta/*, update /META/head { head_txid: T }. Update marker between deltas. Marker.last_completed_step = DeltasReplayed when loop completes.
11. Tx: clear DELTAs in (target_txid, head_old.head_txid] (destructive — txids past target_txid are erased).
12. Tx: recompute /META/storage_used_live by scanning current state; compute delta = recomputed - currently_observed; atomic_add /META/storage_used_live (delta). Same for storage_used_pitr if any cleanup happened. (review M1: atomic_add(delta) composes safely; replaces atomic_set semantics.)
13. Final tx: clear /META/restore_in_progress (op complete); update /META/admin_op/{id}.status = Completed; record result.
14. Release lease.
15. api-public watches /META/admin_op/{id} status → Completed. Calls pegboard.resume(actor_id).
```

If the compactor pod dies mid-restore: the next pod that takes the lease finds `/META/restore_in_progress` exists, reads the marker, resumes from `last_completed_step`. Each step's tx is idempotent. The marker ALSO carries `ckp_txid`, so resumer re-pins the checkpoint refcount before resuming (review m3).

If the user-facing API call times out before completion, the operation continues. The user re-polls via `operation_id`.

### Commit guard against in-flight restore (review C2)

Pegboard suspension is necessary but not sufficient. A residual commit can land between suspension command and pegboard's confirmation. Storage-layer guard:

```
ActorDb::commit:
  Tx start.
  try_join!(tx.get(/META/head), tx.get(/META/storage_used_live), tx.get(/META/restore_in_progress))
  if /META/restore_in_progress exists:
    return Err(SqliteAdminError::ActorRestoreInProgress)
  ... rest of commit unchanged
```

Hot-path cost: one extra `tx.get` per commit, parallelized via `try_join!` so it adds zero RTT on FDB native and saves the await-between-sends gap on RocksDB. Cached: if the WS conn observes `/META/restore_in_progress` is absent on any commit, it sets `ActorDb.restore_observed_clear = AtomicBool(true)` and skips the read on subsequent commits within the same WS conn lifetime. Restore happens at most once per actor lifetime under normal operation, so the cache saves nearly all reads.

When restore enters Tx 0 it writes `/META/restore_in_progress`. A concurrent commit reading the marker now sees it and bails. The marker write being in the same tx as the first destructive clear (`/SHARD/*` clear) ensures a commit either sees pre-restore state OR sees the marker — never the cleared-but-no-marker intermediate state (review C1).

## Fork procedure

Default fork allocates `dst_actor_id`. Explicit `dst_actor_id` opt-in for "import into pre-allocated id" cases.

User-facing:

```
api-public POST /actors/{id}/sqlite/fork { target: RestoreTarget, mode: ForkMode, dst: ForkDstSpec } →
{ operation_id: Uuid, status: "pending" }
```

Where:

```rust
union ForkDstSpec {
    Allocate { dst_namespace_id: Uuid },                 // default; api-public allocates dst_actor_id
    Existing { dst_actor_id: String },                   // caller pre-created/owns dst
}
union ForkMode { Apply, DryRun }
```

Internal compactor flow:

```
1. Take src's /META/compactor_lease.
2. Read src's /META/head; /META/checkpoints; /META/retention.
3. Resolve target_txid (RestoreTarget — same logic as restore).
4. Validate reachability.
5. If mode == DryRun: result = { ckp_used, deltas_to_replay, estimated_bytes, estimated_duration_ms }; release lease; return.
6. Tx A (separate committed tx, MUST commit before lease release per review M2):
   - atomic_add(+1) on /CHECKPOINT/{ckp.txid}/META.refcount
   - For each delta T in (ckp.txid, target_txid]: atomic_add(+1) on /DELTA/{T}/META.refcount
   - Update /META/checkpoints[i].refcount mirror via atomic write of the list
7. Tx B: release src's /META/compactor_lease.
8. Take dst's /META/compactor_lease.
9. Tx C: validate dst is empty (no /META/head). If exists → Tx C': atomic_add(-1) on src ckp + deltas; release dst lease; return ForkDestinationAlreadyExists.
10. Tx D (same tx as first destructive write to dst):
    - Write dst's /META/fork_in_progress = ForkMarker { src_actor_id, ckp_txid, target_txid, started_at_ms, holder_id, op_id, last_completed_step: Started }
    - Initialize dst's empty /META/head sentinel (so concurrent commits see "in fork" not "uninitialized")
11. Tx E..F: copy /CHECKPOINT/{ckp.txid}/SHARD/* to dst's /SHARD/* (paginate).
12. Tx G..H: copy /CHECKPOINT/{ckp.txid}/PIDX/delta/* to dst's /PIDX/delta/*.
13. For each delta T in (ckp.txid, target_txid]: replay into dst's state.
14. Tx final-1: Set dst's /META/head { head_txid: target_txid, db_size_pages: derived }.
15. Tx final-2: Set dst's /META/storage_used_live = scanned bytes (atomic write — fork is the only writer). Set dst's /META/retention = src's /META/retention (or namespace default).
16. Tx final-3: Clear dst's /META/fork_in_progress; update /META/admin_op/{id} = Completed.
17. Release dst lease.
18. Tx final-4 (separate tx): atomic_add(-1) on src ckp + every pinned delta. (review M2: separate from any prior reads.)
```

Fork failure path: at any error past step 6, the compactor records `Failed` in `/META/admin_op` and runs cleanup: clear dst's partially-written prefix; decrement src's pinned refs. Cleanup itself uses idempotent multi-tx pattern; if cleanup crashes, the next compactor pass detects the leaked refs via auto-recovery.

**ForkMode::DryRun** validates target_txid + estimates cost without taking any locks past step 5.

**Cross-namespace forks**: api-public verifies `src.namespace.allow_fork` AND `dst.namespace.allow_fork` (see Authorization chain). Compactor doesn't re-validate.

## Compaction interaction summary

`compactor::compact_default_batch` from base-spec US-014 extends with:

```rust
async fn compact_default_batch(udb, actor_id, batch_size_deltas, cancel_token) -> Result<CompactionOutcome> {
    let retention = load_retention(udb, actor_id).await?;
    let now = now_ms();

    // Plan phase (existing + new)
    let head_txid_at_plan = read_head(udb, actor_id, snapshot=true).await?.head_txid;
    let candidate_ckp_txid = head_txid_at_plan;     // review M3: locked at plan phase

    plan_phase: { /* identify deltas; classify retention */ }

    write_phase: {
        fold + COMPARE_AND_CLEAR + atomic_add /META/storage_used_live -bytes_freed
        // delete only DELTAs where retention rule allows AND refcount == 0
    }

    if checkpoint_due(latest_ckp, retention, now) AND quota_check_pitr(...).ok() {
        create_checkpoint(udb, actor_id, candidate_ckp_txid, cancel_token).await?;
    }

    cleanup_old_checkpoints(udb, actor_id, retention, now).await?;
    detect_refcount_leaks(udb, actor_id, now).await?;
    cleanup_admin_op_records(udb, actor_id, now - 86_400_000).await?;
}
```

## Wire protocol (admin ops)

UPS subject `SqliteOpSubject` (renamed from `SqliteAdminSubject` per review #12 polish). Subscribed by the compactor service with queue group `"compactor"`.

Wire envelope:

```
struct SqliteOpRequest {
    request_id: Uuid,                 // mirrors AdminOpRecord.operation_id
    op: SqliteOp,
    audit: AuditFields,               // injected by api-public (caller_id, ns_id, request_origin_ts)
}

union SqliteOp {
    Restore { actor_id: String, target: RestoreTarget, mode: RestoreMode },
    Fork { src_actor_id: String, target: RestoreTarget, mode: ForkMode, dst: ForkDstSpec },
    DescribeRetention { actor_id: String },
    SetRetention { actor_id: String, config: RetentionConfig },
    ClearRefcount { actor_id: String, kind: RefcountKind, txid: u64 },
}

union RestoreTarget {
    Txid(u64),
    TimestampMs(i64),
    LatestCheckpoint,
    CheckpointTxid(u64),
}

union RestoreMode { Apply, DryRun }            // review #12: renamed Destructive → Apply
union ForkMode { Apply, DryRun }
union RefcountKind { Checkpoint, Delta }
```

There is **no UPS response subject**. The source of truth for op status is `/META/admin_op/{operation_id}` in UDB. api-public's GET handler reads UDB. UPS is purely the wakeup signal that tells some compactor pod "go work this op." The compactor updates `/META/admin_op/{id}` directly. (review #6: this fixes the "HTTP request hangs on partition" failure mode.)

`DescribeRetention` is synchronous and doesn't need persistence: the compactor reads /META state and writes the response directly into `/META/admin_op/{id}.result` with `status = Completed`, then the caller reads it. Same for `SetRetention` / `ClearRefcount`.

`DescribeRetention` response (review #4):

```rust
struct RetentionView {
    head: HeadView,
    fine_grained_window: Option<FineGrainedWindow>,    // None if no checkpoints yet
    checkpoints: Vec<CheckpointView>,                  // ordered by ckp_txid asc
    retention_config: RetentionConfig,
    storage_used_live_bytes: u64,
    storage_used_pitr_bytes: u64,
    pitr_namespace_budget_bytes: u64,
    pitr_namespace_used_bytes: u64,
}

struct FineGrainedWindow { from_txid, to_txid, from_taken_at_ms, to_taken_at_ms, delta_count, total_bytes }
struct CheckpointView   { ckp_txid, taken_at_ms, byte_count, refcount, pinned_reason }
```

### Errors

All admin errors derive `RivetError` under group `sqlite_admin` (review #5):

```rust
#[derive(RivetError, Debug)]
#[error("sqlite_admin")]
pub enum SqliteAdminError {
    #[error("invalid_restore_point", "the requested target is not within the retention window or has had its DELTAs cleaned up")]
    InvalidRestorePoint { target_txid: u64, reachable_hints: Vec<u64> },

    #[error("fork_destination_exists", "the destination actor already has SQLite state")]
    ForkDestinationAlreadyExists { dst_actor_id: String },

    #[error("pitr_disabled_for_namespace", "PITR is not enabled for this namespace")]
    PitrDisabledForNamespace,

    #[error("pitr_destructive_disabled_for_namespace", "destructive PITR (Apply mode restore) is not enabled for this namespace")]
    PitrDestructiveDisabledForNamespace,

    #[error("retention_window_exceeded", "target predates the retention window")]
    RetentionWindowExceeded { oldest_reachable_txid: u64 },

    #[error("restore_in_progress", "a restore operation is already running on this actor")]
    RestoreInProgress { existing_operation_id: Uuid },

    #[error("fork_in_progress", "a fork operation is already targeting this destination actor")]
    ForkInProgress { existing_operation_id: Uuid },

    #[error("actor_restore_in_progress", "the actor is being restored; commits are temporarily blocked")]
    ActorRestoreInProgress,

    #[error("admin_op_rate_limited", "too many concurrent admin operations for this namespace")]
    AdminOpRateLimited { retry_after_ms: u64 },

    #[error("pitr_namespace_budget_exceeded", "creating this checkpoint would exceed the namespace PITR budget")]
    PitrNamespaceBudgetExceeded { used_bytes: u64, budget_bytes: u64 },

    #[error("operation_orphaned", "operation has been pending without a working pod for too long; please retry")]
    OperationOrphaned { operation_id: Uuid },
}
```

The base spec's error envelope `Failed { group, code, message }` is replaced with `RivetErrorPayload` directly so error responses are the same shape as everywhere else in the engine.

## API surface (api-public)

```
POST   /actors/{id}/sqlite/restore               → { operation_id }      (async)
POST   /actors/{id}/sqlite/fork                  → { operation_id }      (async)
GET    /actors/{id}/sqlite/operations/{op_id}    → AdminOpRecord         (poll)
GET    /actors/{id}/sqlite/operations/{op_id}/sse → SSE stream           (live)
GET    /actors/{id}/sqlite/retention             → RetentionView         (DescribeRetention sync)
PUT    /actors/{id}/sqlite/retention             → RetentionView         (SetRetention sync)
POST   /actors/{id}/sqlite/refcount/clear        → ClearRefcountResult   (sync; admin-only)
GET    /namespaces/{ns_id}/sqlite-config         → SqliteNamespaceConfig
PUT    /namespaces/{ns_id}/sqlite-config         → SqliteNamespaceConfig
```

## Authorization chain (review #7)

```
1. api-public: validate caller bearer/service token via existing auth middleware.
2. api-public: load actor.namespace_id; load namespace.sqlite_config.
3. Capability check based on op:
   - DryRun restore + DescribeRetention + GetRetention   → namespace.allow_pitr_read
   - Apply (destructive) restore                          → namespace.allow_pitr_destructive
   - Fork (src=A, dst=B)                                  → A.namespace.allow_fork AND B.namespace.allow_fork
   - SetRetention                                         → namespace.allow_pitr_admin
   - ClearRefcount                                        → namespace.allow_pitr_admin
4. api-public injects AuditFields into the SqliteOp wire envelope: { caller_id, request_origin_ts_ms, namespace_id }.
5. Compactor trusts api-public — does NOT re-validate authz (envoy-internal trust boundary per CLAUDE.md).
6. Audit log: api-public emits structured log + Kafka audit event on Acked + Completed (or Failed) for every Restore/Fork/SetRetention/ClearRefcount.
```

## Per-namespace rate limiting (review #5 ops)

Token bucket at the api-public edge. Defaults:

- `admin_op_rate_per_min`: 10 (per namespace)
- `concurrent_admin_ops`: 4 (per namespace; counts in-flight Restore + Fork ops)
- `concurrent_forks_per_src`: 2 (per src actor)

Exceeding any limit returns `SqliteAdminError::AdminOpRateLimited { retry_after_ms }`.

Per-namespace overrides live in `SqliteNamespaceConfig`. Default-deny: namespaces without PITR enabled hit `PitrDisabledForNamespace` before the rate limiter is consulted.

## WebSocket lifecycle during restore (review #9)

When pegboard suspends an actor for restore:

- Pegboard sends "going away" to all envoys for the actor (existing primitive used in `engine/packages/pegboard-envoy/src/actor_lifecycle.rs`).
- Envoys close client WSes with code `1012` (service restart) and reason `actor.restore_in_progress`. Code `1012` is appropriate per the WebSocket Protocol Registry; existing CLAUDE.md WS rejection guidance applies (post-upgrade close, never pre-upgrade HTTP error).
- HTTP requests get `503 Service Unavailable` with header `Retry-After: 30`.
- Client SDK contract: `1012 actor.restore_in_progress` triggers backoff-and-retry, not permanent failure. Document in public actor SDK docs.

After restore completes, pegboard resumes the actor; envoys reaccept WS connections normally. Clients reconnect transparently.

## Concurrency model

| Op pair | Outcome |
|---|---|
| restore(A) + commit(A) | Pegboard suspends actor; commit blocked at storage layer via `/META/restore_in_progress` guard (commits return ActorRestoreInProgress) |
| restore(A) + compact(A) | compact lease-blocked; compactor skips A until restore completes |
| restore(A) + restore(A) | second is lease-blocked; admin-API rate limiter rejects with RestoreInProgress |
| restore(A) + fork(A → B) | both contend on A's lease; serialized; api-public can also reject the second with RestoreInProgress |
| fork(A → B) + fork(A → C) | parallel; each takes A's lease briefly, increments refcount, releases. Different dst leases serialize per-dst |
| fork(A → B) + commit(A) | parallel; commit on A doesn't block fork (fork reads only checkpoint + pinned deltas, not head) |
| fork(A → B) + compact(A) | fork takes A lease briefly, increments refcounts, releases. Compaction may run in parallel after fork lease release; refcounts protect pinned ckp + deltas |
| compact(A) + checkpoint creation | same pass; no contention |
| Two checkpoints concurrent on different actors | parallel; bounded by `max_concurrent_checkpoints` semaphore |

## Quota accounting (review #8 ops)

The base-spec `/META/quota` splits in two:

- `/META/storage_used_live` — live data (META + PIDX + DELTA + SHARD; excludes /CHECKPOINT/* and includes only DELTAs without retention pinning).
- `/META/storage_used_pitr` — PITR overhead (/CHECKPOINT/* + retention-pinned DELTAs; the bytes "extra" we keep around for restore/fork).

Caps:

- **Live cap**: `SQLITE_MAX_STORAGE_LIVE_BYTES = 10 * 1024 * 1024 * 1024` (per actor, base-spec value, unchanged user-facing semantics).
- **PITR cap**: `pitr_max_bytes_per_actor` from namespace config; default `0` (PITR disabled).
- **PITR namespace aggregate cap**: `pitr_namespace_budget_bytes` (sum across all actors in namespace). Tracked at namespace-level metric key.

Commit enforcement: `cap_check_live(would_be_live)` rejects a commit if `would_be_live > SQLITE_MAX_STORAGE_LIVE_BYTES`. Live cap is the only thing users see at commit time. Their predictable quota.

Checkpoint enforcement: `cap_check_pitr(would_be_pitr_actor, would_be_pitr_namespace)` skips a checkpoint creation if either would exceed cap. Increments `sqlite_checkpoint_skipped_quota_total`. The "your PITR data is being aggressively cleaned up because you're at budget" alert is the operator's signal to lower retention or raise budget.

## Failure modes

- **Restore tx fails partway through**: marker resumes on next lease take. Tx 0 marker write + first destructive write are the SAME tx (review C1) so an actor cannot be in cleared-but-no-marker state. Idempotent step-by-step replay.
- **Fork tx fails partway through**: dst marker rolls back via cleanup path; src refcount decrement is its own committed tx after marker clear.
- **Checkpoint creation fails**: retry on next compaction pass. No correctness impact.
- **Refcount leak**: auto-recovery after `lease_ttl_ms × 10`. ClearRefcount admin op for manual recovery.
- **Live + PITR exceeds 10 GB cap**: explicitly defined. Live cap enforced at commit (user-visible). PITR cap enforced at checkpoint creation (skip + alert). Operator action: lower retention or raise pitr_max_bytes.
- **Admin op orphaned**: 30s without an `Acked` (compactor pod absent / partition / queue group empty) → API marks `OperationOrphaned`. Caller retries with new operation_id. The compactor's lease-takeover path resumes any partially-completed work it finds via the marker, regardless of whether the original `operation_id` is still being polled.
- **FDB cluster loss**: all checkpoints + DELTAs gone. PITR cannot help. External backup + object-store tiering required for infrastructure DR (see Open Questions).
- **`/META/checkpoints` exceeds 16KB**: paginate to `/META/checkpoints/{page}`. Practically impossible at default retention (24 entries × ~32 bytes).
- **Retention shrinking**: compactor deletes newly-out-of-window data on next pass.

## Storage cost analysis

Default config: `retention_ms = 24h`, `checkpoint_interval_ms = 1h`.

| DB size | PITR overhead | % of 10GB live cap | Aggregate at 10k actors | At 100k actors |
|---|---|---|---|---|
| 10 MB    | ~240 MB         | 2.4%   | 2.4 TB   | 24 TB    |
| 100 MB   | ~2.4 GB         | 24%    | 24 TB    | 240 TB   |
| 1 GB     | ~24 GB          | 240%   | 240 TB   | 2.4 PB   |
| 10 GB    | ~240 GB         | 2400%  | 2.4 PB   | 24 PB    |

The non-linearity in DB size is the headline: at 1 GB DB the default config is already over the live cap by itself, before counting actual live data. **Operators MUST tune retention or namespace cap for any actor with >100MB DB.**

Non-default tuning examples:
- `checkpoint_interval = 6h`, `retention = 24h` → 4 checkpoints × DB size = 4× DB overhead.
- `checkpoint_interval = 24h`, `retention = 24h` → 1 checkpoint × DB size + 24× write rate of DELTAs.
- `retention = 0` → 0 overhead, base-spec behavior.

FDB native replication factor (typically 3x) multiplies all these numbers in raw cluster storage.

`pitr_namespace_budget_bytes` enforcement is what keeps SREs in control. Default: 100 GiB per namespace for production deployments, configurable.

## Configuration plumbing

Per-namespace config (`SqliteNamespaceConfig`):

```rust
pub struct SqliteNamespaceConfig {
    pub default_retention_ms: u64,                  // default 0 (off)
    pub default_checkpoint_interval_ms: u64,        // default 3_600_000 (1h)
    pub default_max_checkpoints: u32,               // default 25
    pub allow_pitr_read: bool,                      // default false
    pub allow_pitr_destructive: bool,               // default false
    pub allow_pitr_admin: bool,                     // default false
    pub allow_fork: bool,                           // default false

    // Caps
    pub pitr_max_bytes_per_actor: u64,              // default 0 (off)
    pub pitr_namespace_budget_bytes: u64,           // default 0 (off)
    pub max_retention_ms: u64,                      // upper bound for SetRetention; default 7 days when allow_pitr=true

    // Rate limiting
    pub admin_op_rate_per_min: u32,                 // default 10
    pub concurrent_admin_ops: u32,                  // default 4
    pub concurrent_forks_per_src: u32,              // default 2
}
```

Stored under namespace prefix in UDB. CRUD via `PUT/GET /namespaces/{id}/sqlite-config`. Defaults when key absent: PITR disabled.

Per-actor `/META/retention` overrides namespace defaults but capped by `max_retention_ms`.

## Runtime feature flag

`CompactorConfig.pitr_enabled: bool` (default `false`). Independently of `retention_ms = 0`, the flag short-circuits ALL checkpoint-creation logic (creates no checkpoints, reads no `/CHECKPOINT/*`) so a rollout can stage by region/cluster before any checkpoints are written. Once enabled and stable, retention is per-namespace per actor.

## Metrics

All include `node_id` label.

**Prometheus (per-pod, low cardinality):**
- `sqlite_checkpoint_creation_duration_seconds` (histogram)
- `sqlite_checkpoint_creation_bytes` (histogram)
- `sqlite_compactor_checkpoint_tx_count` (histogram)
- `sqlite_checkpoint_skipped_quota_total` (counter)
- `sqlite_checkpoint_creation_lag_seconds{namespace}` (gauge — `now - latest_checkpoint.taken_at_ms`)
- `sqlite_restore_duration_seconds{outcome}` (histogram, label outcome=success|failed|aborted)
- `sqlite_restore_deltas_replayed` (histogram)
- `sqlite_restore_in_progress_active` (gauge)
- `sqlite_fork_duration_seconds{outcome}` (histogram)
- `sqlite_fork_deltas_replayed` (histogram)
- `sqlite_fork_in_progress_active` (gauge)
- `sqlite_admin_op_total{op,outcome}` (counter — Restore|Fork|DescribeRetention|SetRetention|ClearRefcount × success|failed)
- `sqlite_admin_op_in_flight{op}` (gauge)
- `sqlite_admin_op_rate_limited_total{namespace}` (counter)
- `sqlite_admin_op_orphaned_total` (counter)
- `sqlite_pitr_disabled_total{reason}` (counter; reason=retention_zero|namespace_disallowed|feature_flag)
- `sqlite_checkpoint_refcount_leak_total` (counter)
- `sqlite_storage_pitr_used_bytes_namespace_sum{namespace}` (gauge — namespace aggregate)
- `sqlite_storage_live_used_bytes_namespace_sum{namespace}` (gauge — namespace aggregate)

**Per-actor metrics (UDB-backed namespace counters, NOT Prometheus, per review #6):**
- `MetricKey::SqliteStorageLiveUsed { actor_name }` (replaces base-spec SqliteStorageUsed; live bytes only)
- `MetricKey::SqliteStoragePitrUsed { actor_name }` (PITR overhead bytes)
- `MetricKey::SqliteCheckpointCount { actor_name }` (count of /CHECKPOINT/* entries)
- `MetricKey::SqliteCheckpointPinned { actor_name }` (count with refcount > 0)

These feed the existing metering pipeline (10-byte chunks via `KV_BILLABLE_CHUNK`), not Prometheus, so per-actor cardinality stays bounded.

## Alerts (production rollout requirements)

| Alert | Condition | Severity | Runbook |
|---|---|---|---|
| sqlite_checkpoint_refcount_leak | `rate(sqlite_checkpoint_refcount_leak_total) > 0` for 10m | Page | Investigate; ClearRefcount per actor; check for buggy fork code path |
| sqlite_restore_failure_rate | `rate(sqlite_admin_op_total{op="Restore",outcome="failed"})` > 0.1/min | Page | Check compactor logs; investigate target_txid validity |
| sqlite_compactor_falling_behind | `histogram_quantile(0.99, sqlite_compactor_pass_duration_seconds) > 60` | Warn | Scale compactor pods; check FDB latency |
| sqlite_lease_steal | `rate(sqlite_compactor_lease_renewal_total{outcome="stolen"}) > 0` | Warn | Check for split-brain; verify pod health; check NodeId uniqueness |
| sqlite_pitr_namespace_at_budget | `sqlite_storage_pitr_used_bytes_namespace_sum{ns} / pitr_namespace_budget_bytes{ns} > 0.8` | Warn | Notify namespace owner; recommend retention tuning |
| sqlite_checkpoint_skipped_quota | `rate(sqlite_checkpoint_skipped_quota_total) > 0` | Warn | PITR data is being lost; raise budget or lower retention |
| sqlite_admin_op_orphaned | `rate(sqlite_admin_op_orphaned_total) > 0.1/min` | Page | UPS partition or queue group empty; check compactor pod count |
| sqlite_checkpoint_creation_lag | `sqlite_checkpoint_creation_lag_seconds{ns} > 2 × checkpoint_interval_ms` for 10m | Warn | Compactor not keeping up; scale or investigate |

## Inspector / debugging support (review #11)

New inspector endpoints (mirror api-public surfaces, JSON instead of vbare):

```
GET /actors/{id}/sqlite/checkpoints      — list checkpoints with sizes, refcounts, pinned_reason
GET /actors/{id}/sqlite/retention        — DescribeRetention as JSON
GET /actors/{id}/sqlite/admin-ops        — recent AdminOpRecord history (last 24h, paginated)
GET /namespaces/{ns}/sqlite/overview     — aggregate PITR usage, pinned-checkpoint warnings, recent op counts
```

These reuse the same compactor handlers as the api-public ops; only the response codec differs (JSON vs vbare).

## Testing strategy

Per-module test scope. All tests use `test_db()` (real RocksDB) and the UPS memory driver. Use `tokio::time::pause()` + `advance()` for deterministic timing.

- `tests/checkpoint_create.rs` — checkpoint creation respects `head_txid_observed_at_plan_phase` (M3); multi-tx safety; refcount initial value 0; PITR quota enforcement skips on budget exceeded.
- `tests/checkpoint_cleanup.rs` — old-checkpoint deletion respects refcount + retention boundary; refcount auto-recovery after `lease_ttl_ms × 10`.
- `tests/restore_basic.rs` — restore to current head; restore to past txid via DELTA replay; restore to exact checkpoint; DryRun returns reachability without mutation.
- `tests/restore_validation.rs` — DryRun; unreachable target; target_txid > head; target predates retention.
- `tests/restore_target_resolution.rs` — `RestoreTarget::TimestampMs` resolves to correct txid; `LatestCheckpoint`; `CheckpointTxid(t)` validates exact match.
- `tests/restore_resume.rs` — pod failure between Tx 0 and step 12; marker presence implies "started"; resumer pins ckp_txid and replays from `last_completed_step`; no path leaves "cleared but no marker."
- `tests/restore_commit_guard.rs` — concurrent commit during restore returns `ActorRestoreInProgress`; commit succeeds after restore completes; cached "no restore" optimization works for repeated commits.
- `tests/fork_basic.rs` — fork at head, fork at past txid, fork preserves src state intact; src checkpoints unchanged after fork completes.
- `tests/fork_dst_allocation.rs` — `ForkDstSpec::Allocate` generates a new dst_actor_id; `ForkDstSpec::Existing` validates emptiness.
- `tests/fork_dryrun.rs` — DryRun returns estimates without taking dst lease.
- `tests/fork_concurrent.rs` — two concurrent forks of same src; refcounts correct; dst leases serialize per-dst; src checkpoint NOT prematurely deleted.
- `tests/fork_resume.rs` — pod failure between Tx D and Tx final-3; marker resumption.
- `tests/fork_delta_pinning.rs` — fork pins deltas in (ckp.txid, target_txid] before releasing src lease; concurrent compaction does not delete pinned deltas (review C3).
- `tests/refcount_sequencing.rs` — refcount increment commits before lease release (M2); decrement is its own committed tx (M4).
- `tests/retention_compaction.rs` — DELTAs preserved within retention; deleted past retention; refcount-pinned DELTAs survive.
- `tests/admin_op_record.rs` — operation_id allocation; status transitions; persistence across pod failure (simulated by reopening test DB); orphan detection at 30s no-Acked timeout.
- `tests/admin_op_dispatch.rs` — UPS round-trip via memory driver for each op variant.
- `tests/admin_rate_limit.rs` — token bucket enforces per-namespace cap; concurrent_admin_ops gate; concurrent_forks_per_src gate.
- `tests/admin_authz.rs` — capability checks at api-public; cross-namespace fork double-validation; missing capability rejected before any compactor work.
- `tests/admin_errors.rs` — every `SqliteAdminError` variant is reachable in tests via the right input; error shape matches `RivetError`.
- `tests/quota_split.rs` — `/META/storage_used_live` and `/META/storage_used_pitr` track separately; commits enforce only live cap; checkpoint creation enforces both PITR caps.
- `tests/pitr_disabled.rs` — `retention_ms = 0` mirrors base-spec compaction (no checkpoints); `pitr_enabled = false` short-circuits at the feature-flag layer.
- `tests/ws_close_during_restore.rs` — when restore starts, existing WS connections close with code 1012 reason `actor.restore_in_progress`; new connections rejected the same way until restore completes.

## Implementation strategy

Stages build incrementally on the base spec's stages 1-7. Each stage is independently testable.

### Stage 9: per-actor retention config + checkpoint key layout

- Add `/META/retention`, `/META/storage_used_live`, `/META/storage_used_pitr`, `/CHECKPOINT/*`, `/META/checkpoints`, `/DELTA/{T}/META`, `/META/admin_op/{id}`, `/META/restore_in_progress`, `/META/fork_in_progress` key builders to `pump::keys`.
- Add `RetentionConfig`, `RestoreMarker`, `ForkMarker`, `AdminOpRecord` types.
- Migrate base-spec `/META/quota` → split. On first read, sum into live and zero pitr.

### Stage 10: per-DELTA META + commit guard against restore

- Modify `pump::commit` to write `/DELTA/{T}/META = { taken_at_ms, byte_count, refcount: 0 }` in same UDB tx as chunk writes.
- Add restore-in-progress guard to `pump::commit` (one extra tx.get parallelized via try_join!; cached after first observation).
- Per-WS-conn `restore_observed_clear: AtomicBool` cache.

### Stage 11: namespace config + storage

- Add `SqliteNamespaceConfig`. Stored under namespace prefix in UDB.
- api-public endpoints `PUT/GET /namespaces/{id}/sqlite-config`.
- Defaults match the spec's "default" annotations.

### Stage 12: checkpoint creation in compactor

- Extend `compactor::compact_default_batch`: capture `head_txid_at_plan` once; use everywhere.
- Add `compactor::checkpoint::create_checkpoint(udb, actor_id, ckp_txid, cancel_token)` (multi-tx, lease-protected).
- Add `max_concurrent_checkpoints` semaphore to `CompactorConfig`.
- `pitr_enabled` runtime flag short-circuits.

### Stage 13: retention-aware DELTA cleanup + refcount auto-recovery

- Modify `compactor::compact_default_batch` to skip DELTA blob deletion when retention or refcount requires preservation.
- Add `compactor::cleanup::cleanup_old_checkpoints(...)` (refcount + retention aware).
- Add `compactor::cleanup::detect_refcount_leaks(...)` (auto-recovery at `lease_ttl_ms × 10`).

### Stage 14: SqliteOpSubject protocol + persisted op state

- Add `engine/packages/sqlite-storage/src/admin/` module with subjects, types, errors.
- Add `RivetError` derive on `SqliteAdminError`.
- Wire UPS subject + queue group into `compactor::worker::run` select loop.
- AdminOpRecord persistence; status transitions.
- Orphan detection (30s no-Acked).

### Stage 15: restore op (Apply + DryRun)

- `compactor::admin::handle_restore`: full multi-tx flow with marker resumption.
- Tx 0 marker write must be in same tx as `/SHARD/*` clear (review C1).
- Quota recompute via `atomic_add(delta)` (review M1).
- Tests including resumption + commit guard.

### Stage 16: fork op (Apply + DryRun, Allocate + Existing dst)

- `compactor::admin::handle_fork`: full multi-tx flow with delta pinning + dst marker.
- Refcount sequencing per review M2/M4.
- `ForkDstSpec::Allocate` integration with namespace ID allocation.

### Stage 17: short-running admin ops

- `DescribeRetention`, `SetRetention`, `GetRetention` (synchronous; persist result in AdminOpRecord).
- `ClearRefcount` admin op.

### Stage 18: api-public endpoints + suspend/resume orchestration

- POST/GET endpoints for restore, fork, retention, refcount.
- Suspend/resume orchestration around restore (call pegboard.suspend before publish; pegboard.resume after Completed).
- WebSocket close-code 1012 contract during suspension.
- SSE streaming endpoint for AdminOpRecord.

### Stage 19: authz + audit + rate limiting

- Capability checks at api-public per spec's authz chain.
- AuditFields injection into wire envelope.
- Audit log emission to existing log + Kafka pipeline.
- Token bucket for per-namespace rate limiting; concurrent op gates.

### Stage 20: per-namespace metrics aggregation

- `MetricKey::SqliteStorageLiveUsed`, `SqliteStoragePitrUsed`, `SqliteCheckpointCount`, `SqliteCheckpointPinned`.
- Compactor emits via existing metering rollup pipeline.
- Prometheus-side aggregates by namespace, not per actor.

### Stage 21: inspector endpoints

- JSON mirrors of admin ops at `/actors/{id}/sqlite/{checkpoints,retention,admin-ops}`.
- `/namespaces/{ns}/sqlite/overview`.

### Stage 22: docs + CLAUDE.md updates

- `docs-internal/engine/sqlite-pitr-forking.md` (full guide).
- `engine/CLAUDE.md` PITR/forking section.
- Public docs: `actor.restore`, `actor.fork`, `actor.describeRetention`; SDK reconnect on `1012 actor.restore_in_progress`; operator guide.
- `.claude/reference/docs-sync.md` entry: changes to `SqliteOpSubject` require api-public OpenAPI + SDK regen.

## Open questions

- **Read-only PITR mounting.** Future feature: mount actor at past txid for queries without modifying head.
- **Cross-actor consistent snapshots.** Coordinated point-in-time across multiple actors. Out of scope.
- **Object-store tiering for old checkpoints.** Storage cost is real. Future work: spill checkpoints older than N hours to S3-equivalent. Restore from object store. **This is the path to actual infrastructure DR.**
- **Separate PITR storage SKU.** Today live + PITR are separately tracked but billed to the same actor. Should namespaces have a separate PITR SKU?
- **Forking with delta streaming.** For long DELTA chains between checkpoint and target, we could fold inline during fork instead of step-by-step replay. Optimization.
- **Restore beyond all available checkpoints.** Currently rejects when target_txid < oldest_ckp.txid. Should we expose "restore to oldest checkpoint" as a graceful fallback?
- **Deep-fork (copy parent's checkpoint history).** Currently shallow only. Worth as a separate op?
- **PITR is not a backup.** Documented disclaimer needed in operator and user docs.
