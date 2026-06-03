# SQLite Optimization Merge Plan

Date: 2026-05-02

This is the high-level plan for extracting the SQLite performance work into mergeable PRs. The current optimization branch is useful as a prototype, but the final stack should be moved over as close to 1:1 as practical, split into smaller PRs with clearer boundaries and chat/tool-like benchmark coverage.

The target stack is a new Graphite stack based on `04-29-feat_sqlite_pitr_forking`. Each feature should be transplanted into its own new Graphite branch on top of that base, staying as close to the reference implementation as possible while adapting to the current base branch and removing scope we explicitly cut.

## Core Branches

- `04-28-feat_sqlite_benchmark_cold_reads`: keep as the starting point for cold-read benchmark coverage.
- `04-29-feat_sqlite_add_cold_read_benchmarks_and_simplify_optimizations`: treat as the prototype/source branch for optimization ideas, not as a merge-as-is branch.
- `ralph/sqlite-vfs-pool`: use as the plan source for the read/write connection manager and read pool work.
- `04-29-feat_sqlite_pitr_forking`: target Graphite base branch for the clean reimplementation stack.
- `04-29-chore_sqlite_stateless_storage_refactor`: out of scope for the first optimization merge stack unless a storage API dependency is unavoidable.

## Reference Commits

Use these commits as implementation references for a near-1:1 code move. The plan is to transplant the working implementation into new Graphite branches on top of `04-29-feat_sqlite_pitr_forking`, not to redesign from scratch. Do not merge the prototype stack as-is; split the code by feature, adapt it to the new base, and intentionally omit scope listed under Remove.

### Benchmark And Matrix References

- `3a4c61019`: initial SQLite cold-read benchmark.
- `7043b3859`: baseline benchmark artifact.
- `44321a829`: split cold wake from cold full read.
- `8f5a23104`: compacted vs uncompacted cold-read benchmark coverage.
- `0b572166b`: cold-read benchmark simplification and optimization matrix prototype.
- `a1476932d`: kitchen-sink benchmark coverage for read-pool work.

### VFS And Storage Optimization References

- `f26167e52`: central SQLite optimization feature flags.
- `262bb7817`: record VFS predictor access on cache hits.
- `fa20fd4b3`: larger bounded read-ahead for forward scans.
- `ed57f6f1c`: adaptive forward-scan read-ahead.
- `298ede2b1`: bidirectional scan read-ahead.
- `3943b4f30`: recent-page preload hint tracker.
- `b46e4bf83`: persist recent-page preload hints through envoy-client.
- `67ba1b9ed`: flush preload hints periodically and on actor stop.
- `e8567064b`: use persisted preload hints on actor start.
- `2bf54e22a`: configurable startup preload policy.
- `f71ee1c2f`: configurable and scan-resistant VFS page cache policy.
- `88318e19c`: remove duplicate get-pages meta reads.
- `a4a33c0f3`: cache repeated get-pages actor validation and open checks.
- `08af201f4`: sqlite-storage contiguous range read.
- `a764f0f9e`: range page-read protocol spec.
- `b324c62d8`: wire range get-pages through envoy protocol.
- `1544a7e79`: use range reads from the VFS for forward scans.
- `72b4a0e85`: reduce chunked-value read amplification.
- `f7359e02c`: reduce whole-blob LTX decode amplification.

### Read Pool References

- `2a9c74067`: SQLite statement classification helpers.
- `5262548cc`: split VFS ownership from SQLite connections.
- `ffb06ab13`: enforce read-only VFS roles.
- `9f64ea3c8`: connection manager mode gate.
- `bd4a57f1e`: route write work through exclusive write mode.
- `1f4f90844`: execute read-only statements on read connections.
- `ce839134c`: native execute result API.
- `935de29c4`: remove TypeScript read serialization.
- `bf686a67b`: read-pool config flags and metrics.
- `3931f3006`: lifecycle and fencing stress coverage.
- `95f7914fd`: read-mode/write-mode invariant documentation.

### Older Prototype Reference

- `919e1cba0` on `ralph/sqlite-vfs-pool`: older SQLite VFS pool and batch-atomic prototype. Use only for historical context if the newer `US-*` read-pool commits are insufficient.

## Keep

