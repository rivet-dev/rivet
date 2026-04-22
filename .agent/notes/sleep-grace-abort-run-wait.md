# Sleep-grace abort + run-handle wait regression

Discovered during the 2026-04-22 driver-test-runner pass. Causes observed
runtime crashes during workflow tests that trigger a sleep between ticks, and
is the underlying cause of the `actor-run::active run handler keeps actor
awake past sleep timeout` failure.

## Historical behavior

Sleep shutdown on `feat/sqlite-vfs-v2` followed three ordered steps:

1. **Abort** — fire the actor-level abort signal the moment sleep grace
   begins so user code inside `run` / workflow bodies can observe it and
   unwind.
2. **Grace** — wait for the run handler to actually exit (plus the other
   active-work gates).
3. **Finalize** — only after the run handler has joined, tear down dispatch,
   persist, and finish the stop.

## Current behavior (broken)

Located in `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs`:

- `shutdown_for_sleep_grace()` (around line 1232) cancels the idle timer,
  enqueues `BeginSleep` to fire `onSleep`, then awaits
  `wait_for_sleep_idle_window(deadline)`. **It never calls
  `abort_signal.cancel()`.** The only caller of `cancel()` in core is
  `mark_destroy_requested` at `actor/context.rs:466` (the destroy path).
- `wait_for_sleep_idle_window` polls `ActorContext::can_sleep_state`
  (`actor/sleep.rs::229-260`). `can_sleep_state` checks ready/started,
  `prevent_sleep`, `no_sleep`, `active_http_request_count`,
  `sleep_keep_awake_count`, `sleep_internal_keep_awake_count`,
  `pending_disconnect_count`, non-empty conns, and
  `websocket_callback_count`. **It does NOT check whether the `run_handle`
  task is still alive.** `run_handle` lives on `ActorTask`
  (`task.rs:448,1093-1117`), not on `ActorContext`, so the sleep gate
  cannot see it.
- `SleepFinalize` eventually reaches `ShutdownPhase::AwaitingRunHandle`
  (`task.rs:1626-1655`), which awaits `run_handle` for `timeout_duration`
  and calls `run_handle.abort()` on timeout. That abort is a **Tokio task
  abort** — it cancels the Rust future awaiting the TSF promise, but the
  JavaScript promise itself keeps running in Node's event loop.
- After the task joins, `registry/mod.rs:803` clears
  `configure_lifecycle_events(None)`. Anything that still calls
  `request_save_with_revision` hits
  `actor/state.rs:191` (`lifecycle_event_sender()` returns `None`) and
  throws `"cannot request actor state save before lifecycle events are
  configured"`.

## Two independent gaps

### Gap 1 — abort signal never fires on sleep

User `run` handlers and the workflow engine both observe `c.abortSignal` /
`c.aborted` to know when to wind down. Because sleep never fires the abort,
those handlers have no way to cooperate with shutdown. The workflow engine
in particular wires `executeSleep`'s short-sleep path to
`Promise.race([sleep(remaining), this.waitForEviction()])`
(`packages/workflow-engine/src/context.ts:1491`), where `waitForEviction()`
is tied to the abort signal. That race is effectively a plain `sleep` today
because the abort never fires.

### Gap 2 — grace exit doesn't wait for the run handler

Because `can_sleep_state` has no knowledge of `run_handle`, the idle window
can succeed and `SleepFinalize` can start while the run handler (and
whatever JS work it is awaiting) is still live. Tokio-aborting the Rust
future during `AwaitingRunHandle` does not cancel JS, so the workflow
promise continues executing past the point where the registry has torn
down lifecycle events.

## Observed failure mode

`sleeps and resumes between ticks` (`actor-workflow.test.ts::242-253`) with
`workflowSleepActor` (`fixtures/driver-test-suite/workflow.ts::426-445`,
`sleepTimeout: 50`, `ctx.sleep("delay", 40)`):

1. Actor wakes. Workflow runs `step("tick")`, mutates state,
   `flushStorage` → `EngineDriver.batch` → `Promise.all([kvBatchPut,
   stateManager.saveState({ immediate: true })])`
   (`rivetkit/src/workflow/driver.ts:190-207`).
2. Workflow enters `sleep("delay", 40)` short-sleep path:
   `await Promise.race([sleep(40), waitForEviction()])`.
