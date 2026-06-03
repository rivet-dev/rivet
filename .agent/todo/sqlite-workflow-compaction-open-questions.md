# SQLite Workflow Compaction Open Questions

Track unresolved architecture decisions for `.agent/specs/sqlite-workflow-compaction-architecture.md`.

Terminology: this work now targets the `depot` package. The hot-path role is the conveyer, `compactor` keeps its name, and destructive deletion/eviction questions use the reclaimer role.

## 1. Namespace-Derived Pin Proof

Status: decided

Question: before deleting DB history at or below version `V`, how does the DB manager prove no cheap namespace fork can still inherit that DB at `V`?

Current direction:

- Namespace fork stays O(1).
- DB history deletion resolves unresolved namespace fork facts lazily and materializes DB history pins when needed.
- If the manager cannot produce a bounded negative proof, it retains history.
- Delete planning may do work proportional to relevant namespace descendants for that DB.
- Resolved namespace fork requirements should be cached/materialized as `DB_PIN(kind=NamespaceFork)` records so repeated delete passes do not re-walk the same namespace graph.

Decision details:

- Immutable namespace fork records are `NS_FORK_PIN` and `NS_CHILD`; their values include source namespace branch, target namespace branch, fork versionstamp, and parent cap/versionstamp.
- `NSCAT_BY_DB` lets the DB manager start from namespace branches with explicit database membership instead of scanning every namespace fork.
- Unresolved namespace forks block deletion unless delete-time proof materializes or reads `DB_PIN(kind=NamespaceFork)`.

Candidate shape:

- Keep existing namespace fork parent records as the immutable fork facts: source namespace branch, target namespace branch, fork versionstamp, and parent cap.
- Add an inverse explicit-catalog index keyed by database branch id, e.g. `NSCAT_BY_DB/{database_branch_id}/{namespace_branch_id} -> catalog_versionstamp`.
- Add a namespace child-edge index keyed by source namespace branch, e.g. `NS_CHILD/{source_namespace_branch}/{fork_versionstamp}/{target_namespace_branch}`.
- Optionally add an inverse tombstone index keyed by database branch id if tombstone checks become too expensive.
- During DB history deletion, start from namespace branches where the DB has explicit catalog membership, walk child edges through namespace descendants, and apply the same versionstamp-cap/tombstone visibility rules as normal namespace resolution.
- If a fork could inherit the DB at the deletion version, materialize `DB_PIN/{database_branch_id}/{pin_id(kind=NamespaceFork)}` or block deletion.
- Cache/materialize resolved pins so the manager does not repeat expensive descendant walks every pass.

Decision:

- Preserve O(1) namespace fork.
- Add inverse indexes for delete-time proofs.
- Let deletion resolve relevant namespace descendants lazily.
- If a bounded proof cannot be produced, retain history.

## 2. FDB Manifest Summary Shape

Status: decided

Question: what is the minimal FDB-owned compaction manifest summary needed by readers, operators, recovery, and manager revalidation?

Decision:

- Store published storage truth in FDB.
- Workflows are durable schedulers/executors with reconstructible cache only.
- FDB owns manifest generation, live refs, and pin summaries.
- S3 owns bulky cold shard payload bytes only.
- Commit-owned keys remain direct-to-FDB and do not update manifest generation on every commit.
- Large lists must be chunked in FDB so install/delete transactions stay under FDB limits.

FDB-owned metadata includes:

- `manifest_generation`
- hot materialized watermark
- cold coverage watermark
- live DELTA retention bounds derived from commit-owned DELTA plus watermarks, plus SHARD/cold refs needed by readers and compaction
- DB history pin summaries
- schema version and operator/debug fields

Workflow-owned state is limited to:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected output refs and content hashes
- retry/backoff state
- last processed dirty/timeout cursor

## 3. Cold Manifest Publish Semantics

Status: decided

Question: which reader-visible pointer or index publishes a cold layer, and what fields must be updated atomically with cold coverage?

Decision:

- S3 stores only materialized cold shard payloads at `db/{branch}/shard/{shard_id}/{as_of_txid}-{content_hash}.ltx`.
- FDB stores reader-visible cold refs at `BR/{branch}/CMP/cold_shard/{shard_id}/{as_of_txid}`.
- Publishing a cold job writes cold refs, advances `CMP/root.cold_watermark_*`, and bumps `CMP/root.manifest_generation` in one Serializable FDB transaction.
- Exact pinned bookmarks are represented by publishing shard objects at the pinned txid, not by a separate S3 `pin/` class.

## 4. Job Metadata Location

Status: decided

Question: where do full compaction job inputs, output lists, checksums, and reclaimer batches live if workflow signals only carry small handles?

Decision:

- Store normal job metadata in durable workflow state/signals, not FDB.
- Keep job payloads bounded by splitting hot/cold/reclaimer work into smaller jobs.
- Workflow signals carry job id, base generation, selected ranges, output refs, content hashes, status, and retry state as long as they remain bounded.
- FDB stores published storage state and pin/index state only. It does not store normal job payloads, obsolete queues, or delete authorization logs.

