# Lifecycle Shutdown Unified Drain

## Problem

Actor shutdown in rivetkit-core is distributed across three coordinating subsystems that have drifted out of sync with both the docs and each other.

**Signal paths have duplicated.** `ctx.reset_sleep_timer()` fans out through `LifecycleEvent::ActivityDirty` on the ordered lifecycle-events channel (`context.rs:1241-1272`, `task.rs:347/359/840-843`) *and* through `activity_notify: Arc<Notify>` on a separate select arm (`task.rs:597-601`). Both converge on the same `reset_sleep_deadline`. `AsyncCounter::register_change_notify` (`sleep.rs:615`, `async_counter.rs:37-43`) ties counters into `activity_notify` with `notify_waiters()` semantics, which mixes badly with the alternative `notify_one()` pattern — wakes can be silently lost across select iterations.

**Grace-exit uses a stored boxed future.** `SleepGraceState { deadline, grace_period, idle_wait: Pin<Box<dyn Future<Output = bool>>> }` (`task.rs:375-379, 464`) and `wait_for_sleep_idle_window` (`sleep.rs:379-396`) exist because raw `Notify::notified()` loses wakes across select iterations, and the sleep-grace predicate needs to persist. This is structurally different from the sleep timer (`sleep_deadline: Option<Instant>` + `sleep_until` select arm), even though both are solving "condition change → main loop re-evaluates truth."

**`run_shutdown` contains user-code wait points with its own budget.** `run_shutdown` (`task.rs:1505-1686`) awaits user code at five points: `wait_for_on_state_change_idle`, the `FinalizeSleep`/`Destroy` event reply (which transitively runs `onSleep`/`onDestroy`/`onDisconnect`/`serializeState`), two `drain_tracked_work_with_ctx` calls, `disconnect_for_shutdown_with_ctx`, and a final `run_handle.take()` + select-with-sleep block (`task.rs:1657-1680`). For Sleep, the budget at entry is a *fresh* `now + effective_sleep_grace_period()`, after `start_sleep_grace` already consumed up to `sleep_grace_period` in the idle-wait phase. Total wall-clock is 2× `sleepGracePeriod`, contradicting `website/src/content/docs/actors/lifecycle.mdx:818`.

**Ordering violates the doc contract.** `lifecycle.mdx:838-843` promises: step 2 waits for `run`, step 3 runs `onSleep`. Actual code: `onSleep` is spawned from `BeginSleep` at grace entry (`task.rs:2176`, `napi_actor_events.rs:566-575`), and `run_handle` is awaited at the *end* of `run_shutdown` (`task.rs:1657-1680`) — hooks run concurrently with `run`.

**Self-initiated destroy bypasses grace.** `c.destroy()` and `c.sleep()` set flags on ctx; `handle_run_handle_outcome` (`task.rs:1322, 1337-1349`) observes the flag when `run` returns and jumps straight to `LiveExit::Shutdown` without invoking the grace path. Under the current design that still fires hooks via `FinalizeSleep`/`Destroy` events inside `run_shutdown`. Under a "hooks run during grace" redesign, this path silently skips hooks unless fixed.

**Dead and undocumented timeouts persist.** `run_stop_timeout` is used only at `task.rs:1659` to cap the final run-handle wait inside `run_shutdown`. `on_sleep_timeout` (`config.rs:54/71/106/200-225`) wraps the `onSleep` call spawned from `BeginSleep` (`napi_actor_events.rs:571-574`) and is also referenced as a fallback inside `effective_sleep_grace_period` (`config.rs:245-254`). Neither appears in `lifecycle.mdx` as a user-facing knob.

## Goals

1. One signal primitive for "maybe sleep state changed." Same primitive drives the idle-sleep timer (Started) and the grace-drain predicate (SleepGrace/DestroyGrace).
2. Two evaluation functions, one per lifecycle context: `can_arm_sleep_timer()` for Started, `can_finalize_sleep()` for grace.
3. All arbitrary user code (hooks, waitUntil, async WS handlers, user `run` handler, onDisconnect) runs inside `run_live`. `run_shutdown` contains only core work and a single bounded-internal-timeout call for `serializeState` coordination.
4. One budget per shutdown reason. Sleep = `sleepGracePeriod`. Destroy = `on_destroy_timeout`. Total wall-clock from grace entry to save equals the configured budget, not 2×.
5. `run` exits (either cleanly or via abort) before state save starts. Structurally enforced.
6. Zero stored polled futures, zero polling loops, zero background tasks for shutdown orchestration. `SleepGraceState` shrinks to `{ deadline, reason }`.
7. Self-initiated `c.sleep()` / `c.destroy()` enter grace through the same path as engine-initiated Stop.

## Non-goals

- Changing the engine-side actor2 protocol.
- Reworking the NAPI adapter's JoinSet/TSF architecture beyond what the new events require.
- Changing public `Actor` / `Registry` / `Ctx` API shape.
- Adding a new sync-registered `serializeState` callback on `ActorContext` (deferred; see §11.3).
- Changing `AsyncCounter` (the primitive itself is fine; the consumers misuse it).

## Design

### 1. Single signal primitive

Replace the five-layer `LifecycleEvent::ActivityDirty` path with `activity_notify: Arc<Notify>` + `activity_dirty: AtomicBool` as the only route from "condition changed" to "main loop re-evaluates."

