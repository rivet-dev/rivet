# Parity Audit: `feat/sqlite-vfs-v2` vs Current `rivetkit-core` + `rivetkit-napi`

Date: 2026-04-22

Reference branch access note: Ralph branch-safety rules forbid branch switching and worktrees, so this audit inspected `feat/sqlite-vfs-v2` with `git show` / `git grep` instead of checking it out. The relevant reference tree is `rivetkit-typescript/packages/rivetkit/src/actor/`.

## Lifecycle

- **TS reference**: `ActorInstance.start()` initializes tracing/logging, DB, state, queue, inspector token, vars, `onWake`, alarms, readiness, `onBeforeActorStart`, sleep timer, run handler, then drains overdue alarms. `onStop("sleep" | "destroy")` clears timers, cancels driver alarms, aborts listeners, waits for run/shutdown work, runs `onSleep` or `onDestroy`, disconnects connections, saves immediately, waits writes, and cleans DB.
- **Current code**: Core `ActorTask` owns explicit states (`Loading`, `Started`, `SleepGrace`, `SleepFinalize`, `Destroying`, `Terminated`) and two-phase sleep shutdown. NAPI still owns JS lifecycle callbacks and user tasks through `napi_actor_events.rs`. Startup is split: core loads/persists actor state and restores conns, while NAPI handles `createState`, `createVars`, `onMigrate`, `onWake`, `onBeforeActorStart`, and `serializeState`.
- **Divergence**: Mostly intentional migration architecture, but there is one likely bug: `registry/native.ts` wires `onWake` to `config.onBeforeActorStart` and `onBeforeActorStart` to `config.onWake`, while NAPI expects the names literally.
- **Remediation**: Add targeted lifecycle-order tests, then fix the callback mapping. Keep the two-phase sleep model; it is an intentional improvement over the TS reference.

Tracked references: complaints #2, #8, #19, #21, #22.

## State Save Flow

- **TS reference**: `StateManager` uses an `on-change` proxy, validates CBOR serializability, emits inspector state updates, runs `onStateChange`, throttles saves with `SinglePromiseQueue`, and writes actor state plus dirty hibernatable conns in one KV batch. `saveState({ immediate: true })` waits for durability.
- **Current code**: TS serializes state deltas through the NAPI `serializeState` callback; core applies `StateDelta` values and persists to KV. Core still exposes `set_state` / `mutate_state`; NAPI still exposes public `set_state`; NAPI `save_state` still accepts `Either<bool, StateDeltaPayload>`. `save_guard` is held across KV writes.
- **Divergence**: Bugs / cleanup debt. The desired model is one structured delta path: `requestSave` -> `serializeState` -> `StateDelta` -> KV. Legacy replace-state and boolean-save surfaces can mislead callers.
- **Remediation**: Remove public replace-state APIs, collapse request-save variants, drop the boolean `saveState` shim, and split `save_guard` so KV latency does not serialize save callers.

Tracked references: complaints #9, #14.

## Connection Lifecycle

- **TS reference**: `ConnectionManager.prepareConn()` gates new conns through `onBeforeConnect`, creates conn state, constructs hibernatable or ephemeral conn data, then `connectConn()` inserts the conn synchronously, schedules hibernation persist for hibernatable conns, calls `onConnect`, emits inspector updates, resets sleep, and sends init.
- **Current code**: Core owns raw `ConnHandle`s and disconnect handlers; NAPI adapts them to decoded TS objects. Disconnect callbacks are async and are tracked through NAPI-spawned tasks; shutdown drains after disconnect callbacks before final persistence. Core transport disconnect paths now remove successful conns and aggregate failures.
- **Divergence**: Mostly intentional, but conn-state dirtiness still crosses layers awkwardly. `NativeConnAdapter` stores decoded conn state in TS-side `NativeConnPersistState` and manually calls `ctx.requestSave(false)` for hibernatable writes instead of core owning dirty tracking.
- **Remediation**: Move hibernatable conn dirty tracking into core `ConnHandle::set_state`, emit save requests there, and delete TS-side dirty bookkeeping once core can serialize dirty conns.

Tracked references: complaints #15, #19, #21.

## Queue

