# SQLite Memory Soak Issues

Date: 2026-05-03

This note captures the current known issues from the kitchen-sink SQLite memory soak and release spike work. The goal of the soak was to verify Rivet Actor correctness under SQLite churn and prove whether memory is reclaimed after actors sleep.

## Current conclusion

The lower-concurrency runs show that kitchen-sink memory can be reclaimed after actors sleep. The release 200-concurrency runs are not valid leak proofs because SQLite correctness failed before a clean drain.

The highest-priority issue is the release no-reset spike producing `database disk image is malformed` during SQLite startup/migration. The second issue is the reset-enabled spike failing on fresh actors with many missing-database preload/read errors. Those are correctness issues first; memory conclusions from those runs are secondary.

## Harness context

The current harness lives in `examples/kitchen-sink/scripts/sqlite-memory-soak.ts`.

Important behavior:

- It spawns the local engine and kitchen-sink serverless process so the harness can sample exact PIDs.
- It samples harness, engine, and kitchen RSS from `/proc`.
- It samples CPU, IO, fd count, and smaps-derived memory details when available.
- It calls the kitchen `/debug/memory` endpoint for JS heap, external memory, native estimate, GC hooks, and active actor diagnostics.
- It supports fixed-timer churn so load is time-based instead of throughput-based.
- It can force an actor to sleep after `--churn-sleep-after-ms`, then spawn another actor to keep target concurrency.
- It supports spike mode with `--spike-min-concurrency`, `--spike-max-concurrency`, and `--spike-period-ms`.
- It supports an idle baseline before workload with `--pre-workload-wait-ms`.

Reports are rendered by `examples/kitchen-sink/scripts/proc-metrics-report.ts` to `~/tmp/proc-metrics/<run-id>/index.html`.

The charts currently include RSS, sampled process details, sampled kitchen debug memory, and Envoy-reported active actors as an overlay. Actor wake vertical lines were removed.

## Release build context

Release artifacts were built with:

```bash
cargo build --release -p rivet-engine
pnpm --filter @rivetkit/rivetkit-napi build:force:release
```

Release artifact checks:

- Engine binary: `target/release/rivet-engine`, about 81 MiB, not stripped.
- NAPI module: `rivetkit-typescript/packages/rivetkit-napi/rivetkit-napi.linux-x64-gnu.node`, matching `target/release/librivetkit_napi.so`, about 16 MiB, not stripped.

Earlier spike runs were not release for both sides. The 10 to 50 spike used debug engine and debug NAPI.

## Workload shape

The actor is `examples/kitchen-sink/src/actors/testing/sqlite-memory-pressure.ts`.

Current important behavior:

- `onMigrate` creates the workload tables and index.
- `runCycle` inserts many rows, stores blob-like payloads, scans data, and returns integrity/storage metrics.
- `releaseStorage` no longer deletes rows or runs `VACUUM`; that code is commented out per the current focus.
- `reset` still deletes from `pressure_cycles`, deletes from `pressure_rows`, and runs `VACUUM`.
- `onSleep` increments `c.state.sleepCount` and logs a structured JSON line with `kind: "sqlite_memory_pressure_on_sleep"`.

The original delete/`VACUUM` workload was useful for forcing page churn and cache pressure, but it confused the memory question because it intentionally shrank the database before sleep. For the current concern, actor sleep should release local memory regardless of database contents because the SQLite database is remote.

## Run artifacts

### 2 minute fixed-concurrency run

Run ID: `proc-metrics-2m-c10-20260503-133508`

Report: `/home/nathan/tmp/proc-metrics/proc-metrics-2m-c10-20260503-133508/index.html`

Result:

- Manually stopped around 155s.
- Max actor index: 372.
- Wakes: 373.
- Verified sleeps: 368.
- Envoy active actors: min 3, max 13, final 12.
- Engine RSS: 149.4 MiB start, 317.3 MiB max, 315.2 MiB final.
- Kitchen RSS: 260.1 MiB start, 445.4 MiB max, 264.6 MiB final.

Interpretation:

- Kitchen RSS dropped back near baseline while actors were still cycling.
- Active actors were roughly stable around 10.
- JS heap was roughly flat around 31 MiB.
- External memory was roughly flat around 5 MiB.
- Native estimate dropped from roughly 384 MiB to 300 MiB to 206 MiB.
- smaps anonymous/private dirty memory dropped from roughly 340 MiB to 256 MiB to 167 MiB.
- `MALLOC_ARENA_MAX=2` and `MALLOC_TRIM_THRESHOLD_=131072` likely helped RSS return in large chunks.

