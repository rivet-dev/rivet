# 0009: wake the writer, compactor and worker on local metadata writes

**What it changes.** `SequencedStore` (from `0008-boundary-check-reuse.patch`) announces every manifest and `.compactions` version written through it. Three components that share those stores inside one database now run their existing poll when another of them writes, instead of only on their ticker:

- the compactor runs its manifest poll when the writer writes a manifest (a flush added L0 SSTs);
- the embedded compaction worker runs its `.compactions` poll when the coordinator writes one (a job was scheduled);
- the writer runs its manifest refresh when the compactor writes a manifest (a compaction committed).

Each component ignores versions it already has, which covers its own writes. Notices are coalesced to at most one per second per component (or per poll interval, if shorter). The tickers remain as the fallback for writers in other processes.

**Why.** A flush becomes drained L0 only after three polls in a row, so the poll interval set both the idle request rate and how fast L0 drains. With the notices L0 drains at compaction speed, so the polls stay at 5 s only to bound how long a write from another process goes unseen; L0 drain no longer depends on them. Measured with `cold_store_requests` in `engine/packages/depot-worker/tests/unit`. Tests: `sequenced_store::tests::notices_*` and `compactor_drains_l0_without_polling`.

**Why it cannot lose or resurrect data.** A notice only runs a handler that the component's ticker already runs at arbitrary times: the writer's `refresh_manifest_progress`, the compactor's `handle_ticker` and the worker's `poll_and_claim`. Skipping a notice for a version already seen is the same as no tick. No write path, epoch check, invariant or GC rule changes, and the read each handler issues is the same checked read. Compaction results still become visible to the writer only through the merge it already does on refresh, and a fenced writer still fails that refresh or its next manifest write as before. The only new state is a `watch` channel holding the highest version number written, which carries no data.

**Upstream.** Nothing equivalent in 0.17; components in one process learn of each other's writes only by polling. Candidate for upstream.

**Drop when.** Upstream wakes in-process components on local writes, or Depot runs the compactor in another process.