- **TS reference**: Queue metadata lives at `[5, 1, 1]`, messages under `[5, 1, 2] + u64be(id)`. Enqueue writes message + metadata together, waiters are resolved after writes, receive waits observe actor abort, and `enqueueAndWait` completion waits are independent of actor abort.
- **Current code**: Core matches the key layout and message encoding shape, uses `Notify` plus `ActiveQueueWaitGuard`, tracks queue waits in metrics/sleep activity, and intentionally ignores actor abort for `enqueue_and_wait` completion waits. Inspector queue size updates are core callbacks.
- **Divergence**: Mostly intentional parity. Remaining risk is implementation quality, not semantics: enqueue holds the queue metadata async mutex across the KV write to preserve id/size rollback behavior.
- **Remediation**: Keep current semantics. Consider a follow-up to isolate queue id reservation from KV latency if queue throughput becomes an issue.

Tracked references: none direct beyond the general async-lock invariant in the PRD.

## Schedule / Alarms

- **TS reference**: Scheduled events are stored in actor persist data. `initializeAlarms()` sets the next future host alarm; `onAlarm()` drains due events, reschedules the next alarm, and runs scheduled actions under `internalKeepAwake`. Shutdown cancels driver alarms for both sleep and destroy, relying on wake startup to re-arm.
- **Current code**: Core stores scheduled events in persisted actor state, uses `sync_alarm()`, drains overdue events after startup, and dispatches scheduled actions as tracked tasks. Current shutdown keeps the engine alarm armed across `Sleep` by cancelling only local Tokio alarm timeouts, while `Destroy` still clears the driver alarm.
- **Divergence**: Intentional bug fix vs TS reference. Keeping the engine alarm across sleep is required so scheduled events wake sleeping actors without an external request. Unconditional startup/shutdown alarm pushes remain noisy.
- **Remediation**: Keep sleep alarms armed, add focused driver tests, and deduplicate `set_alarm` pushes with dirty/last-pushed tracking.

Tracked references: complaints #6, #22.

## Inspector

- **TS reference**: `ActorInspector` is an event-emitter facade over live actor state, connections, queue status, database schema/rows, workflow history, replay, and action execution. State updates emit immediately from the proxy path; queue/connection changes update cached inspector counters.
- **Current code**: Core tracks inspector revisions, connected clients, queue size, and active connections. NAPI/native TS serves inspector HTTP and bridges workflow history/replay. Core also has overlay broadcasts driven by inspector attachment changes and serialize-state ticks.
- **Divergence**: Mostly intentional split, but attachment accounting is manually incremented/decremented in `ActorContext`, so early returns or panics can leak the attached count.
- **Remediation**: Introduce an `InspectorAttachGuard` RAII type and route subscriptions through it.

Tracked references: complaint #17.

## Hibernation

- **TS reference**: Hibernatable connections persist under `[2] + conn_id` using actor-persist v4 BARE. Restore loads persisted conns, drops dead hibernatable transports after liveness checks, persists ack metadata before KV writes, and removes hibernation data on disconnect.
- **Current code**: Core uses the same key prefix and embedded-version persistence, restores hibernatable conns, asks envoy about liveness, removes dead conns, and prepares hibernation deltas before saving. TS still owns decoded conn-state caching and hibernatable websocket ack-state serialization inputs.
- **Divergence**: Partly intentional during NAPI migration, partly bug risk. The persistence layout matches, but state dirtiness and ack snapshots are not yet fully core-owned.
- **Remediation**: Complete core-owned dirty tracking for hibernatable conns and keep ack snapshot ordering covered by driver tests.

Tracked references: complaint #15.

## Follow-Up Story Candidates

- **Fix native lifecycle callback mapping**: `registry/native.ts` should wire `config.onWake` to NAPI `onWake` and `config.onBeforeActorStart` to NAPI `onBeforeActorStart`; add a driver test that proves ordering on new and restored actors.
- **Add lifecycle parity driver tests**: Cover `onWake`, `onBeforeActorStart`, run-handler startup, sleep, destroy, and restart ordering against the native runtime so the core/NAPI split cannot drift silently.
- **Audit queue lock scope under KV latency**: Determine whether `Queue::enqueue_message` can avoid holding the metadata mutex across KV writes without breaking id allocation, rollback, or max-size semantics.
- **Document core/NAPI ownership boundaries**: Add an internal note that core owns lifecycle state/persistence/sleep, while NAPI owns JS user tasks and callback invocation until the migration is complete.
