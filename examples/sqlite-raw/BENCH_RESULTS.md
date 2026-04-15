# SQLite Large Insert Results

This file is generated from `bench-results.json` by
`pnpm --dir examples/sqlite-raw run bench:record -- --render-only`.

## Source of Truth

- Structured runs live in `examples/sqlite-raw/bench-results.json`.
- The rendered summary lives in `examples/sqlite-raw/BENCH_RESULTS.md`.
- Later phases should append by rerunning `bench:record`, not by inventing a
  new markdown format.

## Benchmark Modes

- Use `pnpm --dir examples/sqlite-raw run bench:record -- --phase <phase>` for the inline local benchmark path. It is the right tool for actor-side VFS changes and keeps the existing phase history comparable.
- Use `pnpm --dir examples/sqlite-raw run bench:record -- --remote-runner` for pegboard-backed validation. That path spawns `examples/sqlite-raw/src/runner.ts` as a separate runner and defaults to `0.05 MiB` so it stays under the current 15s gateway timeout while still recording server telemetry.

## Phase Summary

The table below shows the latest recorded run for each phase. Use the
regression review below when a single latest run looks suspicious.

| Metric | Phase 0 | Phase 1 | Phase 2/3 | Final |
| --- | --- | --- | --- | --- |
| Status | Recorded | Recorded | Recorded | Recorded |
| Recorded at | 2026-04-15T12:46:45.574Z | 2026-04-15T13:49:47.472Z | 2026-04-15T17:57:56.501Z | 2026-04-15T17:58:21.919Z |
| Git SHA | 78c806c541b8 | dc5ba87b2410 | d0be091571e6 | d0be091571e6 |
| Fresh engine | yes | yes | yes | yes |
| Payload | 10 MiB | 10 MiB | 10 MiB | 10 MiB |
| Rows | 1 | 1 | 1 | 1 |
| Atomic write coverage | begin 0 / commit 0 / ok 0 | begin 0 / commit 0 / ok 0 | begin 0 / commit 0 / ok 0 | begin 0 / commit 0 / ok 0 |
| Buffered dirty pages | total 0 / max 0 | total 0 / max 0 | total 0 / max 0 | total 0 / max 0 |
| Immediate kv_put writes | 2589 | 0 | 0 | 0 |
| Batch-cap failures | 0 | 0 | 0 | 0 |
| Server request counts | write 0 / read 0 / truncate 0 | write 0 / read 0 / truncate 0 | write 7 / read 0 / truncate 0 | write 7 / read 0 / truncate 0 |
| Server dirty pages | 0 | 0 | 2582 | 2582 |
| Server request bytes | write 0 B / read 0 B / truncate 0 B | write 0 B / read 0 B / truncate 0 B | write 10.10 MiB / read 0 B / truncate 0 B | write 10.10 MiB / read 0 B / truncate 0 B |
| Server overhead timing | estimate 0.0ms / rewrite 0.0ms | estimate 0.0ms / rewrite 0.0ms | estimate 0.5ms / rewrite 0.0ms | estimate 0.6ms / rewrite 0.0ms |
| Server validation | ok 0 / quota 0 / payload 0 / count 0 | ok 0 / quota 0 / payload 0 / count 0 | ok 7 / quota 0 / payload 0 / count 0 | ok 7 / quota 0 / payload 0 / count 0 |
| Actor DB insert | 15875.9ms | 898.2ms | 807.3ms | 924.8ms |
| Actor DB verify | 23848.9ms | 3927.6ms | 3973.5ms | 5142.5ms |
| End-to-end action | 40000.7ms | 4922.9ms | 4886.0ms | 8800.7ms |
| Native SQLite insert | 35.7ms | 39.7ms | 392.7ms | 47.3ms |
| Actor DB vs native | 445.25x | 22.65x | 2.06x | 19.57x |
| End-to-end vs native | 1121.85x | 124.12x | 12.44x | 186.22x |

## Regression Review

