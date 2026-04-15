# SQLite Large Insert Results

This file is generated from `bench-results.json` by
`pnpm --dir examples/sqlite-raw run bench:record -- --render-only`.

## Source of Truth

- Structured runs live in `examples/sqlite-raw/bench-results.json`.
- The rendered summary lives in `examples/sqlite-raw/BENCH_RESULTS.md`.
- Later phases should append by rerunning `bench:record`, not by inventing a
  new markdown format.

## Phase Summary

| Metric | Phase 0 | Phase 1 | Phase 2/3 | Final |
| --- | --- | --- | --- | --- |
| Status | Pending | Pending | Pending | Pending |
| Recorded at | Pending | Pending | Pending | Pending |
| Git SHA | Pending | Pending | Pending | Pending |
| Fresh engine | Pending | Pending | Pending | Pending |
| Payload | Pending | Pending | Pending | Pending |
| Rows | Pending | Pending | Pending | Pending |
| Atomic write coverage | Pending | Pending | Pending | Pending |
| Buffered dirty pages | Pending | Pending | Pending | Pending |
| Immediate kv_put writes | Pending | Pending | Pending | Pending |
| Batch-cap failures | Pending | Pending | Pending | Pending |
| Actor DB insert | Pending | Pending | Pending | Pending |
| Actor DB verify | Pending | Pending | Pending | Pending |
| End-to-end action | Pending | Pending | Pending | Pending |
| Native SQLite insert | Pending | Pending | Pending | Pending |
| Actor DB vs native | Pending | Pending | Pending | Pending |
| End-to-end vs native | Pending | Pending | Pending | Pending |

## Append-Only Run Log

No structured runs recorded yet.

## Historical Reference

The section below predates this scaffold. Keep it for context, but append new
phase results through `bench-results.json` and `bench:record`.

### 2026-04-15 Exploratory Large Insert Runs

| Payload | Actor DB Insert | Actor DB Verify | End-to-End Action | Native SQLite Insert | Actor DB vs Native | End-to-End vs Native |
| ------- | --------------- | --------------- | ----------------- | -------------------- | ------------------ | -------------------- |
| 1 MiB   | 832.2ms         | 0.4ms           | 1137.6ms          | 1.8ms                | 461.11x            | 630.34x              |
| 5 MiB   | 4199.6ms        | 3655.5ms        | 8186.3ms          | 25.3ms               | 166.19x            | 323.96x              |
| 10 MiB  | 9438.2ms        | 8973.5ms        | 19244.0ms         | 45.5ms               | 207.34x            | 422.75x              |

- Command: `pnpm --dir examples/sqlite-raw bench:large-insert`
- Additional runs: `BENCH_MB=1`, `BENCH_MB=5`, `BENCH_MB=10`, and one
  `RUST_LOG=rivetkit_sqlite_native::vfs=debug BENCH_MB=1` trace run.
- Debug trace clue: 317 total KV round-trips, 30 `get(...)` calls,
  287 `put(...)` calls, 577 total keys written, 63.1ms traced `get` time,
  and 856.0ms traced `put` time.
- Conclusion: the bottleneck already looked like SQLite-over-KV page churn,
  not raw SQLite execution.

