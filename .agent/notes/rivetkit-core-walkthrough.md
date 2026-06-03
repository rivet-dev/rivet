# rivetkit-core: A Lifecycle and Runtime Walkthrough

A chapter-based technical walkthrough of `rivetkit-rust/packages/rivetkit-core/` and the adjacent layers that make up a live Rivet Actor: the state machine, the event loop, persistence, transport, and the engine bridge.

---

## Chapter 1 — The Cast

`rivetkit-core` is the language-agnostic heart of every Rivet Actor. All load-bearing lifecycle logic lives here; the TypeScript and NAPI layers above it only translate types.

The central types:

- **`ActorTask`** (`actor/task.rs`, ~1400 lines) — the event loop. One per actor. Owns the state machine.
- **`ActorContext`** (`actor/context.rs`) — the surface the user's driver code sees: `set_state`, `mutate_state`, `request_save`, `schedule()`, `queue()`, broadcast, etc.
- **`ActorState`** (`actor/state.rs`) — in-memory state plus the serialization pipeline into KV.
- **`SleepController`** (`actor/sleep.rs`) — idle tracking and the `can_sleep` decision.
- **`ConnectionManager`** (`actor/connection.rs`) — live and hibernatable WebSocket bookkeeping.
- **`Queue`** (`actor/queue.rs`) — durable FIFO queue with receive waits.
- **`Schedule`** (`actor/schedule.rs`) — alarms.
- **`RegistryDispatcher`** (`registry.rs`) — the bridge between Envoy and a fleet of `ActorTask`s.

The state machine itself (`actor/task_types.rs:5-17`):

```rust
pub enum LifecycleState {
    Loading, Migrating, Waking, Ready,
    Started, SleepGrace, SleepFinalize,
    Destroying, Terminated,
}
```

---

## Chapter 2 — Birth

An actor starts when Envoy sends a `StartActorRequest`. `RegistryDispatcher::start_actor` (`registry.rs:380`) handles it in three steps.

**Build the runtime.** `ActorContext::new_runtime(...)` assembles everything the actor will need: `ActorState` with its KV persist key at `[1]`, a `Queue` with metadata at `[5,1,1]` and messages at `[5,1,2]+id`, a `ConnectionManager` with hibernatable connections under prefix `[2]+conn_id`, a `Schedule`, a `SleepController`, and metrics/diagnostics hooks.

**Wire the channels.** Three bounded MPSC channels connect `ActorTask` to the outside world:

- `lifecycle_inbox` — one-shot commands (`Start`, `Stop`, `FireAlarm`).
- `dispatch_inbox` — work (`Action`, `Http`, `OpenWebSocket`, `WorkflowHistory/Replay`).
- `lifecycle_events` — internal self-notifications (`StateMutated`, `SaveRequested`, `SleepTick`, `ActivityDirty`, inspector events).

All producers use `try_reserve`, not `.await` on `.send()`. A full inbox returns `actor/overloaded`.

**Spawn.** `ActorTask::new` starts in `LifecycleState::Loading`. The task is spawned on Tokio. The dispatcher sends `LifecycleCommand::Start`.

---

## Chapter 3 — Startup

`start_actor` (`task.rs:530-567`) runs in a fixed order — deviating from it corrupts resume.

