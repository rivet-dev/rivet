# User Complaints

Running log of complaints raised during the session. Not for implementation.

Started: 2026-04-21

---

## 1. Merge subsystem types into a single `ActorContext`

Today rivetkit-core models each subsystem as its own `Arc<...Inner>` handle: `ActorState`, `Queue`, `Schedule`, `ConnectionManager`, `SleepController`, `EventBroadcaster`, `ActorVars`. Each owns its own slice of fields, exposes its own `configure_*` / `set_*` setters for runtime wiring, and is composed inside `ActorContextInner` *and* passed independently to other subsystems that need it.

This pattern produces real bloat:

- ~22 `configure_*` / `set_*` / `clear_*` plumbing methods that exist only to inject sibling references at runtime (across `state.rs`, `queue.rs`, `schedule.rs`, `connection.rs`, `sleep.rs`, `context.rs`).
- Duplicated fields between subsystems and `ActorContextInner` (`lifecycle_events`, `lifecycle_event_inbox_capacity`, `metrics` all appear on `ActorStateInner` AND `ActorContextInner`; similar duplication on `Schedule`, `Queue`, `ConnectionManager`).
- Every subsystem has a `Mutex<Option<...>>` slot for an `EnvoyHandle` / lifecycle event sender / inspector that gets filled at startup and read at runtime — none of these need to be `Option<...>` if everything's constructed in one go.
- Cross-subsystem wiring code in the constructor: `Schedule::new(state.clone(), actor_id, config)` style, where `Schedule` needs an `Arc<ActorState>` clone to read/write `scheduled_events`.
- `pub use` re-exports of each subsystem type from `lib.rs`, even though no code outside rivetkit-core uses them.

There are no real concurrency benefits to the split. Lock granularity is per-field, not per-struct (`RwLock<Vec<u8>>`, `AsyncMutex<()>`, `AtomicBool`, etc.) — folding fields onto one struct preserves identical concurrency. Refcount cost is the same (one atomic inc per `Arc::clone`, regardless of which Arc). The "cycle" risk only exists when there are multiple Arc-wrapped types pointing at each other; with one struct, methods in different impl blocks just call sibling methods directly — no cycles possible.

### Proposed shape

A single `pub struct ActorContext(Arc<ActorContextInner>)` with one Inner that owns all subsystem fields flat. Methods stay in their existing files via separate `impl ActorContext { ... }` blocks (multi-file, single-type pattern).

**Merge into `ActorContextInner` (flat fields, no Arc-wrapped subsystem):**

