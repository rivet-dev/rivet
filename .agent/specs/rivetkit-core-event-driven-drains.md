# Event-Driven Drain Migration — rivetkit-core SleepController + ctx bridge

Status: **LANDED** on `04-19-chore_move_rivetkit_to_task_model` as of 2026-04-21.

Cross-reference: sleep lifecycle semantics were split after this drain migration into `SleepGrace` (heads-up, dispatch still open) and `SleepFinalize` (teardown gate). The event-driven drain pieces in this spec still apply, but `onSleep` now fires at `SleepGrace` entry via `BeginSleep`, while the old shutdown-side disconnect/save work moved behind `FinalizeSleep`.

## Problem

`rivetkit-core/src/actor/sleep.rs` implements four shutdown drains as 10ms-tick polling loops:

```rust
loop {
    if ready { return true; }
    if Instant::now() >= deadline { return false; }
    sleep((deadline - now).min(Duration::from_millis(10))).await;
}
```

Every sleep/destroy shutdown eats avg ~5ms spurious latency per drain × 2-4 drains per shutdown (20-40ms total), plus unnecessary scheduler wakeups re-checking counters that haven't changed.

Separately, `ctx.sleep()` (`context.rs:367-372`) adds a 1ms wall-clock defer of unclear purpose to every user sleep request.

## Design

Introduce **one primitive** + **one struct** that owns all in-flight work tracking.

### `AsyncCounter` — the only new primitive

```rust
// rivetkit-core/src/actor/async_counter.rs
pub struct AsyncCounter {
    value: AtomicUsize,
    zero_notify: Notify,
}

impl AsyncCounter {
    pub fn new() -> Self { Self { value: AtomicUsize::new(0), zero_notify: Notify::new() } }

    pub fn increment(&self) { self.value.fetch_add(1, Ordering::Relaxed); }

    pub fn decrement(&self) {
        let prev = self.value.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(prev > 0, "AsyncCounter decrement below zero");
        if prev == 1 { self.zero_notify.notify_waiters(); }
    }

    pub fn load(&self) -> usize { self.value.load(Ordering::Acquire) }

    /// Race-safe wait: arm the notify permit before re-checking so a decrement
    /// that lands between check and wait still wakes us.
    pub async fn wait_zero(&self, deadline: Instant) -> bool {
        loop {
            if self.value.load(Ordering::Acquire) == 0 { return true; }
            let notified = self.zero_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.value.load(Ordering::Acquire) == 0 { return true; }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return false;
            }
        }
    }
}
```

### RAII guards over `AsyncCounter`

All region-based counts (`keep_awake`, `internal_keep_awake`, `websocket_callback`) migrate to guard-only APIs:

```rust
pub struct RegionGuard {
    counter: Arc<AsyncCounter>,
}
impl Drop for RegionGuard {
    fn drop(&mut self) { self.counter.decrement(); }
}

impl SleepController {
    pub fn keep_awake(&self) -> RegionGuard {
        self.0.keep_awake.increment();
        RegionGuard { counter: self.0.keep_awake.clone() }
    }
    pub fn internal_keep_awake(&self) -> RegionGuard { ... }
    pub fn websocket_callback(&self) -> RegionGuard { ... }
}
```

The existing `begin_*` / `end_*` pair is **removed entirely** from the public API. Callers must hold a guard across the region; drop = release. No way to mismatch.

### Shutdown tasks: counter for waiting + JoinSet for aborting

`shutdown_tasks` has two orthogonal needs: wait for drain (counter) and abort stuck tasks on teardown (JoinSet). Use both, each for what it's good at.

```rust
struct SleepControllerInner {
    // ... existing fields ...
    work: WorkRegistry,
}

struct WorkRegistry {
    keep_awake: Arc<AsyncCounter>,
    internal_keep_awake: Arc<AsyncCounter>,
    websocket_callback: Arc<AsyncCounter>,
    shutdown_counter: Arc<AsyncCounter>,
    shutdown_tasks: Mutex<JoinSet<()>>,
    idle_notify: Notify,            // composed: fires when any of keep_awake / internal_keep_awake / http reaches zero
    prevent_sleep_notify: Notify,   // pinged on every ctx.set_prevent_sleep flip
}

impl SleepController {
    pub fn track_shutdown_task<F>(&self, fut: F)
    where F: Future<Output = ()> + Send + 'static
    {
        let counter = self.0.work.shutdown_counter.clone();
        counter.increment();
        self.0.work.shutdown_tasks.lock().spawn(async move {
            let _guard = CountGuard { counter };  // CountGuard == RegionGuard reused
            fut.await;
        });
    }
}
```