This is good evidence that kitchen memory can reclaim after actors sleep. It is not enough by itself to prove there is no leak.

### 5 minute 10 to 50 spike

Run ID: `20260503-135136-sqlite-spike-5m-c10-50`

Report: `/home/nathan/tmp/proc-metrics/20260503-135136-sqlite-spike-5m-c10-50/index.html`

Build mode:

- Engine: debug, `/home/nathan/r7/target/debug/rivet-engine`.
- NAPI: debug, matched `target/debug/librivetkit_napi.so`, about 330 MiB with debug info.

Result:

- Completed successfully.
- Run errors: 0.
- SQLite cycles: 1854.
- Actor wakes: 1147.
- Verified sleeps: 1147.
- Harness active concurrency hit 50.
- Envoy active actors peaked at 56.
- Envoy active actors final: 0.
- Kitchen RSS: 249.4 MiB start, 573.0 MiB max, 342.6 MiB final.
- Kitchen post-churn: 426.1 MiB start, 334.6 MiB min, 342.6 MiB final.
- Engine RSS: 146.9 MiB start, 355.1 MiB max, 347.9 MiB final.

Interpretation:

- Kitchen reclaimed substantially after churn.
- Kitchen finished about 93 MiB above start after 60s of post-churn wait.
- Engine stayed near peak.
- This was a useful signal, but not a full no-leak proof because it was debug mode and the post-churn window was short.

### Release 10 to 200 spike with reset enabled

Run ID: `20260503-142448-sqlite-spike-release-5m-c10-200`

Report: `/home/nathan/tmp/proc-metrics/20260503-142448-sqlite-spike-release-5m-c10-200/index.html`

Config:

- Release engine through `RIVET_ENGINE_BINARY=/home/nathan/r7/target/release/rivet-engine`.
- Release NAPI.
- 60s idle baseline.
- 5m target spike from 10 to 200 to 10 concurrent actors.
- 60s post-churn requested.
- Reset enabled.
- Cleanup disabled.
- Forced GC sampling enabled.

Result:

- Failed before completion around 113s elapsed.
- Failure happened in `handle.reset()` before the actor's main SQL cycle.
- Client surfaced only `RivetError: An internal error occurred`.
- Engine logs contained many `sqlite get_pages request failed` and `sqlite database was not found in this bucket branch` messages.

Counts:

- Samples: 112.
- Final elapsed: 113050ms.
- Max target: 200.
- Max harness concurrency: 197.
- Next actor index: 486.
- Cycles: 763.
- Wakes: 491.
- Sleeps: 491.
- Errors: 1.
- Engine RSS: 89.7 MiB start, 385.9 MiB max, 381.0 MiB final.
- Kitchen RSS: 235.3 MiB start, 1068.1 MiB max, 352.8 MiB final.
- Idle baseline engine: 89.7 MiB to 92.9 MiB.
- Idle baseline kitchen: 235.3 MiB to 236.7 MiB.
- Cycle latency: p50 1411.8ms, p95 4303.4ms, p99 4957.7ms, max 5052.5ms.

Interpretation:

- This is not a valid memory-leak result because the run failed early.
- The failing operation was redundant reset on fresh actor IDs.
- The likely issue is in the reset/open/preload/missing-database path under high concurrency.
- Missing fresh DB state should not create a large error storm or poison actor startup.
- Exact root cause was not proven.

### Release 10 to 200 spike with no reset

Run ID: `20260503-142821-sqlite-spike-release-5m-c10-200-no-reset`

Report: `/home/nathan/tmp/proc-metrics/20260503-142821-sqlite-spike-release-5m-c10-200-no-reset/index.html`

Config:

- Release engine.
- Release NAPI.
- 60s idle baseline.
- 5m target spike from 10 to 200 to 10 concurrent actors.
- 60s post-churn requested.
- `--no-reset`.
- Cleanup disabled.
- Forced GC sampling enabled.

Partial progress:

- Around 70s: 0 errors, 187 cycles, 99 sleeps, Envoy active 62, engine RSS 279.7 MiB, kitchen RSS 356.0 MiB.
- Around 146s: max target/harness 199, max Envoy active 192, 1059 cycles, 902 sleeps, no surfaced run errors, engine RSS 390.3 MiB, kitchen RSS 602.8 MiB, kitchen max 880.3 MiB.
- Around 280s: max Envoy active 204, 2375 cycles, 2164 sleeps, no surfaced run errors, engine RSS 412.7 MiB, kitchen RSS 717.1 MiB, kitchen max 1090.0 MiB.
- After scheduling ended, drain got stuck with Envoy active around 15 and about 10 drivers waiting for sleep completion.

