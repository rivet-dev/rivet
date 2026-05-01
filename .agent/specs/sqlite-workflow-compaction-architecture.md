# SQLite Workflow Compaction Architecture

High-level proposal for replacing the current standalone hot/cold/eviction compactor services with durable Gasoline workflows. The goal is to simplify concurrency by making compaction decisions stateful and durable while keeping the SQLite commit path direct to FDB.

Terminology: this targets the `depot` package. The hot-path component is the conveyer, `compactor` keeps its name, and physical deletion/eviction work is the reclaimer role.

This is an architecture spec, not an implementation plan. It intentionally avoids detailed file-by-file changes.

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
- Do not specify exact Rust module layout yet.

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
sqlite_db_manager        tags: database_branch_id
sqlite_hot_worker        tags: database_branch_id
sqlite_cold_worker       tags: database_branch_id
sqlite_reclaimer_worker  tags: database_branch_id
```

The manager is the authority. Conveyer commits stay direct to FDB. Hot compaction, cold upload, and reclaimer physical deletion run in persistent worker workflows so long cold/reclaim activities do not block hot scheduling or manager signal handling.

```text
Commit path -> FDB directly
            -> coalesced wake marker for db-manager

db-manager -> signal hot-worker RunHotJob
db-manager -> signal cold-worker RunColdJob
db-manager -> signal reclaimer-worker RunReclaimJob

workers    -> signal db-manager JobFinished(job_id)
```

Different branches can run independently. A branch workflow group may be started lazily on first commit/access and may sleep when idle. Persistent manager/worker workflows should not complete while the branch is live; completing clears workflow tag indexes and makes stored workflow ids unsafe for future signals.

Do not spawn one workflow per compaction job. Workflow creation is relatively expensive; persistent branch workers preserve parallel hot/cold/reclaim progress without per-job workflow churn.

## Authority Split

### DB Manager

The DB manager owns decisions about the branch's durable storage manifest:

- Observe committed head and hot lag.
- Track live DELTA, SHARD, and cold shard refs.
- Track direct database pins and namespace-derived pins.
- Schedule hot, cold, and reclaimer jobs.
- Install worker outputs after revalidation.
- Decide when old hot/cold state is deletable.
- Persist the branch manifest/version-set edits.

The DB manager is the only workflow that can say:

- this SHARD output is live
- this cold shard ref is official
- these DELTAs are obsolete
- these SHARDs/cold objects are deletable
- this reclaimer job is authorized

### Hot Work

The hot worker builds FDB-internal compaction output:

- Receives `RunHotJob`.
- Reads the job's selected DELTAs/SHARD inputs.
- Builds candidate SHARD output under deterministic staging keys.
- Signals `HotJobFinished`.

It does not write reader-visible SHARD keys, does not mark DELTAs obsolete, and does not decide what can be deleted.

### Cold Work

The cold worker uploads cold-tier output:

- Receives `RunColdJob`.
- Uploads materialized cold shard payloads to deterministic S3 keys.
- Signals `ColdJobFinished` with bounded output refs, sizes, and content hashes.

It does not publish reader-visible cold refs, does not advance the cold watermark by itself, and does not mark pins `Ready` by itself.

### Reclaimer Work

The reclaimer worker performs physical deletes:

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
2. If hot or cold lag crosses a configured threshold, commit emits a rate-limited Gasoline wake signal or updates a scheduling-only dirty index.
3. A periodic sweeper/tick also wakes managers for branches with stale lag so lost best-effort signals cannot stall compaction forever.

The wake signal is only a hint. The manager must refresh from FDB before scheduling or installing work.

## Manager State

FDB stores authoritative published storage metadata. Durable workflow state owns in-flight scheduling state.

FDB-owned metadata includes:

- `manifest_generation`
- current hot materialization watermark
- current cold coverage watermark
- live DELTA ranges
- live SHARD versions
- live cold shard refs and small summaries
- direct pin summaries from database forks/bookmarks
- namespace-derived pin summaries
- schema version and operator/debug fields

Workflow-owned state includes:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected job output refs and content hashes
- retry/backoff state
- last processed dirty/tick cursor

On startup or periodic tick, the manager refreshes FDB/cold-tier state and rebuilds any missing cache. If workflow state and FDB disagree about published compaction metadata, FDB wins. Workers reconcile active jobs against manager state and current manifest generation before reporting or retrying output.

## Compaction Jobs

Every compaction job has:

- `job_id`
- `database_branch_id`
- `base_manifest_generation`
- input ranges/layers
- deterministic staging output keys
- expected output checksums or content fingerprints

Activities are idempotent. Retrying a step with the same `job_id` writes the same output keys or no-ops.

Normal job inputs, output refs, content hashes, and retry state live in durable workflow state/signals, not FDB. Cold shard bytes live in S3 because they are the bulky payload. The manager handles duplicate job completion signals by checking the job id against its workflow state and current FDB manifest generation. Job payloads must remain bounded; split work into smaller jobs if inputs or outputs would become large.

## Install Rules

Worker output is only a candidate until the DB manager installs it.

Workers must write to staging namespaces that readers do not consult. Current readers treat live SHARD keys and cold manifest indexes as reader-visible, so the publish step must be manager-owned:

- Hot workers write staged SHARD output under job-scoped staging keys.
- Cold workers write cold shard payload objects to S3 and report bounded output refs/content hashes through workflow signals.
- The manager install transaction publishes live FDB SHARD/manifest summary updates.
- The manager alone rewrites or advances any reader-visible cold manifest pointer/index.

Before installing any hot or cold output, the manager must:

- refresh relevant FDB state
- verify `base_manifest_generation` is still compatible
- recompute current direct pins
- check namespace-derived pins
- confirm the output does not obsolete history at or below the pin floor
- validate staged output checksums or content fingerprints

If the result is stale, the manager discards it or treats it as orphan output to clean later.

Every manifest install is a Serializable FDB transaction that reads the current manifest generation and writes the next generation. If the generation changed, the install aborts and the manager replans.

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

The commit transaction may emit a rate-limited wake signal or update a scheduling-only dirty index, but it does not update manifest generation. Wake state is not part of storage correctness.

### Published Compaction Manifest

The DB manager owns published compaction state:

```text
BR/{database_branch_id}/CMP/root
BR/{database_branch_id}/CMP/cold_shard/{shard_id_be}/{as_of_txid_be}
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