```rust
// ctx
pub fn reset_sleep_timer(&self) {
    if !self.0.activity_dirty.swap(true, Ordering::AcqRel) {
        self.0.activity_notify.notify_one();
    }
}
```

`notify_one` semantics are load-bearing: it stores one permit, so a wake that arrives while the main loop is in an `.await` is caught on the next select iteration. The `AtomicBool` is a hot-path dedup only.

All wake sources must route through `reset_sleep_timer`. The existing `AsyncCounter::register_change_notify(&activity_notify)` wiring (`sleep.rs:615`) uses `notify_waiters()` (`async_counter.rs:79`) and must be removed or rewrapped. Replacement: a `register_change_callback(Box<dyn Fn()>)` that invokes `reset_sleep_timer` directly on every counter change. `AsyncCounter` itself is unchanged; the consumer pattern changes.

**Deletions:**
- `LifecycleEvent::ActivityDirty` variant (`task.rs:347`) and kind label (`task.rs:359`).
- Main-loop match arm for `ActivityDirty` (`task.rs:840-843`).
- Channel enqueue in `notify_activity_dirty` (`context.rs:1241-1272`). Function becomes thin wrapper over `reset_sleep_timer`.
- Parallel `activity_wait` select arm (`task.rs:597-601`). Single `_ = activity_notify.notified()` arm replaces both.
- `drain_activity_dirty` helper (`connection.rs:1193-1212`) and callers at `connection.rs:1432, 1458`. The `panic!("expected only ActivityDirty")` assertion (`connection.rs:1198`) is no longer reachable.

### 2. Two readiness functions

`can_sleep_state` (`sleep.rs:264-300`) today mixes concerns: readiness flags (`ready`, `started`), activity flags (`prevent_sleep`, `no_sleep`), run state (`run_handler_active_count`), drain counters, conn state. Split into two:

**`can_arm_sleep_timer() -> CanSleep`** (async, for `Started` only). Preserves existing `can_sleep_state` semantics. Used to decide whether `sleep_deadline` is armed.

