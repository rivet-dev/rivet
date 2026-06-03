# SQLite Workflow Compaction Architecture

High-level proposal for replacing the current standalone hot/cold/eviction compactor services with durable Gasoline workflows. The goal is to simplify concurrency by making compaction decisions stateful and durable while keeping the SQLite commit path direct to FDB.

Terminology: this targets the `depot` package. The hot-path component is the conveyer, `compactor` keeps its name, and physical deletion/eviction work is the reclaimer role.

This is an architecture spec with enough implementation order to start work. It intentionally avoids detailed file-by-file Rust module changes.

## Goals

- Keep commits fast and independent of workflow latency.
- Make compaction ownership explicit and durable.
- Replace compactor leases with one branch-scoped workflow authority.
- Keep namespace forks cheap.
- Make compaction steps idempotent and safe under retries.
- Make old-history deletion depend on one recomputed pin/cold-coverage decision.

## Non-Goals

- Do not route commits through workflows.
- Do not make namespace fork proportional to database count.
- Do not require local SQLite files or hydrated branch state.
- Do not remove FDB transaction isolation from the commit path.
- Do not prescribe exact Rust module layout beyond ownership boundaries.

## Gasoline Shape

Use long-running Gasoline workflows in the same style as existing engine workflows:

- `engine/packages/pegboard/src/workflows/actor2/mod.rs` uses `ctx.lupe().with_state(...).run(...)` for durable actor-like state plus signal batches.
- `engine/packages/pegboard/src/workflows/runner_pool.rs` uses a manager workflow that spawns adjacent workflows, signals them, and loops over desired state.
- `engine/packages/epoxy/src/workflows/coordinator/mod.rs` uses a signal-driven coordinator workflow with joined signal enums.
- `engine/packages/gasoline-runtime/src/workflows/pruner.rs` uses `ctx.repeat(...)` plus `ctx.sleep(...)` for periodic durable loops.

SQLite compaction should use the same pattern: durable workflows with signal mailboxes, activities for FDB/S3 work, and loop state committed periodically.

## Workflow Topology

One active database branch has four persistent durable workflows. Gasoline workflow ids are generated `Id`s, not derivable strings, so logical identities are implemented with workflow type plus stable tags:

```text
db_manager          tags: database_branch_id
db_hot_compacter    tags: database_branch_id
db_cold_compacter   tags: database_branch_id
db_reclaimer        tags: database_branch_id
```

The manager is the authority. Conveyer commits stay direct to FDB. Hot compaction, cold upload, and reclaimer physical deletion run in persistent companion workflows so long cold/reclaim activities do not block hot scheduling or manager signal handling.

```text
Commit path -> FDB directly
            -> coalesced wake marker for db_manager

db_manager -> signal db_hot_compacter RunHotJob
db_manager -> signal db_cold_compacter RunColdJob
db_manager -> signal db_reclaimer RunReclaimJob

companions -> signal db_manager JobFinished(job_id)
```

Different branches can run independently. A branch workflow group may be started lazily on first commit/access and may sleep when idle. Persistent manager/compacter/reclaimer workflows should not complete while the branch is live; completing clears workflow tag indexes and makes stored workflow ids unsafe for future signals.

Do not spawn one workflow per compaction job. Workflow creation is relatively expensive; persistent branch companion workflows preserve parallel hot/cold/reclaim progress without per-job workflow churn.

## Authority Split

### DB Manager

The DB manager owns decisions about the branch's durable storage manifest:

- Observe committed head and hot lag.
- Track live DELTA, SHARD, and cold shard refs.
- Track DB history pins and unresolved namespace fork facts.
- Schedule hot, cold, and reclaimer jobs.
- Install compacter/reclaimer outputs after revalidation.
- Treat DB history pins as coverage cut points that may require SHARD or cold materialization before DELTA deletion.
- Decide when old hot/cold state is deletable.
- Persist the branch manifest/version-set edits.

The DB manager is the only workflow that can say:

- this SHARD output is live
- this cold shard ref is official
- these DELTAs are obsolete
- these SHARDs/cold objects are deletable
- this reclaimer job is authorized

### Hot Work

The hot compacter builds FDB-internal compaction output:

- Receives `RunHotJob`.
- Reads the job's selected DELTAs/SHARD inputs.
- Builds candidate SHARD output under deterministic staging keys.
- Signals `HotJobFinished`.

It does not write reader-visible SHARD keys, does not mark DELTAs obsolete, and does not decide what can be deleted.

### Cold Work

The cold compacter uploads cold-tier output:

- Receives `RunColdJob`.
- Uploads materialized cold shard payloads to deterministic S3 keys.
- Signals `ColdJobFinished` with bounded output refs, sizes, and content hashes.

It does not publish reader-visible cold refs, does not advance the cold watermark by itself, and does not mark pins `Ready` by itself.

### Reclaimer Work

The reclaimer performs physical deletes:

- Receives `RunReclaimJob`.
- Deletes only the keys/objects listed in a manager-issued bounded reclaimer job.
- Signals `ReclaimJobFinished`.

It does not scan global eviction state as the primary source of truth and does not decide eligibility.

## Commit Path

Commits stay direct-to-FDB:

```text
read/write BR/{branch}/META/head
write DELTA/{txid}/...
write PIDX/{pgno}
write COMMITS/{txid}
write VTX/{versionstamp}
update quota/access metadata
```

FDB remains the commit authority. The commit transaction still uses Serializable reads and atomic mutations where it does today.

After commit, the caller should not send one Gasoline signal per commit. Gasoline signals are durable writes and are only coalesced after a workflow receives them. Hot branches need an admission/coalescing layer before Gasoline.

Preferred wakeup shape:

