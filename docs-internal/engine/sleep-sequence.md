# Sleep sequence invariants

Design constraints and invariants for the RivetKit actor sleep / destroy lifecycle. Pair with `actor-task-dispatch.md` and `rivetkit-core-internals.md` for surrounding context.

## Authority

- The engine owns lifecycle authority. `ctx.sleep()` and `ctx.destroy()` send fire-and-forget `ActorIntent` events; they do not transition lifecycle state locally. The local `SleepGrace` / `DestroyGrace` transition runs when the engine replies with `StopActor`.
- `envoy-client` retries intent delivery across reconnects via checkpoint-based event replay (`engine/sdks/rust/envoy-client/src/events.rs`). Core does not need its own retry path.

## Public surface: keep-awake primitives

Two user-facing primitives in TypeScript. Both accept a `Promise`, never a closure.

| Method | Blocks idle sleep | Blocks grace finalize | Notes |
| --- | --- | --- | --- |
| `c.keepAwake(promise)` | Yes | Yes | Returns the same promise. Use for work the actor must stay up for. |
| `c.waitUntil(promise)` | No | Yes | Returns void. Use for best-effort flush/cleanup work that is allowed to complete inside the grace window. |

`c.setPreventSleep(b)` and `c.preventSleep` are deprecated no-ops retained for binary / call-site compatibility. They will be removed in 2.2.0.

### Why two primitives and not one

`keepAwake` is scoped, non-leaky, and symmetric with `waitUntil`. `setPreventSleep` was a flag that had to be paired by hand; forgetting to clear it wedged the actor awake. A promise-scoped counter cannot leak: when the promise settles (resolve or reject), the counter decrements.

### Why separate `keep_awake` and `internal_keep_awake` in core

Kept separate for debug visibility. Grace deadline warn logs report each counter independently so diagnostics distinguish user keep-awake sites from framework-owned keep-awake sites (schedule alarms, queue receives).

## Sleep readiness predicates

Two predicates govern the sleep state machine. Both live on `ActorContext` / `SleepState`.

- `can_arm_sleep_timer()` — the idle predicate. Returns `CanSleep::Yes` only when every sleep-affecting counter is zero and the run handler is inactive (or waiting on a queue). Used to start the sleep idle timer.
- `can_finalize_sleep()` — the grace predicate. Returns `true` only when every shutdown-affecting counter is zero: `core_dispatched_hooks`, `shutdown_task_count`, `sleep_keep_awake`, `sleep_internal_keep_awake`, `active_http_requests`, `websocket_callbacks`, `pending_disconnects`. Used to advance from `SleepGrace` to `SleepFinalize` (or finalize destroy).

Removing `preventSleep` deleted both predicate branches. Any future sleep-affecting counter must add an entry in each predicate and must call `ActorContext::reset_sleep_timer()` on transitions that change the result.

## Grace period and abort signals

- `start_grace(reason)` fires at the start of `SleepGrace` / `DestroyGrace`. It cancels the sleep idle timer, cancels the actor abort signal (`actor_abort_signal`), installs a `SleepGraceState` with the effective grace deadline, and resets the sleep timer to arm the grace tick.
- The actor abort signal is a soft signal: "shutdown has started, please wrap up." User code observes it via `c.abortSignal`. It does not force-stop work.
- Destroy requests also use the normal grace path. The actor abort signal fires when destroy grace starts.

## Grace deadline enforcement

When the grace deadline elapses before `can_finalize_sleep()` returns true:

- `on_sleep_grace_deadline` aborts the user `run` handle (`run_handle.abort()`), cancels the shutdown deadline token (`cancel_shutdown_deadline()`), records the timeout, and emits a structured warn log enumerating every non-drained counter.
- The NAPI `RunGracefulCleanup` task observes `shutdown_deadline_token()` via `tokio::select!` and aborts its in-flight `onSleep` / `onDestroy` call so SQLite and KV cleanup in `teardown_sleep_state` do not race against mid-commit user work.
- Foreign-runtime adapters that run user cleanup callbacks must observe the shutdown deadline token the same way.