- Comparison methodology:
- Only compare canonical inline runs recorded with `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-2-3 --fresh-engine` and `pnpm --dir examples/sqlite-raw run bench:record -- --phase final --fresh-engine`.
- Phase labels are metadata only. They do not change the `bench-large-insert` payload or actor behavior, so variance has to be explained by telemetry, not by the label name.
- Use the append-only log for raw history, but use canonical fresh-engine reruns to decide whether a regression is real.
- Phase 2/3 canonical reruns: `n=3`, end-to-end `4800.3ms to 5793.5ms` (median `4886.0ms`), actor insert `779.1ms to 846.2ms`, actor verify `3844.6ms to 4780.0ms`.
- Final canonical reruns: `n=2`, end-to-end `5095.2ms to 8800.7ms` (median `6948.0ms`), actor insert `855.9ms to 924.8ms`, actor verify `4077.7ms to 5142.5ms`.
- Manual final reruns excluded: `1`. The historical US-015 PTY-backed final command is kept in the append-only log, but it is not comparable to canonical `bench:record` fresh-engine runs.
- Attribution:
- The write path stayed flat across the canonical reruns. Final fast-path commits were always `4` attempts / `4` success / `0` fallback, with request envelopes at `10.04 MiB` and sync time at `821.9ms to 866.1ms`.
- The spread comes from the verify side. Phase 2/3 VFS read time was `3839.1ms to 4772.3ms`, while Final VFS read time moved to `4071.7ms to 5133.4ms`, which tracks the actor verify swing much more closely than the write telemetry does.
- The latest Final sample is one of those read-side outliers: `8800.7ms` end-to-end with `5133.4ms` of VFS read time and only `866.1ms` of sync time.
- The original US-015 final outlier doubled sync time to `1735.6ms` and used a one-off PTY-backed command. The canonical reruns did not reproduce that write-path behavior, so the scary 7.8s result is not a stable fast-path regression.
- Updated expectation:
- For the 10 MiB inline benchmark on this branch, the write-path numbers are stable around actor insert `855.9ms to 924.8ms` and sync time `821.9ms to 866.1ms`.
- End-to-end runs in the `4800.3ms to 5793.5ms` band match the healthy canonical samples. Treat slower Final runs as verify or read outliers until the read-side variance is isolated further.

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

## Pegboard Remote Run Log

### Pegboard Remote · 2026-04-15T17:48:34.121Z

- Run ID: `remote-1776275314121`
- Git SHA: `2d5663b53af66b3a976816b6509783bf855aeb15`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --remote-runner --fresh-engine`
- Benchmark command: `BENCH_MB=0.05 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 BENCH_REQUIRE_SERVER_TELEMETRY=1 BENCH_RUNNER_MODE=remote pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Runner command: `RIVET_ENDPOINT=http://127.0.0.1:6420 pnpm --dir examples/sqlite-raw exec tsx src/runner.ts`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `remote`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Runner log: `/tmp/sqlite-raw-bench-runner.log`
- Payload: `0.05 MiB`
- Total bytes: `0.05 MiB`
- Rows: `1`
- Actor DB insert: `7.1ms`
- Actor DB verify: `0.3ms`
- End-to-end action: `87.3ms`
- Native SQLite insert: `0.1ms`
- Actor DB vs native: `48.04x`
- End-to-end vs native: `592.35x`

#### VFS Telemetry

- Reads: `0` calls, `0.00 MiB` returned, `0` short reads, `0.0ms` total
- Writes: `16` calls, `0.06 MiB` input, `16` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `1` calls, `0` metadata flushes, `0.0ms` total
- Atomic write coverage: `begin 1 / commit 1 / ok 1`
- Fast-path commit usage: `attempt 1 / ok 1 / fallback 0 / fail 0`
- Atomic write pages: `total 16 / max 16`
- Atomic write bytes: `0.06 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 0` / `put 0` / `delete 0` / `deleteRange 0`
- KV payload bytes: `0.00 MiB` read, `0.00 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `fast_path`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `6` requests, `32` dirty pages, `6` metadata keys, `128.33 KiB` request bytes, `128.06 KiB` payload bytes, `2.8ms` total
- Path overhead: `0.4ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 6` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T17:36:09.651Z`
- Duration: `279.3ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T17:48:31.235Z`
- Duration: `783.1ms`

## Append-Only Run Log

### Final · 2026-04-15T17:58:21.919Z