1. Commit updates normal FDB commit state.
2. If hot or cold lag crosses a configured threshold, commit updates a scheduling-only dirty index.
3. The first writer that takes over a clean/available dirty marker emits `DeltasAvailable` immediately.
4. While commits continue, the conveyer refreshes the dirty marker and emits periodic `DeltasAvailable` signals according to the branch's signal throttle window.
5. The manager's own deadline timers are the convergence mechanism for hot compaction, cold compaction, and reclaim. On timeout, it refreshes FDB and runs the same planner, including final settled-state compaction/reclaim if the conveyer stopped or the branch went quiet.

`DeltasAvailable` is only a hint. The manager must refresh from FDB before scheduling or installing work.

Logical dirty index:

```text
SQLITE_CMP_DIRTY/{database_branch_id} -> { observed_head_txid, updated_at_ms }
```

The dirty index is not storage truth. The manager may clear it only after refreshing branch state from FDB and confirming there is no actionable hot, cold, or reclaim lag. Clearing must be compare-and-clear against the exact dirty value the manager observed, with a Serializable read conflict on the dirty key. A blind clear can erase a commit's only durable wake hint.

`DeltasAvailable` admission is edge-triggered plus throttled: the first commit that turns a branch dirty should send one immediately, and the conveyer should continue sending periodic signals while commits keep advancing the dirty marker. This keeps first compaction latency low, keeps long commit streams making progress, and avoids creating one durable Gasoline signal per commit.

`DeltasAvailable` is not the only progress mechanism. The manager should maintain durable planning deadlines such as `next_hot_check_at`, `next_cold_check_at`, `next_reclaim_check_at`, and `final_settle_check_at`. The manager loop listens until the nearest deadline or the next signal, whichever comes first. `DeltasAvailable` reduces latency while commits are active; deadlines guarantee clean completion after commits stop.

## Manager State

FDB stores authoritative published storage metadata. Durable workflow state owns in-flight scheduling state.

FDB-owned metadata includes:

- `manifest_generation`
- current hot materialization watermark
- current cold coverage watermark
- live DELTA retention bounds derived from commit-owned DELTA plus watermarks
- live SHARD versions
- live cold shard refs and small summaries
- DB history pin records and optional rebuildable summaries
- unresolved namespace fork facts and cached proof records
- schema version and operator/debug fields

Workflow-owned state includes:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected job output refs and content hashes
- retry/backoff state
- last processed dirty/timeout cursor

On startup or periodic timeout, the manager refreshes FDB/cold-tier state and rebuilds any missing cache. If workflow state and FDB disagree about published compaction metadata, FDB wins. Workers validate their accepted job against FDB generation and fingerprints before reporting or retrying output; the manager decides whether the job is still active.

## Compaction Jobs

Every compaction job has:

- `job_id`
- `database_branch_id`
- `base_manifest_generation`
- input ranges/layers
- deterministic staging output keys
- expected output checksums or content fingerprints

Activities are idempotent. Retrying a step with the same `job_id` writes the same output keys or no-ops.

Workflow signals are durable job messages. `Run*Job` and `*JobFinished` payloads must include `job_id`, `job_kind`, `base_manifest_generation`, and an input or output fingerprint. Companions use `job_id` to make duplicate job handling idempotent. The manager accepts a completion only if job id, kind, base generation, and fingerprint match an active job record; otherwise the result is stale orphan output.

Normal job inputs, output refs, content hashes, and retry state live in durable workflow state/signals, not FDB. Signals must stay small. Large input/output sets are split into smaller jobs instead of adding FDB job metadata. Cold shard bytes live in S3 because they are the bulky payload.

## Install Rules

Worker output is only a candidate until the DB manager installs it.

Workers must write to staging namespaces that readers do not consult. Current readers treat live SHARD keys and cold manifest indexes as reader-visible, so the publish step must be manager-owned:

- Hot compacters write staged SHARD output under job-scoped staging keys.
- Cold compacters write cold shard payload objects to S3 and report bounded output refs/content hashes through workflow signals.
- The manager install transaction publishes live FDB SHARD/manifest summary updates.
- The manager alone rewrites or advances any reader-visible cold manifest pointer/index.

Before installing any hot or cold output, the manager must:

- refresh relevant FDB state
- verify `base_manifest_generation` is still compatible
- recompute current DB history pins
- resolve relevant unresolved namespace fork facts
- confirm the output does not obsolete history at or below the pin floor
- validate staged output checksums or content fingerprints

If the result is stale, the manager discards it or treats it as orphan output to clean later.

Every manifest install is a Serializable FDB transaction that reads the current manifest generation and writes the next generation. If the generation changed, the install aborts and the manager replans.

`manifest_generation` is not a commit fence because commits do not bump it. Any install transaction that changes reader-visible routing must also Serializable-read `/META/head` and every commit-owned key it will reinterpret or clear. PIDX clears must be conditional on the expected txid/value, and DELTA/COMMITS/VTX deletion must happen only after the same transaction proves the current pin floor and cold coverage.

Pinned historical reads are not only retention blockers; they are compaction coverage targets. Hot and cold compaction should materialize replacement coverage for both the latest head and any DB history pin cut points that would otherwise keep DELTAs live. The manager may delete hot history at a pinned/fork-required txid only after that version is still directly readable from retained DELTAs/SHARDs or has replacement coverage from published SHARD or cold refs at the required cut point.

Hot install transaction:

1. Read `CMP/root`, `/META/head`, DB history pins, namespace fork proof records, and the commit-owned PIDX/DELTA keys that the job input fingerprint covers.
2. Validate staged hot shard checksums and input fingerprint.
3. Write reader-visible `SHARD` chunks from staged output for the latest head and any selected pinned/fork-required cut points.
4. Advance `CMP/root.hot_watermark_txid` and `manifest_generation`.
5. Conditionally clear or rewrite PIDX entries only when the expected txid/value still matches.
6. Leave DELTA/COMMITS/VTX physical deletion to manager-authorized reclaimer jobs.

Cold publish:

1. The manager checks that the cold job's object key is unique for this publish attempt and is not already recorded under `CMP/retired_cold_object/...`.
2. The FDB transaction reads `CMP/root`, the database branch record and lifecycle generation, DB history pins, namespace fork proof records, the retired-object key for the candidate object key, and the exact commit-owned DELTA/COMMITS/VTX inputs covered by the cold output fingerprint.
3. Validate reported object key, size, content hash, and output fingerprint.
4. Write `CMP/cold_shard/...` refs, advance cold watermarks, and bump `manifest_generation`.

Every hot install, cold publish, and reclaim transaction must read `BRANCHES/list/{database_branch_id}` and abort if the branch record is missing, deleted, or its lifecycle generation changed since planning.

This moves the hot-fold/fork race into one manager decision:

```text
hot activity folds DELTA 1..100
fork writes desc_pin at txid 50
hot activity reports output
manager recomputes pin floor = 50
manager refuses to delete or obsolete DELTA <= 50
```

## FDB Key Layout

FDB owns all authoritative SQLite branch metadata. S3 object existence is never enough to make data live.

The keys below are logical key names. The implementation may encode them with existing byte prefixes, but it should preserve these ownership boundaries.

### Commit-Owned Keys

Commits continue to write the existing hot path directly:

```text
BR/{database_branch_id}/META/head
BR/{database_branch_id}/DELTA/{txid_be}/{chunk_idx_be}
BR/{database_branch_id}/PIDX/{pgno_be}
BR/{database_branch_id}/COMMITS/{txid_be}
BR/{database_branch_id}/VTX/{versionstamp}
BR/{database_branch_id}/META/quota
BR/{database_branch_id}/META/access
```

The commit transaction may emit a rate-limited `DeltasAvailable` signal or update a scheduling-only dirty index, but it does not update manifest generation. Signal state is not part of storage correctness.

### Published Compaction Manifest

The DB manager owns published compaction state:

```text
BR/{database_branch_id}/CMP/root
BR/{database_branch_id}/CMP/cold_shard/{shard_id_be}/{as_of_txid_be}
BR/{database_branch_id}/CMP/retired_cold_object/{object_key_hash}
BR/{database_branch_id}/SHARD/{shard_id_be}/{as_of_txid_be}/{chunk_idx_be}
```

`CMP/root` contains:

```text
CompactionRoot {
  schema_version,
  manifest_generation,
  hot_watermark_txid,
  cold_watermark_txid,
  cold_watermark_versionstamp,
}
```

`CMP/cold_shard/...` contains:

```text
ColdShardRef {
  object_key,
  object_generation_id,
  shard_id,
  as_of_txid,
  min_txid,
  max_txid,
  min_versionstamp,
  max_versionstamp,
  size_bytes,
  content_hash,
  publish_generation,
}
```

`BR/.../SHARD/...` remains the reader-visible hot materialized shard payload in FDB. If shard payloads exceed the FDB value limit, shard payloads are chunked under `chunk_idx_be`; the key is still reader-visible only after manager publish.

`CMP/retired_cold_object/...` is the FDB-visible handoff between unpublishing a cold ref and physically deleting the S3 object. It exists because FDB refs are MVCC and S3 objects are not. A reader can observe an older `ColdShardRef` from an older FDB snapshot and fetch S3 later, so S3 deletion must wait until the retired object passes a configured wall-clock grace window.

Decision: use a grace window for the initial implementation, not reader epochs. The grace window must be longer than any expected depot read transaction or read path lifetime, with margin for scheduler delay and retry jitter. Depot reads should be short-lived, so this is simpler than registering active reader epochs. Reader epochs remain a future option only if real read lifetimes make the grace window too conservative.

```text
RetiredColdObject {
  object_key,
  object_generation_id,
  content_hash,
  retired_manifest_generation,
  retired_at_ms,
  delete_after_ms,
  delete_state, // Retired | DeleteIssued
}
```

`delete_state = DeleteIssued` is irreversible for reconciliation purposes. If an S3 delete activity may have reached S3, the manager must treat the object as possibly gone even if the workflow did not record activity completion.

### Read Path Precedence

The new manifest preserves the existing depot read shape. For each requested page and read cap:

1. Resolve the database branch and bounded ancestry.
2. Pick the first matching PIDX entry from the nearest branch/source whose txid is at or below that source's read cap.
3. Try the referenced DELTA blob.
4. If the DELTA is missing or intentionally reclaimed, fall back to the latest published SHARD for that page's shard at or below the read cap.
5. If no hot SHARD covers the page, fall back to published cold refs that cover the page, owner txid, shard, and read cap.
6. If no layer contains the page, return a zero page.

Normal latest reads still use PIDX and DELTA first. Hot compaction may update latest read routing and clear PIDX. Pinned/fork-required versions use the same precedence, but DELTA deletion is allowed only after replacement SHARD or cold coverage exists at the exact pinned/fork-required txid.

### Workflow Job State

Workflow jobs are tracked in durable workflow state and signals. FDB is not the normal store for in-flight job bookkeeping.

```text
BR/{database_branch_id}/CMP/stage/{job_id}/hot_shard/{shard_id_be}/{as_of_txid_be}/{chunk_idx_be}
```

`CMP/stage/.../hot_shard` is for staged hot shard payloads that cannot be reader-visible until manager publish. Oversized job payloads are invalid and should be split before scheduling.

### Reclaim Derivation

Obsolete data is derived from the current published manifest, hot/cold watermarks, DB history pins, unresolved namespace fork facts, and cold coverage. FDB does not store a separate obsolete queue or delete authorization log in the normal design.

The manager may enqueue bounded reclaimer jobs in workflow state. Before doing so, it should proactively schedule hot or cold compaction outputs for pinned/fork-required cut points that are preventing DELTA deletion. Reclaimer activities re-read manifest generation and pin/cold-coverage predicates in the delete transaction before clearing. FDB deletes use conditional semantics when stale delete could remove changed data.

### DB History Pins And Namespace Fork Facts

A DB history pin means one database branch must preserve readable history at an exact version because something depends on it.

```text
DB_PIN/{database_branch_id}/{pin_id}
```

`DB_PIN` value:

```text
DbHistoryPin {
  at_versionstamp,
  at_txid,
  kind,         // DatabaseFork | NamespaceFork | Bookmark
  owner_id,     // target db branch, target namespace branch, or bookmark id
  created_at_ms,
}
```

The manager consumes only this unified DB pin abstraction when computing coverage targets:

```text
coverage_targets =
  { latest_head_txid }
  ∪ { pin.at_txid for DB_PIN/{database_branch_id}/... }
```

Live pins do not expire by retention-window age. A `DatabaseFork` pin is removed when the descendant database branch no longer needs the source point. A `NamespaceFork` pin is removed when the namespace-derived dependency no longer needs the source point. A `Bookmark` pin is removed only when the bookmark owner explicitly releases or deletes it. There is no TTL policy for pins in this design.

Direct database forks and bookmarks write `DB_PIN` records directly. Namespace forks stay cheap and write unresolved namespace fork facts first:

```text
NS_FORK_PIN/{source_namespace_branch_id}/{fork_versionstamp}/{target_namespace_branch_id}
NS_CHILD/{source_namespace_branch_id}/{fork_versionstamp}/{target_namespace_branch_id}
NSCAT_BY_DB/{database_branch_id}/{namespace_branch_id}
NS_PROOF_EPOCH/{root_namespace_branch_id}
```

`NS_FORK_PIN` and `NS_CHILD` are immutable fork facts. Their values must include the source namespace branch, target namespace branch, fork versionstamp, and parent cap/versionstamp used by namespace inheritance. `NSCAT_BY_DB` maps explicit database membership to namespace branches and includes the catalog versionstamp plus any tombstone/delete versionstamp for that database membership. `NS_PROOF_EPOCH` is incremented by every namespace fork, catalog membership, and catalog tombstone mutation in the namespace tree.

Before install/reclaim planning, the DB manager must check both:

1. Existing `DB_PIN/{database_branch_id}/...` records.
2. Unresolved `NS_FORK_PIN` facts that could inherit this database branch.

If an unresolved namespace fork can inherit this database at namespace versionstamp `V`, the manager must resolve `V` to the latest visible database commit with commit versionstamp at or below `V`, then materialize a normal `DB_PIN` with `kind = NamespaceFork` at that exact DB txid or conservatively retain history. Lazy database open may also materialize the same `DB_PIN`, but compaction correctness must not depend on lazy open happening first.

There are two possible `NS_PROOF_EPOCH` shapes:

Option A: per-source namespace branch epoch.

```text
NS_PROOF_EPOCH/{source_namespace_branch_id}
```

Every namespace fork, catalog membership write, or catalog tombstone that can affect inheritance from that source branch bumps that source's epoch. This can reduce false invalidation, but it is easy to under-fence cached negative proofs because a later mutation can make a previously uninspected source branch relevant.

Option B: per-namespace-tree epoch.

```text
NS_PROOF_EPOCH/{root_namespace_branch_id}
```

Every mutation anywhere in the namespace tree bumps one root/tree epoch. Proof validation is simpler because the delete transaction reads one epoch, but unrelated catalog changes in the same tree invalidate more proofs.

Decision: use option B for the initial implementation. The extra invalidation is acceptable because namespace-fork delete proofs are on the destructive cleanup path, not the commit path. This avoids subtle under-fencing where a cached negative proof only read epochs for branches it inspected and misses a newly relevant namespace/catalog path.

## S3 Key Layout

S3 stores only bulky cold payload bytes. FDB stores the authoritative object refs, checksums, and publish state.

S3 object keys are immutable and unique per cold publish attempt. Cold compacters write directly to the final object key after computing the payload hash. The object is not reader-visible until the DB manager publishes an FDB `ColdShardRef` to it. Publishing does not rename, copy, or mutate S3 objects.

All keys are relative to the configured cold-tier root:

```text
db/{database_branch_id_hex}/
  shard/
    {shard_id_be_hex:8}/
      {as_of_txid_be_hex:16}-{object_generation_id}-{content_hash_hex}.ltx
```

Each object contains one materialized shard image at one `as_of_txid`. Exact pinned bookmarks are represented by publishing shard objects at the pinned txid, not by a separate `pin/` object class.

`object_generation_id` is the cold `job_id`. Retrying the same job reuses the same key and converges. Replanning after an object has been retired or scheduled for deletion must use a new job id and therefore a new `object_generation_id`, even if the bytes and content hash are identical. This prevents a stale delete request for an old key from deleting later live data.

The content hash in the key prevents stale compacter outputs from overwriting a published object with different bytes. Retries that produce different bytes create a different orphan object that is not live unless the manager publishes its matching FDB ref.

FDB cold refs point at these objects:

```text
ColdShardRef {
  object_key,
  object_generation_id,
  shard_id,
  as_of_txid,
  min_txid,
  max_txid,
  min_versionstamp,
  max_versionstamp,
  size_bytes,
  content_hash,
  publish_generation,
}
```

The branch id is part of the key because cold objects belong to a database branch's history graph. The shard id and `as_of_txid` are part of the key because they identify the logical output. `object_generation_id` makes each publish attempt non-reusable after retirement. The content hash appears in both the S3 key and FDB ref so readers and cleanup jobs can verify the exact payload.

No S3 key encodes live or published state. An object is live only if the current FDB manifest generation references it.

## Branch Liveness

Use the database branch record as the branch lifecycle authority. The existing code stores branch metadata at:

```text
BRANCHES/list/{database_branch_id}
```

It currently has `state: Live | Frozen`, namespace tombstones, and refcount-driven GC. There is no dedicated monotonic branch liveness generation key today.

For workflow compaction, extend the branch lifecycle metadata so every database branch record has a monotonic `lifecycle_generation`. The branch record remains the single FDB record that install/reclaim transactions read to prove branch lifecycle state.

Rules:

- Commits require a writable/live branch state.
- Compaction publish and reclaim may run for live or frozen branches that still retain history for descendants, bookmarks, or namespace-derived DB history pins.
- Final branch deletion/GC advances lifecycle state or removes the branch record.
- Every hot install, cold publish, and reclaim transaction reads `BRANCHES/list/{database_branch_id}` and aborts if the record is missing, deleted, or its `lifecycle_generation` changed since planning.

Do not add a separate branch liveness key unless the branch record becomes too large or too contentious in implementation.

## Reclaim Rules

Physical deletion must remain conservative.

The manager may schedule a reclaimer job only after recomputing:

- DB history pins
- unresolved namespace fork facts relevant to this DB branch
- cold coverage
- replacement coverage for every retained/pinned/fork-required version affected by the delete
- current manifest generation

FDB deletes must use conditional semantics such as `CompareAndClear` when a stale delete could otherwise remove a value that was rewritten since planning. Workflow idempotency handles duplicate execution; conditional deletes handle stale execution.

Reclaimer jobs carry expected values, versionstamps, or content fingerprints. The reclaimer activity re-checks manifest generation, branch lifecycle generation, DB history pins, unresolved namespace fork proof validity, current PIDX dependency, and cold coverage in the delete transaction before clearing. Avoid `clear_range` unless the same transaction has a fence that proves every key in the range is still safe to remove.

Before deleting DELTA rows, the reclaimer must prove no current read path still depends on them. The delete transaction must either:

1. Read the expected PIDX entries for every page touched by the deleted DELTAs and prove none still points at a deleted txid; or
2. Prove the read path ignores those PIDX entries because replacement SHARD/cold coverage is reader-visible for every affected page and read version being retained.

If the affected PIDX set is too large, split the reclaimer job.

This verification is the destructive boundary, but the planner should not rely on reclaimer rejection as normal flow control. If pins or namespace forks require historical versions, the manager should first schedule hot/cold materialization at those cut points, then schedule DELTA reclaim after the coverage is published.

S3 deletes are idempotent and must target only cold shard object keys listed in manager-planned reclaimer jobs.

Cold object deletion is a two-phase retire/delete flow:

1. FDB retire transaction: remove the `ColdShardRef` from the live manifest, write `CMP/retired_cold_object/...`, and bump `manifest_generation`.
2. Wait until the object passes the configured reader-safety grace window. This proves old readers that could have observed the retired `ColdShardRef` should no longer need the S3 object.
3. S3 delete activity: delete the exact retired `object_key` and treat the delete as issued before calling S3.
4. FDB cleanup transaction: after delete completion, clear or mark complete the retired-object record if the exact `ColdShardRef` is still absent and the retired record still matches.

Do not run the S3 delete in parallel with the FDB retire transaction. FDB unpublish makes the object unreachable for new readers only; old FDB snapshots may still contain the old `ColdShardRef`.

S3 delete jobs must not race a later publish of the same object key. Once an object key is retired, that exact key must never be republished. If a later cold job produces the same bytes, the manager replans with a new `object_generation_id`.

The reclaimer must never derive a cold object key from `{shard_id, as_of_txid, content_hash}`. The manager reads the exact obsolete `ColdShardRef` from FDB and places that exact `object_key`, `object_generation_id`, `content_hash`, and expected `publish_generation` into the bounded reclaim job. The S3 delete activity deletes only that exact key.

```text
DeleteColdObject {
  object_key,
  object_generation_id,
  content_hash,
  expected_publish_generation,
}
```

Cold-object reconciliation resolves ambiguity in this order:

1. If FDB references an object and there is no retired-object record for the same object key, it is live.
2. If a retired-object record exists with `delete_state = Retired`, the object is unpublished but must not be physically deleted until its grace window has passed.
3. If a retired-object record exists with `delete_state = DeleteIssued`, the object is possibly gone. A live FDB ref to the same object key is corruption and must log/alert instead of assuming the delete is stale.
4. If workflow state has an active compatible cold job for an unreferenced, unretired object, it is candidate output.
5. If neither FDB, retired-object state, nor workflow state references the object, it is orphan output and may be deleted after the same retired-object safety policy if any reader-visible ref ever pointed to it.

## Namespace Forks

Namespace forks must stay cheap. Do not eagerly fan out DB pins to every inherited database.

On namespace fork, write one unresolved namespace fork fact:

```text
NS_FORK_PIN/{source_namespace_branch}/{at_versionstamp}/{target_namespace_branch}
```

The namespace manager owns namespace catalog pinning. DB managers own database history deletion.

When a forked namespace accesses a database, or when a DB manager wants to delete old history, materialize or check DB-specific namespace fork requirements lazily as normal DB history pins:

```text
DB_PIN/{database_branch_id}/{pin_id(kind=NamespaceFork)}
```

A namespace fork is a namespace/catalog versionstamp, not a single DB txid and not necessarily the current DB head. Each database has its own txid stream. To materialize a namespace-derived DB pin, resolve the namespace fork versionstamp through the database commit mapping:

```text
BR/{database_branch_id}/VTX/{commit_versionstamp} -> txid
BR/{database_branch_id}/COMMITS/{txid} -> { commit_versionstamp, ... }
```

The resolved pin uses the latest commit for that database branch whose commit versionstamp is at or below the namespace fork cap. Different databases in the same namespace fork may therefore pin different txids.

Safety rule:

Before deleting database history, the DB manager must either prove no unresolved namespace fork fact can include this database or conservatively retain the history. Deleting first and discovering the namespace-derived DB pin later is not allowed.

"Resolve on delete" means the DB manager checks unresolved namespace fork facts before authorizing deletion of SHARD, DELTA, cold layer, or history rows. The manager asks: can any cheap namespace fork still inherit this database at the version being deleted? If yes, the manager materializes a `DB_PIN(kind=NamespaceFork)` or keeps the history. If no, deletion may proceed.

The preferred shape is:

- Keep namespace fork O(1) by writing only the namespace-level pin at fork time.
- Lazily materialize namespace-derived `DB_PIN` records on database access and before DB history deletion.
- Treat unresolved namespace fork facts as retention blockers until proven irrelevant.