- `ActorState` fields: `current_state`, `persisted`, `dirty`, `revision`, `save_request_revision`, `save_requested`, `save_requested_immediate`, `save_requested_within_deadline`, `last_save_at`, `pending_save`, `tracked_persist`, `save_guard`, `request_save_hooks`.
- `Queue` fields: queue metadata, message store, init `OnceCell`, plus the wait-activity / inspector callback slots.
- `Schedule` fields: `envoy_handle`, `generation`, `local_alarm_callback`, `local_alarm_task`, `local_alarm_epoch`, `alarm_dispatch_enabled`, `driver_alarm_cancel_count`, `internal_keep_awake`.
- `ConnectionManager` fields: the conn map, hibernation state, disconnect/transport callbacks, runtime config.
- `SleepController` state machine fields.
- `EventBroadcaster` (likely trivial; flatten or delete).
- `ActorVars` (per complaint #11, removed entirely from core).

**Methods stay where they live now.** `state.rs` becomes `impl ActorContext { fn set_state, fn save_state, fn is_dirty, ... }`. Same for `queue.rs`, `schedule.rs`, `connection.rs`, `sleep.rs`. The file split survives; only the type split goes away.

### What stays separate

- **Backend handles** (`Kv`, `SqliteDb`) — external systems, not actor state. Kept as fields on Inner: `kv: Kv`, `sql: SqliteDb`.
- **External-shared values** (`ActorMetrics`, `ActorDiagnostics`) — cloned out to Prometheus / metric handlers. Their internal Arc-shape stays.
- **Optional plug-ins** (`Inspector`) — already `Option<Inspector>`. Stays.
- **Pure data values** (`PersistedActor`, `StateDelta`, `ConnHandle`, `Hibernation`, `QueueMessage`, `ActorKey`, `ActorConfig`, `LifecycleCommand`, `LifecycleEvent`, `DispatchCommand`, `ActorEvent`, `ActorStart`, etc.) — stay as their own types.
- **The runner** (`ActorTask`) — stays separate from `ActorContext`. Context is the cheap-to-clone handle; Task owns the inboxes and runs the select loop. Two distinct roles.
- **Lifecycle channels** (`lifecycle_inbox`, `lifecycle_events`, `dispatch_inbox`, `actor_events` mpsc channels) — receivers on `ActorTask`, senders on `ActorContext`. Split is meaningful (back-pressure isolation, biased-select priority — see complaint #8).
- **Pure-data state machines** — for complex sub-state worth grouping conceptually (e.g., a sleep state-machine enum + timer fields), introduce a plain inner struct (`SleepState`) with no `Arc` and no `Mutex`, as a single field on `ActorContextInner`. Grouping without subsystem overhead.

### What gets deleted

- ~22 `configure_*` / `set_*` / `clear_*` plumbing methods.
- Duplicated `lifecycle_events`, `lifecycle_event_inbox_capacity`, `metrics`, `actor_id` fields across subsystems.
- All `Mutex<Option<EnvoyHandle>>` / `Mutex<Option<Inspector>>` / `RwLock<Option<...>>` runtime-wiring slots — fields populated at construction become plain values.
- The `Schedule::new(state.clone(), actor_id, config)` style cross-subsystem wiring in the constructor.
- The `pub use` re-exports of `ActorState`, `Queue`, `Schedule`, `ConnectionManager`, `SleepController`, `ActorVars`.

### Concurrency cost: none

- Lock granularity preserved (per-field, not per-struct).
- Refcount cost identical (one atomic inc per `Arc::clone`).
- Async borrows unchanged (interior mutability via `RwLock` / `AsyncMutex` / atomics).
- No cycle risk (single struct, no inter-type Arc references).

### Cost paid

- `ActorContextInner` becomes much larger (50-80 fields). Mitigated by impl-block splitting across files.
- Loss of type-level access control between subsystems (today `ActorState::dirty` is private to `state.rs` because `ActorStateInner` is private; folded, anything in the crate could touch `inner.dirty`). Enforced by impl-block discipline and field visibility, not type boundaries. Worth flagging in CLAUDE.md.
- Tests that construct a bare `ActorState::new(kv, config)` need an `ActorContext::new_for_state_tests(kv, config)` helper instead. Minor.

### Phasing

If implemented, do it one subsystem at a time, smallest to largest, to keep PRs reviewable:

1. `ActorVars` (complaint #11 already removes it entirely)
2. `EventBroadcaster`
3. `SleepController`
4. `Schedule`
5. `Queue`
6. `ActorState`
7. `ConnectionManager`

Each step deletes a few `configure_*` methods, removes one `Arc<*Inner>` wrapper, moves methods to `impl ActorContext` blocks in the existing file. Tests should mostly survive with one helper-constructor change per file.

## 2. Unused `LifecycleState` variants should be removed

`rivetkit-rust/packages/rivetkit-core/src/actor/task_types.rs:5-17` declares 9 variants but only 6 are ever transitioned to in `actor/task.rs`:

- **Live:** `Loading` (default), `Started`, `SleepGrace`, `SleepFinalize`, `Destroying`, `Terminated`.
- **Dead (no `transition_to` call):** `Migrating`, `Waking`, `Ready`.

`transition_to` at task.rs:1309 still has match arms for the dead variants (1312-1320), and `dispatch_lifecycle_error` groups them under a `NotReady` branch (518-524). Removing the three unused variants simplifies both match sites and makes the state machine match the declared design in the codebase layer docs.

## 3. rivetkit-core and rivetkit-napi need extensive debug/info logging

There's very little tracing output across the actor lifecycle in either crate. Debugging hibernation bugs, sleep timing, dispatch dead-ends, inbox overloads, or runtime-state desyncs currently requires reading source and adding ad-hoc `println!`s.

Wanted coverage in `rivetkit-rust/packages/rivetkit-core/`:

- Lifecycle transitions (`transition_to` at `task.rs:1309`) — every state change at `info!` with actor_id, old, new.
- Every `LifecycleCommand` received and replied at `debug!` (Start, Stop, FireAlarm).
- Every `DispatchCommand` received at `debug!` with variant + dispatch_lifecycle_error outcome.
- `ActorEvent` enqueue/drain at `debug!` (Action, WebSocket lifetime, SerializeState reason, BeginSleep).
- Sleep controller decisions — activity reset, idle-out, keep-awake engage/disengage, grace start, finalize start.
- `Schedule` activity — event added/cancelled, local alarm armed/fired, envoy `set_alarm` push (with old/new values once complaint 6 lands).
- Persistence — every `apply_state_deltas` with delta count + revision, `SerializeState` reason + bytes, alarm-write waits.
- Connection manager — conn added/removed/hibernation-restored/hibernation-transport-removed, dead-conn settle outcomes.
- KV backend calls — `batch_get` / `batch_put` / `delete` / `list_prefix` key counts and latencies at `debug!`.
- Inspector attach/detach, overlay broadcasts.
- Shutdown path — sleep grace entered, sleep finalize entered, destroy entered, each shutdown step (wait_for_run_handle, disconnect waves, sql cleanup, alarm cancel).

Wanted coverage in `rivetkit-typescript/packages/rivetkit-napi/`:

- Every TSF callback invocation with kind + payload shape summary at `debug!`.
- Runtime shared-state cache hit/miss for `ActorContextShared` by actor_id.
- Bridge error paths — structured error prefix decode/encode outcomes.
- `AbortSignal` -> `CancellationToken` bridge trigger.
- N-API class lifecycle (construct/drop) for `ActorContext`, `JsNativeDatabase`, queue-message wrappers.

Use structured tracing (`tracing::info!(actor_id = %id, ...)`) rather than formatted messages, per existing CLAUDE.md convention.

## 4. Engine process manager should not live in `registry.rs`

`rivetkit-rust/packages/rivetkit-core/src/registry.rs` is 4083 lines and mixes three unrelated concerns: the registry/dispatcher, the inspector HTTP surface, and the engine subprocess supervisor. The subprocess code has nothing to do with actor registration or dispatch and should move to its own module (e.g. `engine_process.rs`).

Items that belong in a separate file:

- `struct EngineHealthResponse` (registry.rs:129)
- `struct EngineProcessManager` (registry.rs:136) and its `impl` (registry.rs:2387)
- `fn engine_health_url` (registry.rs:2499)
- `fn spawn_engine_log_task` (registry.rs:2503)
- `async fn join_log_task` (registry.rs:2523)
- `async fn wait_for_engine_health` (registry.rs:2532)
- `async fn terminate_engine_process` (registry.rs:2576)
- `fn send_sigterm` (registry.rs:2615)

Only the spawn/shutdown call sites in `CoreRegistry::serve` (registry.rs:325, 354) need to remain in `registry.rs`, and those just call into the new module.

## 5. Remove preload KV entirely; use a single batch get on startup

Preload KV today is half-committed: the engine ships a `PreloadedKv { entries }` bundle in `on_actor_start`, but rivetkit-core only extracts the `[1]` (actor state) entry and discards the rest. Connections (`[2]+*`) and queue (`[5,1,2]+*`, `[5,1,1]`) still do their own prefix scans at startup. So you pay the plumbing cost without getting the full RTT savings.

Kill the preload path entirely and replace startup with a single batched KV fetch.

Protocol impact (requires a new envoy-protocol version — per CLAUDE.md, do not modify existing published `*.bare`):

- `PreloadedKv` and `PreloadedKvEntry` are defined in `engine/sdks/schemas/envoy-protocol/v1.bare` and `v2.bare`.
- Current live version is v2. Adding `v3.bare` without the preload fields is the path forward. Migrate `versioned.rs` to bridge v1/v2 messages by dropping the preload payload on the way in.

Rivetkit-core deletions:

- `protocol::PreloadedKv` parameter on `on_actor_start` (`rivetkit-rust/packages/rivetkit-core/src/registry.rs:2238`)
- `decode_preloaded_persisted_actor` (`registry.rs:2689-2703`)
- `StartActorRequest.preload_persisted_actor` field (`registry.rs:102`) and its plumbing through `start_actor`
- `ActorTask.preload_persisted_actor` field + constructor param (`task.rs:241, 262, 290`)
- The `if let Some(preloaded) = self.preload_persisted_actor.take()` branch in `load_persisted_actor` (`task.rs:611-613`)

Engine deletions:

- `engine/packages/pegboard/src/actor_kv/preload.rs` (`fetch_preloaded_kv`)
- The `fetch_preloaded_kv` call sites in `pegboard-outbound/src/lib.rs:252` and `pegboard-envoy/src/sqlite_runtime.rs:48-50`
- All `preloaded_kv: Option<protocol::PreloadedKv>` threading in `envoy-client` (`actor.rs:126, 138, 153, 190`, `events.rs:95`, `config.rs:104`, `commands.rs:23`)

Replacement:

- `start_actor` issues one `kv.batch_get` for the known fixed keys (`[1]`, `[5,1,1]`) plus two `list_prefix` calls (for `[2]` connections and `[5,1,2]` queue messages). Each subsystem consumes its portion.
- Mental model collapses from two paths (preloaded-or-fetch) to one.

## 6. Deduplicate engine `set_alarm` pushes — two distinct cases

The engine's `alarm_ts` is durable per-actor (stored as a field on the actor workflow state at `engine/packages/pegboard/src/workflows/actor2/runtime.rs:20` and `actor/runtime.rs:60`), and it persists across sleep/wake cycles. Rivetkit-core currently pushes `set_alarm` unconditionally, wasting round-trips in two different scenarios.

### 6a. Shutdown re-sync is unneeded when nothing changed

`finish_shutdown_cleanup` at `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:1056` calls `sync_alarm_logged()` unconditionally before teardown. If no `Schedule` mutation happened during the actor's awake period (no `at(...)`, no `cancel(...)`, no `schedule_event(...)`), this just re-pushes the same value that was pushed on startup.

Fix: `Schedule` tracks a `dirty_since_push: bool` flag. Any mutation sets it to true. `sync_alarm` / `sync_future_alarm` check it and skip the push when false. The flag resets to false after a successful push.

### 6b. Startup push is unneeded when the engine already holds the correct value

Example: actor has a scheduled event 3 days out. Goes to sleep. Client request arrives now (not the alarm firing). Engine wakes actor on new generation. `init_alarms` at `task.rs:602` pushes `set_alarm(T_3days)` — but the engine *already has* `state.alarm_ts = Some(T_3days)` from the previous generation. The push is identical-value noise.

The wrinkle: rivetkit-core on a fresh boot has no in-memory record of what was last pushed (new process, new `Schedule` struct). Three options:

- **(a) Persist last-pushed in the actor's own KV.** Add a small KV entry like `LAST_PUSHED_ALARM_KEY = [6]` holding the last-pushed `Option<i64>`. On startup, load it alongside `PersistedActor`, compare against the current desired value, and skip the push when equal. Cost: one extra KV read per start (or zero if it rides the same batch as complaint #5).

- **(b) Engine returns current `alarm_ts` in `on_actor_start`.** Extend the protocol so the `on_actor_start` callback payload includes the engine's current view of `alarm_ts`. Startup compares locally and skips if equal. Cost: protocol bump (pairs naturally with the envoy-protocol v3 from complaint #5).

- **(c) Engine-side idempotency.** Keep the client always pushing, but have `EventActorSetAlarm` handlers short-circuit when `state.alarm_ts == alarm_ts`. This doesn't save the round-trip, only engine-side work.

Option (b) is cleanest if protocol is already being bumped. Option (a) is a contained local fix.

## 7. Document why `try_reserve` is used instead of `try_send`

The pattern is everywhere in rivetkit-core (see `reserve_actor_event` at `task.rs:465-481`, `try_send_lifecycle_command` / `try_send_dispatch_command` at `registry.rs:47`, and various `.try_reserve_owned()` call sites), and it's mandated in CLAUDE.md: "Actor-owned lifecycle/dispatch/lifecycle-event inbox producers must use `try_reserve` helpers and return `actor.overloaded`; do not await bounded `mpsc::Sender::send`."

But there's no comment on any of those helpers explaining *why*. A reader sees `try_reserve_owned()` followed by `permit.send(...)` and wonders why it isn't just `try_send(...)`. The reasons should be documented inline:

- `try_reserve` returns a permit before the message is constructed. If the channel is full, the caller can build an error and return without allocating the payload (oneshot reply channels, CBOR buffers, cloned `ActorContext`s, etc.). `try_send` requires the fully-constructed message up front and hands it back inside the `Err(Full)` variant — at which point you've already paid for building it.
- `try_reserve` decouples "is there capacity?" from "here's the value." That makes structured backpressure (log + metric + `actor.overloaded` error) cheap, whereas `try_send` conflates the two and leaks the half-built message on the reject path.
- For lifecycle commands specifically, constructing `LifecycleCommand::Start { reply: oneshot::channel().0 }` allocates; if we discover the channel is full only *after* that, the oneshot is orphaned and the receiver half immediately errors. `try_reserve` avoids the spurious oneshot creation.

Add a module-level `//!` doc or a short comment on the `reserve_actor_event` / `try_send_lifecycle_command` helpers explaining this so the pattern isn't cargo-culted without understanding.

## 8. Document why `ActorTask` has multiple separate inboxes

`ActorTask` holds four separate `mpsc::Receiver`s (`rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:234-243`):

- `lifecycle_inbox: Receiver<LifecycleCommand>` — Start, Stop, FireAlarm
- `lifecycle_events: Receiver<LifecycleEvent>` — StateMutated, ActivityDirty, SaveRequested, InspectorSerializeRequested, InspectorAttachmentsChanged, SleepTick
- `dispatch_inbox: Receiver<DispatchCommand>` — Action, Http, OpenWebSocket, WorkflowHistory, WorkflowReplay
- `actor_event_rx: Receiver<ActorEvent>` — user-run-loop events

The design is deliberate but undocumented. A module-level `//!` doc on `task.rs` (or a dedicated `docs-internal/` note) should spell out:

**Back-pressure isolation.** Each inbox has its own capacity (`config.lifecycle_command_inbox_capacity`, `config.lifecycle_event_inbox_capacity`, `config.dispatch_command_inbox_capacity`). A burst of user-dispatched actions (dispatch_inbox full) must never block a `Stop` command (lifecycle_inbox). A flood of internal `StateMutated` events (lifecycle_events full) must never block a `Start` / `Stop`. Sharing one channel would couple these back-pressure domains and let a high-frequency producer starve a low-frequency critical producer.

**Priority via `biased` select.** The run loop at `task.rs:256-310` uses `tokio::select! { biased; ... }`, which checks arms in declaration order: lifecycle commands first, then lifecycle events, then dispatch, then timers. When messages race, commands always win. Single-channel-with-tags couldn't express this without an ad-hoc priority queue on top.

**Overload semantics.** Command inbox overload (`lifecycle_inbox_overload_total`) surfaces as an `actor.overloaded` error to an external envoy caller who can retry or fail explicitly. Event inbox overload (`lifecycle_event_inbox_overload_total`) is an internal bug class (dropped save request, missed sleep timer reset). Different inboxes → different metrics → different alerting.

**Sender/trust topology.** Lifecycle commands are constructed only in `registry.rs` by envoy callbacks and the local alarm callback. Lifecycle events are pushed from many points in `ActorState`, `ActorContext`, `Queue`, etc. Splitting by channel makes the trust boundary visible in the types: external callers can't accidentally construct internal events, and internal subsystems can't accidentally trigger lifecycle transitions.

Note this in a code comment on the `ActorTask` struct and in `docs-internal/engine/rivetkit-core-lifecycle.md` (or similar) so the pattern is self-explaining.

## 9. Simplify and clarify the state-mutation API surface

`rivetkit-core` and `rivetkit-napi` currently expose five overlapping state-mutation entrypoints with unclear roles:

- `state.set_state(Vec<u8>)` — replace bytes wholesale (delegates to `mutate_state`)
- `state.mutate_state<F>(reason, F)` — general closure-based primitive
- `state.request_save(immediate: bool)` — fire `SaveRequested` lifecycle event
- `state.request_save_within(ms)` — same with max-wait deadline
- `ctx.save_state(deltas)` — apply structured deltas + immediate KV write

The TS layer uses only two of these in user-facing paths (`requestSave(false)` for dirty hints, `saveState(deltas)` for structured immediate saves). `set_state` is only used internally during boot (`set_state_initial`). `mutate_state` is not exposed to NAPI at all because closures can't cross the language boundary.

### 9a. Remove `set_state` from the public NAPI surface entirely

The NAPI `set_state` method at `rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:229` is only valid during boot. User-facing TS code calls `saveState(deltas)` instead. Delete it from the public NAPI surface. The boot-only `set_state_initial` (`actor_context.rs:159-161`) stays as a private bootstrap entrypoint, but no public NAPI method should expose state-replace semantics outside the structured-deltas + serialize-callback flow.

### 9b. Drop the `Either<bool, StateDeltaPayload>` shim on NAPI `save_state`

`rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:355-371` accepts both a bool (legacy `request_save` shim) and a real payload. CLAUDE.md already warns: "the legacy boolean `ctx.saveState(true)` path only flips `request_save` and returns before the KV commit lands." The shim is a footgun — callers think they got a durable save and didn't. Remove it; force callers to either `requestSave(immediate)` (hint) or `saveState(payload)` (real save).

### 9c. Remove `mutate_state` and `set_state` from the core `ActorState` API

Both `set_state` (`rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:132-137`) and `mutate_state` (`state.rs:139-174`) should be deleted. The only state-mutation API rivetkit-core exposes for the actor lifetime is the lifecycle save-request + `serializeState` callback flow:

- Caller signals "I changed something, please save" → `request_save(immediate)` (or `request_save_within(ms)`).
- Actor task picks up `LifecycleEvent::SaveRequested`, schedules a serialize tick (immediate or debounced).
- Tick fires → core invokes the `serializeState` callback to collect a `Vec<StateDelta>` from the foreign runtime.
- Core applies the deltas via `apply_state_deltas` → KV write.

Why this is the right shape:

- The TS runtime never used `set_state` outside boot — JS state lives in JS memory, and core only sees serialized bytes via the deltas payload.
- A future Rust runtime would store its state in the language-side type and serialize via the same callback flow; it doesn't need core to mutate a `Vec<u8>` in place.
- Removing both methods kills the entire `StateMutated` lifecycle event, the `replace_state` helper, the reentrancy check around `in_on_state_change_callback`, the `StateMutationReason::UserSetState` / `UserMutateState` metric labels, and the `set_state` delegate on `ActorContext` (`context.rs:239-247`).

Boot stays special: `set_state_initial` (`rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:159-161`) keeps existing as a private bootstrap entry that calls `state.set_state` once during startup before the lifecycle event channel is configured. After boot, the only path is request-save + serialize-callback. Pairs with 9f below.

### 9d. Document the role of each method on a single page

Add a `docs-internal/engine/rivetkit-core-state-management.md` (or top-of-`state.rs` `//!` doc) covering:

- The TS runtime owns its state in JS memory; core only sees serialized bytes via `saveState(deltas)`.
- `set_state` / `mutate_state` are the Rust-runtime entrypoints for actors whose state lives in core.
- `request_save(false)` and `request_save_within(ms)` are debounced "please serialize me eventually" hints — they do NOT mutate state.
- `save_state(deltas)` is the structured immediate-save path that crosses the language boundary.
- `persist_state(opts)` is internal flush logic, not for callers.

### 9e. Consider collapsing `request_save(immediate)` and `request_save_within(ms)`

These could be one `request_save(opts: { immediate?: bool, max_wait_ms?: u32 })` to give a single ergonomic surface. Cost: a struct allocation per call vs. raw bool/u32. Worth it for clarity.

### 9f. Unify immediate and deferred save paths through one serialize callback

Today there are two parallel save flows that produce the same payload via the same `serializeForTick("save")` function but reach the KV write through different code paths:

- **Immediate** (`saveState({ immediate: true })` at `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:2586-2590`): TS synchronously calls `serializeForTick("save")` → passes payload to NAPI `ctx.saveState(payload)` → core converts to `Vec<StateDelta>` → `apply_state_deltas` → KV write.
- **Deferred** (`requestSave(false)` / `requestSaveWithin(ms)`): TS fires a dirty hint → core fires `LifecycleEvent::SaveRequested` → actor task debounces → state-save tick fires → core calls back into TS via the `serializeState` TSF callback → TS calls `serializeForTick("save")` → returns payload → core applies + writes.

Risks of the duplication:

- **Drift**: any new field added to `serializeForTick` that the immediate path forgets to thread through gets silently dropped on immediate saves.
- **Asymmetric "dirty" handling**: immediate skips `hasNativePersistChanges` (`native.ts:2593`); deferred respects it. That's a hidden surprise.
- **Two test surfaces**: every save behavior gets tested twice or one path lags coverage.

Simplification: collapse to one core API that always fires `serializeState` to collect the payload, with caller-controlled debounce. `saveState({ immediate: true })` becomes "schedule with zero debounce, await completion." `requestSave(false)` stays "schedule with debounce, fire-and-forget." TS code stops calling `serializeForTick` directly outside the callback.

Today's three immediate-save callers (`native.ts:3774`, `actor-inspector.ts:224`, `hibernatable-websocket-ack-state.ts:109`) all want durability before continuing — none depend on the synchronous-serialize behavior. The extra Rust→JS→Rust hop per immediate save is microseconds in-process and a worthwhile trade for one pipeline.

### 9g. Align connection state with actor state through the same dirty/notify/serialize system

Today connection state and actor state live on different systems. The asymmetry:

| Concern | Actor state | Connection state |
|---|---|---|
| Dirty bit in core | Yes (`state.rs:69`) | **No** — lives in TS as `persistChanged` |
| Lifecycle event on mutation | `StateMutated` fires | **None** |
| Auto-triggers save flow | Yes (via `mutate_state`) | **No** — TS must call `ctx.requestSave(false)` manually |
| Serialize callback returns bytes | Yes (`serializeForTick("save")` → `StateDelta::ActorState`) | Also yes (`StateDelta::ConnHibernation { conn, bytes }`) but only if TS remembers to include it |

The `StateDelta` enum at `rivetkit-rust/packages/rivetkit-core/src/actor/callbacks.rs:234` already has the right variants (`ActorState`, `ConnHibernation`, `ConnHibernationRemoved`) — the delta path is there. What's missing is the dirty-tracking and notify machinery on the *connection* side that would drive that path automatically, matching what actor state already has.

#### Target design

Same flow for both. `ctx.setState(...)` or `conn.setState(...)` both:

1. Mark a dirty bit in core (per-actor for actor state, per-conn for conn state — hibernatable only).
2. Fire `LifecycleEvent::SaveRequested { immediate: false }` to nudge the actor task.
3. Actor task debounces, then invokes the `serializeState` callback.
4. Foreign runtime returns a `Vec<StateDelta>` covering both actor state and any dirty conn states.
5. Core applies the deltas via `apply_state_deltas` and writes to KV.

Concrete changes:

- `ConnHandle` (`rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs:92-104`) gets a `dirty: AtomicBool` field for hibernatable conns.
- `ConnHandle::set_state` (connection.rs:142-148) marks the conn dirty AND marks the actor dirty AND fires `LifecycleEvent::SaveRequested { immediate: false }`.
- Non-hibernatable conns' `set_state` stays in-memory only, no dirty tracking (their state isn't persisted anyway, so no reason to nudge a save).
- `serializeForTick` callback contract becomes: "return deltas for any state (actor or conn) that's marked dirty in core." Core iterates dirty hibernatable conns and asks the foreign runtime to serialize each into `StateDelta::ConnHibernation { conn_id, bytes }`.
- Delete the TS-side `ensureNativeConnPersistState` / `persistChanged` tracking — dirty tracking now lives in core.
- Delete the per-site `callNativeSync(() => ctx.requestSave(false))` calls in `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` (found at ~line 2409, 2602, 2784, 3035, 4310, 4362, 4408 before the recent line shifts). The `conn.setState(...)` call now triggers the save automatically.
- Remove the CLAUDE.md rule "Every `NativeConnAdapter` construction path... must keep both the `CONN_STATE_MANAGER_SYMBOL` hookup and a `ctx.requestSave(false)` callback" — that rule only exists to work around the missing auto-nudge.

#### Why this is the right scope (and where to be careful)

- **Hibernatable-only dirty tracking**: conn volume can be high (dozens to thousands per actor). Firing `LifecycleEvent::SaveRequested` per conn mutation is fine *if* it's debounced (it is, by design) and *if* it's only tracked for hibernatable conns. Non-hibernatable conns must not enter this path — their state is ephemeral by contract.
- **Conn lifetime vs actor lifetime**: when a conn disconnects, its dirty bit dies with it. No pending-save semantics need to cross the disconnect boundary, because `StateDelta::ConnHibernationRemoved(conn)` is a separate delta type for the "this conn is going away" case.
- **Pairs with 9c above** (remove `set_state` / `mutate_state`): both actor state and conn state would use the same `request_save → serializeState → deltas → apply` pipeline. One system, one mental model.

## 10. Make preload efficient end-to-end

This builds on complaint #5 but assumes preload is kept (per the user's clarification that we are not removing it).

Today's preload bundle ships from engine to actor in `on_actor_start`, but only the `[1]` (actor state) entry is consumed on the actor side. The engine already includes other prefix entries (see `engine/packages/pegboard/src/actor_kv/preload.rs:181-240` for connection-prefix entries) but the actor discards them. Net result is one round-trip saved on wake (the `kv.get([1])`) and zero saved for hibernation restore or queue init.

Reference: the original TypeScript implementation at git ref `feat/sqlite-vfs-v2` per CLAUDE.md guidance. Compare its preload consumption behavior to confirm parity targets.

To make preload genuinely efficient end-to-end:

- **Hibernatable connections** (`[2]+conn_id`): `ConnectionManager::restore_persisted` (`rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs:746-778`) currently always does `kv.list_prefix([2])`. Change to: consume `[2]+*` entries from the preload bundle when present, only fall back to `list_prefix` when absent. Saves 1 RTT per wake on hibernation-using actors.
- **Queue metadata** (`[5,1,1]` and `[5,1,2]+*`): `Queue::ensure_initialized` (`rivetkit-rust/packages/rivetkit-core/src/actor/queue.rs:586-595`) is lazy today, but if the engine includes queue entries in the bundle they should be consumed before the first queue operation, eliminating the lazy-init `kv.get([5,1,1])` (and the rebuild `list_prefix` if metadata was lost).
- **Distinguish "fresh actor" from "preload missing"**: today `decode_preloaded_persisted_actor` (`registry.rs:2689-2703`) returns `Ok(None)` for both "no bundle at all" and "bundle exists but no `[1]` entry." `load_persisted_actor` then falls back to `kv.get([1])` in both cases. For fresh-actor creates, the engine already confirmed FDB is empty during preload — the actor shouldn't pay the round-trip again. Change the decode to return a tri-state: `NoBundle` / `BundleExistsButEmpty` / `Some(persisted)`. The middle case means "no fallback get needed, just use defaults."
- **Engine-side budget honoring**: confirm the engine's `max_total_bytes` budget gets used proportionally across all the prefixes the actor will need (state + connections + queue), not just biased toward the state blob.

End-state RTT counts with efficient preload kept:

- **Wake (full preload)**: 0 (state) + 0 (conns) + 0 (queue) = could be down to **just the `has_initialized` re-write** = 1 RTT, or 0 RTT if complaint #9 / immediate-save dedup also lands.
- **Create**: 0 (preload says nothing exists) + 1 (first state write) + 0 (no conns) = **1 RTT**.

Compared to today's 2 RTTs wake / 3 RTTs create, that's measurable improvement. Also worth measuring against the original TS impl at `feat/sqlite-vfs-v2` to make sure the engine isn't shipping more than the actor needs.

## 11. Remove `vars` from rivetkit-core; keep it as a TS-runtime-only construct

`ActorVars` (`rivetkit-rust/packages/rivetkit-core/src/actor/vars.rs`) is a thin `Arc<RwLock<Vec<u8>>>` wrapper that just stores a byte blob. There's nothing core-specific about it: no persistence, no lifecycle event integration, no inspector hook, no metric, no callback wiring. It's literally a getter and setter around bytes.

Vars are a TypeScript-runtime concept (per-instance, in-memory, non-persistent JS values). The TS runtime can manage them entirely on the JS side. There's no reason core needs to carry the bookkeeping.

Removals:

- Delete `rivetkit-rust/packages/rivetkit-core/src/actor/vars.rs` entirely.
- Remove `vars: ActorVars` field from `ActorContextInner` (`context.rs:54`) and the default init at `context.rs:201`.
- Remove `ActorContext::vars` and `ActorContext::set_vars` methods (`context.rs:274-281`).
- Remove the NAPI surface `vars()` and `set_vars(buffer)` (`rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:224-225, 241-242`).
- Remove the `set_vars` call in the NAPI bootstrap path (`napi_actor_events.rs:191`).

Public API stays: TS user code keeps calling `ctx.vars` / `ctx.setVars` (or whatever the TS surface is), but the implementation lives entirely in `rivetkit-typescript/packages/rivetkit/` rather than crossing NAPI to a core type that does nothing useful. Reduces the rivetkit-core surface, reduces the NAPI surface, deletes a redundant `Arc<RwLock<Vec<u8>>>` and the bridging code that exists only to forward bytes through a wrapper.

## 12. Default to async mutex; audit and convert `std::sync::Mutex` usages

The conventional Rust advice ("use `std::sync::Mutex` for short critical sections") is wrong for a fully-async runtime like rivetkit-core. Sync mutex is a footgun:

- Compiles silently when held across `.await` (`MutexGuard` is often `Send`).
- Poisons on panic — every call site needs `.expect("... lock poisoned")` boilerplate (already littered throughout the codebase).
- Forces a per-site judgment ("is this short enough?") that gets it wrong over time as code evolves.
- The microsecond performance win is dwarfed by the I/O latencies in any realistic actor operation.

New rule for rivetkit-core and rivetkit-napi:

- **Default**: `tokio::sync::Mutex` / `tokio::sync::RwLock` everywhere in async code.
- **Forced-sync exceptions**: `parking_lot::Mutex` / `parking_lot::RwLock` only when sync is mandated by the call context — `Drop::drop` impls, sync trait impls (`Display`, `Debug`, `Hash`, `PartialEq`, `Iterator`), FFI / C callback contexts (notably SQLite VFS callbacks in `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs` and `v2/vfs.rs`), and atomic-style read paths exposed as sync `&self` methods.
- **Never**: `std::sync::Mutex` — replaced by `tokio::sync::Mutex` (async) or `parking_lot::Mutex` (forced-sync). Poisoning gone in either case.

Action items:

- Add this rule to root `CLAUDE.md` so the convention is durable.
- Audit every `std::sync::Mutex` and `std::sync::RwLock` in `rivetkit-rust/packages/rivetkit-core/src/`, `rivetkit-rust/packages/rivetkit-sqlite/src/`, and `rivetkit-typescript/packages/rivetkit-napi/src/`. Classify each as forced-sync or convertible-to-async, and apply the rule.
- Notable convertible candidates surfaced by audit (re-evaluate under this rule):
  - `actor/queue.rs:105` `config: StdMutex<ActorConfig>`
  - `actor/queue.rs:113-114` callback slots `StdMutex<Option<Callback>>` (or replace with `ArcSwap` for the lock-free read path)
  - `actor/state.rs:75-77` `save_requested_within_deadline`, `last_save_at`, `pending_save`
  - `actor/state.rs:80` `lifecycle_events: RwLock<Option<mpsc::Sender<...>>>` (note: also dies if complaint #1 lands)
  - `actor/context.rs` various `RwLock<Option<...>>` runtime-wiring slots (also dies if complaint #1 lands)

## 13. KV `delete_range` TOCTOU race on the in-memory backend

`rivetkit-rust/packages/rivetkit-core/src/kv.rs:82-111` `delete_range` for `KvBackend::InMemory` reads keys under a read lock, then upgrades to a write lock to delete them. Between the two locks, another task can mutate the map — keys collected may no longer exist (no-op delete), or new keys in the range may appear and get missed.

```rust
let keys: Vec<Vec<u8>> = entries.store.read()...collect();
let mut entries = entries.store.write()...;
for key in keys { entries.remove(&key); }
```

Fix: single write lock with `BTreeMap::retain` doing the range check inline:

```rust
let mut entries = entries.store.write().expect("...");
entries.retain(|key, _| !(key.as_slice() >= start && key.as_slice() < end));
```

Test-only backend, but this is the kind of subtle bug that produces flaky tests when run under load.

## 14. `save_guard` held across the KV write — backpressure pile-up

`rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:79` `save_guard: AsyncMutex<()>` is held across `kv.apply_batch(...).await` (state.rs:310-347) and `kv.put(...).await` (state.rs:734-755). Other save attempts queue behind one in-flight KV operation. Even though serialization is intentional (don't want two saves racing), holding the guard across the actual I/O serializes everything on network latency.

Fix: split the critical section. Hold `save_guard` long enough to read state, snapshot deltas, and prepare the put list — then release before issuing the KV call. Acquire a separate "in-flight write" handle (or use a `Notify` + atomic to signal completion) for downstream waiters that need to know the write landed.

```rust
// Today:
let _save_guard = self.0.save_guard.lock().await;
// ... read state, encode, build puts/deletes ...
self.0.kv.apply_batch(&puts, &deletes).await?;     // ← pile-up here
// Drop guard.

// Better:
let (puts, deletes) = {
    let _save_guard = self.0.save_guard.lock().await;
    // read state, encode, build puts/deletes
    (puts, deletes)
};
self.0.kv.apply_batch(&puts, &deletes).await?;
```

Concurrent `apply_batch` calls then go in parallel rather than queueing.

## 15. SQLite `aux_files` double-lock TOCTOU race

`rivetkit-rust/packages/rivetkit-sqlite/src/v2/vfs.rs:1080-1090` `open_aux_file` reads `aux_files.read()` to check if a key exists, then upgrades to `aux_files.write()` to insert. Two threads opening the same aux file concurrently can both pass the read check and both allocate a new `AuxFileState`.

Fix: single write lock + `BTreeMap::entry()`:

```rust
let mut aux_files = self.aux_files.write();
let state = aux_files.entry(key).or_insert_with(|| Arc::new(AuxFileState::new()));
```

## 16. SQLite test-only `Mutex<usize>` polling counter and `Mutex<bool>` gate

`rivetkit-rust/packages/rivetkit-sqlite/src/v2/vfs.rs`:

- Lines 551, 596-598: `awaited_stage_responses: Mutex<usize>` in `MockProtocol`. Test code polls this via a getter that locks. Should be `AtomicUsize` paired with `Notify` — increment + `notify_one()` on each stage response, test code awaits `notified()` instead of polling.
- Lines 679-680: `mirror_commit_meta: Mutex<bool>` gate. Should be `AtomicBool` checked via `load(SeqCst)`.

Per CLAUDE.md: "Never poll a shared-state counter with `loop { if ready; sleep(Nms).await; }`. Pair the counter with `tokio::sync::Notify`."

## 17. Replace `inspector_attach_count` manual increment/decrement with RAII drop guard

`rivetkit-rust/packages/rivetkit-core/src/actor/task.rs:348` `inspector_attach_count: Arc<AtomicU32>`. Increment at `actor/context.rs:1105` (`fetch_add(1, SeqCst)`); decrement at `actor/context.rs:1114-1123` (`fetch_update` with `checked_sub`). The increment and decrement are at separate call sites with no RAII tying them together. If anything panics or returns early between them (lock poisoning, channel closure, error path inside the inspector subscription setup), the count leaks high.

Compare to `active_queue_wait_count` (`rivetkit-rust/packages/rivetkit-core/src/actor/queue.rs:112`) which IS correctly RAII-guarded via `ActiveQueueWaitGuard` — that's the model to mirror.

Fix: introduce `InspectorAttachGuard` that increments in `new()` and decrements in `Drop::drop`. Sketch:

```rust
struct InspectorAttachGuard {
    attach_count: Arc<AtomicU32>,
    ctx: ActorContext,  // or Weak<...>
}

impl InspectorAttachGuard {
    fn new(ctx: ActorContext) -> Option<Self> {
        let count = ctx.inspector_attach_count_arc()?;
        let was_zero = count.fetch_add(1, SeqCst) == 0;
        if was_zero {
            ctx.notify_inspector_attachments_changed();
        }
        Some(Self { attach_count: count, ctx })
    }
}

impl Drop for InspectorAttachGuard {
    fn drop(&mut self) {
        let prev = self.attach_count.fetch_sub(1, SeqCst);
        if prev == 1 {
            self.ctx.notify_inspector_attachments_changed();
        }
    }
}
```

Counters that are NOT candidates and should stay as bare atomics: `state.revision`, `save_request_revision`, `local_alarm_epoch`, `NEXT_CANCEL_TOKEN_ID`, inspector listener IDs, and the various inspector revision counters — those are monotonic sequences, not live counts.

## 18. Fix `actor/overloaded` → `actor.overloaded` in CLAUDE.md

Root `/home/nathan/r6/CLAUDE.md:298` reads: "Actor-owned lifecycle/dispatch/lifecycle-event inbox producers must use `try_reserve` helpers and return `actor/overloaded`...". The canonical Rivet error format is `{group}.{code}` (dot, not slash), as confirmed by:

- `engine/artifacts/errors/actor.overloaded.json` filename
- `rivetkit-rust/packages/rivetkit-core/src/error.rs:22-31` defining group `actor` and code `overloaded`
- The existing `actor.aborted.json`, `actor.destroying.json`, etc. in the same artifacts dir all using dot

The slash in CLAUDE.md is the source of the inconsistency — anyone (human or model) reading that line will propagate the wrong format. Fix the rule text to use `actor.overloaded`.

## 19. Async `onDisconnect` must be awaited and gate sleep via `pending_disconnect_count`

Goal: match the prior TypeScript implementation at ref `feat/sqlite-vfs-v2`, where the user-facing `onDisconnect` hook was async, could do database/KV/state work, and blocked sleep until it finished.

### What the TS impl does (parity target)

`rivetkit-typescript/packages/rivetkit/src/actor/instance/connection-manager.ts::connDisconnected` at ref `feat/sqlite-vfs-v2`:

```ts
async connDisconnected(conn) {
  this.#connections.delete(conn.id);
  this.#pendingDisconnectCount += 1;           // block sleep
  this.#actor.resetSleepTimer();
  try {
    if (this.#actor.config.onDisconnect) {
      const result = this.#actor.config.onDisconnect(this.#actor.actorContext, conn);
      if (result instanceof Promise) await result;   // awaited, DB/state/KV allowed
    }
  } finally {
    this.#pendingDisconnectCount = Math.max(0, this.#pendingDisconnectCount - 1);
    this.#actor.resetSleepTimer();             // unblock, re-evaluate
  }
}
```

The sleep gate at `actor/instance/mod.ts::#canSleep` returns `CanSleep.ActiveDisconnectCallbacks` while `pendingDisconnectCount > 0`. The sleep-timer callback re-checks `#canSleep()` and reschedules rather than firing `startSleep()` if disconnect work is still in flight.

The wire-level WebSocket close callback itself (`engine-runner/src/tunnel.ts:365-389`) stays sync — it just deletes request tracking and sends the close frame. Async work is exclusively in `onDisconnect`.

### What the current Rust code has and lacks

Current Rust state:

- `rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs:29-30` — `DisconnectCallback` is already typed as `BoxFuture<'static, Result<()>>`. Good — the type is right.
- `rivetkit-rust/packages/rivetkit-core/src/websocket.rs:10-17` — wire-level WebSocket callbacks are sync. This matches the TS pattern and is correct. **No change needed here.**

What's missing:

1. **A `pending_disconnect_count: AtomicUsize` on `ActorContext`** (or equivalent), incremented before the `DisconnectCallback` future is awaited and decremented after.
2. **A `CanSleep::ActiveDisconnectCallbacks` variant** (or equivalent gate) in `SleepController::can_sleep` / `wait_for_sleep_idle_window` that blocks while the count > 0.
3. **An RAII drop guard** (`DisconnectCallbackGuard`) that increments in `new()` and decrements in `Drop::drop`, so panics and error paths don't leak the count (pairs with the drop-guard pattern in complaint #17).
4. **Sleep-timer re-evaluation at boundaries** — the equivalent of `resetSleepTimer()` both before the callback runs and after it completes, so the sleep controller notices the counter change.

### Proposed shape

```rust
// On ActorContext (or flattened onto ActorContextInner per complaint #1):
pending_disconnect_count: AtomicUsize,

struct DisconnectCallbackGuard {
    count: Arc<AtomicUsize>,
    sleep_ctx: ActorContext,
}

impl DisconnectCallbackGuard {
    fn new(ctx: &ActorContext) -> Self {
        ctx.inner().pending_disconnect_count.fetch_add(1, SeqCst);
        ctx.reset_sleep_timer();
        Self { count: ctx.pending_disconnect_count_arc(), sleep_ctx: ctx.clone() }
    }
}

impl Drop for DisconnectCallbackGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, SeqCst);
        self.sleep_ctx.reset_sleep_timer();
    }
}

// At the disconnect call site:
async fn run_disconnect(ctx: ActorContext, callback: DisconnectCallback, conn_id: Option<String>) {
    let _guard = DisconnectCallbackGuard::new(&ctx);
    if let Err(error) = callback(conn_id).await {
        tracing::error!(?error, "disconnect callback failed");
    }
}
```

And in `SleepController::can_sleep`:

```rust
if ctx.pending_disconnect_count.load(SeqCst) > 0 {
    return CanSleep::ActiveDisconnectCallbacks;
}
```

### Why wire-level close callbacks stay sync

The earlier framing ("make all WebSocket callbacks async") was too broad. The TS parity target is:

- **Wire-level send / close / message-event callbacks** (`websocket.rs:10-17`): stay sync. Match TS.
- **User-facing `onDisconnect`** (through `DisconnectCallback` at `connection.rs:29-30`, already `BoxFuture`): must be awaited and sleep-gated. This is the real fix.

The confusion came from conflating the two layers. The wire-level callback is an envoy-client bookkeeping callback; the user-facing hook is separate and already async-shaped in the type, just not gated against sleep.

## 20. Audit counter-polling patterns across rivetkit-core, rivetkit-napi, rivetkit-sqlite

CLAUDE.md already has the rule: "Never poll a shared-state counter with `loop { if ready; sleep(Nms).await; }`. Pair the counter with a `tokio::sync::Notify` (or `watch::channel`) that every decrement-to-zero site pings, and wait with `AsyncCounter::wait_zero(deadline)` or an equivalent `notify.notified()` + re-check guard that arms the permit before the check."

But there's no audit on record that enforces it. Complaint #16 covers one specific SQLite test instance. No broader sweep has been done.

### Known candidates from prior audits

- **SQLite test-only `awaited_stage_responses: Mutex<usize>`** (`rivetkit-rust/packages/rivetkit-sqlite/src/v2/vfs.rs:551, 596-598`) — polled via a getter. Covered by #16; keep here as the canonical example.
- **SQLite test-only `mirror_commit_meta: Mutex<bool>`** (`v2/vfs.rs:679-680`) — gate check via polling. Covered by #16; should be `AtomicBool` paired with the existing `finalize_started` / `release_finalize` `Notify`.

### Broader audit scope

Systematically grep for these patterns and classify each:

1. **`loop { ... sleep(Duration::from_millis(N)).await; ... }`** where the loop body checks a shared counter, atomic flag, or map size.
2. **Polling getters called from test or production code** that return cached counter values alongside `tokio::time::sleep` for retries.
3. **Any `AtomicUsize` / `AtomicU32` / `AtomicU64` used as a "count of live things" that has an awaiter somewhere** — those need a paired `Notify` (or `watch::Sender`) that every decrement pings.
4. **`Mutex<usize>` / `Mutex<bool>` fields** — either upgrade to atomic, or wrap with `Notify` + atomic for the wait-for-zero pattern.

For each candidate found, classify as:

- **Event-driven already** (has a `Notify` / `watch::channel` / `oneshot` pair) — no change.
- **Polling** — convert to `AsyncCounter::wait_zero(deadline)` or equivalent `Notify`-paired atomic + re-check guard.
- **Monotonic sequence** (revision, epoch, ID) — not a candidate.

### Directories to sweep

- `/home/nathan/r6/rivetkit-rust/packages/rivetkit-core/src/`
- `/home/nathan/r6/rivetkit-rust/packages/rivetkit-sqlite/src/`
- `/home/nathan/r6/rivetkit-typescript/packages/rivetkit-napi/src/`

### Also codify the rule

- CLAUDE.md already has the rule. Add a supplementary rule: "For every shared counter that has an awaiter, the decrement-to-zero site must ping a paired `Notify` / `watch` / release-permit. Waiters must arm the permit before re-checking the counter (to avoid lost wakeups)."
- Add a clippy-style lint or review checklist item so this gets caught in review rather than re-emerging.

## 21. WebSocket close callbacks should be async to match prior TS behavior

`rivetkit-rust/packages/rivetkit-core/src/websocket.rs:10-17` defines all four WebSocket callbacks as sync closures returning `Result<()>`:

- `WebSocketSendCallback = Arc<dyn Fn(WsMessage) -> Result<()> + Send + Sync>`
- `WebSocketCloseCallback = Arc<dyn Fn(Option<u16>, Option<String>) -> Result<()> + Send + Sync>`
- `WebSocketMessageEventCallback = Arc<dyn Fn(WsMessage, Option<u16>) -> Result<()> + Send + Sync>`
- `WebSocketCloseEventCallback = Arc<dyn Fn(u16, String, bool) -> Result<()> + Send + Sync>`

This breaks parity with the TypeScript implementation, which allowed async work in WebSocket cleanup so cleanup gated sleep — the actor would not be allowed to sleep while close handlers were still running.

Inconsistency in the current Rust code itself: `rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs:29-30` defines `DisconnectCallback` as `BoxFuture<'static, Result<()>>` — async — for the connection-level disconnect path. So at the connection layer, async cleanup is supported. At the WebSocket layer it's not. There's no architectural reason for the asymmetry.

Also relevant: CLAUDE.md guidance "rivetkit-core sleep readiness should stay centralized in `SleepController`, with queue waits, scheduled internal work, disconnect callbacks, and websocket callbacks reporting activity through `ActorContext` hooks so the idle timer stays accurate." That requirement is hard to meet with sync close callbacks — there's no future to await against, no point at which the close handler's work has "completed" from the sleep controller's perspective.

Fix: change the close callbacks to async (`BoxFuture<'static, Result<()>>`), consistent with `DisconnectCallback` and the broader CLAUDE.md guidance "rivetkit-core boxed callback APIs should use `futures::future::BoxFuture<'static, ...>`":

```rust
pub(crate) type WebSocketCloseCallback =
    Arc<dyn Fn(Option<u16>, Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub(crate) type WebSocketCloseEventCallback =
    Arc<dyn Fn(u16, String, bool) -> BoxFuture<'static, Result<()>> + Send + Sync>;
```

Wire each invocation through a `WebSocketCallbackGuard` (already exists at `actor/context.rs`) so the in-flight close work counts toward sleep readiness — the actor stays awake until cleanup completes, matching the prior TS contract.

Send and message-event callbacks could stay sync if they're truly fire-and-forget on the network path, but if they ever need async cleanup (e.g., persist hibernation state on send), they should also become async for consistency.

### Non-standard async `close` event handler support

The user-facing API supports `ws.addEventListener('close', async (event) => { ... })` or `ws.onclose = async (event) => { ... }`. This is deliberately non-standard — the browser `WebSocket` spec treats `onclose` as a fire-and-forget event listener — but Rivet actors need to allow async work (persist state, release resources, send final acks on a sibling conn) inside the close handler and gate sleep until that work completes.

`WebSocketCloseEventCallback` at `websocket.rs:13` is the type carrying that user-facing handler through to core. It must be async (`BoxFuture<'static, Result<()>>`) AND its in-flight invocations must count toward sleep readiness via `WebSocketCallbackGuard`. Same shape as `DisconnectCallback` (complaint #19) but for the WebSocket-event path.

### May need a new sleep state to represent "waiting on close handlers"

Current `SleepController` states (per `actor/sleep.rs`): `Idle`, `Armed`, `Grace`, `Finalize`, probably a couple more. None explicitly represent "blocked on in-flight user close-handler futures."

Options:

- **(a) Reuse the existing activity counter**: treat every outstanding close handler as activity, sleep controller already knows how to wait for activity counters to reach zero (similar to `pending_disconnect_count` in complaint #19 and `active_queue_wait_count`). Cleanest if the existing plumbing generalizes. Likely sufficient.
- **(b) New `CanSleep::ActiveCloseHandlers` variant** like complaint #19's `ActiveDisconnectCallbacks` — explicit per-kind reporting for debuggability and metrics.
- **(c) New `LifecycleState` variant (`SleepWaitingOnCloseHandlers`)** — only justified if the state transition logic needs to branch differently (e.g., cancel vs. wait behavior on `Stop` arriving mid-close-handler). Avoid unless the behavior genuinely differs from existing sleep-grace semantics.

Decision deferred until the implementation surfaces a concrete reason for the sleep-state split. Default: start with (a) + (b), promote to (c) only if necessary.

Pairs tightly with complaint #19 (`pending_disconnect_count`) — probably the same underlying atomic counter plus `Notify` pair, differentiated only by metric labels.

## 22. Alarm-during-sleep wake path is broken and blocking the driver test suite

The driver test suite is blocked on a single root cause: when an actor goes to sleep with a scheduled alarm pending, the alarm never fires to wake it back up. HTTP-triggered wakes work, but alarm-triggered wakes do not.

Tracked in `.agent/todo/alarm-during-destroy.md`. Manifests as at least three failing driver tests as of 2026-04-22:

- `actor-sleep-db` (2 of 14 fail): `scheduled alarm can use c.db after sleep-wake` (`actor_ready_timeout`), `schedule.after in onSleep persists and fires on wake` (timeout).
- `actor-conn-hibernation` (4 of 5 fail): `basic conn hibernation`, `conn state persists through hibernation`, `onOpen is not emitted again after hibernation wake`, `messages sent on a hibernating connection during onSleep resolve after wake`. All 30s timeouts. Likely same root cause via the hibernation-wake path that also depends on driver alarms.
- `actor-sleep` `alarms wake actors` is intermittent on this branch and hits the same window under load.

Mechanism:

- `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs::finish_shutdown_cleanup_with_ctx` unconditionally calls `cancel_driver_alarm_logged` before teardown. This matches the TS reference behavior at ref `feat/sqlite-vfs-v2`.
- The TS ref comment on that path says alarms are re-armed via `initializeAlarms` on wake. The Rust side does the equivalent via `init_alarms` → `sync_future_alarm_logged` during startup.
- But alarm-triggered wake from the engine never happens on the sleeping actor's behalf because the engine's driver alarm was cleared at sleep, and the actor isn't awake to re-arm it until something else (HTTP) wakes it. So `schedule.after` timers that should fire during sleep silently die.
- A naive fix (skip the cancel on `Sleep`, keep it on `Destroy`) causes alarm + HTTP wake races and does not safely land without coordination with the sleep finalize path.

Blocker ownership:

- The merge-readiness of this branch is gated on this bug. All three failing test files are symptoms of the same underlying issue; a correct fix should light them up together.
- Paired with complaint #6 (engine `set_alarm` dedup): the engine already stores `alarm_ts` durably, so the correct design probably keeps the engine alarm armed across sleep and lets the engine's alarm dispatch re-wake the actor on its own timer, with the actor side re-syncing alarm state on wake rather than on sleep. Needs design coordination before implementation.

Pairs with `.agent/notes/driver-test-progress.md` which tracks the full test matrix. If this is fixed, expect the fast-tier slot (`actor-db-pragma-migration`, `gateway-query-url`, `actor-inspector` re-check) and the remaining slow-tier runs (`actor-run`, `hibernatable-websocket-protocol`, `actor-db-stress`) to run as a single pass afterward.