1. Load `PersistedActor` from KV `[1]`. Payloads carry a 2-byte little-endian vbare version prefix before the BARE body; actor blobs are version 4.
2. Persist `has_initialized = true` immediately (so a crash during first-start doesn't re-run init on resume).
3. Resync alarms: walk `PersistedActor.scheduled_events`, find the earliest, call `set_alarm(timestamp_ms)` to notify Envoy.
4. Restore hibernatable connections: scan KV `[2]` prefix, check each against the gateway, drop the dead ones.
5. Transition to `Ready`.
6. Spawn the driver. The factory is invoked with an `ActorStart` packet (ctx, input, state snapshot, hibernated connections, event receiver) inside a detached, panic-catching Tokio task.
7. Transition to `Started`.
8. Reset the sleep deadline; drain overdue scheduled events so missed alarms fire immediately.

---

## Chapter 4 — The Event Loop

`ActorTask::run()` (`task.rs:256-320`) is one `tokio::select!` multiplexing five sources:

1. Lifecycle commands.
2. Lifecycle events.
3. Dispatch commands — gated by `accepting_dispatch()`, which returns true only in `Started` and `SleepGrace`.
4. The run-handle outcome — resolves when the driver task finishes.
5. Timers — `state_save_deadline`, `inspector_serialize_deadline`, `sleep_deadline`.

Each dispatch kind spawns a tracked child task, categorized by `UserTaskKind` (`task_types.rs:34-70`): `Action`, `Http`, `WebSocketLifetime`, `WebSocketCallback`, `QueueWait`, `ScheduledAction`, `DisconnectCallback`, `WaitUntil`. The kind is used for metrics and for knowing what to wait for on shutdown.

Two design points worth noting:

- Action children run concurrently. There is no per-actor action lock, because long-running actions must coexist with `unblock`/`finish` actions.
- Alarm-originated actions dispatch with `conn: None`. Do not synthesize a placeholder connection for scheduled actions.

---

## Chapter 5 — State and Persistence

User state changes flow through `ActorContext::set_state` / `mutate_state` (`state.rs:132-174`):

1. Reentrancy check — mutating state from inside `on_state_change` returns `actor/state_mutation_reentrant`.
2. Update in-memory state, mark dirty.
3. Emit `LifecycleEvent::StateMutated { reason }`.
4. The loop resets the sleep deadline (activity).

The `StateMutationReason` variants (`task_types.rs:72-102`) are tagged for metrics: `UserSetState`, `UserMutateState`, `InternalReplace`, `ScheduledEventsUpdate`, `InputSet`, `HasInitialized`.

Saves are throttled. `request_save(immediate)` sets a flag and bumps `save_request_revision`. The loop arms `state_save_deadline` — either `now` or `now + state_save_interval` (default 1s). When it fires, the loop sends `ActorEvent::SerializeState { reason: Save, reply }` to the driver, gets back `Vec<StateDelta>`, and applies each:

- `ActorState(bytes)` — write to KV `[1]` with the version prefix.
- `ConnHibernation { conn, bytes }` — write to KV `[2] + conn_id`.
- `ConnHibernationRemoved(conn)` — delete.

A `save_guard` serializes writes. `finish_save_request(revision)` clears the pending-save flag; several shutdown paths depend on "no pending save" to proceed.

---

## Chapter 6 — Schedule and Queue

`ctx.schedule().after(duration, action, args)` or `.at(timestamp_ms, ...)` (`schedule.rs`) append to `PersistedActor.scheduled_events`, mutate state with reason `ScheduledEventsUpdate`, kick an immediate save, and resync the alarm.

When Envoy fires the alarm, it sends `LifecycleCommand::FireAlarm`. The task calls `drain_overdue_scheduled_events` and dispatches each due event as `ActorEvent::Action { name, args, conn: None }`.

The queue (`queue.rs`) persists metadata at `[5,1,1]` and messages at `[5,1,2] + u64be(id)`, so prefix scans come back in FIFO order. Receive waits observe the `ActorContext`-owned abort `CancellationToken` and are cancelled by `mark_destroy_requested`. `enqueue_and_wait` completion waits deliberately do *not* observe actor abort — they rely on the tracked user task for shutdown cancellation.

---

## Chapter 7 — Inspector

When an inspector attaches, `inspector_attach_count` increments. Every 50ms, if the count is non-zero and there's dirty state, the loop fires `ActorEvent::SerializeState { reason: Inspector }` and broadcasts deltas through `InspectorSignal` subscriptions. Inbound frames accept wire versions v1-v4; outbound stays on v4. Unsupported features downgrade to explicit `Error` messages (`inspector.*_dropped` codes), never silent drops.

---

## Chapter 8 — Sleep (Two Phases)

Sleep is triggered by the idle timer. When `sleep_deadline` fires, `SleepController::can_sleep` checks: not-ready, `prevent_sleep`, `no_sleep` config, active HTTP, keep-awake regions, non-hibernatable connections, or outstanding WebSocket callbacks all block it.

### Phase 1: SleepGrace

Dispatch is still accepted — this is the crucial design point. Late work arriving during grace must run, not be dropped.

- Drain `dispatch_inbox`.
- Send `ActorEvent::BeginSleep` so the driver can start wrapping up.
- Call `wait_for_sleep_idle_window` with deadline `now + effective_sleep_grace_period()`. The grace period defaults to 15s; if `sleep_grace_period_overridden` is false, it's derived from `on_sleep_timeout + wait_until_timeout` for back-compat (`config.rs:235-262`).
- Save timers and alarm dispatch stay live.

### Phase 2: SleepFinalize

Point of no return. Dispatch closed, alarms suspended.

1. Send `ActorEvent::FinalizeSleep`.
2. Send `ActorEvent::SerializeState { reason: Save }`, apply the final deltas.
3. Drain `before_disconnect` tracked work.
4. Persist hibernatable connections first; then disconnect non-hibernatable.
5. Drain `after_disconnect` tracked work.
6. Wait for the driver's run-handle with the remaining grace budget; abort on timeout.
7. Run the shared stop sequence. Transition to `Terminated`.

---

## Chapter 9 — Destroy

Destroy skips grace entirely (`task.rs:895-966`).

- Transition straight to `Destroying`.
- Mark every hibernatable connection for transport removal so they won't resume.
- Send `ActorEvent::Destroy`.
- Persist final state.
- Drain `before_disconnect`.
- Disconnect every connection (`preserve_hibernatable=false`).
- Drain `after_disconnect`.
- Wait for the driver with `effective_on_destroy_timeout()` (default 5s) — a separate budget from sleep grace. Abort on timeout.
- Run the shared stop sequence.

---

## Chapter 10 — The Stop Sequence

Every death path ends with the same cleanup, in this exact order:

1. Immediate state save.
2. Wait for pending state writes.
3. Wait for pending alarm writes.
4. SQLite cleanup.
5. Cancel the driver alarm.

If the driver exits on its own, `handle_run_handle_outcome` inspects flags: `destroy_requested` → destroy path; `sleep_requested` → sleep path; otherwise clean exit → terminated via stop sequence directly.

---

## Chapter 11 — Errors at the Boundary

- Internal code returns `anyhow::Result`.
- `rivetkit-core` extracts structured `RivetError`s (`group/code/message/metadata`) at boundaries with `rivet_error::RivetError::extract`.
- HTTP dispatch errors become 500 responses.
- WebSocket open errors become logged 1011 closes.
- Channel saturation returns `actor/overloaded`.
- State mutation from inside `on_state_change` returns `actor/state_mutation_reentrant`.
- Optional chaining is banned on required lifecycle and bridge paths (sleep, destroy, alarm dispatch, ack, websocket dispatch). If a capability is required, validate and throw; don't return `None`.

---

## Chapter 12 — The State Machine, Compressed

```
Loading ──Start──▶ Ready ──spawn driver──▶ Started
                                              │
     ┌────────────────────────────────────────┤
     │                                        │
     │ idle timer + can_sleep                 │ Destroy command
     ▼                                        ▼
  SleepGrace ─── grace window closes ──▶ Destroying
     │                                        │
     ▼                                        │
  SleepFinalize ──── stop sequence ───────────┤
                                              ▼
                                          Terminated
```

The event loop is the only authority over this machine. `ActorContext` never mutates lifecycle directly — it emits `LifecycleEvent`s that the loop consumes. That single-writer invariant is what makes the whole thing safe.

---

## Chapter 13 — KV: The Byte Store Underneath

Everything persistent in an actor lands in a single hierarchical byte-key/byte-value store. `Kv` (`src/kv.rs`) is the wrapper; the backend is pluggable (`KvBackend::Envoy` in production, `InMemory` for tests, `Unconfigured` for early contexts).

The public surface is compact:

- `get`, `put`, `delete`, `delete_range`
- `list_prefix`, `list_range`
- `batch_get`, `batch_put`, `batch_delete`, `apply_batch`

Single calls delegate to batch calls (`get` → `batch_get(&[key])`). Batching is logical, not transport-level: one batch call is one wire request.

Every operation is actor-scoped at the engine. envoy-client bakes the actor_id into the request, so Kv itself never sees cross-actor traffic. Requests carry a u32 request id; responses match back to the waiter. If no response arrives within `KV_EXPIRE_MS` (30s, `envoy-client/src/kv.rs`), the waiter is dropped with an error.

Internal key conventions (rivetkit-core reserves these):

| Prefix                    | Owner                              |
|---------------------------|------------------------------------|
| `[1]`                     | Persisted actor blob               |
| `[2] + conn_id`           | Hibernatable connection payload    |
| `[5, 1, 1]`               | Queue metadata                     |
| `[5, 1, 2] + u64be(id)`   | Queue message body                 |
| `[0x08, 0x01, ...]`       | SQLite VFS (see next chapter)      |

There is no user/internal split at the API level — `ctx.kv()` can touch any key. Writing into these prefixes from user code corrupts the actor. The convention is enforced by naming, not by the type system.

All internal payloads use the vbare "embedded version" format: a 2-byte little-endian u16 version prefix followed by the BARE body. Actor blobs are version 4, connection blobs are version 3. The TypeScript runtime uses the same layout, which is what makes a Rust-hosted actor's KV directly readable by a TypeScript resume and vice versa.

---

## Chapter 14 — SQLite: A Relational Database Bolted onto KV

SQLite support is feature-gated. The stack has two halves.

**Core half — `src/sqlite.rs`.** `SqliteDb` is a thin gateway: `open(preloaded_entries)`, `query`, `run`, `exec`, plus the internal protocol hooks (`get_pages`, `commit`, `commit_stage_begin`, `commit_stage`, `commit_finalize`). It holds an `Arc<Mutex<Option<NativeDatabaseHandle>>>` that's lazily populated on first use. All query execution runs inside `spawn_blocking` — libsqlite3-sys is strictly synchronous.

**Native VFS — `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs` + `kv.rs`.** The VFS is the shim between SQLite's sync C callbacks and the async KV transport. Pages are stored as 4 KiB chunks under the `0x08` subspace:

```
meta:  [0x08, 0x01, 0x00, file_tag]
chunk: [0x08, 0x01, 0x01, file_tag, u32be(chunk_index)]
```

File tags: `0x00` main DB, `0x01` rollback journal, `0x02` WAL, `0x03` SHM. The `u32be` suffix keeps `BTreeMap`/`scan_prefix` ordering numerically correct. Truncation uses `delete_range` with a 4-byte sentinel end key (`[0x08, 0x01, 0x01, file_tag+1]`) to blow away every chunk for a file in one operation.

**v2 slow path.** Large commits that don't fit the one-shot path encode delta blocks as full LTX v3 frames and stuff them directly under the DELTA chunk keys. There is no `/STAGE` prefix, no fixed one-chunk-per-page mapping. A chunk key may contain a raw 4 KiB page *or* an LTX frame; the v3 decoder handles both.

**Native-only.** The VFS lives only in `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs`. There is no runtime WASM/TypeScript VFS — the `@rivetkit/sqlite-wasm` npm package is deprecated and the Rust crate is statically linked into `@rivetkit/rivetkit-napi` via `libsqlite3-sys`. Per-VFS rules (4 KiB chunks, `journal_mode=DELETE`, `locking_mode=EXCLUSIVE`, `auto_vacuum=NONE`) live in this one source.

---

## Chapter 15 — The Queue

`actor/queue.rs` implements a durable FIFO with optional synchronous completion responses. It's the most feature-dense primitive in the crate.

**Enqueue (`send`).** Validate size against config. Serialize with embedded-version encoding. Hold the metadata lock, increment `next_id`, increment `size`, construct new metadata. Atomic batch_put: message at `[5,1,2] + u64be(id)` plus metadata at `[5,1,1]`. If the caller passed a completion waiter, register it in an `scc::HashMap` keyed by id. Fire waiter `Notify` and inspector hooks.

**Receive (`next`, `next_batch`).** First try `try_receive_batch` — if anything is already durable, return immediately. Otherwise enter the wait loop wrapped in an `ActiveQueueWaitGuard` (so user-task metrics see it and sleep activity gets pinged). The wait races three futures: the actor's abort `CancellationToken`, an external cancel signal, and a `Notify` pinged by `send`. Timeout returns empty; abort returns an error; notify loops and retries.

**`enqueue_and_wait`.** Same enqueue path, but installs a completion waiter and then waits for the response. This wait **deliberately ignores the actor abort token** (`queue.rs` line ~891). Only the external cancel and timeout can end it. Reason: the completion might legitimately arrive after the actor has slept and re-woken. The owning user task is responsible for its lifecycle, not the actor core. Changing this breaks the hibernation story.

**FIFO comes from the keys.** Messages are keyed by monotonic `u64` ids encoded big-endian, so any prefix scan over `[5,1,2]*` reads them back in insertion order. Metadata (`{ next_id, size }`) sits at `[5,1,1]`. On startup, metadata is loaded from KV; if it's missing or fails to decode, the queue rebuilds by full-scanning `[5,1,2]*`. Slow but safe.

**Cancellation wiring.** `Queue::new(...)` takes the `ActorContext`-owned abort token. `ActorContext::mark_destroy_requested` cancels it. Receive waits observe this; `enqueue_and_wait` completion waits don't. External JS-side cancel signals (from NAPI, via an explicit standalone `CancellationToken`) ride on the side for non-idempotent waits.

**Sleep integration.** The `ActiveQueueWaitGuard` fires the `wait_activity` hook. As long as any user task is parked in `queue.next()`, the sleep controller treats the actor as active.

**Inspector.** `notify_inspector_update` fires after every enqueue/dequeue. The registered callback is set during actor startup and drives live queue-depth counters in debug sessions. Native inspector reads go through `ctx.inspectorSnapshot().queueSize`, not a TS-side cache.

---

## Chapter 16 — HTTP (`onRequest`)

Raw HTTP is orthogonal to the actor event flow in an important way: size limits.

**Flow.** Client → engine gateway → `pegboard-envoy` → `envoy-client` → `RegistryDispatcher::handle_fetch` in the native registry (`src/registry.rs`) → `DispatchCommand::Http { request, reply }` → `ActorEvent::HttpRequest` → user callback → `Response` → reply.

The child task is tracked as `UserTaskKind::Http`. In-flight HTTP increments an `Arc<AsyncCounter>`; sleep readiness reads it. As long as the counter is non-zero, the actor stays awake.

**Size-limit bypass.** Raw `onRequest` deliberately bypasses `maxIncomingMessageSize` and `maxOutgoingMessageSize`. Those guards live only on `/action/*` and `/queue/*` message routes in the TypeScript registry (`rivetkit-typescript/packages/rivetkit/src/registry/native.ts`), not in `RegistryDispatcher::handle_fetch`. This is intentional: file uploads and large responses should work on raw HTTP.

**Error boundary.** Errors thrown from the user callback are logged and turned into HTTP 500 responses with error details in JSON. Panics in the callback are caught by the panic wrapper on the spawned child. The gateway connection never sees a Rust-level failure.

**Actor-scoped fetch tracking.** `envoy-client` holds a `JoinSet` plus `Arc<AtomicUsize>` of in-flight fetches per actor. Sleep checks read the counter; shutdown aborts and joins the set before sending `Stopped`. This is the primitive that prevents the engine from cutting a request mid-flight.

**Sleep-timer landmine.** Static native actor HTTP requests go through `RegistryDispatcher::handle_fetch`, **not** through `actor/event.rs`. If you're fixing a sleep-timer bug around HTTP request lifecycle, you need to patch `src/registry.rs` as well as the lower-level staging helpers — there are two entry points and they both matter.

---

## Chapter 17 — WebSockets

Two flavors, same transport: ephemeral (raw) and durable (hibernatable).

### Raw WebSocket

`DispatchCommand::OpenWebSocket { ws, request, reply }` enters the event loop. The task spawns a `UserTaskKind::WebSocketLifetime` child and sends `ActorEvent::WebSocketOpen` to the driver with a `WebSocket` wrapper (`src/websocket.rs`) and the HTTP upgrade request.

The driver attaches message and close callbacks on the `WebSocket` and replies `Ok(())`. Messages and closes dispatch **inline, under the WebSocket callback guard** — not as separate events through the dispatch channel. The callback guard is what pings sleep activity and prevents the actor from hibernating mid-callback.

Transport errors become logged 1011 "server error" closes. Failed sends are logged and not propagated — user code sees a reply or a close, never a transport-level panic.

### Hibernatable WebSocket

Hibernatable connections are the interesting ones: they outlive the actor's awake period. On disconnect during sleep, their metadata gets persisted; on wake, they're rehydrated and the `on_connect` handler runs again with the restored state.

**Persistence.** `PersistedConnection` carries `gateway_id`, `request_id`, `server_message_index`, `client_message_index`, `request_path`, `request_headers`, plus user-defined state. It's encoded with the vbare 2-byte version prefix (version 3) and written at `[2] + conn_id`. The field order matches TypeScript's BARE v4 schema so both runtimes read the same bytes.

**Wake-time rehydration.** At startup (`start_actor`), after loading `PersistedActor`, the task scans KV under `[2]`, decodes each payload, and settles the connections. Each one is validated against the gateway: `envoy-client`'s `SharedContext.actors` mirror knows which tunnel requests are live and which restores are pending. Dead connections are removed.

**State-only save path.** During normal runtime, hibernatable connection state is saved via `StateDelta::ConnHibernation { conn, bytes }` and `ConnHibernationRemoved(conn)`. These ride the same `SerializeState { reason: Save }` tick as the main actor state, so connection state and actor state stay in sync.

**Sweeping disconnects.** The bulk disconnect helper must sweep every matching connection, remove successful disconnects, update connection/sleep bookkeeping, and *only then* aggregate per-connection failures into the returned error. Don't short-circuit on the first failure — half-torn-down connections leak.

---

## Chapter 18 — The Connection Manager

`ConnectionManager` in `actor/connection.rs` holds the three connection kinds under one roof: stateless action-driven calls, raw WebSockets, and hibernatable WebSockets. They share:

- A `ConnId` (UUID).
- Optional parameters.
- Mutable state behind an `RwLock`.
- A subscription set (`BTreeSet<String>`) that `EventBroadcaster` reads to filter broadcasts.
- Registered event and disconnect callbacks.

Hibernatable connections additionally carry the `PersistedConnection` metadata. A raw connection has no persisted footprint.

**Connection state vs hibernation blob.** The connection's user-state (settable via `conn.state()` setters) is part of the hibernation blob. At wake time, the typed `Start<A>` wrapper in the Rust high-level crate is responsible for rehydrating each `ActorStart.hibernated` state blob back onto its `ConnHandle` before exposing `ConnCtx`. If that step is skipped, `conn.state()` silently diverges from the wake snapshot.

**Sleep interactions.** Active non-hibernatable connections block sleep (`can_sleep` returns No). Pending disconnect callbacks block finalize until drained. Hibernatable connections do not block — they're persisted and then disconnected during finalize.

---

## Chapter 19 — envoy-client, the Wire Bridge

`engine/sdks/rust/envoy-client/` is the transport layer between an actor's `rivetkit-core` and the engine.

**`EnvoyHandle`** is the actor-side API — the struct the registry hands into `ActorContext::new_runtime` and that the KV/SQLite layers call against.

**`SharedContext`** is the in-process mirror of engine state. Key maps:

- `actors: Mutex<HashMap<actor_id, HashMap<generation, ActorInfo>>>` — the live actor inventory. Sync lookups for actor state (e.g., during hibernatable-connection validation) read this directly. Blocking back through the envoy task would panic on current-thread Tokio runtimes, so the mirror is load-bearing, not just a cache.
- `live_tunnel_requests` — active WebSocket tunnels at the gateway.
- `pending_hibernation_restores` — connections waiting to resume.

**Wire format.** BARE serialization end-to-end. Messages are defined in `.bare` schemas and compiled into Rust types. KV and SQLite each have their own request/response unions.

**Request/response matching.** Every KV or SQLite call gets a u32 request id. The envoy-client holds a map of pending waiters keyed by request id; responses are matched back and delivered over oneshot channels. Stale requests expire after `KV_EXPIRE_MS` (30s) via a periodic cleanup task.

**Fetches as `JoinSet` + counter.** Actor-scoped HTTP fetches from inside the actor go through a per-actor `JoinSet` plus `Arc<AtomicUsize>`. Sleep reads the counter; shutdown aborts and joins the set before signaling `Stopped`.

**Graceful teardown.** `EnvoyCallbacks::on_actor_stop_with_completion` is the preferred shutdown path — the callback gets a completion handle so it can signal "I'm ready to stop" after persisting final state. The default implementation auto-completes immediately for back-compat with the older `on_actor_stop` shape.

**Never modify a published `*.bare` protocol version.** Add a new version and bridge forward through `versioned.rs`. Bumping protocol requires updating both `PROTOCOL_MK2_VERSION` (`engine/packages/runner-protocol/src/lib.rs`) and `PROTOCOL_VERSION` (`rivetkit-typescript/packages/engine-runner/src/mod.ts`) in the same change — the two must match the latest schema version.

---

## Chapter 20 — The Inspector, Fully Wired

The inspector is a live window into everything discussed so far.

**Atomic counters.** `Inspector` holds `queue_size`, `state_revision`, `connections_revision`, and a set of `InspectorSignal` broadcast channels. Mutations on each primitive bump the relevant counter and fire the matching signal.

**Live snapshots.** `ctx.inspectorSnapshot()` reads queue size, state revision, and connection count directly from atomics — never from a TS-side cache. The native queue-size hook is the authoritative source; the old pattern of hardcoding fallback values or caching in TS was wrong and has been ripped out.

**Two transports.**

- **WebSocket inspector** — long-lived subscription. Inbound frames accept wire versions v1-v4; outbound always uses v4. Unsupported-feature downgrades produce explicit `Error` messages with `inspector.*_dropped` codes — never silent drops.
- **HTTP inspector** — snapshot endpoints on `actor/router.ts` (TypeScript side), mirroring the WebSocket payloads for agent-based debugging. When you add or modify an inspector endpoint, update both transports *and* the docs at `website/src/metadata/skill-base-rivetkit.md` and `website/src/content/docs/actors/debugging.mdx`.

**Serialize pathway.** Every 50ms, if an inspector is attached and there's dirty state, the event loop fires `ActorEvent::SerializeState { reason: Inspector }`. The driver returns `Vec<StateDelta>`. User state and hibernatable connection deltas are included verbatim. Queue and connection live counts come off the atomic snapshot, not off these deltas — the deltas describe what changed, the snapshot describes where we are.

**Workflow support.** Workflow inspector availability is inferred from mailbox replies. `actor/dropped_reply` on a workflow request means the driver doesn't support workflows; the inspector falls back gracefully. There's no standalone "workflow enabled" boolean — the absence of support is signaled by the reply, not by a flag.

---

## Chapter 21 — Invariants Worth Tattooing

Consolidated from the above:

1. KV internal prefixes (`[1]`, `[2]*`, `[5]*`, `[0x08]*`) are reserved. No runtime enforcement. User writes into them corrupt the actor.
2. `enqueue_and_wait` completion waits ignore the actor abort token. Breaking this breaks hibernation.
3. Queue metadata rebuilds by full-scan on decode failure. Slow, safe, never lose messages.
4. SQLite VFS is native-only (`rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs`). Chunk size, key layout, PRAGMAs, truncate strategy, journal mode all live here.
5. Raw `onRequest` HTTP bypasses message-size limits. Action and queue routes do not.
6. Static native actor HTTP flows through `RegistryDispatcher::handle_fetch`, not `actor/event.rs`. Sleep-timer fixes need both entry points.
7. WebSocket message/close callbacks run inline under the callback guard, not as dispatch events.
8. Hibernatable connection state must be rehydrated onto `ConnHandle`s before `ConnCtx` is exposed at wake, or `conn.state()` desyncs.
9. Sleep readiness lives in `SleepController`, pinged via `ActorContext` hooks. Queue waits, scheduled work, disconnect callbacks, WebSocket callbacks all report through this one channel.
10. HTTP errors become 500s. WebSocket errors become logged 1011s. Actor code never sees transport failures.
11. Never modify a published `*.bare` protocol version in place. Version up and bridge.

That's the full stack: the state machine sitting on top of KV, SQLite, the queue, HTTP, WebSockets, the connection manager, and the envoy-client bridge — with the inspector watching it all from above.