## Lost generations

- `StopActor { reason: Lost }` or an envoy-side lost timeout means the engine has given up on the generation and may already be running the next one. envoy-client cancels a per-generation lost `CancellationToken` synchronously, before the stop is queued, and passes it to core through `on_actor_start_with_lost_signal`.
- Core adopts the token as the context lost signal (`ActorContext::is_lost`). Once it fires, `SqliteDb` rejects every statement except `ROLLBACK` and `LegacyActorKv` rejects writes, so state, KV, queue, and workflow writes from the lost generation are revoked without waiting on the task.
- `ActorTask` observes the token in `run_live`, in graceful shutdown, and in serialization waits, and jumps to `abort_lost()`. That path runs no user sleep hooks, skips the final save and alarm sync, dispatches runtime cleanup once, and bounds every wait by `LOST_SHUTDOWN_GRACE_PERIOD`. Cleanup that has not finished stays owned by its retained task.
- The final `ActorStateStopped` for a lost generation is always `StopCode::Error` with "envoy connection lost", including when Lost escalates a pending graceful stop. `StopCode::Ok` would make the engine destroy the actor.
- Already-submitted SQLite commits cannot be recalled. Depot generation checks are the backstop for those.
- A lost generation starts no new user callback. Every runtime callback passes through `refuseCallbacksAfterLost` in `registry/native.ts` when JS enters it, except the `onSleep` and `onDestroy` cleanup wrappers, which release runtime state and skip the user hook themselves. Callbacks the framework starts on its own (workflow steps, run, inspector workflow calls, deferred `onStateChange`, WebSocket open and message listeners, database provider hooks) check the lost state first. Continuations inside a user call that is already running, and abort or close listeners, are not blocked.
- envoy-client declares all its actors lost once no engine ping has arrived for `envoy_lost_threshold` minus a 3s margin. The engine refreshes its liveness timestamp just before each ping and reallocates actors once that timestamp is older than the threshold, so the envoy normally stops first, including on half-open connections that never report a close. This is best effort: a ping delayed in transit by more than the margin can still let the engine give up first. The deadline counts from `EngineLiveness::baseline` on a monotonic clock. After a reconnect the engine refreshes its liveness timestamp only when it claims the connection, after sending `ToEnvoyInit`, so the baseline keeps the last proven refresh (a ping, or the install time of a connection claimed by commands) until a ping or WebSocket commands arrive on the new connection, then counts from when the client started opening that connection. Serverless start payloads injected over HTTP do not count as a claim. A generation created after the baseline counts from its creation instead, since the engine only assigns it over a claimed connection, and only generations past their own deadline are stopped.

## Guarding lifecycle requests

- `ctx.sleep()` and `ctx.destroy()` return `Result<()>`. They fail with `actor/starting` if called before startup completes and `actor/stopping` if the request flag has already been swapped to true for this generation. An atomic `swap(true, ...)` on `sleep_requested` / `destroy_requested` enforces single-shot request semantics per generation.
- The idle sleep timer request path (`spawn_sleep_timer_task`) and the `ActorTask` sleep-tick path both suppress the already-requested error: idle-driven requests may race user-driven requests and the warning is informational.

## Serialize-state shutdown cap

`SERIALIZE_STATE_SHUTDOWN_SANITY_CAP = 15s` is the upper bound on how long the shutdown `SerializeState` reply wait is allowed to pend before `save_final_state` falls back to empty deltas (preserving prior state). This is a sanity cap, not a deadline anyone should ever hit; the normal drain finishes in milliseconds.

## Test harness parity

- Rust integration tests live in `rivetkit-core/tests/modules/sleep.rs` and pin predicate behavior, grace period selection, and `save_final_state` cap.
- TypeScript driver tests in `rivetkit-typescript/packages/rivetkit/tests/driver/actor-sleep*.test.ts` cover abort-signal-at-grace-entry, `keepAwake` holding shutdown, `c.db` writes surviving `onSleep`, and regression coverage for `setPreventSleep` being a no-op.