Decision: resolve namespace fork facts on delete. The DB manager pays the namespace lookup cost only when it is about to delete database history, not when the namespace fork is created.

The deletion proof must be bounded. It cannot rely on unbounded scans of namespace forks or current namespace visibility. The namespace fork indexes must encode enough immutable fork context to prove whether a database was inheritable at the fork version, including the source namespace branch, target namespace branch, fork versionstamp, and the catalog/tombstone versionstamp window used for the proof. If the manager cannot produce a bounded negative proof, it must retain the history.

Concrete delete-time proof:

1. Start from `NSCAT_BY_DB/{database_branch_id}/...` records whose catalog/tombstone window could make the database inheritable by a namespace fork whose fork cap still requires the candidate history.
2. Walk `NS_CHILD` edges from those namespace branches, bounded by immutable child-edge ranges and fork versionstamp caps.
3. For every fork that could inherit the database at the deletion version, materialize or read `DB_PIN(kind=NamespaceFork)`.
4. The physical delete transaction must either re-run this bounded proof or read proof records plus the root/tree `NS_PROOF_EPOCH` captured by that proof.
5. If any proof range is too large, missing, or ambiguous, deletion keeps the history.

Cached proof records are not safe just because their creation transaction had FDB read conflicts. Those conflicts protected only the proof-creation transaction. A later delete transaction must read an epoch/fence that every relevant namespace mutation updates, or must rerun the proof itself.

## Signals

Representative DB manager signals:

```text
DeltasAvailable
HotJobFinished { job_id, base_generation, input_fingerprint, status, output_refs }
ColdJobFinished { job_id, base_generation, input_fingerprint, status, output_refs }
ReclaimJobFinished { job_id, base_generation, input_fingerprint, status }
DestroyDatabaseBranch { branch_generation }
```

Representative companion workflow signals:

```text
RunHotJob { job_id, base_generation, input_fingerprint, inputs }
RunColdJob { job_id, base_generation, input_fingerprint, inputs }
RunReclaimJob { job_id, base_generation, input_fingerprint, inputs }
DestroyDatabaseBranch { branch_generation }
```

Signals are wakeups and durable job messages, not trusted truth. Workflows re-read the required FDB/cold-tier state before publishing, retrying, or deleting.

Use batch listening where signal volume can be high, following the `listen_n` pattern in runner pool workflows. Batch listening is not publish-time coalescing; wake admission must happen before signals are written to Gasoline.

Periodic manager work is not a durable signal and is not a busy loop. The manager should normally sit idle in `listen_with_timeout(...)`, `listen_n_with_timeout(...)`, `listen_n_until(...)`, or an equivalent `ctx.sleep(...)` loop until the nearest planner deadline. Normal commit-driven progress comes from conveyer-sent `DeltasAvailable` signals. On timeout, the workflow performs the same FDB refresh/planning pass it would perform after `DeltasAvailable`, mainly to advance hot/cold/reclaim timers and converge a branch after commit activity settles. If useful, the loop may synthesize an in-memory enum variant for code organization, but it must not publish a persistent `Tick` signal.

There is intentionally no pin-change signal. Bookmark writes, database forks, and namespace fork resolution write `DB_PIN` or `NS_FORK_PIN` state in FDB. The DB manager re-reads DB pins and unresolved namespace fork facts on `DeltasAvailable`/timeout and before every install/reclaim decision. Lost signals may delay compaction, but destructive transactions still re-check FDB pin/proof state.

## What Gets Dropped

The workflow model removes legacy standalone compactor coordination entirely:

- Drop old standalone hot compactor mutation paths.
- Drop old standalone cold compactor mutation paths.
- Drop old standalone eviction/global deletion mutation paths.
- Drop the hot compactor lease.
- Drop the cold compactor lease.
- Drop the eviction global lease for per-branch correctness.
- Drop cold `in_flight_uuid` as a separate FDB handoff concept.
- Drop pending marker as the primary handoff mechanism.
- Drop pending-pin handoff state.
- Drop staged cold job metadata outside workflow state.
- Drop old obsolete/delete queue state as a correctness mechanism.
- Drop `last_hot_pass_txid` as the eviction OCC fence.
- Drop eviction's global scan as the primary deletion authority.
- Drop legacy dual-read/rollback compatibility paths for the current compactor architecture.

The replacement concepts are narrower:

- Workflow `job_id` identifies bounded workflow work. It is not an FDB handoff or lease mechanism.
- Pending markers are not required. Trust durable workflow job state for orchestration and cleanup.
- Eviction becomes manager-authorized physical deletion.
- Workflow uniqueness/stable tags provide workflow identity. Do not add compactor-specific lease keys.

## What Remains

Keep the mechanisms that protect data correctness at the storage boundary:

- FDB Serializable reads for commits and pin/cap checks.
- FDB atomic operations for `ByteMin`, `Add`, and versionstamped writes.
- Conditional physical deletes such as `CompareAndClear`.
- Unified DB history pins: `DB_PIN/{database_branch_id}/{pin_id}`.
- Unresolved namespace fork facts plus proof epochs for cheap namespace forks.
- Deterministic S3/FDB output keys for idempotent workflow steps.
- FDB-visible retired cold-object records plus a wall-clock grace window before S3 deletion.
- Manager generation checks for stale compacter/reclaimer output.
- FDB manifest-generation compare-and-set on every install/delete decision.

## Workflow Ownership

Use stable workflow tags, not derivable workflow ids:

```text
db_manager          tags: database_branch_id
db_hot_compacter    tags: database_branch_id
db_cold_compacter   tags: database_branch_id
db_reclaimer        tags: database_branch_id
```

Use Gasoline `.unique()` with exactly the stable branch tag for each workflow type. Do not include job id, generation, companion state, or other volatile tags on unique workflows. The DB manager is responsible for spawning the hot compacter, cold compacter, and reclaimer workflows for its branch and storing their returned workflow ids in manager state.