Final partial stats:

- Completed: false.
- Manually stopped: true.
- Surfaced harness run errors: 0.
- Samples: 520.
- Final elapsed: 532497ms.
- Max target: 199.
- Max harness concurrency: 199.
- Max Envoy active actors: 204.
- Next actor index: 2265.
- Cycles: 2469.
- Sleeps: 2255.
- Engine RSS: 90.0 MiB start, 415.8 MiB max, 396.9 MiB final.
- Kitchen RSS: 237.0 MiB start, 1090.0 MiB max, 512.2 MiB final.
- Idle engine: 90.0 MiB start, 94.2 MiB final.
- Idle kitchen: 237.0 MiB start, 233.1 MiB final.
- Active engine: 94.2 MiB start, 415.8 MiB max, 396.9 MiB final.
- Active kitchen: 233.1 MiB start, 1090.0 MiB max, 512.2 MiB final.
- Cycle latency: p50 8182.9ms, p95 19027.5ms, p99 26105.4ms, max 34667.8ms.

Important log errors:

- `sqlite batch atomic probe failed`.
- `database disk image is malformed`.
- `failed to verify sqlite batch atomic writes`.
- `encoded structured bridge error`.
- `actor run handler failed`.
- `actor start failed`.
- `Cannot read properties of undefined (reading 'sleepCount')`.
- Many `actor_ready_timeout` errors for specific actor keys.
- Many `sqlite get_pages request failed`.
- Many `sqlite database was not found in this bucket branch`.

Approximate error counts from logs:

- `database disk image is malformed`: 30.
- `actor_ready_timeout`: 402.
- `Cannot read properties of undefined`: 14.
- `sqlite get_pages request failed`: 2269.
- `encoded structured bridge error`: 5.

The batch atomic probe SQL that failed:

```sql
BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS __rivet_batch_probe(x INTEGER);
INSERT INTO __rivet_batch_probe VALUES(1);
DELETE FROM __rivet_batch_probe;
DROP TABLE IF EXISTS __rivet_batch_probe;
COMMIT;
```

Interpretation:

- This is the strongest correctness failure found so far.
- It happened in actor startup/migration, not just during steady `runCycle`.
- It points at a VFS/depot/page-cache/commit consistency problem under high churn.
- The run did not produce a valid leak conclusion because actors failed and the harness got stuck draining.

### Release 10 to 100 spike with no reset

Run ID: `20260503-145252-sqlite-spike-release-5m-c10-100-no-reset`

Report: `/home/nathan/tmp/proc-metrics/20260503-145252-sqlite-spike-release-5m-c10-100-no-reset/index.html`

Config:

- Release engine.
- Release NAPI.
- 60s idle baseline.
- 5m target spike from 10 to 100 to 10 concurrent actors.
- 60s post-churn requested.
- `--no-reset`.
- Cleanup disabled.
- Forced GC sampling enabled.

Result:

- Did not complete cleanly.
- Failed at about 178s elapsed, after reaching target 100.
- Failure: `timed out waiting for actor sleeping log for 1474y8ra7z2o4835yezyh7zbc0bl00`.
- No `database disk image is malformed` errors were found.
- No `sqlite batch atomic probe failed` errors were found.
- No `Cannot read properties` errors were found.
- No `encoded structured bridge error` errors were found.

Counts:

- Samples: 174.
- Cycles: 1172.
- Actor API sleep calls: 953.
- Verified sleeps: 944.
- Max target concurrency: 100.
- Max harness active concurrency: 100.
- Max Envoy active actors sampled: 91.
- Final Envoy active actors sampled: 10.
- Engine RSS: 88.5 MiB start, 361.2 MiB max, 354.2 MiB final.
- Kitchen RSS: 228.8 MiB start, 596.3 MiB max, 328.3 MiB final.
- Kitchen JS heap used: 28.9 MiB start, 34.0 MiB max, 31.8 MiB final.
- Kitchen external memory: 5.0 MiB start, 5.1 MiB max, 5.0 MiB final.
- Kitchen native non-V8 resident estimate: 168.9 MiB start, 526.1 MiB max, 291.7 MiB final.
- Cycle latency: p50 3549.5ms, p95 18823.9ms, p99 29156.4ms, max 37399.0ms.