3. Actor `sleepTimeout: 50` fires. `shutdown_for_sleep_grace` begins.
   Abort is never fired → `waitForEviction()` never resolves. Workflow
   is just in a setTimeout.
4. `can_sleep_state` returns `CanSleep::Yes` (no HTTP, no conns, no
   keep-awake gates, and no run-handler gate) → idle window succeeds →
   transition to `SleepFinalize`.
5. `SleepFinalize` drains phases. `AwaitingRunHandle` awaits the run
   handler. After `timeout_duration`, `run_handle.abort()` cancels the
   Rust future. JS promise keeps running.
6. Task joins. `configure_lifecycle_events(None)` at
   `registry/mod.rs:803`.
7. The JS workflow's setTimeout fires. It marks the sleep entry completed
   and calls `flushStorage()` again. `EngineDriver.batch` tries
   `stateManager.saveState({ immediate: true })` →
   `NativeActorContextAdapter.saveState` → native `requestSaveAndWait` →
   core `request_save_with_revision` → `lifecycle_event_sender()` returns
   `None` → throws `"cannot request actor state save before lifecycle
   events are configured"`.
8. Error propagates out of `Promise.all`, becomes an unhandled rejection,
   Node runner crashes. Subsequent `CommandStartActor` deliveries land on
   a dead runner and return `no_envoys`; in-flight NAPI replies resolve as
   `"Actor reply channel was dropped without a response"`.

## Suspected related flakes from the same root cause

From `.agent/notes/driver-test-progress.md`:

- Workflow tests `replays steps and guards state access`,
  `completed workflows sleep instead of destroying the actor`,
  `tryStep and try recover terminal workflow failures` — all hit
  `no_envoys` post-crash.
- `actor-db::handles parallel actor lifecycle churn` — intermittent
  `no_envoys` under concurrent sleep churn.
- `actor-queue::drains many-queue child actors created from actions while
  connected` / `...from run handlers while connected` — intermittent
  "Actor reply channel was dropped without a response" after child-actor
  sleep.

Confirmation that all of these share the same root cause requires a rerun
with `DRIVER_RUNTIME_LOGS=1` and grep for `cannot .* before lifecycle
events are configured` on each failure, but the symptom pattern fits.

## What a fix must do

Restore the three-step ordering from `feat/sqlite-vfs-v2`:

1. On `shutdown_for_sleep_grace()` entry, fire the actor abort signal so
   user `run` handlers and workflow bodies observe
   `c.aborted === true` / `waitForEviction()` resolves. A dedicated
   "sleeping" token separate from the destroy token is fine if we want to
   keep "destroy is stronger" semantics, but the signal observable by user
   code must fire.
2. Gate `wait_for_sleep_idle_window` / the idle window on the run handler
   having actually exited. Add a run-handler-alive signal visible to
   `ActorContext::can_sleep_state` (this is what the original US-103
   story covers). The signal must clear when the run handler returns on
   its own so that `run handler that exits early sleeps instead of
   destroying` still works.
3. Keep the existing `AwaitingRunHandle` timeout abort as a last-resort
   backstop; the new ordering should mean the await almost always sees
   the handle already joined.

## Test evidence after fix

- `active run handler keeps actor awake past sleep timeout`
  (`actor-run.test.ts:43-62`) passes — the user's `while (!c.aborted)`
  only exits when the abort is fired, and firing the abort is tied to
  sleep starting, so while no sleep condition applies the loop keeps
  running.
- `run handler that exits early sleeps instead of destroying` and
  `run handler that throws error sleeps instead of destroying` still
  pass — clearing the flag on run-handler completion means the idle
  window can succeed naturally.
- `sleeps and resumes between ticks` (and the related workflow tests
  currently failing with `no_envoys`) pass because the workflow's short
  `executeSleep` returns via `waitForEviction()` as soon as the abort
  fires, flushes its state while lifecycle events are still live, and
  returns from the run handler before `SleepFinalize` tears down.

## Out of scope

- Destroy path already fires abort through `mark_destroy_requested`;
  only sleep needs the new abort firing.
- The two other crash-symptom paths I saw during the test pass (queue
  drain + parallel lifecycle churn) may share root cause; confirming
  that is a separate verification step, not a fix step.

## Resolved

- Resolved by US-103 in commit 1cecba8a7.
