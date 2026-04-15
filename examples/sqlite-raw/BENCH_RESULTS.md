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
| Status | Recorded | Recorded | Pending | Pending |
| Recorded at | 2026-04-15T12:46:45.574Z | 2026-04-15T13:49:47.472Z | Pending | Pending |
| Git SHA | 78c806c541b8 | dc5ba87b2410 | Pending | Pending |
| Fresh engine | yes | yes | Pending | Pending |
| Payload | 10 MiB | 10 MiB | Pending | Pending |
| Rows | 1 | 1 | Pending | Pending |
| Atomic write coverage | begin 0 / commit 0 / ok 0 | begin 0 / commit 0 / ok 0 | Pending | Pending |
| Buffered dirty pages | total 0 / max 0 | total 0 / max 0 | Pending | Pending |
| Immediate kv_put writes | 2589 | 0 | Pending | Pending |
| Batch-cap failures | 0 | 0 | Pending | Pending |
| Server request counts | write 0 / read 0 / truncate 0 | write 0 / read 0 / truncate 0 | Pending | Pending |
| Server dirty pages | 0 | 0 | Pending | Pending |
| Server request bytes | write 0 B / read 0 B / truncate 0 B | write 0 B / read 0 B / truncate 0 B | Pending | Pending |
| Server overhead timing | estimate 0.0ms / rewrite 0.0ms | estimate 0.0ms / rewrite 0.0ms | Pending | Pending |
| Server validation | ok 0 / quota 0 / payload 0 / count 0 | ok 0 / quota 0 / payload 0 / count 0 | Pending | Pending |
| Actor DB insert | 15875.9ms | 898.2ms | Pending | Pending |
| Actor DB verify | 23848.9ms | 3927.6ms | Pending | Pending |
| End-to-end action | 40000.7ms | 4922.9ms | Pending | Pending |
| Native SQLite insert | 35.7ms | 39.7ms | Pending | Pending |
| Actor DB vs native | 445.25x | 22.65x | Pending | Pending |
| End-to-end vs native | 1121.85x | 124.12x | Pending | Pending |

## SQLite Fast-Path Batch Ceiling

### 2026-04-15T15:28:36.645Z

- Chosen SQLite fast-path ceiling: `3328` dirty pages
- Generic actor-KV cap: `128` entries
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --evaluate-batch-ceiling --chosen-limit-pages 3328 --batch-pages 128,512,1024,2048,3328 --fresh-engine`
- Endpoint: `http://127.0.0.1:6420`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Notes:
- These samples measure the SQLite fast path above the generic 128-entry actor-KV cap on the local benchmark engine.
- The local benchmark path reports request bytes and commit latency from VFS fast-path telemetry because pegboard metrics stay zero when the actor runs in-process.
- Engine config still defaults envoy tunnel payloads to 20 MiB, so request bytes should stay comfortably below that envelope before raising the ceiling again.

| Target pages | Payload | Path | Actual dirty pages | Request bytes | Commit latency | Actor DB insert |
| --- | --- | --- | --- | --- | --- | --- |
| 128 | 0.38 MiB | fast_path | 101 | 404.80 KiB | 32.1ms | 33.7ms |
| 512 | 1.88 MiB | fast_path | 485 | 1.90 MiB | 140.1ms | 156.8ms |
| 1024 | 3.88 MiB | fast_path | 998 | 3.91 MiB | 291.6ms | 318.5ms |
| 2048 | 7.88 MiB | fast_path | 2023 | 7.92 MiB | 630.3ms | 674.8ms |
| 3328 | 12.88 MiB | fast_path | 3304 | 12.93 MiB | 1062.7ms | 1129.9ms |

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T15:22:26.969Z`
- Duration: `249.2ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T15:27:46.449Z`
- Duration: `2108.2ms`

Older evaluations remain in `bench-results.json`; the latest successful rerun is rendered here.

## Append-Only Run Log

### Phase 1 · 2026-04-15T13:49:47.472Z

- Run ID: `phase-1-1776260987472`
- Git SHA: `dc5ba87b2410a02a1e64c315156d0bd491ef5785`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-1 --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `898.2ms`
- Actor DB verify: `3927.6ms`
- End-to-end action: `4922.9ms`
- Native SQLite insert: `39.7ms`
- Actor DB vs native: `22.65x`
- End-to-end vs native: `124.12x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 0 / ok 0 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `898.2ms` (`-14977.6ms`, `-94.3%`)
- Actor DB verify: `23848.9ms` -> `3927.6ms` (`-19921.3ms`, `-83.5%`)
- End-to-end action: `40000.7ms` -> `4922.9ms` (`-35077.8ms`, `-87.7%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `3922.6ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `856.5ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 28` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `10.05 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `generic`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `0` requests, `0` dirty pages, `0` metadata keys, `0 B` request bytes, `0 B` payload bytes, `0.0ms` total
- Path overhead: `0.0ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 0` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T13:34:46.356Z`
- Duration: `266.3ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T13:49:35.017Z`
- Duration: `784.2ms`

### Phase 0 · 2026-04-15T12:46:45.574Z

- Run ID: `phase-0-1776257205574`
- Git SHA: `78c806c541b8736ec0525c0971fb94af213bf044`
- Workflow command: `cargo build --bin rivet-engine && pnpm --dir rivetkit-typescript/packages/rivetkit-native run build:force && setsid env RUST_BACKTRACE=full RUST_LOG='opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug' RUST_LOG_TARGET=1 ./target/debug/rivet-engine start >/tmp/sqlite-manual-engine.log 2>&1 < /dev/null & BENCH_OUTPUT=json pnpm --dir examples/sqlite-raw exec tsx scripts/bench-large-insert.ts -- --json`
- Benchmark command: `BENCH_OUTPUT=json RIVET_ENDPOINT=http://127.0.0.1:6420 pnpm --dir examples/sqlite-raw exec tsx scripts/bench-large-insert.ts -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-manual-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `15875.9ms`
- Actor DB verify: `23848.9ms`
- End-to-end action: `40000.7ms`
- Native SQLite insert: `35.7ms`
- Actor DB vs native: `445.25x`
- End-to-end vs native: `1121.85x`

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `23843.6ms` total
- Writes: `2589` calls, `10.05 MiB` input, `0` buffered calls, `2589` immediate `kv_put` fallbacks
- Syncs: `4` calls, `0` metadata flushes, `0.0ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2584` / `put 2590` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.05 MiB` read, `10.11 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `generic`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `0` requests, `0` dirty pages, `0` metadata keys, `0 B` request bytes, `0 B` payload bytes, `0.0ms` total
- Path overhead: `0.0ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 0` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T05:03:06-07:00`
- Duration: `284.0ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T05:44:45-07:00`
- Duration: `990.0ms`

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