**`can_finalize_sleep() -> bool`** (sync, for `SleepGrace | DestroyGrace` only). Returns `true` iff:
- `core_dispatched_hooks.load() == 0` (new counter — core-owned accounting for `RunGracefulCleanup` and `DisconnectConn` events; §9 "counter ownership")
- `shutdown_counter == 0` (user `waitUntil` + async WS handlers — this counter is new as a `can_*` input; it's currently tracked separately)
- `sleep_keep_awake_count == 0` (adapter's own tracked-work counter; retained for non-hook tracked work)
- `sleep_internal_keep_awake_count == 0`
- `active_http_request_count == 0`
- `websocket_callback_count == 0`
- `pending_disconnect_count == 0`
- `!prevent_sleep` (honors `lifecycle.mdx:818,746` promise)

Explicitly **not** checked in `can_finalize_sleep`:
- `ready` / `started` — flipped to `false` at grace entry (§4); not relevant to drain.
- `run_handler_active_count` — the `run_handle.is_none()` gate handles this at the caller (§3); subsuming it into `can_finalize_sleep` produces a double-gate.
- `conns().is_empty()` — conns are torn down during grace via `DisconnectConn` events; `pending_disconnect_count` covers outstanding `onDisconnect` callbacks.

### 3. Single main-loop handler

```rust
_ = self.ctx.activity_notify().notified() => {
    self.ctx.acknowledge_activity_dirty();
    match self.lifecycle {
        LifecycleState::Started => {
            let armable = self.ctx.can_arm_sleep_timer().await == CanSleep::Yes;
            self.sleep_deadline = armable
                .then(|| Instant::now() + self.factory.config().sleep_timeout);
        }
        LifecycleState::SleepGrace | LifecycleState::DestroyGrace => {
            if self.ctx.can_finalize_sleep() && self.run_handle.is_none() {
                return LiveExit::Shutdown {
                    reason: self.sleep_grace.as_ref().unwrap().reason,
                };
            }
        }
        _ => {}
    }
}
```

The `run_handle.is_none()` gate is what structurally enforces "run exits before save." `handle_run_handle_outcome` (`task.rs:1322`) must call `ctx.reset_sleep_timer()` immediately after writing `self.run_handle = None`, otherwise the drain path silently degrades to the deadline path when `run` returns after the last tracked task.

### 4. Grace entry

All four triggers (engine Sleep, engine Destroy, `c.sleep()`, `c.destroy()`) route through one `begin_stop(reason, source)`:

- Engine `Stop { reason }` from `lifecycle_inbox` → `begin_stop(reason, External)`.
- `c.sleep()` → `ctx.mark_sleep_requested()` enqueues `LifecycleCommand::Stop { Sleep, source: Internal }` onto `lifecycle_inbox` → `begin_stop`.
- `c.destroy()` → `ctx.mark_destroy_requested()` enqueues `LifecycleCommand::Stop { Destroy, source: Internal }` → `begin_stop`.

`handle_run_handle_outcome`'s shortcut to `LiveExit::Shutdown` for self-initiated requests (`task.rs:1337-1349`) is **removed**. The flag still clears `run_handle`; the Stop command for grace entry comes from the inbox.

`begin_stop(reason, _)` when `lifecycle == Started`:

1. Register shutdown reply on `shutdown_reply`.
2. `drain_accepted_dispatch()` — pull already-accepted dispatch into tracked work so it completes within the window.
3. **Alarm cleanup moved here** (was in `run_shutdown`):
   - `ctx.suspend_alarm_dispatch()`
   - `ctx.cancel_local_alarm_timeouts()`
   - `ctx.set_local_alarm_callback(None)`
   - For `Destroy` only: `ctx.cancel_driver_alarm()`. (Sleep keeps the driver alarm armed so wake-from-alarm works.)
4. **Fire the abort signal.** `self.shutdown_cancel_token.cancel()` is the single abort primitive for this actor. `c.abortSignal` is a JS-side wrapper over the same token (wired at NAPI ctx init), so `c.aborted === true` observers and adapter tracked-task cancellation both fire from this one call. The legacy `ctx.cancel_abort_signal_for_sleep()` becomes a thin wrapper that calls `shutdown_cancel_token.cancel()` and is kept only for source compatibility during migration.
5. Transition:
   - `Sleep` → `LifecycleState::SleepGrace`.
   - `Destroy` → `LifecycleState::DestroyGrace` (new variant).
   - `transition_to` sets `ready=false`, `started=false` for both, **and calls `reset_sleep_timer` after the flip** so the predicate re-evaluates.
6. Compute `deadline`:
   ```rust
   let deadline = Instant::now() + match reason {
       StopReason::Sleep => config.sleep_grace_period,
       StopReason::Destroy => config.on_destroy_timeout,
   };
   self.sleep_grace = Some(SleepGraceState { deadline, reason });
   ```
7. **Bump the drain counter and emit cleanup events.** Core owns a new dedicated counter `core_dispatched_hooks: AsyncCounter` that feeds `can_finalize_sleep` (separate from `sleep_keep_awake_count` which the adapter already tracks for its own tasks — rationale in §9 "counter ownership"). For each event emitted below, increment `core_dispatched_hooks` **before** the emit so the main loop's next evaluation of `can_finalize_sleep` cannot observe a stale zero. The adapter's hook-completion path signals core back via a completion callback that decrements.

   Events are emitted on an unbounded channel (see §10) so `send()` cannot block `begin_stop`:
   - One `ActorEvent::RunGracefulCleanup { reason }`.
   - Per-conn `ActorEvent::DisconnectConn { conn_id }`:
     - `Sleep`: non-hibernatable conns only. For hibernatable conns, call `ctx.request_hibernation_transport_removal(conn_id)` which flushes hibernation metadata into `pending_hibernation_updates`. `onDisconnect` is **not** fired for hibernatable conns on sleep/wake — they survive the transition; only a legitimate disconnect (user close, error, explicit `conn.disconnect()`, or Destroy) fires `onDisconnect`.
     - `Destroy`: all conns, including hibernatable. `onDisconnect` fires for all of them. `request_hibernation_transport_removal` is **not** called (the actor is terminating; hibernation metadata is not preserved).
8. `ctx.reset_sleep_timer()` — prime the loop for one evaluation pass with the new predicate set.

`begin_stop` when `lifecycle == SleepGrace | DestroyGrace`:

- Matching reason: reply `Ok` idempotently, return. No re-entry.
- Different reason: `debug_assert!(false, "engine actor2 sends one Stop per actor instance and does not upgrade Sleep→Destroy")`, log warning, reply `Ok`. This case is unreachable under the engine actor2 invariant (`engine/packages/pegboard/src/workflows/actor2/mod.rs:990-1023` skips re-sending Stop once one has been issued, with comment `// Stop command was already sent`). Self-initiated `c.destroy()` from inside `onSleep` would hit this path, and is also unreachable because self-initiated requests flow through the same engine round-trip and the engine de-dups.

`begin_stop` when `lifecycle == SleepFinalize | Destroying | Terminated | Loading`: reply `Ok` (idempotent), log a warning if unexpected. No re-entry into grace.

### 5. Grace exit

**Drain path.** `on_activity_signal` evaluates `can_finalize_sleep() && run_handle.is_none()`. Both true → return `LiveExit::Shutdown { reason }`.

**Deadline path.** `_ = sleep_until(grace.deadline), if self.sleep_grace.is_some()` fires:
1. `self.run_handle.as_mut().map(JoinHandle::abort)`. Do not await.
2. `ctx.record_shutdown_timeout(reason)` for metrics.
3. Return `LiveExit::Shutdown { reason }`.

The abort signal was already cancelled at grace entry (§4 step 4), so tracked tasks observing `shutdown_cancel_token` have been seeing cancellation since t=0. No separate cancel call is needed here — adapter JoinSet teardown is already in motion by the time the deadline fires.

Both paths exit `run_live` with `run_handle` drained or aborted, and all tracked work either completed or abort-signaled. There is no user code left that could block `run_shutdown`.

`shutdown_cancel_token` is a new `CancellationToken` field on `ActorTask`, cloned once into `ActorContextShared` at ctx init. Core owns it; core cancels it at grace entry (§4 step 4); adapter tracked tasks select on it alongside their own futures. `c.abortSignal` on the JS surface is a wrapper over the same token: when the token is cancelled, both user code reading `c.aborted` and the adapter's tracked-task cancellation observe the same event.

### 6. Select-loop arm inventory during grace

Arms that fire during grace:

| Arm | Purpose | Notes |
|---|---|---|
| `lifecycle_inbox.recv()` | receive duplicate Stop | Routes through `begin_stop` (§4); idempotent ack. |
| `activity_notify.notified()` | any readiness flip or counter change | Runs `on_activity_signal` (§3). |
| `wait_for_run_handle(run_handle)` | user `run` returns or panics | `handle_run_handle_outcome` clears handle **and calls `reset_sleep_timer`**. |
| `sleep_until(grace.deadline)` | deadline hit | Deadline path (§5). |

Arms that are gated off during grace:

| Arm | Gate | Why |
|---|---|---|
| `dispatch_inbox.recv()` | `accepting_dispatch()` requires `Started` | No new dispatch during grace. |
| `fire_due_alarms()` | Lifecycle check + alarm dispatch suspended in step 4.3 | No new alarm dispatch during grace. |

The `activity_wait` parallel arm (`task.rs:597-601`) is deleted (§1).

**`DestroyGrace` match-arm decisions.** `DestroyGrace` is a new variant. Every existing match arm that includes `Started | SleepGrace` or checks `lifecycle` must pick whether `DestroyGrace` joins:

| Predicate / match site | Started | SleepGrace | DestroyGrace | Rationale |
|---|---|---|---|---|
| `accepting_dispatch()` | yes | no | no | Readiness is false during grace; no new dispatch. |
| `state_save_timer_active()` | yes | yes | **no** | Destroy flushes state once in `run_shutdown`; no incremental saves during terminal grace. |
| `inspector_serialize_timer_active()` | yes | yes | **no** | Inspector serialize paused during terminal grace. |
| `fire_due_alarms` lifecycle gate | yes | no | no | Alarms already suspended at grace entry. |
| `schedule_state_save` early-return | no-op (continues) | no-op | **early-return** | No scheduled saves during terminal grace. |
| `transition_to` `set_ready`/`set_started` | true / true | **false / false** | **false / false** | Engine routing stops for both graces. |
| "actor is logically live" checks (generic `Started | SleepGrace` matches outside the table above) | yes | yes | case-by-case | Default to **no** for DestroyGrace unless there's a positive reason; audit each during implementation. |

Incremental saves during `SleepGrace` stay allowed so mutations from `onSleep` can persist without waiting for `run_shutdown`. Incremental saves during `DestroyGrace` are skipped because `save_final_state` is the authoritative flush and there's no post-destroy actor to observe intermediate state.

### 7. `run_shutdown` — pure core

```rust
async fn run_shutdown(&mut self, reason: StopReason) -> Result<()> {
    self.sleep_grace = None;
    self.sleep_deadline = None;
    self.state_save_deadline = None;
    self.inspector_serialize_state_deadline = None;

    self.transition_to(match reason {
        StopReason::Sleep => LifecycleState::SleepFinalize,
        StopReason::Destroy => LifecycleState::Destroying,
    });

    self.save_final_state().await?;

    if matches!(reason, StopReason::Destroy) {
        self.ctx.mark_destroy_completed();
    }

    self.finish_shutdown_cleanup(reason).await?;
    self.transition_to(LifecycleState::Terminated);
    self.ctx.record_shutdown_wait(reason, /* elapsed */);
    Ok(())
}
```

**Deleted from `run_shutdown`:**
- `wait_for_on_state_change_idle` call (`task.rs:1541`) — `onStateChange` callbacks must be counter-tracked (see §11.1) so they drain during grace, not in `run_shutdown`.
- `FinalizeSleep`/`Destroy` event enqueue + `timeout(reply_rx)` (`task.rs:1560-1612`).
- Both `drain_tracked_work_with_ctx` calls (`task.rs:1617`, `:1643`).
- `disconnect_for_shutdown_with_ctx` (`task.rs:1632`).
- `run_handle.take()` + select-with-sleep (`task.rs:1657-1680`).
- `remaining_shutdown_budget` helper (`task.rs:2185`) — no callers remain.
- `effective_run_stop_timeout()` call at `task.rs:1659` — the whole block is gone, so this is the last caller.

`save_final_state` handles both state-delta serialization and hibernation-metadata flush. See §8.

### 8. State save wiring

Keep `ActorEvent::SerializeState { reply }` for state-delta serialization. It is the only way user-owned state becomes bytes, and replacing it with a sync core-callable surface is a large NAPI change deferred to §11.3.

`save_final_state`:

```rust
async fn save_final_state(&mut self) -> Result<()> {
    const SERIALIZE_SANITY_CAP: Duration = Duration::from_secs(30);

    let (reply_tx, reply_rx) = oneshot::channel();
    match self.actor_event_tx.as_ref().unwrap().try_reserve_owned() {
        Ok(permit) => permit.send(ActorEvent::SerializeState { reply: reply_tx.into() }),
        Err(_) => {
            tracing::error!("shutdown serialize-state enqueue failed");
            // Proceed with empty deltas rather than block.
            return self.ctx.save_state(Vec::new()).await;
        }
    }

    let deltas = match timeout(SERIALIZE_SANITY_CAP, reply_rx).await {
        Ok(Ok(Ok(deltas))) => deltas,
        Ok(Ok(Err(error))) => {
            tracing::error!(?error, "serializeState callback returned error");
            Vec::new()
        }
        Ok(Err(_)) | Err(_) => {
            tracing::error!("serializeState timed out or dropped reply");
            Vec::new()
        }
    };

    self.ctx.save_state(deltas).await?;
    self.ctx.flush_pending_hibernation_updates().await?;
    Ok(())
}
```

The 30s cap is a hard-coded sanity bound, **not** a user-configurable timeout. `serializeState` is expected to be a fast in-memory transformation; the cap only exists to prevent a stuck callback from hanging shutdown forever. If it fires, we log and proceed with empty deltas (the user's in-memory state is lost; better than a hang).

`flush_pending_hibernation_updates` is a new method (or a rename of existing logic inside `finish_shutdown_cleanup_with_ctx`'s `request_hibernation_transport_save` + `save_state(Vec::new())` dance at `task.rs:1757-1778`). Extract and move before `finish_shutdown_cleanup`.

### 9. `ActorEvent` surface

**Add:**
- `ActorEvent::RunGracefulCleanup { reason: StopReason }` — no reply channel. Adapter spawns `onSleep` or `onDestroy` as a task and signals core on completion (see "counter ownership" below).
- `ActorEvent::DisconnectConn { conn_id: ConnId }` — no reply channel. Adapter spawns `onDisconnect(ctx, conn)` as a task, closes the conn transport, and signals core on completion.

**Keep:**
- `ActorEvent::SerializeState { reply }` — still used by `save_final_state`.

**Delete:**
- `ActorEvent::BeginSleep`.
- `ActorEvent::FinalizeSleep { reply }`.
- `ActorEvent::Destroy { reply }`.
- `ActorEvent::ConnectionClosed { conn }` — audit whether `DisconnectConn` subsumes it. If `ConnectionClosed` is only fired from connection-layer teardown (not shutdown), keep it and do not emit `DisconnectConn` from the conn teardown path. If it's also shutdown-related, consolidate.

**Counter ownership for `RunGracefulCleanup` and `DisconnectConn`.** There's a fire-and-forget race: if core emits the event, then `on_activity_signal` runs before the adapter processes the event, `can_finalize_sleep` reads zero, and grace exits while the hook was never dispatched. The fix is for **core** to own the accounting, using a dedicated `core_dispatched_hooks: AsyncCounter` that's independent of the adapter's `sleep_keep_awake_count`:

- `can_finalize_sleep` ands in `core_dispatched_hooks.load() == 0`.
- `begin_stop` (§4 step 7) increments `core_dispatched_hooks` once per event **before** emitting.
- Adapter handler runs the hook, then calls `ctx.mark_hook_completed(event_id)` (new NAPI callback into core) which decrements `core_dispatched_hooks`.
- If the adapter drops an event entirely (bug), the counter never decrements and grace exits via the deadline path with `shutdown_cancel_token` forcing cleanup — acceptable fallback for a protocol violation.

This is deliberately separate from `sleep_keep_awake_count`: the adapter continues its existing RAII pattern for waitUntil / async WS handlers and other "arbitrary tracked work", and core owns accounting for hooks it dispatched. Two counters, both participate in `can_finalize_sleep`. The boundary is clean — each side owns one counter — and no protocol coordination is required beyond the completion callback.

### 10. Config cleanup

**Delete from `rivetkit-core/src/actor/config.rs`:**
- `run_stop_timeout: Option<Duration>` / `Duration` fields (`:57, :75`).
- `run_stop_timeout_ms: Option<u32>` (`:109`).
- `run_stop_timeout_ms` wiring (`:161-162`).
- `effective_run_stop_timeout()` (`:218-225`).
- `DEFAULT_RUN_STOP_TIMEOUT` const.
- `run_stop_timeout` default in the `Default` impl (`:281`).
- `on_sleep_timeout` field and Duration/ms variants (`:54, :71, :106`).
- `on_sleep_timeout_ms` wiring (`:152-153`).
- `effective_on_sleep_timeout()` (`:200-206`).
- `DEFAULT_ON_SLEEP_TIMEOUT` const.
- `on_sleep_timeout` default in `Default` impl (`:277`).

**Rework** `effective_sleep_grace_period` (`:241-254`). The current fallback `on_sleep_timeout + wait_until_timeout` is no longer meaningful. New default is a single `DEFAULT_SLEEP_GRACE_PERIOD` const (15s, matching current docs). No fallback; the value is either explicitly set or the default.

**Delete from NAPI bridge `rivetkit-typescript/packages/rivetkit-napi/`:**
- `actor_factory.rs`: `on_sleep_timeout_ms` (`:75`), `run_stop_timeout_ms` (`:78`), `on_sleep_timeout: Duration` (`:209`), defaults (`:354`), conversion plumbing (`:1061, :1064`).
- `napi_actor_events.rs`: `on_sleep_timeout: timeout` (`:1369`) and surrounding wiring at `:566-575` (the `BeginSleep` handler itself is also deleted, see §9).
- Generated `index.d.ts`: `onSleepTimeoutMs?` (`:61`), `runStopTimeoutMs?` (`:64`) — regenerate after source edits.

**Delete from TS `rivetkit-typescript/packages/rivetkit/`:**
- `src/actor/config.ts`: `onSleepTimeout` field (`:842`), `runStopTimeout` field (`:852`), Zod deprecated descriptions (`:1747, :1773`).
- `src/registry/native.ts`: `onSleepTimeoutMs` and `runStopTimeoutMs` forwarding (`:3065, :3068`).

**Switch `actor_event_tx` to unbounded.** `begin_stop` emits `RunGracefulCleanup` + N `DisconnectConn` atomically and cannot tolerate backpressure that would block the main loop (§4 step 7). Moving `actor_event_tx` from `mpsc::channel(capacity)` to `mpsc::unbounded_channel()` removes the backpressure path for these events. Dispatch is unaffected — `dispatch_inbox` stays bounded so engine-side backpressure for dispatch remains. See §11.4 for the required pre-change audit before committing to unbounded.

### 11. Required audits

#### 11.1 Counter-dependent sites call `reset_sleep_timer`

Every site that mutates an input of `can_arm_sleep_timer` or `can_finalize_sleep` must call `ctx.reset_sleep_timer()`. Audit list:

- All four drain counters' increment/decrement sites. Ensure each decrement-to-zero calls `reset_sleep_timer`. Today `AsyncCounter::register_change_notify(&activity_notify)` (`sleep.rs:615`) covers counter changes via `notify_waiters`; that wiring is replaced per §1 with a callback that invokes `reset_sleep_timer`.
- `set_ready`, `set_started` — add `reset_sleep_timer` calls (they currently don't). `transition_to` in `task.rs:2147-2167` will invoke them.
- `notify_prevent_sleep_changed` (`sleep.rs:569`) — add `reset_sleep_timer`.
- `conn` add/remove — already call `reset_sleep_timer` (`context.rs:748, :755`).
- `handle_run_handle_outcome` — add `reset_sleep_timer` after `self.run_handle = None` (`task.rs:1322`).
- `ActorContext::on_state_change` callback completion — new; see 11.2.

#### 11.2 `onStateChange` counter-tracking

`onStateChange` callbacks are currently drained via `wait_for_on_state_change_idle` in `run_shutdown` (`state.rs:424`, called from `task.rs:1541`). After the move, `onStateChange` must increment `sleep_keep_awake_count` on spawn and decrement on completion so it drains during grace. If the callbacks don't currently counter-track, add the tracking.

#### 11.3 Replace `ActorEvent::SerializeState` with a sync callback (deferred)

`save_final_state` uses an event-channel round-trip because core has no direct handle to the JS `serializeState` callback. Longer-term, register a `Box<dyn Fn() -> BoxFuture<...> + Send + Sync>` on `ActorContextShared` at adapter init so core can invoke it without going through the event loop. Defer; not blocking this change.

#### 11.4 Audit `actor_event_tx` senders before switching to unbounded

Before committing §10's unbounded-channel change, enumerate everything that currently sends on `actor_event_tx`. If it's only lifecycle-phase events (grace cleanup, disconnect, serialize, inspector, workflow hooks) with a bounded event count per actor lifecycle, unbounded is safe. If there's a hot-path sender (per action, per state change, per conn message) that can sustain high volume, unbounded opens a memory hazard and we need a different approach — either a dedicated shutdown-only channel, or a pattern that buffers shutdown events in a small fixed-size ring on `ActorTask` and drains them synchronously from `begin_stop`. Reserve the unbounded switch until this audit is done.

### 12. Docs

`website/src/content/docs/actors/lifecycle.mdx`:

- Delete `runStopTimeout` row from timeouts tables (`:825, :920`).
- Delete `onSleepTimeout` row if present. Add changelog note for its removal (undocumented previously, but may be set by users via NAPI).
- Delete `runStopTimeout: 15_000` from example options block (`:891`).
- Rewrite shutdown sequence (`:836-846`):

  > When an actor sleeps or is destroyed, it enters the graceful shutdown window:
  >
  > 1. `c.abortSignal` fires and `c.aborted` becomes `true`. New connections and dispatch are rejected. Alarm timeouts are cancelled. On sleep, scheduled events are persisted and will be re-armed when the actor wakes.
  > 2. `onSleep` (or `onDestroy`) and `onDisconnect` for each closing connection run concurrently with the `run` handler's return. User `waitUntil` promises and async raw WebSocket handlers are drained. Hibernatable WebSocket connections are preserved for live migration on sleep; on destroy they are closed.
  > 3. Once `run` has returned and all the above work has completed, state is saved and the database is cleaned up.
  >
  > The entire window is bounded by `sleepGracePeriod` on sleep or `onDestroyTimeout` on destroy (defaults: 15 seconds each). If it is exceeded, the actor force-aborts any remaining work and proceeds to state save anyway.

- Update options table default for `sleepGracePeriod`: "Default 15000ms. Total graceful shutdown window for hooks, waitUntil, async raw WebSocket handlers, disconnects, and waiting for `preventSleep` to clear."

## Invariants (post-change)

1. **Single budget.** Wall-clock from grace entry to state save start ≤ `sleep_grace_period` (Sleep) or `on_destroy_timeout` (Destroy).
2. **`run` before save.** Drain path asserts `run_handle.is_none()`. Deadline path aborts `run_handle` before save starts.
3. **No arbitrary user code in `run_shutdown`.** Only core work and a bounded `serializeState` coordination call (30s internal cap, not user-configurable).
4. **Single signal primitive.** `activity_notify` + `activity_dirty: AtomicBool`. All wakes are `notify_one`.
5. **Two readiness functions.** `can_arm_sleep_timer` (Started). `can_finalize_sleep` (SleepGrace | DestroyGrace).
6. **Two grace states, asymmetric but parallel.** `SleepGrace` and `DestroyGrace` share structure (deadline arm + activity arm + idempotent Stop) but differ in (a) which conns get `DisconnectConn`, (b) whether driver alarm is cancelled, (c) which budget applies, (d) incremental saves allowed (Sleep only), (e) whether `mark_destroy_completed` runs in `run_shutdown`.
7. **No inter-grace transitions.** Once in `SleepGrace` or `DestroyGrace`, the only next state is the matching finalize state (`SleepFinalize` or `Destroying`). Sleep→Destroy upgrades are unreachable under the engine actor2 invariant and are handled with `debug_assert!(false)`.
8. **Unified entry.** All four triggers (engine Sleep, engine Destroy, `c.sleep()`, `c.destroy()`) route through `begin_stop` on `lifecycle_inbox`. No bypass.
9. **No stored polled futures for shutdown orchestration.** `SleepGraceState = { deadline: Instant, reason: StopReason }`.
10. **Single abort primitive.** `shutdown_cancel_token` is the one abort concept for the actor. `c.abortSignal` on the JS surface is a wrapper over the same token. Core cancels it at grace entry; adapter tracked tasks and user `run` observe the same cancellation.
11. **Counter ownership split.** `core_dispatched_hooks` (core-owned, for `RunGracefulCleanup` + `DisconnectConn` hook dispatch) and `sleep_keep_awake_count` (adapter-owned, for all other tracked work) both participate in `can_finalize_sleep`. Each side owns one counter; no protocol coordination beyond hook-completion callbacks.

## Implementation plan

Each step is independently shippable and revertable. Tests must pass before the next step starts.

**Step 1 — Unify signal primitive.** Rewrite `reset_sleep_timer` / `notify_activity_dirty` as notify-only (§1). Delete `LifecycleEvent::ActivityDirty` variant, handler, `drain_activity_dirty`, parallel arm. Add `reset_sleep_timer` calls at `set_ready`/`set_started`/`notify_prevent_sleep_changed`/`handle_run_handle_outcome` (§11.1). Replace `AsyncCounter::register_change_notify` consumer with a callback that calls `reset_sleep_timer`. Existing tests for sleep timer + activity dedup must still pass.

**Step 2 — Split readiness.** Introduce `can_arm_sleep_timer` (rename of `can_sleep_state`) and new `can_finalize_sleep`. Update existing `Started`-state callers. No grace callers yet. Tests unchanged.

**Step 3 — SleepGraceState collapse + new select arms + abort token.** Shrink `SleepGraceState` to `{ deadline, reason }`. Delete `wait_for_sleep_idle_window`, `SleepGraceWait`, `poll_sleep_grace`, `Box::pin` of idle future. Add `sleep_until(grace.deadline)` arm. Add `on_activity_signal` branch for `SleepGrace | DestroyGrace`. Introduce `shutdown_cancel_token: CancellationToken` on `ActorTask` + `ActorContextShared`; rewire `c.abortSignal` as a wrapper over the token (unify abort primitive). Tests for grace exit on idle and deadline must pass.

**Step 4 — New events + counter + adapter handlers.** Add `ActorEvent::RunGracefulCleanup` and `ActorEvent::DisconnectConn`. Add `core_dispatched_hooks: AsyncCounter` on `ActorContextShared`. Implement adapter handlers that run user hooks observing `shutdown_cancel_token`, then call `ctx.mark_hook_completed(event_id)` on completion to decrement the counter. Audit `actor_event_tx` senders (§11.4); if safe, switch to unbounded. Run in parallel with existing `BeginSleep`/`FinalizeSleep`/`Destroy` events; both paths coexist. Tests for hook invocation via new events + counter decrement on completion must pass.

**Step 5 — New grace entry.** Add `DestroyGrace` lifecycle variant and enumerate match-arm decisions (§6 table). Rewrite `begin_stop` per §4: alarm cleanup, readiness flip, core-side counter increment before event emission, plain `send()` on the (now unbounded) actor_event channel. Route `c.sleep()` / `c.destroy()` through `lifecycle_inbox` instead of `handle_run_handle_outcome` shortcut. Remove the upgrade-Sleep→Destroy branch — replaced with `debug_assert!(false)` for the now-unreachable case. Tests for engine-initiated + self-initiated hooks firing during grace must pass.

**Step 6 — Strip `run_shutdown`.** Replace `run_shutdown` body with §7's pure-core version. Implement `save_final_state` (§8). Delete old event paths (`FinalizeSleep`, `Destroy`, `BeginSleep`) from both core and adapter. Tests for state save correctness + hibernation flush must pass.

**Step 7 — Config cleanup.** Delete `run_stop_timeout`, `on_sleep_timeout` across Rust core, NAPI bridge, generated DTS, TS config, Zod descriptions (§10). Rework `effective_sleep_grace_period` default. Tests referencing these knobs must be updated or deleted.

**Step 8 — Docs.** Update `lifecycle.mdx` per §12.

## Test plan

**New tests:**

- `shutdown_grace_exits_on_drain_fast_path`: actor enters grace, `run` returns in 10ms, `onSleep` returns in 20ms, conns disconnect in 30ms → grace exits via drain path in <100ms, state saved, no warnings logged.
- `shutdown_grace_exits_on_deadline`: `onSleep` hangs indefinitely → deadline fires at `sleepGracePeriod`, `shutdown_cancel_token` cancels, `run_handle` aborted, state still saved.
- `shutdown_single_budget`: measure wall-clock from grace entry to save. Assert ≤ `sleepGracePeriod + small_tolerance`. Verifies 2× budget bug is fixed.
- `shutdown_run_exits_before_save`: assert `run_handle.is_none()` at the moment `save_final_state` is entered. Verified via a probe that records lifecycle transitions.
- `self_destroy_fires_onDestroy`: `c.destroy()` from an action handler → `onDestroy` runs during grace. Verifies the shortcut-removal doesn't skip hooks.
- `self_sleep_fires_onSleep`: same for `c.sleep()`.
- `duplicate_stop_during_grace_is_idempotent`: engine sends Stop { Sleep }, grace starts, engine sends Stop { Sleep } again → second call replies `Ok` without re-entering grace, no extra events emitted.
- `conflicting_stop_during_grace_debug_asserts`: engine sends Stop { Sleep }, grace starts, engine sends Stop { Destroy } → `debug_assert!(false)` in debug; release logs and idempotent-acks. This transition is unreachable under the engine invariant; test verifies the assertion fires.
- `core_counter_increments_before_emit`: assert `core_dispatched_hooks.load() == N+1` (RunGracefulCleanup + N DisconnectConn) immediately after `begin_stop` returns, before any adapter processing.
- `core_counter_decrements_on_hook_completion`: verify that the completion callback decrements `core_dispatched_hooks` exactly once per event, and that grace exits via drain path only when counter reaches zero.
- `hibernatable_conn_preserved_on_sleep`: hibernatable conn's state is flushed via `pending_hibernation_updates`, `onDisconnect` NOT called.
- `hibernatable_conn_fires_ondisconnect_on_destroy`: same conn on destroy fires `onDisconnect`.
- `preventSleep_during_grace_delays_finalize`: `setPreventSleep(true)` in `onSleep` → grace waits until `setPreventSleep(false)` or deadline.
- `alarm_does_not_fire_during_grace`: scheduled alarm due during grace does not invoke user alarm handler.
- `dispatch_drained_on_grace_entry`: dispatch in inbox at Stop arrival completes as tracked work, not dropped.
- `activity_signal_dedup`: 1000 rapid `reset_sleep_timer` calls produce ≤ a few main-loop re-evaluations.
- `missing_serialize_state_does_not_hang`: mock `serializeState` callback to hang → 30s sanity cap fires, save proceeds with empty deltas, error logged.
- `run_exit_after_drain_wakes_main_loop`: last tracked task ends, then `run` returns — assert grace exits via drain path, not deadline. Verifies the `handle_run_handle_outcome` reset_sleep_timer fix.

**Updated tests (remove or rewrite):**

- `tests/modules/task.rs:2158` `shutdown_run_handle_join_uses_run_stop_timeout` — delete; timeout no longer exists.
- `tests/modules/config.rs:13/22/48/52/121/137` — remove `on_sleep_timeout` and `run_stop_timeout` assertions.
- `tests/modules/context.rs:799/803/810/837/899` — rewrite `wait_for_sleep_idle_window` callers against the new counter-based drain.
- `tests/modules/task.rs` ~40 match arms on `ActorEvent::BeginSleep|FinalizeSleep|Destroy` — update to `RunGracefulCleanup|DisconnectConn|SerializeState`.
- `tests/modules/task.rs:38/3083/3122` `LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD` — remove; the threshold is no longer a core concept.

## Risks & known limitations

**R1 — Cooperative abort only.** On the deadline path, `run_handle.abort()` + `shutdown_cancel_token.cancel()` rely on tokio's cooperative cancellation. A user future that never `.await`s (e.g., a tight sync loop calling sync N-API) is not abortable. Document as a limitation.

**R2 — `serializeState` hang loses state.** The 30s sanity cap proceeds with empty deltas on timeout. User's in-memory state since last incremental save is lost. Document.

**R3 — Hibernation metadata flush under deadline abort.** If the deadline fires mid-`pending_hibernation_updates` flush, some hibernation metadata may not persist. Either (a) accept and document, or (b) make the flush atomic at the KV layer. Default: accept.

**R4 — `onStateChange` callback drain requires counter tracking.** §11.2 is a dependent audit. If counter tracking is not added, `onStateChange` callbacks can race with `save_final_state`. Must land in the same change as step 6.

**R5 — Dropped event exits grace via deadline.** If the adapter drops a `RunGracefulCleanup` or `DisconnectConn` event (bug or crash), `core_dispatched_hooks` stays non-zero forever and grace exits via the deadline path. State is still saved and the actor terminates; `onSleep`/`onDestroy`/`onDisconnect` simply never runs. This is a protocol-violation fallback, not an expected path; log an error if observed.

**R6 — Unbounded channel memory cap.** If §11.4's audit forces us to stay bounded, §10's unbounded change doesn't land and `begin_stop` needs a different non-blocking emit strategy. Possible alternatives noted in §11.4.
