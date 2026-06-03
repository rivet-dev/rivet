# US-120: actor-sleep `alarms wake actors` flake

## Repro summary

- Rebuilt per PRD:
  - `pnpm --filter @rivetkit/rivetkit-napi build:force`
  - `pnpm build -F rivetkit`
  - `curl -sf http://127.0.0.1:6420/health` -> `{"runtime":"engine","status":"ok","version":"2.3.0-rc.4"}`
- Repro command:
  - `cd rivetkit-typescript/packages/rivetkit && pnpm test tests/driver/actor-sleep.test.ts -t 'static registry.*encoding \(bare\).*Actor Sleep Tests'`
- Consecutive results on the same warmed engine process:

| Run | Start | Result | Duration | Failure shape |
| --- | --- | --- | --- | --- |
| 1 | `00:51:16` | fail | `82.15s` | `alarms wake actors` timed out at `30011ms`; 2x `guard/actor_ready_timeout` |
| 2 | `00:52:39` | fail | `81.72s` | same |
| 3 | `00:54:02` | fail | `81.72s` | same |
| 4 | `00:55:24` | fail | `82.81s` | same |
| 5 | `00:56:47` | pass | `54.10s` | no `actor_ready_timeout` |

Logs are in `.agent/notes/us120-repro/run{1..5}.log`.

## Reference TS check

Verified against `origin/feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts`:

- `cancelAlarm(actorId)` is **local-only**. It aborts `handler.alarmTimeout` and clears local fields. It does **not** call `envoy.setAlarm(actorId, null)`.
- `setAlarm(actor, timestamp)` does persist to the engine via `this.#envoy.setAlarm(actor.id, timestamp)`.

Rust diverges here:

- `rivetkit-core/src/actor/task.rs::finish_shutdown_cleanup_with_ctx(...)` calls `ctx.schedule().sync_alarm_logged()` and then unconditionally `ctx.schedule().cancel_driver_alarm_logged()`.
- `rivetkit-core/src/actor/schedule.rs::cancel_driver_alarm_logged()` cancels local alarm state **and** sends `envoy_handle.set_alarm(actor_id, None, generation)`.

That means sleep shutdown currently does:

1. persist/resync the next scheduled alarm to the engine
2. immediately clear that same engine alarm

TS does step 1 but not step 2 during sleep.

## Failing sequence (`run1`)

Relevant timestamps from `.agent/notes/us120-repro/run1.log`:

1. `07:51:40.421Z`: `setAlarm` request sent for `sleep` actor.
2. Test schedules the alarm for `SLEEP_TIMEOUT + 250ms`, so with the current fixture the alarm deadline is about `1.25s` after this point.
3. Sleep timeout is `1.0s`, so the actor should enter sleep around `07:51:41.421Z`.
4. `07:51:41.676Z`: follow-up `getCounts` request is sent, about `255ms` after the sleep deadline and roughly on top of the alarm deadline race window.
5. `07:51:51.684Z`: first `guard/actor_ready_timeout`.
6. `07:52:01.803Z`: second `guard/actor_ready_timeout`.
7. `07:52:21.xxxZ`: test finally dies at Vitest's `30000ms` timeout.

Observed behavior:

- The actor never becomes ready again.
- There is no runtime panic in the test output.
- The failure is not a wrong assertion on counts; it is a stuck wake where readiness never flips.

## Passing sequence (`run5`)

Relevant timestamps from `.agent/notes/us120-repro/run5.log`:

1. `07:57:11.428Z`: `setAlarm` request sent.
2. `07:57:12.641Z`: follow-up `getCounts` request sent in the same alarm/sleep race window.
3. `07:57:12.713Z`: client disposes normally; the test continues green.

Observed behavior:

- Same actor/test path, same timing shape, but the wake completes immediately instead of wedging behind guard retries.
- This confirms the bug is a race in runtime/engine coordination, not a deterministic bad test expectation.

## Rust lifecycle + engine-alarm state

Current sleep path in `rivetkit-core`:

1. Actor is `Started`.
2. `shutdown_for_sleep_grace()` sends `BeginSleep` and waits for the idle window.
3. `enter_shutdown_state_machine(StopReason::Sleep)` transitions to `SleepFinalize`, cancels local sleep/alarm timers, and disables further alarm dispatch.
4. `finish_shutdown_cleanup_with_ctx(...)`:
   - waits for pending state writes
   - calls `ctx.schedule().sync_alarm_logged()` to re-arm the earliest persisted scheduled event in the engine
   - waits for pending alarm writes
   - cleans up SQLite
   - calls `ctx.schedule().cancel_driver_alarm_logged()`, which sends `set_alarm(None)` to the engine

Engine alarm state across that sequence:

- After `schedule.after(...)`: engine alarm is set to the future alarm timestamp.
- During sleep finalization: the actor still has the persisted scheduled event on disk.
- After `sync_alarm_logged()`: engine alarm is correctly re-set to that persisted timestamp.
- After `cancel_driver_alarm_logged()`: engine alarm is cleared even though the persisted scheduled event still exists.

That leaves the runtime in a split-brain state:

- disk says "future scheduled wake exists"
- engine alarm says "nothing to wake"

## Root-cause hypothesis

The flake is caused by **sleep shutdown clearing the engine-side alarm even though the persisted scheduled event survives sleep**.

Why that matches the observed race:

- The `alarms wake actors` test lands its second `getCounts` request almost exactly when the actor has just slept and the scheduled alarm is about to fire.
- When the engine-side alarm has been cleared during sleep finalization, the actor can end up in a stale "sleeping with persisted future work but no wake trigger" state.
- In the failing runs, that stale state is enough for the HTTP-driven wake to stall until `guard/actor_ready_timeout`.
- In the passing run, the race happens to resolve cleanly before the stale state wedges readiness.

## Proposed fix direction

For `StopReason::Sleep`, preserve the engine-side alarm and only cancel local tokio alarm timeouts. The engine alarm should only be explicitly cleared during `Destroy`, where there is no future instance that needs the wake.