### Workflow Job State

Workflow jobs are tracked in durable workflow state and signals. FDB is not the normal store for in-flight job bookkeeping.

```text
BR/{database_branch_id}/CMP/stage/{job_id}/hot_shard/{shard_id_be}/{as_of_txid_be}/{chunk_idx_be}
```

`CMP/stage/.../hot_shard` is for staged hot shard payloads that cannot be reader-visible until manager publish. Oversized job payloads are invalid and should be split before scheduling.

### Reclaim Derivation

Obsolete data is derived from the current published manifest, hot/cold watermarks, direct pins, namespace-derived pins, and cold coverage. FDB does not store a separate obsolete queue or delete authorization log in the normal design.

The manager may enqueue bounded reclaimer jobs in workflow state. Reclaimer activities re-read manifest generation and pin/cold-coverage predicates in the delete transaction before clearing. FDB deletes use conditional semantics when stale delete could remove changed data.

### Pins And Namespace-Derived Pin Index

Existing direct branch pins remain authoritative:

```text
BRANCHES/{database_branch_id}/desc_pin
BRANCHES/{database_branch_id}/bk_pin
BRANCHES/{database_branch_id}/pin_count
```

Namespace-derived pin proof uses inverse indexes:

```text
NS_FORK_PIN/{source_namespace_branch_id}/{fork_versionstamp}/{target_namespace_branch_id}
NS_CHILD/{source_namespace_branch_id}/{fork_versionstamp}/{target_namespace_branch_id}
NSCAT_BY_DB/{database_branch_id}/{namespace_branch_id}
DB_NS_PIN/{database_branch_id}/{at_versionstamp}/{target_namespace_branch_id}
```

`NSCAT_BY_DB` maps explicit database membership to namespace branches. `NS_CHILD` lets delete planning walk relevant namespace descendants. `DB_NS_PIN` caches resolved namespace-derived database pins.

## S3 Key Layout

S3 stores only bulky cold payload bytes. FDB stores the authoritative object refs, checksums, and publish state.

S3 object keys are immutable and deterministic by logical cold artifact plus content hash. Cold workers write directly to the final object key after computing the payload hash. The object is not reader-visible until the DB manager publishes an FDB `ColdShardRef` to it. Publishing does not rename, copy, or mutate S3 objects.

All keys are relative to the configured cold-tier root:

```text
db/{database_branch_id_hex}/
  shard/
    {shard_id_be_hex:8}/
      {as_of_txid_be_hex:16}-{content_hash_hex}.ltx
```

Each object contains one materialized shard image at one `as_of_txid`. Exact pinned bookmarks are represented by publishing shard objects at the pinned txid, not by a separate `pin/` object class.

The content hash in the key prevents stale workers from overwriting a published object with different bytes. Retries that produce identical bytes converge on the same key. Retries that produce different bytes create a different orphan object that is not live unless the manager publishes its matching FDB ref.

FDB cold refs point at these objects:

```text
ColdShardRef {
  object_key,
  shard_id,
  as_of_txid,
  min_txid,
  max_txid,
  min_versionstamp,
  max_versionstamp,
  size_bytes,
  content_hash,
}
```