- Keep the benchmark harness and feature-flag matrix runner.
- Branch 1 should copy over the current chat/tool benchmark additions: recent chat reads, indexed chat reads, count/sum aggregates, parallel fan-out reads, and matrix report output.
- Keep central SQLite optimization flags so every landed optimization defaults on and can be disabled by env for benchmark ablation or emergency diagnosis.
- Keep VFS/storage metrics for cache hits, misses, fetched pages, fetched bytes, get-pages calls, read-pool routing, and mode transitions.
- Keep read-ahead, range reads, storage read cache, and preload hints in the planned stack.
- Keep preload hint types independently configurable so early pages, hot pages, scan ranges, first-page preload, and byte budget can be benchmarked separately.
- Keep statement classification as a native SQLite concern.
- Keep the native single-statement execution path.
- Keep the read/write connection manager.
- Keep parallel read-only connections, with the invariant that write mode has exactly one writable connection and no readers.
- Keep removal of TypeScript-side read serialization once native routing owns read/write policy.
- Keep correctness tests for read/write mode transitions, reader close-before-write behavior, and failed mutation attempts while readers are open.

## Remove

- Remove public multi-statement SQL execution from the optimized database API.
- Remove the TypeScript fallback that detects semicolons and routes multi-statement SQL through `exec`.
- Remove the expectation that `db.execute("BEGIN; INSERT ...; COMMIT")` works as one call.
- Remove read-pool handling for multi-statement SQL. It should fail clearly instead of routing conservatively.
- Remove or defer protected/adaptive cache policy unless a targeted chat/tool matrix shows it is a major win.
- Remove or replace preload-hint persistence as implemented if it cannot distinguish early pages from hot pages.
- Remove any VFS page-cache mode matrix that exists only to benchmark internal cache-policy variants we do not plan to ship.

## Planned Optimization Branches

- Read-ahead is part of the first optimization stack and must default on with an env disable.
- Range page reads are part of the first optimization stack and must default on with an env disable.
- Storage read cache is part of the first optimization stack and must default on with env disables for decoded LTX cache and chunk-read batching.
- Preload hints are part of the first optimization stack, but must use separate early-page, hot-page, and scan-range buckets.
- Preload hint types must be independently configurable so the matrix can run early-only, hot-only, range-only, and combined preload comparisons.
- Startup preload must keep a strict byte budget and default on with env disables for first-page preload, persisted hints, hot pages, early pages, and scan ranges.
- Adaptive read-ahead may land if it remains simple and default-on; otherwise land bounded read-ahead first and keep adaptive tuning behind the same disable path.

## Missing Work We Need

- Implement true early-page hints. Persist early pages separately from hot pages so startup can prefer pages touched immediately after wake.
- Extend the preload hint schema from `pgnos` and `ranges` to separate buckets such as `early_pgnos`, `hot_pgnos`, and `ranges`.
- Add first-hit tracking in the VFS. Record the first N unique target pages after open/wake before normal hot-page scoring takes over.
- Persist preload hints on sleep/close with generation fencing, then feed them into `OpenConfig` on the next actor start.
- Add env/config switches for preload hint types: early pages, hot pages, scan ranges, first pages, persisted hints on open, hint flushing, and max preload bytes.
- Add a negative test that multi-statement SQL is rejected with a clear error.
- Update driver fixtures that currently rely on multi-statement SQL to use explicit sequential calls.
- Copy the chat/tool-like workload implementation into Branch 1: recent chat reads, indexed chat reads, count/sum aggregates, and parallel fan-out reads.
- Prove the read pool with counters: routed reads must be nonzero, reader opens must exceed one under parallel load, and write transitions must close readers first.
- Rebuild `docs-internal/engine/SQLITE_OPTIMIZATIONS.md` on the new stack so it reflects only the features that actually land.
- Use the matrix against cold-read, aggregate, random lookup, read/write transition, and chat/tool-like workloads to tune each optimization while keeping it default-on and env-disableable.

## Single-Statement SQL Policy

- `db.execute("SELECT ...")` works.
- `db.execute("INSERT ...")` works.
- `db.execute("BEGIN")`, `db.execute("COMMIT")`, and `db.execute("ROLLBACK")` work as individual calls.
- `db.execute("CREATE TABLE ...")` works as one call.
- `db.execute("CREATE TABLE ...; CREATE INDEX ...")` fails.
- Migration runners must split migration files into individual statements before calling the database API.

## Recommended PR Order

