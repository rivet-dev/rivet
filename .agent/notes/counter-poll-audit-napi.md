# rivetkit-napi counter-poll audit

Date: 2026-04-22
Story: US-029

## Scope

Searched `rivetkit-typescript/packages/rivetkit-napi/src/` for:

- `loop { ... sleep(Duration::from_millis(_)) ... }`
- `while ... { ... sleep(Duration::from_millis(_)) ... }`
- `tokio::time::sleep`, `std::thread::yield_now`, and retry loops
- `AtomicUsize`, `AtomicU32`, `AtomicU64`, and `AtomicBool` fields with waiters
- `Mutex<usize>`, `Mutex<bool>`, and similar scalar locks
- polling-shaped exports such as `poll_*`

## Converted polling sites

- `cancel_token.rs::lock_registry_for_test`
  - Classification before: test-only spin polling.
  - Problem: tests serialized access to the global cancel-token registry by spinning on an `AtomicBool` with `std::thread::yield_now()`.
  - Fix: replaced the spin gate with a test-only `parking_lot::Mutex<()>`, returning a real guard from `lock_registry_for_test()`.
  - Coverage: existing cancel-token cleanup tests still exercise the same serialized registry path.

## Event-driven sites

- `bridge_actor.rs` response waits
  - Classification: event-driven.
  - Uses per-request `oneshot` channels in `ResponseMap`; no counter or sleep-loop polling.

- `napi_actor_events.rs::drain_tasks`
  - Classification: event-driven.
  - Pumps already-registered tasks, then awaits `JoinSet::join_next()` until the set is empty; no timed polling interval.

- `napi_actor_events.rs` callback tests with `Notify`
  - Classification: event-driven test gates.
  - Uses `tokio::sync::Notify` and `oneshot` channels for deterministic ordering.

- Queue wait bindings in `queue.rs`
  - Classification: event-driven through core.
  - Delegates to `rivetkit-core` queue waits and optional cancellation tokens; no local counter polling.

## Monotonic sequence / diagnostic atomics

- `cancel_token.rs::NEXT_CANCEL_TOKEN_ID`
  - Classification: monotonic ID generator.
  - No waiter.

- `cancel_token.rs::active_token_count`
  - Classification: test diagnostic snapshot.
  - Tests read it after guarded operations complete; no async waiter or sleep-loop polls it.

- `actor_context.rs::next_websocket_callback_region_id`
  - Classification: monotonic region ID generator.
  - No waiter.

- `actor_context.rs::ready` and `started`
  - Classification: lifecycle flags.
  - Read synchronously to validate lifecycle transitions; no sleep-loop waiter.

- `napi_actor_events.rs` test `AtomicU64` / `AtomicBool` values
  - Classification: test observation flags.
  - Tests combine these with `oneshot`, `Notify`, or task joins; no timed polling loop waits on them.

## Non-counter sleep / polling-shaped sites

- `napi_actor_events.rs::with_timeout` and tests
  - Classification: timeout assertion.
  - Uses `tokio::time::timeout` or a bounded `select!` branch to prove a future is still pending, not to poll shared state.

- `napi_actor_events.rs` test `sleep(Duration::from_secs(60))`
  - Classification: pending-work fixture.
  - The sleep is intentionally cancelled by an abort token; no shared counter is polled.

- `queue.rs` and `schedule.rs` `Duration::from_millis(...)`
  - Classification: user-supplied timeout/delay conversion.
  - Converts JS options to core durations; no polling loop.

- `cancel_token.rs::poll_cancel_token`
  - Classification: explicit JS cancellation polling surface, not a counter waiter.
  - This is a public NAPI sync read used by the TS abort-signal bridge. It reads a cancellation token's state once and does not loop or wait in Rust.