Error counts:

- `sqlite get_pages request failed`: 955.
- `sqlite database was not found in this bucket branch`: 955.
- `actor_ready_timeout`: 44.
- `timed out waiting for actor sleeping log`: 1.
- `database disk image is malformed`: 0.
- `sqlite batch atomic probe failed`: 0.
- `Cannot read properties`: 0.
- `encoded structured bridge error`: 0.

Interpretation:

- At max concurrency 100, the malformed database failure did not reproduce.
- The missing-database preload/read storm still reproduced without reset.
- Actor ready timeouts appeared under load and likely contributed to the harness failing to observe one sleep log.
- Kitchen memory again dropped substantially from peak before failure, and the growth was mostly native non-V8 memory.
- This is still not a valid leak proof because the run failed before clean drain and post-churn measurement.

## Issue 1: SQLite malformed database during batch atomic probe

Severity: high.

Evidence:

- Release no-reset 10 to 200 spike logged `database disk image is malformed`.
- The error happened while verifying SQLite batch atomic writes.
- The failing path was in actor startup/migration, with `onMigrate` in the stack.
- The same run then produced actor startup failures and repeated ready timeouts.

Current theory:

- The VFS or depot read/write path is returning inconsistent SQLite page state under high actor churn.
- Likely areas are batch atomic write handling, page cache invalidation, commit staging/finalization, preload hydration, or read-after-write visibility.
- This is not proven yet.

What would prove it:

- A focused repro that fails without the full harness.
- Extra VFS logging around database ID, generation, branch, page count, dirty page commit, preload hints, and batch atomic probe lifecycle.
- A direct VFS/depot stress test that repeatedly opens, migrates, writes, sleeps, and reopens the same shape of actor databases under bounded concurrency.

## Issue 2: Missing-database storm during reset on fresh actors

Severity: high, but probably lower than malformed DB.

Evidence:

- Release reset-enabled 10 to 200 spike failed in `handle.reset()`.
- Actor IDs were fresh, so reset was redundant.
- Engine logs had many `sqlite get_pages request failed` and `sqlite database was not found in this bucket branch`.
- Client surfaced only a generic internal error.

Current theory:

- The fresh DB/open/reset/preload path treats an expected empty or not-yet-created database as a hard error somewhere.
- Under high concurrency this becomes an error storm and can fail actor startup.
- Reset also does delete plus `VACUUM`, making it a much heavier startup operation than needed for fresh actor IDs.

What would prove it:

- Reproduce with a smaller harness that calls only `reset` on fresh actors.
- Log whether the missing database is from preload, `get_pages`, reset, `VACUUM`, or initial open.
- Distinguish expected missing fresh database from an actually corrupted or lost database branch.

## Issue 3: Actor ready timeout and stuck drain after startup failure

Severity: medium-high.

Evidence:

- The release no-reset spike got stuck after the scheduling window ended.
- Envoy still reported active actors.
- Logs repeated `actor_ready_timeout` for specific actors.
- The harness had no surfaced `run_error` even while the engine logs showed startup failures.

Current theory:

- Actor startup failures can leave the client or harness waiting for sleep/drain forever.
- The engine/client retry path may repeatedly try actors that are already poisoned by startup failure.
- The harness should have a bounded drain grace period so soak failures become explicit instead of hanging.

What would prove it:

- Add per-actor lifecycle event logging for create, ready, cycle start, cycle done, force sleep request, sleep observed, and startup failure.
- Make the harness emit an error if drain exceeds a configured grace period.
- Verify whether actor-ready retries stop once the underlying startup error is terminal.

## Issue 4: `onSleep` received undefined state after failed startup

Severity: medium.

Evidence:

- Logs showed `Cannot read properties of undefined (reading 'sleepCount')` in `sqlite-memory-pressure.ts`.
- This happened after startup/migration failures.
- The `onSleep` hook expects `c.state` to be initialized.

Current theory:

- This is likely secondary fallout from failed startup.
- Either user lifecycle hooks are being called in a state where actor state was not initialized, or the test actor should defensively tolerate failed-start cleanup.

What would prove it:

- Reproduce a forced `onMigrate` failure and observe whether `onSleep` is called with undefined state.
- Inspect core/NAPI lifecycle cleanup to decide whether `onSleep` is valid after failed startup.

## Memory observations

Known good signals:

- Kitchen memory drops substantially after actors sleep in lower-concurrency and 10 to 50 spike runs.
- In the 2 minute fixed-concurrency run, kitchen RSS returned from 445.4 MiB max to 264.6 MiB final.
- smaps showed native anonymous/private dirty memory dropping, while JS heap stayed roughly flat.
- The memory source in kitchen appears mostly native, not JS heap.

Known concerning signals:

- Engine RSS did not drop much in the successful 10 to 50 spike: 355.1 MiB max, 347.9 MiB final.
- The release 10 to 200 partial no-reset run had engine RSS at 396.9 MiB final and kitchen RSS at 512.2 MiB final, but this is not a clean memory result because correctness failed and the run was manually stopped.
- The release 10 to 50/200 question needs another clean run after correctness issues are isolated or avoided.

Known release baselines:

- Release engine idle baseline was about 90 to 94 MiB.
- Release kitchen idle baseline was about 233 to 237 MiB.

## Serverless and payload-size context

The kitchen sink runs serverless by default in this path.

The start payload size concern was that serverless actor start carries startup data needed to hydrate the actor, including SQLite preload data. If body-size limits are too low, actor start can fail or silently create misleading runtime behavior. The production checklist was updated to verify that serverless request start body size has generous headroom for SQLite preload data.

The default engine preload size should be treated as the floor for sizing the serverless start body limit. Configure a generous margin above the maximum default SQLite page preload payload, not a tight limit equal to the current default.

## Things that are not proven yet

- We have not proven there are no remaining memory leaks.
- We have not proven whether engine RSS staying high is a leak, allocator behavior, cache retention, or intentionally retained engine state.
- We have not proven the exact root cause of `database disk image is malformed`.
- We have not proven whether `sqlite database was not found in this bucket branch` is expected missing fresh DB state being over-reported or a real storage lookup bug.
- We have not proven whether `onSleep` with undefined state is a core lifecycle bug or just a test actor assumption exposed by failed startup.

## Recommended next steps

1. Focus first on `database disk image is malformed` from the release no-reset spike.
2. Reduce the repro to the smallest concurrency that still fails, likely trying max concurrency 100, 150, then 200.
3. Add structured VFS/depot logs for batch atomic probe, page reads, page writes, branch/generation, and commit finalize.
4. Build a focused direct VFS/depot stress test for repeated open, migrate, write, sleep, and reopen.
5. Separately isolate fresh-actor `reset` failures by running only reset on fresh actors.
6. Add a harness drain timeout so stuck actor-ready retries become an explicit failure.
7. Add a clean post-fix release soak: 60s idle baseline, 5m spike, at least 2 to 5m post-drain, concurrency 10 to 200.
8. Only use clean completed runs for leak conclusions.

## Useful commands

Release build:

```bash
cargo build --release -p rivet-engine
pnpm --filter @rivetkit/rivetkit-napi build:force:release
```

Release reset-enabled spike command shape:

```bash
RIVET_ENGINE_BINARY=/home/nathan/r7/target/release/rivet-engine \
pnpm --filter kitchen-sink memory-soak -- \
  --endpoint http://127.0.0.1:6634 \
  --seed 20260503-142448-sqlite-spike-release-5m-c10-200 \
  --actors 20000 \
  --duration-ms 300000 \
  --cycle-interval-ms 1000 \
  --churn-sleep-after-ms 2000 \
  --spike-min-concurrency 10 \
  --spike-max-concurrency 200 \
  --spike-period-ms 60000 \
  --sample-interval-ms 1000 \
  --pre-workload-wait-ms 60000 \
  --post-churn-wait-ms 60000 \
  --post-cleanup-wait-ms 10000 \
  --no-cleanup \
  --force-gc-samples
```

Release no-reset spike command shape:

```bash
RIVET_ENGINE_BINARY=/home/nathan/r7/target/release/rivet-engine \
pnpm --filter kitchen-sink memory-soak -- \
  --endpoint http://127.0.0.1:6634 \
  --seed 20260503-142821-sqlite-spike-release-5m-c10-200-no-reset \
  --actors 20000 \
  --duration-ms 300000 \
  --cycle-interval-ms 1000 \
  --churn-sleep-after-ms 2000 \
  --spike-min-concurrency 10 \
  --spike-max-concurrency 200 \
  --spike-period-ms 60000 \
  --sample-interval-ms 1000 \
  --pre-workload-wait-ms 60000 \
  --post-churn-wait-ms 60000 \
  --post-cleanup-wait-ms 10000 \
  --no-reset \
  --no-cleanup \
  --force-gc-samples
```