Signals from the manager to companion workflows may use stored workflow ids while the workflows are live. External `DeltasAvailable` signals should target the manager by workflow type plus stable tags or go through a helper that creates the unique workflow if missing.

Decision: rely on workflow uniqueness and stable tags for manager ownership. Do not add an FDB manager epoch unless future implementation work proves Gasoline uniqueness is insufficient.

Persistent manager/compacter/reclaimer workflows should sleep/listen while idle instead of completing. If a workflow does complete, the next signal path must recreate it by workflow type plus stable tags; do not publish to stale raw workflow ids.

Companion workflow progress is manager-driven. Compacter/reclaimer workflows do not independently reconcile against manager state. A companion receives one bounded job, validates the job against FDB generation and fingerprint, reports completion or failure, then returns to idle. The manager decides whether the result is still active, stale, retryable, or orphan cleanup.

## Branch Deletion Lifecycle

Gasoline handles idle sleeping automatically. The four branch workflows remain persistent while the database branch is live.

The manager is the lifecycle authority for branch deletion:

1. The manager receives the `DestroyDatabaseBranch` signal.
2. The manager observes the FDB-authoritative branch record and lifecycle generation, then records stopping/deleted state in durable workflow state.
3. The manager stops scheduling new hot, cold, and reclaimer work.
4. The manager sends `DestroyDatabaseBranch` signals to the hot compacter, cold compacter, and reclaimer.
5. Companion workflows finish or abandon current bounded/idempotent work according to role and complete.
6. The manager completes after recording stopped/deleted state and any required final cleanup or ownership release.

Companion workflows must not decide branch liveness themselves. Compacter/reclaimer workflows also must not publish output after the manager has marked the branch stopping/deleted.

Branch deletion state must be anchored in FDB, not only workflow state. Commits, branch access, manager startup, compacter validation, and reclaimer validation read the FDB branch record and lifecycle generation. A manager recreated by stable tags after completion must observe the FDB branch lifecycle state and must not schedule or publish compaction work for a deleted branch.

## Backpressure

Backpressure is out of scope for this spec. Do not make global or branch-local backpressure policy part of the storage-correctness architecture.

## Manifest State Placement

Decision: store published storage truth in FDB. Store in-flight job orchestration and delete planning in durable workflow state.

FDB owns:

- manifest generation
- hot materialization watermark
- cold coverage watermark
- live DELTA retention bounds derived from commit-owned DELTA plus watermarks
- live SHARD versions
- live cold layer root refs and small summaries
- direct database pin summaries or cached proof records
- DB history pin summaries or cached proof records
- schema version and operational debug fields

Workflow state owns:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected output refs and content hashes
- retry/backoff state
- last processed dirty/timeout cursor

S3 owns bulky cold shard payload bytes only. FDB owns the authoritative refs, summaries, and published state. In-flight job metadata stays in workflows.

### Source-of-Truth Split

The FDB manifest summary must not duplicate commit-owned state unless the commit transaction updates it.

Commit-owned source of truth remains:

- `/META/head`
- `DELTA`
- `PIDX`
- `COMMITS`
- `VTX`
- quota/access metadata

Manager-owned FDB metadata covers compaction-derived state:

- manifest generation
- hot materialization watermark
- cold coverage watermark
- live published SHARD versions
- live published cold manifest references
- pinned/fork-required coverage cut points that have been materialized
- DB history pin summaries or rebuildable caches
- schema version

If a future summary field is derived from each commit, the commit transaction must update it or the field must be documented as a cache that the manager can rebuild. The compaction manifest must not become a second source of truth for commit-owned DELTA, PIDX, COMMITS, VTX, or head state.

All manager writes to FDB compaction metadata use Serializable read of the current manifest generation followed by generation-advancing write when they publish live state. Operators and repair jobs that mutate this metadata must follow the same compare-and-set contract.

## Reconciliation

Workflow state is durable, but FDB owns compaction metadata. Reconciliation is FDB-first:

1. Refresh FDB head, compact state, pins, and cold coverage.
2. Reload manager and companion workflow state.
3. Reconcile active jobs against the current FDB manifest generation.
4. Keep compatible completed outputs.
5. Schedule cleanup for orphan job outputs.
6. Resume scheduling.

Cold S3 objects should be deterministic by branch, shard, and `as_of_txid` so retries can safely re-upload or delete them.

No separate pending marker is required for correctness. Job output lists live in durable workflow state/signals, and deterministic object keys make retry and cleanup idempotent.

Workflow correctness relies on normal durable workflow semantics plus idempotent activities. Do not add special checkpoint machinery beyond the workflow engine's normal persistence. Pure read/build/write-candidate work may live inside one activity as long as retrying the same `job_id` is safe. Manager publish remains a separate FDB compare-and-set step. Reclaimer activities must revalidate safety in their FDB transaction before clearing.

Long activities must be bounded. Cold upload and reclaim jobs should be chunked into activities with clear byte/object limits so shutdown, retry, and stale-generation handling do not wait behind an unbounded S3 operation.

Initial conservative bounds:

- Gasoline signal payloads: at most 64 KiB.
- Workflow job descriptors: at most 256 KiB in durable workflow state.
- FDB install/reclaim transaction batches: at most 500 keys or 2 MiB of values per activity, whichever comes first.
- S3 cold upload activity: at most 64 MiB or one cold shard object.
- S3 delete activity: at most 100 object keys.
- Activity wall time target: split any planned activity expected to exceed 30 seconds.

These are starting constants, not storage semantics. Tune them after implementation metrics show real object sizes and FDB transaction pressure.

## Reclaimer Job Scope

Prefer smaller atomic activities over mixed reclaimer jobs.

Reclaimer work should be split by backend and operation shape:

- FDB hot-row deletes.
- S3 cold-object deletes.
- Manifest/index cleanup.

This gives narrower retries, clearer error handling, and easier idempotency. The DB manager still decides eligibility and sequences jobs; activities execute small planned steps.