The branch id is part of the key because cold objects belong to a database branch's history graph. The shard id and `as_of_txid` are part of the key because retries for the same logical output should converge on the same object. The content hash appears in both the S3 key and FDB ref so readers and cleanup jobs can verify the exact payload.

No S3 key encodes live or published state. An object is live only if the current FDB manifest generation references it.

## Reclaim Rules

Physical deletion must remain conservative.

The manager may schedule a reclaimer job only after recomputing:

- direct DB pin floor
- namespace-derived pin floor
- cold coverage
- current manifest generation

FDB deletes must use conditional semantics such as `CompareAndClear` when a stale delete could otherwise remove a value that was rewritten since planning. Workflow idempotency handles duplicate execution; conditional deletes handle stale execution.

Reclaimer jobs carry expected values, versionstamps, or content fingerprints. The reclaimer activity re-checks manifest generation, pin floor, and cold coverage in the delete transaction before clearing. Avoid `clear_range` unless the same transaction has a fence that proves every key in the range is still safe to remove.

S3 deletes are idempotent and should target only cold shard object keys listed in manager-planned reclaimer jobs.

## Namespace Forks

Namespace forks must stay cheap. Do not eagerly fan out DB pins to every inherited database.

On namespace fork, write one namespace-derived pin record:

```text
NS_FORK_PIN/{source_namespace_branch}/{at_versionstamp}/{target_namespace_branch}
```

The namespace manager owns namespace catalog pinning. DB managers own database history deletion.

When a forked namespace accesses a database, or when a DB manager wants to delete old history, materialize or check DB-specific namespace pins lazily:

```text
DB_NS_PIN/{database_branch_id}/{at_versionstamp}/{target_namespace_branch}
```

Safety rule:

Before deleting database history, the DB manager must either prove no unresolved namespace fork pin can include this database or conservatively retain the history. Deleting first and discovering the namespace-derived pin later is not allowed.

"Resolve on delete" means the DB manager checks namespace-derived pins before authorizing deletion of SHARD, DELTA, cold layer, or history rows. The manager asks: can any cheap namespace fork still inherit this database at the version being deleted? If yes, the manager materializes `DB_NS_PIN` or keeps the history. If no, deletion may proceed.

The preferred shape is:

- Keep namespace fork O(1) by writing only the namespace-level pin at fork time.
- Lazily materialize DB-specific namespace pins on database access and before DB history deletion.
- Treat unresolved namespace pins as retention blockers until proven irrelevant.

Decision: resolve namespace-derived pins on delete. The DB manager pays the namespace-pin lookup cost only when it is about to delete database history, not when the namespace fork is created.

The deletion proof must be bounded. It cannot rely on unbounded scans of namespace forks or current namespace visibility. The namespace-derived pin index must encode enough immutable fork context to prove whether a database was inheritable at the fork version, including the source namespace branch, target namespace branch, fork versionstamp, and the catalog/tombstone versionstamp window used for the proof. If the manager cannot produce a bounded negative proof, it must retain the history.

## Signals

Representative DB manager signals:

```text
WakeRequested
DirectPinChanged
NamespacePinMaterialized { at_versionstamp }
HotJobFinished { job_id, status, output_refs }
ColdJobFinished { job_id, status, output_refs }
ReclaimJobFinished { job_id, status }
Tick
Shutdown
```

Representative worker signals:

```text
RunHotJob { job_id, base_generation, inputs }
RunColdJob { job_id, base_generation, inputs }
RunReclaimJob { job_id, base_generation, inputs }
Shutdown
```

Signals are wakeups and durable job messages, not trusted truth. Workflows re-read the required FDB/cold-tier state before publishing, retrying, or deleting.

Use batch listening where signal volume can be high, following the `listen_n` pattern in runner pool workflows. Batch listening is not publish-time coalescing; wake admission must happen before signals are written to Gasoline.

## What Gets Dropped

The workflow model replaces same-role compactor coordination:

- Drop the hot compactor lease.
- Drop the cold compactor lease.
- Drop the eviction global lease for per-branch correctness.
- Drop cold `in_flight_uuid` as a separate FDB handoff concept.
- Drop pending marker as the primary crash-recovery mechanism.
- Drop `last_hot_pass_txid` as the eviction OCC fence.
- Drop eviction's global scan as the primary deletion authority.

Some concepts remain in simpler form:

- `in_flight_uuid` becomes workflow `job_id`.
- Pending marker is not required. Trust durable workflow job state for recovery and cleanup.
- Eviction becomes manager-authorized physical deletion.
- Leases become workflow uniqueness/ownership, not compactor-specific keys.

## What Remains

Keep the mechanisms that protect data correctness at the storage boundary:

- FDB Serializable reads for commits and pin/cap checks.
- FDB atomic operations for `ByteMin`, `Add`, and versionstamped writes.
- Conditional physical deletes such as `CompareAndClear`.
- Direct database pins: `desc_pin`, `bk_pin`, `pin_count`.
- Namespace-derived pin index for cheap namespace forks.
- Deterministic S3/FDB output keys for idempotent workflow steps.
- Manager generation checks for stale worker output.
- FDB manifest-generation compare-and-set on every install/delete decision.

## Workflow Ownership

Use stable workflow tags, not derivable workflow ids:

```text
sqlite_db_manager        tags: database_branch_id
sqlite_hot_worker        tags: database_branch_id
sqlite_cold_worker       tags: database_branch_id
sqlite_reclaimer_worker  tags: database_branch_id
```

Use Gasoline `.unique()` with exactly the stable branch tag for each workflow type. Do not include job id, generation, worker state, or other volatile tags on unique workflows. The DB manager is responsible for spawning the hot, cold, and reclaimer worker workflows for its branch and storing their returned workflow ids in manager state.

Signals from manager to workers may use stored workflow ids while the workflows are live. External wakeups should target the manager by workflow type plus stable tags or go through a helper that creates the unique workflow if missing.

Decision: rely on workflow uniqueness and stable tags for manager ownership. Do not add an FDB manager epoch unless future implementation work proves Gasoline uniqueness is insufficient.

Persistent manager and worker workflows should sleep/listen while idle instead of completing. If a workflow does complete, the next signal path must recreate it by workflow type plus stable tags; do not publish to stale raw workflow ids.

## Branch Deletion Lifecycle

Gasoline handles idle sleeping automatically. The four branch workflows remain persistent while the database branch is live.

The manager is the lifecycle authority for branch deletion:

1. The manager receives the branch deletion/tombstone signal.
2. The manager records stopping/deleted state in durable workflow state.
3. The manager stops scheduling new hot, cold, and reclaimer work.
4. The manager sends `StopBranch` signals to the hot, cold, and reclaimer workers.
5. Workers finish or abandon current bounded/idempotent work according to role, acknowledge stop, and complete.
6. The manager completes after worker stop acknowledgements and any required final cleanup or ownership release.

Workers must not decide branch liveness themselves. Workers also must not publish output after the manager has marked the branch stopping/deleted.

## Backpressure

Backpressure is out of scope for this spec. Do not make global or branch-local backpressure policy part of the storage-correctness architecture.

## Manifest State Placement

Decision: store published storage truth in FDB. Store in-flight job orchestration and delete planning in durable workflow state.

FDB owns:

- manifest generation
- hot materialization watermark
- cold coverage watermark
- live DELTA ranges and SHARD versions
- live cold layer root refs and small summaries
- direct database pin summaries
- namespace-derived pin summaries
- schema version and operational debug fields

Workflow state owns:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected output refs and content hashes
- retry/backoff state
- last processed dirty/tick cursor

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
- direct and namespace-derived pin summaries
- schema version

If a future summary field is derived from each commit, the commit transaction must update it or the field must be documented as a cache that the manager can rebuild.

All manager writes to FDB compaction metadata use Serializable read of the current manifest generation followed by generation-advancing write when they publish live state. Operators and repair jobs that mutate this metadata must follow the same compare-and-set contract.

## Recovery

Workflow state is durable, but FDB owns compaction metadata. Crash recovery is FDB-first reconciliation:

1. Refresh FDB head, compact state, pins, and cold coverage.
2. Reload manager and worker workflow state.
3. Reconcile active jobs against the current FDB manifest generation.
4. Keep compatible completed outputs.
5. Schedule cleanup for orphan job outputs.
6. Resume scheduling.

Cold S3 objects should be deterministic by branch, shard, and `as_of_txid` so replay can safely re-upload or delete them.

No separate pending marker is required for correctness. Job output lists live in durable workflow state/signals, and deterministic object keys make retry and cleanup idempotent.

Workflow correctness relies on normal durable workflow semantics plus idempotent activities. Do not add special checkpoint machinery beyond the workflow engine's normal persistence. Pure read/build/write-candidate work may live inside one activity as long as retrying the same `job_id` is safe. Manager publish remains a separate FDB compare-and-set step. Reclaimer activities must revalidate safety in their FDB transaction before clearing.

Long activities must be bounded. Cold upload and reclaim jobs should be chunked into activities with clear byte/object limits so shutdown, retry, and stale-generation handling do not wait behind an unbounded S3 operation.

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

## Open Questions

- Old compactor coexistence and branch-level ownership mode.
- Repair and operator semantics when FDB, workflow state, and S3 disagree.
- Final validation of removed legacy mechanisms versus migration/rollback compatibility.
