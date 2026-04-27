# rivetkit-core internals

Internal wiring reference for `rivetkit-rust/packages/rivetkit-core/`. These are facts about the current implementation. For the principles that govern how new code is added, see the root `CLAUDE.md` layer + fail-by-default sections. For state-mutation semantics, see `docs-internal/engine/rivetkit-core-state-management.md`.

## Storage organization

Actor subsystems are composed into `ActorContextInner`, not separate managers.

- Queue storage lives on `ActorContextInner`. Behavior sits in `actor/queue.rs` `impl ActorContext` blocks. Do not reintroduce `Arc<QueueInner>` or a public `Queue` re-export.
- Connection storage lives on `ActorContextInner`. Behavior sits in `actor/connection.rs` `impl ActorContext` blocks. Do not reintroduce `Arc<ConnectionManagerInner>` or a public `ConnectionManager` re-export.
- Actor state storage lives on `ActorContextInner`. Behavior sits in `actor/state.rs` `impl ActorContext` blocks. Do not reintroduce `Arc<ActorStateInner>` or a public `ActorState` re-export.
- Schedule storage lives on `ActorContextInner`. Behavior sits in `actor/schedule.rs` `impl ActorContext` blocks. Do not reintroduce `Arc<ScheduleInner>` or a public `Schedule` re-export.
- Event fanout lives directly in `ActorContext::broadcast`. Do not reintroduce a separate `EventBroadcaster` subsystem.

## Persisted KV layout

Values are serialized with a vbare-compatible 2-byte little-endian embedded version prefix before the BARE body, matching the TypeScript `serializeWithEmbeddedVersion(...)` format.

| Key | Contents |
|---|---|
| `[1]` | `PersistedActor` snapshot (matches TypeScript `KEYS.PERSIST_DATA`) |
| `[2] + conn_id` | Hibernatable websocket connection payload, TypeScript v4 BARE field order |
| `[5, 1, 1]` | Queue metadata |
| `[5, 1, 2] + u64be(id)` | Queue messages (FIFO prefix scan) |
| `[6]` | `LAST_PUSHED_ALARM_KEY` — `Option<i64>` last pushed driver alarm |

Preload handling is tri-state for each prefix:

- `[1]`: no bundle falls back to KV, requested-but-absent means fresh actor defaults, present decodes the persisted actor.
- `[2] + conn_id`: consumed from preload when `PreloadedKv.requested_prefixes` includes `[2]`; fall back to `kv.list_prefix([2])` only when that prefix is absent.
- `[5, 1, 1]` + `[5, 1, 2]`: consumed from preload when requested; fall back to KV only when absent.

## State persistence flow

- `request_save` uses `RequestSaveOpts { immediate, max_wait_ms }`. NAPI callers use `ctx.requestSave({ immediate, maxWaitMs })`. Do not use a boolean `requestSave` or `requestSaveWithin`.
- Receive-loop persistence routes deferred saves through `ActorContext::request_save(...)` + `ActorEvent::SerializeState { reason: Save, .. }`.
- Shutdown adapters persist explicitly with `ActorContext::save_state(Vec<StateDelta>)` because `Sleep`/`Destroy` replies are unit-only. Direct durability must still clear pending save-request flags after a successful write.
- Actor state is post-boot delta-only. Use `request_save` / `save_state(Vec<StateDelta>)`. Do not reintroduce `set_state` / `mutate_state`.
- Schedule mutations update `ActorState` through a single helper, then immediately kick `save_state(immediate = true)` and resync the envoy alarm to the earliest event.
- State mutations from inside `on_state_change` callbacks fail with `actor/state_mutation_reentrant`. Use vars or another non-state side channel for callback-run counters.

## Inspector wiring

- Live inspector state rides `ActorContext::inspector_attach()` returning an `InspectorAttachGuard` plus `subscribe_inspector()`. Hold the guard for the websocket lifetime so `ActorTask` can debounce `SerializeState { reason: Inspector, .. }` off request-save hooks.
- Cross-cutting inspector hooks stay anchored on `ActorContext`. Queue-specific callbacks carry the current size; connection updates read the context connection count so unconfigured inspectors stay cheap no-ops.

## Schedule + alarms

- `Schedule` alarm sync is guarded by `dirty_since_push`. Fresh schedules start dirty, mutations set dirty, and unchanged shutdown syncs must not re-push identical envoy alarms.
- Persisted driver-alarm dedup stores the last pushed `Option<i64>` at actor KV key `[6]`. Startup loads it with `PERSIST_DATA_KEY` and skips identical future alarm pushes.

## Transport helpers

- HTTP and WebSocket staging helpers keep transport failures at the boundary. `on_request` errors become HTTP 500 responses; `on_websocket` errors become logged 1011 closes. `ConnHandle` and `WebSocket` wrappers surface explicit configuration errors through internal `try_*` helpers.
- Bulk transport disconnect helpers sweep every matching connection, remove the successful disconnects, update connection/sleep bookkeeping, then aggregate any per-connection failures into the returned error.
- Receive-loop `ActorEvent::Action` dispatch uses `conn: None` for alarm-originated work and `Some(ConnHandle)` for real client connections. Do not synthesize placeholder connections for scheduled actions.
- Sleep readiness stays centralized in `ActorContext` sleep state. Queue waits, scheduled internal work, disconnect callbacks, and websocket callbacks report activity through `ActorContext` hooks so the idle timer stays accurate.
- User-facing `onDisconnect` work runs inside `ActorContext::with_disconnect_callback(...)` so `pending_disconnect_count` gates sleep until the async callback finishes.