FDB and S3 reclaim activities may run in parallel if the reclaimer workflow can await or join multiple activities concurrently. If Gasoline cannot join activities within one workflow, reassess before serializing reclaim work. Reclaimer activities should receive bounded payloads through workflow state. Large batches are split into smaller jobs.

## Migration And Rollback

This code has not shipped, so do not design around backward compatibility with the current standalone compactor state.

Use the best final implementation directly:

- Workflow-managed compaction starts from the new FDB manifest shape, durable workflow state, and deterministic S3 cold shard keys.
- Do not import legacy `in_flight_uuid`, compactor leases, pending markers, partial-upload state, or pending-pin handoff state into workflows.
- Remove or ignore legacy in-flight compactor metadata during implementation.
- If local/dev data exists, reset or one-shot rebuild it rather than preserving a production migration path.

Rollback compatibility with the current compactor architecture is out of scope.

## Legacy Coexistence

Do not build a legacy/workflow coexistence mode. This code has not shipped, so workflow compaction should become the only compaction writer in the final design.

Remove or disable old standalone hot, cold, and eviction mutation paths instead of adding branch-level `legacy | workflow` ownership. Keep FDB manifest generation/CAS for storage correctness, but do not add a separate legacy ownership key.

## Repair And Operator Semantics

FDB published manifest state is the reader-visible source of truth. Workflow state is durable orchestration state, and S3 object existence only proves payload availability.

Repair follows these rules, in precedence order:

- FDB ref exists but S3 object is missing: corruption. Log/alert and repair from retained hot history or backup if possible.
- FDB ref exists for an object whose retired record has `delete_state = DeleteIssued`: corruption. Log/alert because S3 may already have deleted the object.
- Workflow job says completed but FDB did not publish it: candidate output. Revalidate/publish through manager CAS or discard.
- Workflow job is stale against FDB manifest generation: discard job state and replan.
- S3 object exists but FDB, retired-object records, and workflow state do not reference it: orphan. Delete through reclaimer.
- Staged FDB output is not referenced by manifest/workflow: orphan. Conditionally clear.

Every inconsistency path must emit structured logs with database branch id, manifest generation, object/key ref, job id when available, and selected repair action.

## Code Comment Requirements

Implementation should include short comments next to non-obvious safety checks that explain the invariant being protected. This is required for behavior that looks redundant unless the reader understands the concurrency boundary:

- S3 cold-object deletion must be split into FDB retire, grace-window wait, S3 delete, then FDB cleanup because FDB snapshots are MVCC and S3 is not.
- `SQLITE_CMP_DIRTY` clearing must compare against the exact observed value so the manager does not erase a commit's durable wake hint.
- There is no pin-change signal because install/reclaim transactions re-read pins and namespace fork facts at the storage boundary.
- Hot/cold watermarks are scheduling summaries only; DELTA deletion needs exact SHARD/cold replacement coverage.
- Namespace negative proofs need an epoch/fence read in the destructive transaction or must be rerun.
- Conditional deletes such as `CompareAndClear` protect against stale workflow retries, not against ordinary duplicate execution.

## Implementation Order

Implement in this order so each slice has one owner and one failure mode:

1. Add new FDB/S3 key helpers and versioned payload types for `CMP/root`, `CMP/cold_shard`, `CMP/retired_cold_object`, staged hot shards, dirty index records, namespace proof records, and cold refs.
2. Update readers to understand the new published manifest shape. Do not route writes through workflows.
3. Add manager workflow skeleton with stable tags, FDB refresh, dirty-index wake handling, companion workflow spawning, and branch tombstone checks.
4. Add exact signal enums and bounded workflow state for active jobs, retry/backoff, companion workflow ids, and stop state.
5. Implement hot compacter staging plus manager-owned hot install transaction, including materialization at selected pinned/fork-required cut points.
6. Implement cold compacter S3 upload plus manager-owned cold publish transaction, including cold coverage for selected pinned/fork-required cut points.
7. Implement namespace fork proof and DB history pin refresh used by install/delete.
8. Implement manager planning that schedules pinned/fork-required coverage before DELTA reclaim.
9. Implement reclaimer workflow activities for FDB conditional clears, cold-object retirement, grace-window-gated S3 object deletes, and staged orphan cleanup.
10. Remove old standalone hot/cold/eviction mutation paths, leases, pending markers, and `in_flight_uuid`.
11. Rewrite tests around manager-owned publish/delete, stale job rejection, fork/pin races, namespace-derived DB pins, pinned txid materialization before DELTA reclaim, cold read coverage, and branch deletion lifecycle.

## Ownership Boundaries

```text
conveyer/commit path       -> commit-owned FDB keys, dirty-index hint only
db_manager workflow        -> scheduling, manifest publish, delete authorization
db_hot_compacter workflow  -> staged FDB shard construction only
db_cold_compacter workflow -> deterministic S3 payload upload only
db_reclaimer workflow      -> manager-planned physical deletion only
namespace/catalog code     -> namespace fork facts and catalog membership indexes
read path                  -> FDB published manifest plus commit-owned hot state
```

## Job State Machine

Each companion workflow kind allows at most one active job per branch in the initial implementation.

```text
planned -> signaled -> running -> finished -> installing -> installed
                                   |             |
                                   |             -> stale_orphan_cleanup
                                   -> failed_retryable -> planned
                                   -> failed_terminal -> stale_orphan_cleanup
```

The manager owns state transitions after it receives `finished`. Companion workflows only move their local accepted job between `idle`, `running`, and `stopping`; after reporting completion or failure they return to `idle`. No manager ack/reset signal is required.

## Remaining Implementation Details

The high-level architecture is decided, but implementation should still add concrete versioned schemas and constants for:

- manager and companion workflow state
- job input/output fingerprints
- signal payload size and batch limits
- activity byte/object/key-count limits
- namespace proof record payloads and epoch sets
- branch lifecycle generation on `DatabaseBranchRecord`
- cold-object grace-window constants