- Run ID: `final-1776275901919`
- Git SHA: `d0be091571e6f40366212d132dd89dcc5bd967bb`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase final --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `924.8ms`
- Actor DB verify: `5142.5ms`
- End-to-end action: `8800.7ms`
- Native SQLite insert: `47.3ms`
- Actor DB vs native: `19.57x`
- End-to-end vs native: `186.22x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `924.8ms` (`-14951.1ms`, `-94.2%`)
- Actor DB verify: `23848.9ms` -> `5142.5ms` (`-18706.4ms`, `-78.4%`)
- End-to-end action: `40000.7ms` -> `8800.7ms` (`-31200.0ms`, `-78.0%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `924.8ms` (`+26.5ms`, `+3.0%`)
- Actor DB verify: `3927.6ms` -> `5142.5ms` (`+1214.9ms`, `+30.9%`)
- End-to-end action: `4922.9ms` -> `8800.7ms` (`+3877.8ms`, `+78.8%`)

#### Compared to Phase 2/3

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `807.3ms` -> `924.8ms` (`+117.4ms`, `+14.5%`)
- Actor DB verify: `3973.5ms` -> `5142.5ms` (`+1169.0ms`, `+29.4%`)
- End-to-end action: `4886.0ms` -> `8800.7ms` (`+3914.8ms`, `+80.1%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `5133.4ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `866.1ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `fast_path`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `7` requests, `2582` dirty pages, `7` metadata keys, `10.10 MiB` request bytes, `10.08 MiB` payload bytes, `96.7ms` total
- Path overhead: `0.6ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 7` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T17:55:42.670Z`
- Duration: `423.3ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T17:58:06.358Z`
- Duration: `1326.2ms`

### Phase 2/3 · 2026-04-15T17:57:56.501Z

- Run ID: `phase-2-3-1776275876501`
- Git SHA: `d0be091571e6f40366212d132dd89dcc5bd967bb`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-2-3 --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `807.3ms`
- Actor DB verify: `3973.5ms`
- End-to-end action: `4886.0ms`
- Native SQLite insert: `392.7ms`
- Actor DB vs native: `2.06x`
- End-to-end vs native: `12.44x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `807.3ms` (`-15068.5ms`, `-94.9%`)
- Actor DB verify: `23848.9ms` -> `3973.5ms` (`-19875.5ms`, `-83.3%`)
- End-to-end action: `40000.7ms` -> `4886.0ms` (`-35114.7ms`, `-87.8%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `807.3ms` (`-90.9ms`, `-10.1%`)
- Actor DB verify: `3927.6ms` -> `3973.5ms` (`+45.9ms`, `+1.2%`)
- End-to-end action: `4922.9ms` -> `4886.0ms` (`-36.9ms`, `-0.7%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `3967.6ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `776.4ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `fast_path`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `7` requests, `2582` dirty pages, `7` metadata keys, `10.10 MiB` request bytes, `10.08 MiB` payload bytes, `72.4ms` total
- Path overhead: `0.5ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 7` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T17:55:42.670Z`
- Duration: `256.5ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T17:57:44.562Z`
- Duration: `817.4ms`

### Final · 2026-04-15T17:57:16.614Z

- Run ID: `final-1776275836614`
- Git SHA: `d0be091571e6f40366212d132dd89dcc5bd967bb`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase final --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `855.9ms`
- Actor DB verify: `4077.7ms`
- End-to-end action: `5095.2ms`
- Native SQLite insert: `135.0ms`
- Actor DB vs native: `6.34x`
- End-to-end vs native: `37.74x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `855.9ms` (`-15019.9ms`, `-94.6%`)
- Actor DB verify: `23848.9ms` -> `4077.7ms` (`-19771.2ms`, `-82.9%`)
- End-to-end action: `40000.7ms` -> `5095.2ms` (`-34905.5ms`, `-87.3%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `855.9ms` (`-42.3ms`, `-4.7%`)
- Actor DB verify: `3927.6ms` -> `4077.7ms` (`+150.2ms`, `+3.8%`)
- End-to-end action: `4922.9ms` -> `5095.2ms` (`+172.4ms`, `+3.5%`)

#### Compared to Phase 2/3

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `807.3ms` -> `855.9ms` (`+48.6ms`, `+6.0%`)
- Actor DB verify: `3973.5ms` -> `4077.7ms` (`+104.3ms`, `+2.6%`)
- End-to-end action: `4886.0ms` -> `5095.2ms` (`+209.3ms`, `+4.3%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `4071.7ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `821.9ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `fast_path`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `7` requests, `2582` dirty pages, `7` metadata keys, `10.10 MiB` request bytes, `10.08 MiB` payload bytes, `90.1ms` total
- Path overhead: `0.6ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 7` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T17:55:42.670Z`
- Duration: `253.2ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T17:57:04.566Z`
- Duration: `773.2ms`

### Phase 2/3 · 2026-04-15T17:56:51.436Z

- Run ID: `phase-2-3-1776275811436`
- Git SHA: `d0be091571e6f40366212d132dd89dcc5bd967bb`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-2-3 --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `846.2ms`
- Actor DB verify: `4780.0ms`
- End-to-end action: `5793.5ms`
- Native SQLite insert: `37.7ms`
- Actor DB vs native: `22.43x`
- End-to-end vs native: `153.54x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `846.2ms` (`-15029.6ms`, `-94.7%`)
- Actor DB verify: `23848.9ms` -> `4780.0ms` (`-19068.9ms`, `-80.0%`)
- End-to-end action: `40000.7ms` -> `5793.5ms` (`-34207.2ms`, `-85.5%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `846.2ms` (`-52.0ms`, `-5.8%`)
- Actor DB verify: `3927.6ms` -> `4780.0ms` (`+852.4ms`, `+21.7%`)
- End-to-end action: `4922.9ms` -> `5793.5ms` (`+870.7ms`, `+17.7%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `4772.3ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `806.7ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

#### Server Telemetry

- Metrics endpoint: `http://127.0.0.1:6430/metrics`
- Path label: `fast_path`
- Reads: `0` requests, `0` page keys, `0` metadata keys, `0 B` request bytes, `0 B` response bytes, `0.0ms` total
- Writes: `7` requests, `2582` dirty pages, `7` metadata keys, `10.10 MiB` request bytes, `10.08 MiB` payload bytes, `102.6ms` total
- Path overhead: `0.6ms` in `estimate_kv_size`, `0.0ms` in clear-and-rewrite, `0` `clear_subspace_range` calls
- Truncates: `0` requests, `0 B` request bytes, `0.0ms` total
- Validation outcomes: `ok 7` / `quota 0` / `payload 0` / `count 0` / `key 0` / `value 0` / `length 0`

#### Engine Build Provenance

- Command: `cargo build --bin rivet-engine`
- CWD: `.`
- Artifact: `target/debug/rivet-engine`
- Artifact mtime: `2026-04-15T17:55:42.670Z`
- Duration: `15478.9ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T17:55:43.790Z`
- Duration: `917.2ms`

### Final · 2026-04-15T17:17:43.512Z

- Run ID: `final-1776273463512`
- Git SHA: `60181c4c8460d9b63d78074800c8cb7362ad6b2d`
- Workflow command: `cargo build --bin rivet-engine && pnpm --dir rivetkit-typescript/packages/rivetkit-native run build:force && RUST_BACKTRACE=full RUST_LOG='opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug' RUST_LOG_TARGET=1 ./target/debug/rivet-engine start >/tmp/us015-engine.log 2>&1 & script -q -c "BENCH_READY_TIMEOUT_MS=900000 BENCH_READY_ATTEMPT_TIMEOUT_MS=120000 pnpm --dir examples/sqlite-raw exec tsx scripts/bench-large-insert.ts -- --json" /tmp/us015-benchmark.log`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=900000 BENCH_READY_ATTEMPT_TIMEOUT_MS=120000 pnpm --dir examples/sqlite-raw exec tsx scripts/bench-large-insert.ts -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/us015-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `1775.8ms`
- Actor DB verify: `5942.6ms`
- End-to-end action: `7840.1ms`
- Native SQLite insert: `36.6ms`
- Actor DB vs native: `48.47x`
- End-to-end vs native: `213.99x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `1775.8ms` (`-14100.1ms`, `-88.8%`)
- Actor DB verify: `23848.9ms` -> `5942.6ms` (`-17906.3ms`, `-75.1%`)
- End-to-end action: `40000.7ms` -> `7840.1ms` (`-32160.6ms`, `-80.4%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `1775.8ms` (`+877.5ms`, `+97.7%`)
- Actor DB verify: `3927.6ms` -> `5942.6ms` (`+2015.0ms`, `+51.3%`)
- End-to-end action: `4922.9ms` -> `7840.1ms` (`+2917.2ms`, `+59.3%`)

#### Compared to Phase 2/3

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `807.3ms` -> `1775.8ms` (`+968.4ms`, `+120.0%`)
- Actor DB verify: `3973.5ms` -> `5942.6ms` (`+1969.1ms`, `+49.6%`)
- End-to-end action: `4886.0ms` -> `7840.1ms` (`+2954.1ms`, `+60.5%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `5936.3ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `1735.6ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

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
- Artifact mtime: `2026-04-15T08:56:54.812496469-07:00`
- Duration: `280.0ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T10:14:50.294307165-07:00`
- Duration: `904.0ms`

### Phase 2/3 · 2026-04-15T15:51:19.124Z

- Run ID: `phase-2-3-1776268279124`
- Git SHA: `df83e0aafced0efb48a524e54eb7a1c6d2549e35`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-2-3 --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 BENCH_READY_TIMEOUT_MS=300000 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
- Fresh engine start: `yes`
- Engine log: `/tmp/sqlite-raw-bench-engine.log`
- Payload: `10 MiB`
- Total bytes: `10.00 MiB`
- Rows: `1`
- Actor DB insert: `779.1ms`
- Actor DB verify: `3844.6ms`
- End-to-end action: `4800.3ms`
- Native SQLite insert: `34.9ms`
- Actor DB vs native: `22.35x`
- End-to-end vs native: `137.69x`

#### Compared to Phase 0

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `2589` -> `0` (`-2589`, `-100.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `15875.9ms` -> `779.1ms` (`-15096.8ms`, `-95.1%`)
- Actor DB verify: `23848.9ms` -> `3844.6ms` (`-20004.3ms`, `-83.9%`)
- End-to-end action: `40000.7ms` -> `4800.3ms` (`-35200.4ms`, `-88.0%`)

#### Compared to Phase 1

- Atomic write coverage: `begin 0 / commit 0 / ok 0` -> `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 0 / ok 0 / fallback 0 / fail 0` -> `attempt 4 / ok 4 / fallback 0 / fail 0`
- Buffered dirty pages: `total 0 / max 0` -> `total 0 / max 0`
- Immediate `kv_put` writes: `0` -> `0` (`0`, `0.0%`)
- Batch-cap failures: `0` -> `0` (`0`)
- Actor DB insert: `898.2ms` -> `779.1ms` (`-119.2ms`, `-13.3%`)
- Actor DB verify: `3927.6ms` -> `3844.6ms` (`-82.9ms`, `-2.1%`)
- End-to-end action: `4922.9ms` -> `4800.3ms` (`-122.6ms`, `-2.5%`)

#### VFS Telemetry

- Reads: `2565` calls, `10.01 MiB` returned, `2` short reads, `3839.1ms` total
- Writes: `2589` calls, `10.05 MiB` input, `2589` buffered calls, `0` immediate `kv_put` fallbacks
- Syncs: `4` calls, `4` metadata flushes, `743.5ms` total
- Atomic write coverage: `begin 0 / commit 0 / ok 0`
- Fast-path commit usage: `attempt 4 / ok 4 / fallback 0 / fail 0`
- Atomic write pages: `total 0 / max 0`
- Atomic write bytes: `0.00 MiB`
- Atomic write failures: `0` batch-cap, `0` KV put
- KV round-trips: `get 2565` / `put 1` / `delete 0` / `deleteRange 0`
- KV payload bytes: `10.02 MiB` read, `0.00 MiB` written

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
- Artifact mtime: `2026-04-15T15:45:24.929Z`
- Duration: `249.6ms`

#### Native Build Provenance

- Command: `pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force`
- CWD: `.`
- Artifact: `rivetkit-typescript/packages/rivetkit-native/rivetkit-native.linux-x64-gnu.node`
- Artifact mtime: `2026-04-15T15:50:20.841Z`
- Duration: `725.7ms`

### Phase 1 · 2026-04-15T13:49:47.472Z

- Run ID: `phase-1-1776260987472`
- Git SHA: `dc5ba87b2410a02a1e64c315156d0bd491ef5785`
- Workflow command: `pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-1 --fresh-engine`
- Benchmark command: `BENCH_MB=10 BENCH_ROWS=1 RIVET_ENDPOINT=http://127.0.0.1:6420 pnpm --dir examples/sqlite-raw run bench:large-insert -- --json`
- Endpoint: `http://127.0.0.1:6420`
- Runner mode: `inline`
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
- Runner mode: `inline`
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