Key properties:
- `CountGuard::drop` runs on normal completion, panic (unwind), AND JoinSet abort — so the counter stays in sync regardless of termination path.
- Drain awaits `shutdown_counter.wait_zero(deadline).await` — uniform primitive with the other three drains.
- `JoinSet` is a **cancellation-handle bag only**. Never `join_next()` from the drain path. It exists so that when `SleepController` is dropped, tokio aborts every outstanding task.

### HTTP request counter lives in envoy-client

`active_http_request_count: Arc<AtomicUsize>` at `engine/sdks/rust/envoy-client/src/actor.rs:90` already is drop-guarded via `HttpRequestGuard`. Upgrade it:

```rust
// envoy-client/src/actor.rs
pub struct HttpRequestGuard { counter: Arc<AsyncCounter> }

impl HttpRequestGuard {
    fn new(counter: Arc<AsyncCounter>) -> Self {
        counter.increment();
        Self { counter }
    }
}
impl Drop for HttpRequestGuard {
    fn drop(&mut self) { self.counter.decrement(); }
}
```

Expose on `EnvoyHandle`:

```rust
// envoy-client/src/handle.rs
impl EnvoyHandle {
    pub fn http_request_counter(&self, actor_id: &str, generation: Option<u32>)
        -> Option<Arc<AsyncCounter>> { ... }
}
```

rivetkit-core drain calls `counter.wait_zero(deadline).await` directly — no more RPC polling.

`AsyncCounter` needs to live in a shared crate so both envoy-client and rivetkit-core can use the same type. Options:

1. Put it in `rivet-util` (workspace util crate) and depend from both.
2. Put it in envoy-client (since that's where the cross-crate use originates) and rivetkit-core depends on envoy-client anyway.
3. Put it in rivetkit-core and make envoy-client depend on rivetkit-core (adds a dep edge that doesn't exist today).

**Recommendation**: option 1 — new module in `rivet-util` (or whichever workspace util crate already has shared async primitives). Zero new dep edges.

### Composed `wait_for_sleep_idle_window`

The aggregate drain requires `keep_awake == 0 && internal_keep_awake == 0 && active_http_requests == 0`. Use the shared `idle_notify`:

- Every one of the three contributing AsyncCounters pings `idle_notify.notify_waiters()` when it reaches zero (attach a second `Notify` to the counter, or just call `idle_notify.notify_waiters()` from the counter's decrement hook via a callback registry).
- Waiter:
  ```rust
  async fn wait_for_sleep_idle_window(&self, ctx: &ActorContext, deadline: Instant) -> bool {
      loop {
          if self.sleep_shutdown_idle_ready(ctx).await { return true; }
          let notified = self.0.work.idle_notify.notified();
          tokio::pin!(notified);
          notified.as_mut().enable();
          if self.sleep_shutdown_idle_ready(ctx).await { return true; }
          if tokio::time::timeout_at(deadline, notified).await.is_err() {
              return false;
          }
      }
  }
  ```

Simpler alternative: expose an `AsyncCounter::subscribe(Arc<Notify>)` that pipes zero-transitions to an external Notify. Both approaches work; pick whichever reads cleanest.

### `prevent_sleep` bool

Rarely flipped. Two options:

1. `watch::channel<bool>` on `ActorContext`, subscribers re-check on every send.
2. Dedicated `prevent_sleep_notify: Notify` pinged on every flip.

Either is fine. Recommend (2) for symmetry with the other notify sites.

### `ctx.sleep()` cleanup

Remove the `tokio::time::sleep(Duration::from_millis(1))` at `context.rs:368`. The `runtime.spawn(async move { ... })` already decouples from the calling task; the 1ms wall-clock delay is unjustified.

If the intent was a scheduler yield, use `tokio::task::yield_now().await`. If the intent was nothing, remove the sleep entirely.

Audit `ctx.destroy()` at `context.rs:382-389` for consistency — it has no sleep today and should stay that way.

## Call sites to migrate

### rivetkit-core

| File:line | Function | Action |
|-----------|----------|--------|
| `actor/sleep.rs:240-258` | `wait_for_sleep_idle_window` | Replace poll loop with `idle_notify`-driven wait (composed over keep_awake, internal_keep_awake, http counters) |
| `actor/sleep.rs:260-281` | `wait_for_shutdown_tasks` | Replace with `shutdown_counter.wait_zero(deadline).await` + `websocket_callback.wait_zero` + `prevent_sleep` notify |
| `actor/sleep.rs:283-303` | `wait_for_internal_keep_awake_idle` | Replace with `internal_keep_awake.wait_zero(deadline)` |
| `actor/sleep.rs:305-326` | `wait_for_http_requests_drained` | Replace with `envoy_handle.http_request_counter(...).wait_zero(deadline)` |
| `actor/sleep.rs:24-26` | `keep_awake_count`, `internal_keep_awake_count`, `websocket_callback_count` AtomicUsize fields | Replace with `Arc<AsyncCounter>` fields on `WorkRegistry` |
| `actor/sleep.rs:28` | `shutdown_tasks: Mutex<Vec<JoinHandle<()>>>` | Replace with `Mutex<JoinSet<()>>` + `shutdown_counter: Arc<AsyncCounter>` |
| `actor/sleep.rs:329-360` | `begin_keep_awake`, `end_keep_awake` (and internal, websocket variants) | Delete public begin/end pairs. Replace with `keep_awake() -> RegionGuard` etc. |
| `actor/sleep.rs:362-380` | `track_shutdown_task` | Rewrite: `joinset.spawn(async move { let _g = CountGuard{...}; fut.await })` |
| `actor/task.rs:851-865` | `wait_for_sleep_idle_window` wrapper | Delegates to the new SleepController method; remove internal 10ms tick |
| `actor/task.rs:867-898` | `drain_tracked_work` | Replace 10ms poll with `tokio::select!{ counter.wait_zero(deadline), sleep(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD) }` where the timeout arm emits the warning once, then the outer `timeout_at(deadline, counter.wait_zero)` continues |
| `actor/context.rs:367-372` | `ctx.sleep()` 1ms defer | Remove the `sleep(1ms)`. Keep the `runtime.spawn(async move { request_sleep })` |

### envoy-client

| File:line | Action |
|-----------|--------|
| `engine/sdks/rust/envoy-client/src/actor.rs:90,112-123` | Change `active_http_request_count: Arc<AtomicUsize>` to `Arc<AsyncCounter>`. `HttpRequestGuard::new` calls `counter.increment()`; Drop calls `counter.decrement()` |
| `engine/sdks/rust/envoy-client/src/envoy.rs:49, 112, 287-288` | Propagate the type change through `EnvoyContext`, `ActorInfo`, snapshot responses |
| `engine/sdks/rust/envoy-client/src/handle.rs:102-109` | Add `pub fn http_request_counter(&self, actor_id, generation) -> Option<Arc<AsyncCounter>>`. Keep `get_active_http_request_count` for any existing external callers, implemented as `counter.load()` |

### New module location

| Artifact | Location |
|----------|----------|
| `AsyncCounter` primitive | `rivet-util` (or existing workspace util crate) — both envoy-client and rivetkit-core depend on it |
| `RegionGuard` / `CountGuard` | Inline in `rivetkit-core/src/actor/sleep.rs` (consumer of AsyncCounter) |
| `WorkRegistry` struct | New file `rivetkit-core/src/actor/work_registry.rs`, owned by `SleepControllerInner` |

## Acceptance criteria

Status: **All acceptance criteria below are implemented on this branch.** The final regression lock-in lives in `rivetkit-rust/packages/rivetkit-core/tests/modules/task.rs` plus `rivetkit-rust/packages/rivetkit-core/scripts/check-event-driven-drains.sh`.

1. `AsyncCounter { increment, decrement, load, wait_zero(deadline) -> bool }` lives in a shared util crate. Unit tests: single waiter fires on decrement-to-zero, waiter races decrement (permit armed before check), multiple concurrent waiters all wake, non-zero decrement does not fire the notify, deadline timeout returns `false`, below-zero decrement triggers `debug_assert`.
2. `RegionGuard` + `CountGuard` (same shape) defined in `work_registry.rs`. Unit tests: normal drop decrements, panic-unwind still runs Drop (use `catch_unwind`), forget() intentionally leaks the counter (document this).
3. `SleepControllerInner.keep_awake_count`, `internal_keep_awake_count`, `websocket_callback_count` AtomicUsize fields replaced with `Arc<AsyncCounter>` fields on `WorkRegistry`. Public `begin_*` / `end_*` methods replaced with `keep_awake() -> RegionGuard`, `internal_keep_awake() -> RegionGuard`, `websocket_callback() -> RegionGuard`. All existing call sites updated to hold guards instead of calling begin/end pairs.
4. `SleepControllerInner.shutdown_tasks: Mutex<Vec<JoinHandle<()>>>` replaced with `Mutex<JoinSet<()>>` + `shutdown_counter: Arc<AsyncCounter>`. `track_shutdown_task` spawns into the JoinSet wrapped in a `CountGuard`. The JoinSet is **never** drained via `join_next` from the shutdown path — it exists solely so that Drop on `SleepController` aborts outstanding tasks.
5. `envoy-client::HttpRequestGuard.active_http_request_count` upgraded from `Arc<AtomicUsize>` to `Arc<AsyncCounter>`. `HttpRequestGuard::new` / `Drop` route through `increment` / `decrement`. `EnvoyHandle::http_request_counter(actor_id, generation) -> Option<Arc<AsyncCounter>>` exposed. Existing `get_active_http_request_count` kept as a `.load()` convenience.
6. All four drain functions (`wait_for_sleep_idle_window`, `wait_for_shutdown_tasks`, `wait_for_internal_keep_awake_idle`, `wait_for_http_requests_drained`) use `AsyncCounter::wait_zero(deadline)` or the composed `idle_notify` pattern. Zero `sleep(Duration::from_millis(10))` calls remain in `sleep.rs`.
7. `task.rs drain_tracked_work` uses `counter.wait_zero` + a side-channel `sleep(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD)` in a `tokio::select!` to emit the warning once. No 10ms poll remains.
8. `ctx.sleep()` (`context.rs:367-372`) no longer calls `tokio::time::sleep(1ms)`. Direct spawn only.
9. Regression test: a sleep shutdown with no in-flight work completes in `< 5ms` wall-clock (use `tokio::time::pause()` + explicit advances to assert the drain does not take a polling tick).
10. Regression test: a sleep shutdown with one in-flight HTTP request blocks until the `HttpRequestGuard` drops, then completes within one scheduler tick. Use `tokio::test` with `start_paused = true`.
11. Regression test: shutdown tasks registered via `track_shutdown_task` drain in FIFO-ish order as they complete; the drain returns `true` exactly when the counter reaches zero. A `track_shutdown_task` that panics does not wedge the drain (guard decrements during unwind).
12. Regression test: `SleepController` drop aborts any still-running shutdown task (prove via a task that awaits a never-firing oneshot; assert it is cancelled within one tick of drop).
13. `grep -RE 'sleep\(Duration::from_millis\(10\)\)' rivetkit-rust/packages/rivetkit-core/src/actor/` returns zero results. Grep for `Mutex<Vec<JoinHandle` in the same path returns zero results.
14. `cargo check -p rivetkit-core -p rivet-envoy-client` passes. `cargo test -p rivetkit-core` passes. Existing TS driver-test-suite baseline from `.agent/notes/driver-test-progress.md` stays green.

## Out of scope

- `state.rs:645` debounced `pending_save` worker — deliberate `sleep(delay)` for save debouncing. Keep.
- `schedule.rs:363` local alarm timer — alarm firing is inherently time-based. Keep.
- `queue.rs:857/897` `enqueue_and_wait` `tokio::select! { msg, sleep(timeout) }` — correct timeout pattern. Keep.
- `ActorTask::run` `sleep_until(deadline)` arms for `state_save_tick`, `inspector_serialize_state_tick`, `sleep_tick` — event-driven deadlines. Keep.
- `envoy-client` periodic `tokio::time::interval` for ACK batch flush and KV cleanup — naturally time-based. Keep.
- NAPI `with_timeout` wrappers on TSF dispatches — per-call deadlines on JS futures. Keep.
- TS `actor-handle.ts #waitForDestroyActionToSettle` — crosses a network boundary, needs gateway push support to event-drive. Separate follow-up.
- TS `actor-conn.ts:218` missing `setInterval` delay arg bug — unrelated to core runtime. Separate follow-up.
- `ctx.conns().is_empty()` participation in aggregate idle check — ConnManager owns its own lifecycle events; subscribe to its existing channel in a follow-up if needed.

## Rollout

1. Land `AsyncCounter` in workspace util crate with unit tests.
2. Land envoy-client `HttpRequestGuard` upgrade + `EnvoyHandle::http_request_counter` accessor (behind a deprecated-but-kept `get_active_http_request_count` load accessor).
3. Land `WorkRegistry` + RegionGuard in rivetkit-core; migrate `begin_*`/`end_*` call sites.
4. Replace the four drain functions one at a time; keep the 10ms poll fallback behind a `#[cfg(test)]` feature for A/B comparison during the transition if needed.
5. Remove `ctx.sleep()` 1ms defer.
6. Delete the `sleep(Duration::from_millis(10))` sites.
7. Gate CI grep check to prevent regressions.