## Registry + dispatch

- Registry startup builds configured `ActorContext`s with `ActorContext::build(...)` so state, queue, and connection managers inherit the actor config before lifecycle startup runs. `ActorContext::build(...)` must seed owned queue, connection, and sleep config storage from its `ActorConfig`; do not initialize those fields with `ActorConfig::default()`.
- Registry actor task handles live in one `actor_instances: SccHashMap<String, ActorInstanceState>`. Use `entry_async` for Active/Stopping transitions.
- `RegistryDispatcher::handle_fetch` owns framework HTTP routes `/metrics`, `/inspector/*`, `/action/*`, and `/queue/*`. TypeScript NAPI callbacks keep action/queue schema validation and queue `canPublish`.
- Raw `onRequest` HTTP fetches bypass `maxIncomingMessageSize` / `maxOutgoingMessageSize`. Those message-size guards apply only to `/action/*` and `/queue/*` framework routes, not unmatched user `onRequest` paths.
- Framework HTTP error payloads omit absent `metadata` for JSON/CBOR responses so missing metadata stays `undefined`. Only explicit metadata `null` serializes as `null`.

## Startup sequence

1. Load `PersistedActor` into `ActorContext` before factory creation.
2. Persist `has_initialized` immediately.
3. Resync persisted alarms and restore hibernatable connections.
4. Set `ready` before the driver hook.
5. Reset the sleep timer.
6. Spawn `run` in a detached panic-catching task.
7. Drain overdue scheduled events after `started`.
8. Set `started` after the driver hook completes.

## Shutdown sequences

### Sleep

Two-phase:

- `SleepGrace` fires `onSleep` immediately and keeps dispatch/save timers live.
- `SleepFinalize` gates dispatch, suspends alarms, and runs teardown.

Sleep grace must fire the actor abort signal on entry and wait for the run handler to exit before finalize. Destroy abort firing remains unchanged.

Finalize:

1. Wait for the tracked `run` task.
2. Poll `ActorContext` sleep state for the idle window and shutdown-task drains.
3. Wait for `ActorContext::wait_for_on_state_change_idle(...)` before sending final save events so async `onStateChange` work cannot race durability.
4. Persist hibernatable connections.
5. Disconnect non-hibernatable connections.
6. Immediate state save.

### Destroy

- Skip the idle-window wait.
- Use the unified `sleep_grace_period` budget for the destroy phase.
- Wait for `wait_for_on_state_change_idle(...)` before final saves.
- Disconnect every connection.
- Immediate state save + SQLite cleanup.

### Stop

Persistence order:

1. Immediate state save.
2. Pending state write wait.
3. Alarm write wait.
4. SQLite cleanup.
5. Driver alarm cancellation.

## ActorConfig

- `sleep_grace_period_overridden` distinguishes an explicit `sleep_grace_period` from runtime override defaults.

## envoy-client interop

- Graceful actor teardown flows through `EnvoyCallbacks::on_actor_stop_with_completion`. The default implementation preserves the old immediate `on_actor_stop` behavior by auto-completing the stop handle after the callback returns.
- Sync `EnvoyHandle` lookups for live actor state read the shared `SharedContext.actors` mirror keyed by actor id/generation. Blocking back through the envoy task can panic on current-thread Tokio runtimes.

## Callbacks

- Boxed callback APIs use `futures::future::BoxFuture<'static, ...>` plus the shared `actor::callbacks::Request` and `Response` wrappers so config and HTTP parsing helpers stay in core for future runtimes.

## Test isolation

- Process-global `ActorTask` test hooks (`install_shutdown_cleanup_hook`, lifecycle-event/reply hooks) must be actor-scoped and serialized in tests. Parallel `cargo test` runs will otherwise cross-wire unrelated actors.

## High-level wrapper (`rivetkit`) interop

- Typed `Ctx<A>` stays a stateless wrapper over `rivetkit-core::ActorContext`. Actor state lives in the user receive loop. There is no typed vars field. CBOR encode/decode stays at wrapper method boundaries like `broadcast` and `ConnCtx`.
- Typed `Ctx<A>::client()` builds and caches `rivetkit-client` from core Envoy client accessors. Keep actor-to-actor client construction in the wrapper, not core.
- Typed `Start<A>` wrappers rehydrate each `ActorStart.hibernated` state blob back onto the `ConnHandle` before exposing `ConnCtx`, or `conn.state()` stops matching the wake snapshot.
- `rivetkit-rust/packages/rivetkit/src/persist.rs` owns typed actor-state `StateDelta` builders. `SerializeState`/`Sleep`/`Destroy` in `src/event.rs` stay thin reply helpers that reuse those builders instead of open-coding persistence bytes per wrapper.