- Branch 1: Copy the benchmark changes from this prototype branch: matrix runner, report output, chat/tool-like workload coverage, and per-branch benchmark artifact writing.
- Branch 2: Metrics and central optimization flags. Landed optimizations default on, with env toggles to disable them.
- Branch 3: Native single-statement execution and multi-statement rejection.
- Branch 4: Statement classification tests and native read/write routing primitives.
- Branch 5: Read/write connection manager without public parallelism enabled.
- Branch 6: Remove TypeScript read serialization so native routing owns SQL concurrency.
- Branch 7: Parallel read pool default-on, with metrics proving routing and mode transitions.
- Branch 8: Read-ahead default-on, with env disable and matrix artifacts showing chat/history and aggregate impact.
- Branch 9: Range reads default-on, with env disable and matrix artifacts showing scan, aggregate, fan-out, and random-lookup impact.
- Branch 10: Storage read cache default-on, with separate env disables for chunk-read batching and decoded LTX cache.
- Branch 11: Preload hints default-on, with separately configurable early-page, hot-page, scan-range, first-page, persisted-hint, and byte-budget controls.
- Branch 12: Rebuild `docs-internal/engine/SQLITE_OPTIMIZATIONS.md` from the landed stack and remove prototype-only options.

## Future Exploration

- The chat/tool matrix landed in the prototype branch and should be copied into Branch 1 of the clean stack.
- In the chat/tool smoke matrix, disabling range reads was 15.3% slower on average, but the fan-out workload was 8.7% faster with range reads disabled.
- In the chat/tool smoke matrix, disabling read-ahead was 10.5% slower on average and nearly neutral for fan-out.
- In the chat/tool smoke matrix, disabling storage read cache was 4.2% slower on average, but the fan-out workload was 12.9% faster with storage read cache disabled.
- In the chat/tool smoke matrix, disabling preload was 0.6% faster on average, which is not enough to justify moving preload as implemented.
- The read-pool row in the chat/tool matrix is not valid proof yet because `routed_reads=0`; Branch 7 must prove this with native counters.
- Range reads are included in the planned stack, but the prior matrix shows mixed workload impact that Branch 9 must record.
- In the smoke matrix, disabling range reads improved `random-point-lookups` by 25.9%, `migration-create-indexes-large` by 15.1%, and `parallel-read-write-transition` by 7.7%.
- In the same run, disabling range reads hurt `secondary-index-scattered-table` by 22.8%, `write-batch-after-wake` by 9.2%, and `parallel-read-aggregates` by 7.6%.
- This suggests range reads may help scattered/index or write-after-wake workloads, but may overfetch or add overhead for random lookup and migration workloads.
- Read-ahead is included in the planned stack, but the prior matrix shows mixed workload impact that Branch 8 must record.
- In the smoke matrix, disabling read-ahead improved `small-schema-read` by 36.6%, `random-point-lookups` by 16.2%, `migration-create-indexes-large` by 15.3%, and `parallel-read-write-transition` by 6.5%.
- In the same run, disabling read-ahead hurt `rowid-range-forward` by 17.0%, `aggregate-status` by 9.7%, and `parallel-read-aggregates` by 2.3%.
- This suggests read-ahead may help rowid scans and aggregate scans, but can hurt random, tiny, and migration-shaped workloads.
- Revisit range reads, read-ahead, storage read cache, and preload tuning after the full stack lands and the matrix can measure real parallel read routing.

## Per-Branch Benchmark Artifacts

- Every feature branch must record benchmark metrics to files before moving to the next branch.
- Store branch benchmark artifacts under `.agent/benchmarks/sqlite-optimization-merge/<branch-name>/`.
- Each branch artifact directory should include raw machine-readable results and a short markdown summary.
- Record at least the standard matrix subset for branches that affect performance behavior.
- Record targeted correctness/performance counters for branches that only add routing, metrics, or tests.
- Keep the artifact names stable enough to compare branch-over-branch deltas.
- Do not rely on console output as the benchmark record.

## Optimization Defaults

- Every optimization that lands in the clean stack should be enabled by default.
- Every landed optimization should also have a central env-backed disable flag so the matrix can run ablations and production can diagnose regressions.
- Env flags are for disabling or comparing landed behavior, not for hiding incomplete features.
- All planned optimizations land default-on, with env-backed disables for benchmark ablations and production diagnosis.

## Acceptance Bar

- Each PR must pass targeted Rust and TypeScript tests for the touched layer.
- The matrix report must include per-workload deltas, not only averages.
- Each branch must write benchmark or counter artifacts to `.agent/benchmarks/sqlite-optimization-merge/<branch-name>/`.
- Parallel reads must be proven by native counters, not inferred from timing alone.
- Multi-statement SQL must have a clear public error and fixture migration path.
- Landed optimizations must default on and have env-backed disable coverage.
