# SQLite Workflow Compaction Open Questions

Track unresolved architecture decisions for `.agent/specs/sqlite-workflow-compaction-architecture.md`.

Terminology: this work now targets the `depot` package. The hot-path role is the conveyer, `compactor` keeps its name, and destructive deletion/eviction questions use the reclaimer role.

## 1. Namespace-Derived Pin Proof

Status: decided

Question: before deleting DB history at or below version `V`, how does the DB manager prove no cheap namespace fork can still inherit that DB at `V`?

Current direction:

- Namespace fork stays O(1).
- DB history deletion resolves namespace-derived pins lazily.
- If the manager cannot produce a bounded negative proof, it retains history.
- Delete planning may do work proportional to relevant namespace descendants for that DB.
- Resolved namespace pins should be cached/materialized as `DB_NS_PIN` records so repeated delete passes do not re-walk the same namespace graph.

Need to decide:

- What immutable records exist for namespace fork, DB catalog membership, and DB tombstone windows.
- Which index lets the DB manager check relevant namespace forks without scanning every namespace fork.
- Whether unresolved namespace forks materialize `DB_NS_PIN` eagerly during delete planning or simply block deletion.

Candidate shape:

- Keep existing namespace fork parent records as the immutable fork facts: source namespace branch, target namespace branch, fork versionstamp, and parent cap.
- Add an inverse explicit-catalog index keyed by database branch id, e.g. `NSCAT_BY_DB/{database_branch_id}/{namespace_branch_id} -> catalog_versionstamp`.
- Add a namespace child-edge index keyed by source namespace branch, e.g. `NS_CHILD/{source_namespace_branch}/{fork_versionstamp}/{target_namespace_branch}`.
- Optionally add an inverse tombstone index keyed by database branch id if tombstone checks become too expensive.
- During DB history deletion, start from namespace branches where the DB has explicit catalog membership, walk child edges through namespace descendants, and apply the same versionstamp-cap/tombstone visibility rules as normal namespace resolution.
- If a fork could inherit the DB at the deletion version, materialize `DB_NS_PIN/{database_branch_id}/{at_versionstamp}/{target_namespace_branch}` or block deletion.
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
- live DELTA/SHARD/cold root refs needed by readers and compaction
- direct and namespace-derived pin summaries
- schema version and operator/debug fields

Workflow-owned state is limited to:

- active hot/cold/reclaimer jobs
- selected job input ranges
- expected output refs and content hashes
- retry/backoff state
- last processed dirty/tick cursor

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
- On database branch deletion, the manager receives the deletion/tombstone signal, marks the branch stopping/deleted in durable workflow state, stops scheduling new work, and proxies `StopBranch` signals to hot, cold, and reclaimer workers.
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

Status: pending

Question: what branch-level ownership mode prevents old standalone compactors and new workflows from both mutating the same branch?

## 11. Repair And Operator Semantics

Status: pending

Question: if FDB manifest summary, workflow state, and cold storage disagree, which state wins and how does repair converge them?

## 12. Final Drop List Validation

Status: pending

Question: which legacy mechanisms are removed in the final architecture, and which must remain readable during migration or rollback?