## 5. Workflow Checkpoint Boundaries

Status: decided

Question: which manager and worker state transitions must be checkpointed before/after side effects?

Decision:

- Do not add special checkpoint machinery beyond normal durable workflow semantics.
- Put pure read/build/write-candidate work inside idempotent activities.
- Keep manager publish as a separate FDB CAS step.
- Keep reclaimer work in idempotent bounded activities that revalidate before clearing.
- Design every activity so retrying the same `job_id` is safe.

## 6. Reclaimer Activity Boundaries

Status: decided

Question: how should reclaimer work split physical deletion across FDB row deletes, manifest cleanup, S3 object deletes, and staged orphan cleanup?

Decision:

- Split by backend/failure domain: FDB conditional clears and S3 object deletes are separate bounded activities.
- Run FDB and S3 reclaim activities in parallel if the reclaimer workflow can await/join multiple activities concurrently.
- If Gasoline cannot join parallel activities inside one workflow, reassess before serializing reclaim work.
- FDB reclaim activities revalidate manifest generation, pins, and cold coverage before clearing.
- S3 reclaim activities are idempotent object deletes over manager-planned keys.

## 7. Global Backpressure Owner

Status: out of scope

Question: where are global and per-tenant limits enforced for workflow starts, active jobs, FDB throughput, and S3 throughput?

Decision: disregard backpressure for this spec. Do not block the high-level architecture on global or branch-local backpressure policy.

## 8. Worker Lifecycle And Cardinality

Status: decided

Question: when may the branch manager/worker workflows complete, tombstone, or be recreated, and what branch states count as active?

Decision:

- Gasoline automatically sleeps idle workflows, so live branches keep four persistent workflows: manager, hot worker, cold worker, and reclaimer.
- The manager is the lifecycle authority.
- On database branch deletion, the manager receives the `DestroyDatabaseBranch` signal, observes the FDB-authoritative branch record lifecycle generation, marks the branch stopping/deleted in durable workflow state, stops scheduling new work, and proxies `DestroyDatabaseBranch` signals to hot, cold, and reclaimer workers.
- Workers finish or abandon current bounded/idempotent work according to role, then acknowledge stop and complete.
- Workers do not decide branch liveness themselves and do not publish after the manager has marked the branch stopping/deleted.
- The manager may complete permanently after worker stop acknowledgements and any required final cleanup/ownership release.

## 9. Migration From Current Cold State

Status: decided

Question: how does workflow mode import, discard, or clean up current `in_flight_uuid`, pending markers, partial uploads, and pending pins?

Decision:

- This code has not shipped, so do not design for backward compatibility or legacy rollback.
- Do not import legacy in-flight compactor state into workflows.
- Prefer the best final implementation: workflow-managed compaction state starts from the new FDB manifest shape, durable workflow state, and deterministic S3 shard keys.
- Remove or ignore legacy `in_flight_uuid`, leases, pending markers, partial-upload state, and pending-pin handoff mechanisms as part of the implementation.
- If local/dev data exists during development, reset or one-shot rebuild it rather than preserving a production migration path.

## 10. Old Compactor Coexistence

Status: decided

Question: what branch-level ownership mode prevents old standalone compactors and new workflows from both mutating the same branch?

Decision:

- Do not build a legacy/workflow coexistence mode.
- This code has not shipped, so remove or disable old standalone compactor mutation paths instead of adding branch-level ownership compatibility.
- Workflow compaction is the only compaction writer in the final design.
- Keep FDB manifest generation/CAS for storage correctness, but do not add a separate legacy ownership key.

## 11. Repair And Operator Semantics

Status: decided

Question: if FDB manifest summary, workflow state, and cold storage disagree, which state wins and how does repair converge them?

Decision:

- FDB published manifest state wins for reader-visible truth.
- Workflow state is durable orchestration state, not published storage truth.
- S3 object existence is payload availability, not liveness.
- FDB ref exists but S3 object is missing: treat as corruption, log/alert, and repair from retained hot history or backup if possible.
- Workflow job says completed but FDB did not publish it: treat as candidate output and either revalidate/publish through manager CAS or discard.
- Workflow job is stale against FDB manifest generation: discard job state and replan.
- S3 object exists but FDB and workflow state do not reference it: treat as orphan and delete through reclaimer.
- Staged FDB output not referenced by manifest/workflow: treat as orphan and conditionally clear.
- Log structured inconsistency events for every repair path, including branch id, manifest generation, object/key ref, job id when available, and selected repair action.

## 12. Final Drop List Validation

Status: decided

Question: which legacy mechanisms are removed in the final architecture, and which must remain readable during migration or rollback?

Decision:

- Simplify aggressively because this code has not shipped.
- Remove old standalone hot compactor mutation paths.
- Remove old standalone cold compactor mutation paths.
- Remove old standalone eviction/global deletion mutation paths.
- Remove hot/cold/eviction compactor leases.
- Remove cold `in_flight_uuid` as an FDB handoff mechanism.
- Remove pending markers as crash-recovery state.
- Remove pending-pin handoff state.
- Remove staged cold job metadata outside workflow state.
- Remove old obsolete/delete queue state as a correctness mechanism.
- Remove `last_hot_pass_txid` as an eviction OCC fence.
- Remove legacy dual-read/rollback compatibility paths for the current compactor architecture.
- Keep only storage-boundary correctness mechanisms: FDB transactions/CAS, manifest generation, DB history pins, unresolved namespace fork facts, deterministic S3 keys, workflow job ids, stable workflow tags, and conditional deletes.

## 13. Subagent Review Clarifications

Status: decided

Question: after adversarial review, which ambiguous implementation details needed to be filled in before implementation?

Decision:

- Use `SQLITE_CMP_DIRTY/{database_branch_id}` as a scheduling-only dirty index; do not emit one Gasoline signal per commit.
- Send `DeltasAvailable` immediately on the first dirty-marker takeover, then have the conveyer send periodic throttled `DeltasAvailable` signals while commits keep refreshing the dirty marker.
- Use manager-owned planner deadlines for hot compaction, cold compaction, reclaim, and final settle. `DeltasAvailable` only reduces commit-stream latency; timers guarantee convergence after commits stop.
- Treat workflow signals as at-least-once and out-of-order; include job id, kind, base generation, and fingerprint in run/finish payloads.
- Replace stale `Shutdown`, `StopBranch`, and `WorkerStopped` signal wording with `DestroyDatabaseBranch`.
- Do not let manifest-generation CAS stand alone as a commit fence; installs that reinterpret PIDX/DELTA/head must also Serializable-read the relevant commit-owned keys.
- Make pinned historical reads block hot-history deletion until exact pinned coverage is retained or cold-published.
- Resolve unresolved namespace fork facts inside the physical delete transaction or through proof records validated with namespace proof epochs.
- Serialize cold publish and S3 object delete decisions in manager workflow state so an active/unresolved delete job blocks publishing the same object key.
- Anchor branch lifecycle in FDB on `BRANCHES/list/{database_branch_id}` plus a monotonic lifecycle generation; workflow stopping state is orchestration only.
- Add implementation order, ownership boundaries, and a small job state machine to the spec.
- Use unique cold `object_generation_id`s so retired S3 object keys are never reused by later publish attempts.
- Reclaimer jobs delete exact `ColdShardRef.object_key` values read by the manager from FDB; they must not reconstruct S3 keys from logical shard fields.
- Cached namespace proof records must be validated by reading captured `NS_PROOF_EPOCH` values in the physical delete transaction.
- Cold publish must fence branch liveness plus the exact commit-owned DELTA/COMMITS/VTX inputs covered by the cold output.
- Dirty-index clearing must compare-and-clear the exact observed dirty value/head.
- DELTA reclaim must prove no current PIDX entry still depends on deleted txids, or prove replacement SHARD/cold coverage makes those PIDX entries irrelevant.
- Use one concrete `DB_PIN/{database_branch_id}/{pin_id}` concept for database forks, namespace forks after DB-specific resolution, and bookmarks.
- Keep `NS_FORK_PIN` as an unresolved namespace-level fact so namespace forks stay cheap; compaction must resolve relevant namespace facts before deleting DB history.
- Do not emit pin-change signals. Pin writers only write FDB state; managers re-read `DB_PIN` and unresolved `NS_FORK_PIN` state on `DeltasAvailable`/timeout and before install/reclaim.
- Do not model `Tick` as a durable manager signal. Periodic manager work uses Gasoline timeout patterns such as `listen_n_with_timeout`, `listen_n_until`, `listen_with_timeout`, or `ctx.sleep`.
- Rename manager wake hints to `DeltasAvailable`; periodic work is a listen timeout, not a durable signal or busy loop.
- Treat DB history pins as compaction coverage cut points, not only delete blockers.
- Manager planning should proactively materialize SHARD/cold coverage at pinned/fork-required txids before authorizing DELTA reclaim.
- Preserve current read-path precedence: PIDX/DELTA first for normal hot reads, then SHARD fallback, then cold refs, with pinned/fork-required versions requiring exact replacement coverage before DELTA deletion.
- Use `job_id` as `object_generation_id` for cold S3 object keys.
- Prefer per-source-namespace `NS_PROOF_EPOCH/{source_namespace_branch_id}`; fall back to per-tree epoch only if implementation cannot cheaply update source epochs.
- Reuse `BRANCHES/list/{database_branch_id}` as branch lifecycle authority and add a monotonic lifecycle generation to the branch record instead of adding a detached liveness key.
- Start with conservative bounds: 64 KiB signals, 256 KiB job descriptors, 500 FDB keys or 2 MiB per FDB activity, one 64 MiB S3 upload object, 100 S3 deletes, and 30 second target activity chunks.
