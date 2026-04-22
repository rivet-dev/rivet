# rivetkit-core counter-poll audit

Date: 2026-04-22
Story: US-027

## Scope

Searched `rivetkit-rust/packages/rivetkit-core/src/` for:

- `loop { ... sleep(Duration::from_millis(_)).await; ... }`
- `loop { ... tokio::time::sleep(...).await; ... }`
- `while ... { ... sleep(Duration::from_millis(_)).await; ... }`
- `AtomicUsize`, `AtomicU32`, `AtomicU64`, and `AtomicBool` fields with async waiters

## Converted polling sites

- `registry.rs::Registry::handle_fetch`
  - Classification before: polling.
  - Problem: after an HTTP request, the rearm task checked `can_sleep() == ActiveHttpRequests` and slept in 10 ms slices until the envoy HTTP request counter reached zero.
  - Fix: added `SleepController::wait_for_http_requests_idle(...)` and `ActorContext::wait_for_http_requests_idle()`, both backed by the existing `AsyncCounter` zero-notify registration on `work.idle_notify`.
  - Coverage: added `http_request_idle_wait_uses_zero_notify` to prove the waiter wakes on decrement-to-zero without advancing a polling interval.

## Event-driven sites

- `actor/state.rs::wait_for_save_request`
  - Classification: event-driven.
  - Uses `save_completion: Notify`; waiters arm `notified()` before re-checking `save_completed_revision`.

- `actor/state.rs::wait_for_pending_writes` / `wait_for_in_flight_writes`
  - Classification: event-driven.
  - Tracked persist tasks are awaited directly; KV writes use `in_flight_writes` plus `write_completion: Notify` on decrement-to-zero.

- `actor/sleep.rs::wait_for_sleep_idle_window`
  - Classification: event-driven.
  - Arms `work.idle_notify` before checking HTTP request, keep-awake, internal keep-awake, and websocket callback counters.

- `actor/sleep.rs::wait_for_shutdown_tasks`
  - Classification: event-driven.
  - Uses `AsyncCounter::wait_zero(deadline)` for shutdown tasks and websocket callbacks, plus `prevent_sleep_notify` for the prevent-sleep flag.

- `actor/sleep.rs::wait_for_internal_keep_awake_idle`
  - Classification: event-driven.
  - Uses `AsyncCounter::wait_zero(deadline)`.

- `actor/sleep.rs::wait_for_http_requests_drained`
  - Classification: event-driven.
  - Uses the envoy `AsyncCounter::wait_zero(deadline)` after registering zero notifications on `work.idle_notify`.

- `actor/context.rs::wait_for_destroy_completion`
  - Classification: event-driven.
  - Uses `destroy_completion_notify` and re-checks `destroy_completed`.

- `actor/queue.rs::next_batch`, `wait_for_names`, and `wait_for_names_available`
  - Classification: event-driven.
  - Message waits use queue `Notify`; active wait counts are RAII-owned by `ActiveQueueWaitGuard`.

## Monotonic sequence / one-shot atomics

- `actor/state.rs` save revisions (`revision`, `save_request_revision`, `save_completed_revision`)
  - Classification: monotonic sequence with notify-backed awaiters.

- `actor/task.rs` inspector attach count
  - Classification: event-triggering counter.
  - The counter is owned by `InspectorAttachGuard`; transitions enqueue lifecycle events instead of polling.

- `actor/schedule.rs::local_alarm_epoch`
  - Classification: monotonic sequence guard.
  - Spawned local alarm tasks check the epoch once after the timer fires to ignore stale work.

- `actor/schedule.rs::driver_alarm_cancel_count`
  - Classification: diagnostic/test counter.
  - No production awaiter.

- `inspector/mod.rs` revision counters and active counts
  - Classification: snapshot/revision counters.
  - Subscribers are notified through listener callbacks; no async counter polling.

- `kv.rs` stats counters
  - Classification: diagnostic/test counters.
  - No async awaiter in production code.

## Non-counter sleep loops

- `registry.rs::wait_for_engine_health`
  - Classification: retry backoff.
  - Sleeps between external HTTP health-check attempts, not waiting on shared memory.

- `actor/state.rs::persist_state` and pending-save task
  - Classification: intentional debounce timers.
  - Sleeps until a configured save delay elapses, not polling a counter.

- `actor/schedule.rs::reschedule_local_alarm`
  - Classification: timer scheduling.
  - Sleeps until the next alarm timestamp, then checks the monotonic epoch to avoid stale dispatch.

- Protocol read/write loops in `registry.rs` and `sqlite.rs`
  - Classification: codec loops.
  - No async sleep or shared-state polling.
