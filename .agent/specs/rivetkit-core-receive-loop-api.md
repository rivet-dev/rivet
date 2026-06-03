# rivetkit-core Receive-Loop Actor API

Status: **DRAFT — design proposal, not accepted, not implemented.**

Scope:

- Rewrites the public actor-authoring API of `rivetkit-core` from a callback table (`ActorInstanceCallbacks`) to a single receive-loop entry function that pulls `ActorEvent`s from a mailbox.
- Internal machinery (`ActorTask`, lifecycle state, `ActorContext`, KV/SQLite/queue/schedule subsystems, wire protocol, persistence layout) is unchanged. Only the factory surface and the actor-author-facing types change.
- Does **not** touch `rivetkit-typescript/packages/rivetkit-napi/`, `rivetkit-typescript/packages/rivetkit/`, or the Rust `rivetkit` typed wrapper crate. Any typed ergonomics layer on top of this API lives outside core and is out of scope for this spec.

## Goals

- One construct per actor (an `async fn`) instead of a table of optional `Fn` closures.
- Actor owns its state as ordinary local variables / struct fields. No `ctx.state()` / `ctx.set_state(bytes)` round trips for user-level state; no implicit `on_state_change` notification.
- Dynamic dispatch on action names stays — no procedural macros, no derives, no compile-time action enums. Actions arrive as `{ name: String, args: Vec<u8> }` and the actor matches on `name.as_str()`.
- Events are only things the runtime must drive into the actor. Things the actor can pull on its own schedule (queue reads, KV lookups, SQLite queries, scheduled-event registration) stay on `ActorContext`.

## Non-Goals

- No wire protocol changes.
- No KV, queue, SQLite, schedule, inspector, or persisted snapshot layout changes.
- No changes to `ActorContext` public surface except where a callback-only helper becomes redundant.
- No derive macros, proc macros, or typed action enums in `rivetkit-core`.
- No typed wrapper in core. If a higher layer wants `#[derive(Action)]`, it lives in a separate crate on top of this API.
- No changes to `ActorConfig`, `ActorFactory::config()`, or the engine process management in `CoreRegistry::serve()`.

## Motivation

The current `ActorInstanceCallbacks` model has five structural problems:

1. **State ownership is inverted**: state lives in `ActorContext` as opaque `Vec<u8>`. Every read is `ctx.state()`, every write is `ctx.set_state(bytes)`, and every external observer hooks `on_state_change`. Actor authors write serialization code in three places.
2. **Fifteen optional `Fn` closures per actor**: `ActorInstanceCallbacks` has 15 `Option<Callback>` fields. Most actors populate 2–4. The runtime must null-check every one on every event.
3. **`run` races the rest of the lifecycle**: the `run` callback is a long-running future supervised in `ActorTask.children`, separate from all other callbacks, with its own abort-and-restart machinery (`restart_run_handler`, `run_handler_abort`, `set_run_handler_active`). This is a large fraction of `ActorTask`'s complexity.
4. **Two-phase connect is historical**: `on_before_connect` + `on_connect` existed because the old API needed somewhere to produce `ConnState`. In a receive-loop model the actor owns per-conn data in its own fields; the split loses its reason to exist.
5. **`on_before_action_response` is a leaky abstraction**: it exists because action handlers returned opaque bytes the actor layer wanted to post-process. If the actor *is* the handler, post-processing happens at the call site.

## Proposal

### Public surface (all in `rivetkit-core`)

```rust
use anyhow::Result;
use futures::future::BoxFuture;
use tokio::sync::{mpsc, oneshot};

/// Bundle of arguments passed to an actor instance's entry function.
/// Given to the actor once per instance (create, wake, or migrate).
pub struct ActorStart {
    pub ctx: ActorContext,
    /// Raw input bytes from the start request. `None` if the start request
    /// did not include an input payload.
    pub input: Option<Vec<u8>>,
    /// Prior persisted actor-state snapshot, if any.
    /// - `None` on first-create.
    /// - `Some(bytes)` on wake or migrate (content of KV `[1]`).
    pub snapshot: Option<Vec<u8>>,
    /// Hibernatable connections the engine is still holding open from a
    /// prior sleep. Empty on first-create. Each tuple pairs the live
    /// `ConnHandle` with the per-conn bytes the actor previously handed
    /// back via `StateDelta::ConnHibernation` (content of KV `[2] + conn_id`).
    /// The runtime only includes conns that are still live; dead conns are
    /// filtered out before the actor is started.
    ///
    /// **These connections do NOT also fire `ActorEvent::ConnectionOpen`.**
    /// Core hands them over exactly once, in this field, and that's it.
    /// Subsequent events for these conns are `Action`, `HttpRequest`,
    /// `WebSocketOpen`, `SubscribeRequest`, and `ConnectionClosed`. If an
    /// adapter wants to emulate "hibernated conns look like fresh conns"
    /// semantics (e.g. fire a user-facing `onConnect` for each one), it
    /// does that itself by iterating `hibernated` before entering the
    /// receive loop.
    pub hibernated: Vec<(ConnHandle, Vec<u8>)>,
    /// Receiver for framework events. Closed when the runtime wants the
    /// actor to terminate and has no further events to deliver.
    pub events: ActorEvents,
}

/// Why core is asking the actor to serialize state. Passed to the
/// adapter via `ActorEvent::SerializeState { reason, .. }`; lets the adapter
/// make reason-specific decisions (notably whether to clear dirty
/// flags on return).
pub enum SerializeStateReason {
    /// Periodic `state_save_interval` tick, `request_save(true)` flush,
    /// or `request_save_within(ms)` deadline. Core writes the resulting
    /// deltas atomically to KV. Adapter should clear dirty flags.
    Save,
    /// An inspector is attached and the actor has dirty state. Core
    /// distributes the resulting bytes to inspector subscribers but
    /// does NOT write to KV (a coincident `Save` tick will). Adapter
    /// should NOT clear dirty flags — the next `Save` still needs to
    /// flush them.
    Inspector,
}

/// A single persistable change. The runtime writes each entry to its
/// dedicated KV key in one transaction per flush.
pub enum StateDelta {
    /// New actor state bytes. Written to KV `[1]`.
    ActorState(Vec<u8>),
    /// New hibernation bytes for a connection. Upserts KV `[2] + conn_id`.
    ConnHibernation { conn: ConnId, bytes: Vec<u8> },
    /// Removes a connection's hibernation bytes. Deletes KV `[2] + conn_id`.
    ConnHibernationRemoved(ConnId),
}

/// The actor's entire lifetime. Returning `Ok(())` terminates the actor
/// cleanly. Returning `Err(..)` records the error and terminates.
pub type ActorEntryFn =
    dyn Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync;

impl ActorFactory {
    pub fn new<F>(config: ActorConfig, entry: F) -> Self
    where
        F: Fn(ActorStart) -> BoxFuture<'static, Result<()>>
            + Send + Sync + 'static;
}

/// Thin wrapper over `mpsc::Receiver<ActorEvent>`.
pub struct ActorEvents { /* private */ }

impl ActorEvents {
    pub async fn recv(&mut self) -> Option<ActorEvent>;
    pub fn try_recv(&mut self) -> Option<ActorEvent>;
}

/// Events the runtime drives into the actor.
pub enum ActorEvent {
    Action {
        name: String,
        args: Vec<u8>,
        /// `None` for alarm-originated or other system-dispatched actions.
        /// `Some(..)` for actions from an actual client connection.
        conn: Option<ConnHandle>,
        reply: Reply<Vec<u8>>,
    },
    HttpRequest {
        request: Request,
        reply: Reply<Response>,
    },
    WebSocketOpen {
        ws: WebSocket,
        request: Option<Request>,
        reply: Reply<()>,
    },
    ConnectionOpen {
        conn: ConnHandle,
        params: Vec<u8>,
        request: Option<Request>,
        /// Reply `Err(..)` to reject the connection. `Ok(())` accepts it.
        reply: Reply<()>,
    },
    ConnectionClosed {
        conn: ConnHandle,
    },
    /// A connection is attempting to subscribe to a broadcast event. The
    /// actor replies `Ok(())` to allow the subscription or `Err(..)` to
    /// reject it (analogous to today's `canSubscribe` / `onBeforeSubscribe`
    /// in the TypeScript runtime). The event name is the broadcast name
    /// the client is trying to subscribe to.
    SubscribeRequest {
        conn: ConnHandle,
        event_name: String,
        reply: Reply<()>,
    },
    /// Runtime is asking for current serialized state. Unified event for
    /// all state-pull requests — periodic saves, inspector reads,
    /// explicit flushes, etc. Reply with the deltas for whatever is
    /// currently dirty. Core's handling of the reply depends on `reason`
    /// (see `SerializeStateReason`).
    ///
    /// Fires when:
    /// - A `request_save` tick fires (reason: `Save`).
    /// - An inspector is attached and the actor marks state dirty
    ///   (reason: `Inspector`).
    ///
    /// **Not** fired for Sleep/Destroy. Those are termination signals;
    /// adapters that need pre-termination persistence call
    /// `ctx.save_state(deltas)` explicitly from their Sleep/Destroy
    /// handlers.
    SerializeState {
        reason: SerializeStateReason,
        reply: Reply<Vec<StateDelta>>,
    },
    /// Runtime is asking the actor to sleep. No state in the reply —
    /// the adapter owns its shutdown sequence and calls
    /// `ctx.save_state(..)` explicitly whenever it wants to persist.
    /// Reply `Ok(())` once the adapter has finished its pre-termination
    /// work; core then tears down the actor process.
    Sleep {
        reply: Reply<()>,
    },
    /// Runtime is asking the actor to destroy. Same shape as `Sleep`.
    Destroy {
        reply: Reply<()>,
    },
    /// Workflow engine asks for replay history. Fires only when an
    /// upstream consumer (inspector endpoint, workflow-engine replay
    /// request) asks; never on routine operation. Actors that don't
    /// integrate with workflows simply never receive this event and can
    /// ignore it in a default `_ => {}` match arm. Reply with the
    /// serialized history or `None` if unavailable.
    WorkflowHistoryRequested {
        reply: Reply<Option<Vec<u8>>>,
    },
    /// Workflow engine asks to replay from a specific entry. Fires only
    /// on upstream request (see `WorkflowHistoryRequested`). `entry_id`
    /// is the workflow-engine entry identifier (the `entry.id` field
    /// stored at KV `[4, entry_id]` inside the workflow-engine's entry
    /// metadata layout). `None` means replay from the start of the
    /// workflow. Reply with the resulting serialized output or `None` if
    /// the entry is not replayable (already completed, missing, or the
    /// actor is in a state where replay is disallowed).
    WorkflowReplayRequested {
        entry_id: Option<String>,
        reply: Reply<Option<Vec<u8>>>,
    },
}

/// Typed one-shot reply channel. Dropping without calling `send` always
/// produces `ActorLifecycle::DroppedReply`. There is no escape hatch: if
/// the actor intentionally wants to refuse, it must call
/// `reply.send(Err(..))` explicitly. Thin wrapper over
/// `tokio::sync::oneshot::Sender`.
pub struct Reply<T> { /* private */ }

impl<T> Reply<T> {
    /// Send a result. Discards the send error if the receiver is gone.
    pub fn send(self, result: Result<T>);
}

impl<T> Drop for Reply<T> {
    fn drop(&mut self) {
        // if not already sent, send Err(ActorLifecycle::DroppedReply.build())
    }
}
```

### Alarms

Alarms continue to fire via the action machinery. `ctx.schedule().after(duration, "action_name", args)` dispatches as `ActorEvent::Action { name: "action_name", args, .. }`. The actor does not distinguish alarm-originated actions from client-originated actions at the type level. (An `origin: ActionOrigin` field on `ActorEvent::Action` is a possible future extension if a real consumer needs it; out of scope here.)

### Concurrency and task management

The runtime owns **zero** user-level tasks in the new model. The only task per actor instance is the entry future itself. No `JoinSet`, no runtime-tracked action children, no `run_handler_abort`, no `pending_replies` vector on `ActorTask`. Everything the actor wants to run concurrently, it spawns — and it owns the lifecycle of what it spawned.

Actions dispatched inside the receive loop are serialized by default — whatever the match arm awaits blocks the next `events.recv()`. Parallelism is an **actor implementation choice**:

```rust
ActorEvent::Action { name, args, reply, .. } => {
    tokio::spawn(async move {
        let result = handle(name, args).await;
        reply.send(result);
    });
}
```

The runtime does not track or join these spawned tasks. If the actor returns from the entry fn while spawned tasks are still running, those tasks are detached. Their `Reply` drop-guards fire normally when the tasks finish. If an actor wants to cancel spawned work on shutdown, it maintains its own `CancellationToken` (or equivalent) and triggers it from the `Sleep` / `Destroy` match arm *before* replying:

```rust
ActorEvent::Sleep { reply } => {
    shutdown_token.cancel();           // tell my spawned tasks to wind down
    // (optionally) wait for them here before replying
    reply.send(Ok(build_deltas(..)));
    break;
}
```

Because of this, `ctx.abort_signal()` and `ctx.aborted()` from the callback-era API are **removed** in the new model — their purpose (broadcast cancellation to runtime-spawned children) no longer applies. Actors that need cross-task cancellation use their own `tokio_util::sync::CancellationToken`.

### Unknown action names

The runtime does not keep a registry of action names. Unknown names arrive at the actor as `ActorEvent::Action { name, .. }` like any other action, and the actor is responsible for replying `Err(..)` for names it does not recognize. This is a deliberate simplification: the runtime stays opaque to action semantics.

### Persistence

The persistence path is **bidirectional and lazy**. The actor notifies the runtime that something changed; the runtime asks for deltas via `ActorEvent::SerializeState { reason, .. }` when it wants them; the actor serializes only at that point. One event type covers both save-to-KV and inspector-fresh-read paths; the `reason` field tells the adapter what core is going to do with the bytes.

**Notification (actor → runtime):**

```rust
impl ActorContext {
    /// Marks persisted state dirty. Runtime schedules `SerializeState` within
    /// `state_save_interval`. Debounced — multiple calls collapse into one tick.
    ///
    /// If `immediate` is true, the tick fires as soon as the receive loop
    /// processes the next event (bypasses the debounce interval).
    pub fn request_save(&self, immediate: bool);

    /// Marks persisted state dirty with a maximum deadline. Same debounce
    /// as `request_save(false)`, but if the next tick would fire later
    /// than `ms` milliseconds from now, the tick is rescheduled to fire
    /// at `now + ms` instead. Earlier-scheduled ticks are not pushed out.
    ///
    /// Load-bearing for hibernatable WebSocket ack state, which must flush
    /// within a known deadline. Replaces the `maxWait` option on the
    /// TypeScript `saveState({ maxWait })` API.
    pub fn request_save_within(&self, ms: u32);
}
```

The actor calls `request_save` whenever it mutates anything it wants persisted — actor-level state, per-conn hibernation bytes, or both. The call is cheap: it flips a boolean and (optionally) arms the immediate flag. No bytes cross the boundary yet.

**Pull (runtime → actor):**

When the debounce elapses or the immediate flag is set, the runtime fires `ActorEvent::SerializeState { reason: Save, reply: Reply<Vec<StateDelta>> }`. The actor builds a `Vec<StateDelta>` containing exactly the changes it wants flushed — typically by consulting its own per-field dirty flags — and replies. The runtime writes all entries in one UniversalDB transaction covering every affected key.

If an inspector is attached and the dirty flag flips, the runtime fires `ActorEvent::SerializeState { reason: Inspector, .. }` on a short debounce (see "Inspector integration" below). Core does **not** write the reply to KV for Inspector reason — a coincident `Save` tick covers that. Adapter replies with current deltas but leaves its dirty flags set.

**Dirty tracking lives in the actor.** The runtime knows only "something is dirty"; the actor tracks which fields changed. This is a deliberate low-level choice — higher layers can wrap state in smart pointers that auto-track dirty, but core stays manual.

**Sleep and Destroy are separate.** They carry `Reply<()>`, no state in the reply. Adapters that want to persist before termination call `ctx.save_state(deltas).await` explicitly from their Sleep/Destroy handler. Keeping state out of the terminal-event reply lets the adapter decide:
- When to call `onSleep` / `onDestroy` relative to serializing.
- Whether to serialize at all (a pure ephemeral actor may skip).
- Whether to interleave disconnect + serialize + save in a non-trivial order.

**Atomicity guarantees:**

- Every `SerializeState(Save)` flush writes **all** the returned `StateDelta` entries in one UniversalDB transaction. No partial snapshot can land in KV.
- `ctx.save_state(deltas).await` is the same atomicity guarantee, driven synchronously by the adapter.
- Per-conn hibernation bytes for hibernatable connections can be persisted continuously via `StateDelta::ConnHibernation`, so process crashes no longer leave stale `[2] + conn_id` keys from the last sleep.
- `StateDelta::ConnHibernationRemoved` lets the actor reap hibernation keys immediately when a hibernatable conn disconnects, rather than waiting for sleep.

**Synchronous durability path (bypasses SaveTick):**

```rust
impl ActorContext {
    /// Writes `deltas` atomically and awaits durability before returning.
    /// Bypasses the `SerializeState` mechanism — the caller has already
    /// serialized, so the debounced request/reply round trip would be
    /// redundant.
    pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()>;
}
```

Why this is necessary: the TypeScript `ctx.saveState({ immediate: true })` is a `Promise` that resolves only after the write hits disk. The NAPI adapter on top of this Rust API needs a direct path to await durability. Going through `SerializeState` would deadlock — the actor calling `save_state` is already inside an event handler, so it cannot simultaneously receive and reply to a `SerializeState` in the same task.

The contract:

- Actor serializes its current state into `Vec<StateDelta>` at the call site.
- Runtime writes all entries in one UniversalDB transaction.
- The future resolves only after the transaction commits.
- `request_save`'s dirty flag and debounce timer are reset by this write (a `SerializeState` would otherwise fire redundantly immediately after).

For the TypeScript adapter, this maps cleanly: TS holds the current state bytes via its write-through proxy, builds a single `StateDelta::ActorState(bytes)` (plus any hibernatable conn entries), calls `rust_ctx.save_state(deltas).await`, and returns the resolved Promise to user code. Same semantics as today's `saveState({ immediate: true })`.

Pure-Rust actors that don't need await-for-durability stick to `request_save` + `SerializeState` and pay no sync cost.

### Inspector integration

The inspector runs Rust-side (`handle_inspector_http_in_runtime`) and maintains:
- A **base layer** of last-written KV bytes. Served immediately when an inspector attaches. Stale by at most `state_save_interval`.
- A **live overlay layer** driven by `ActorEvent::SerializeState { reason: Inspector, .. }`. When dirty flips while any inspector is attached, core fires this event (debounced by a short `inspector_serialize_state_interval`, default ~50ms, to coalesce bursts). The reply's bytes overlay the base layer for attached clients.

Overlay semantics:

- Inspector clients see `merge(kv_base, overlay)` — overlay wins per key. When a `Save` tick fires and the bytes are written to KV, the overlay collapses into the base (base updated, overlay cleared).
- When the last inspector detaches, core stops firing `SerializeState(Inspector)`. Zero cost when no inspector is watching.
- When a `Save` tick and an Inspector serialize would both fire, core fires `SerializeState(Save)` (a strict superset — Save also distributes bytes to attached inspectors).

Adapter contract for `reason: Inspector`:
- Reply with current deltas.
- **Do not clear dirty flags.** The next `Save` still needs to write them. Adapters track this by branching on the `reason` field before clearing dirty state at the end of their `serializeForTick` handler.

This gives attached inspectors near-realtime reads without any KV write cost, and keeps actors with no attached inspector on the pure periodic-save path.

**Implementation seams:**

- **Attach/detach.** The inspector HTTP handler calls `ctx.inspector_attach()` on connect and `ctx.inspector_detach()` on disconnect. These increment/decrement an `AtomicU32` held by `ActorTask`. Count > 0 means at least one inspector is watching; count transitioning from 0 → 1 arms the debouncer if state is already dirty; count transitioning to 0 clears any pending inspector deadline.
- **Debouncer location.** `ActorTask` holds `inspector_serialize_state_deadline: Option<Instant>` alongside the existing `state_save_deadline` / `sleep_deadline` fields, with a matching `select!` branch in `run()`. On `request_save` when the inspector count > 0, the branch sets `deadline = now + inspector_serialize_state_interval` if not already armed. When the branch fires, it pushes `ActorEvent::SerializeState { reason: Inspector, reply }` into the actor event channel and clears the deadline. A coincident `Save` tick cancels the Inspector deadline (Save is a superset).
- **Fan-out.** Each actor owns one `tokio::sync::broadcast::Sender<Arc<Vec<u8>>>` for inspector overlay bytes, stored on `ActorTask`. The inspector HTTP handler calls `ctx.subscribe_inspector()` on attach to get a `broadcast::Receiver`, then streams received bytes to its WS/SSE client. Broadcast lag (slow client) drops old frames and logs a warning — the inspector will re-sync on the next `SerializeState` reply. Sender bytes are the serialized `StateDelta::ActorState` extracted from the reply; `StateDelta::ConnHibernation*` entries also flow through so attached inspectors see conn-state overlays.

### Hibernation decision stays on `ActorConfig`

Whether a WebSocket connection is hibernatable is a runtime decision made at connect time using `ActorConfig::can_hibernate_websocket` (either a `bool` or a `Callback(fn(&HttpRequest) -> bool)`). This is unchanged from today. The actor does not see a per-conn "should I hibernate" event — by the time `ActorEvent::ConnectionOpen` or `ActorEvent::WebSocketOpen` arrives, the runtime has already classified the connection. If the actor needs to know, `conn.is_hibernatable()` exposes the decision on `ConnHandle`.

Rationale: keeping the decision in config matches the existing TypeScript `canHibernateWebSocket` surface exactly, avoids a synchronous round trip on every connection open, and keeps the hibernation contract stable across runtime restarts.

### Per-conn event causality

For any given `ConnHandle`, core guarantees that conn-scoped events are
enqueued in order and the next event is not enqueued until the prior
event's `Reply` has been received. Conn-scoped events are: `Action` (when
`conn.is_some()`), `HttpRequest` (when carrying a conn), `WebSocketOpen`,
`ConnectionOpen`, `SubscribeRequest`, and `ConnectionClosed`.

This generalizes the existing `ConnectionOpen` reply-gating model to
everything scoped to a specific conn. It lets the NAPI adapter (or any
other adapter) `tokio::spawn` per-event handlers for parallelism across
different conns while still observing per-conn ordering, without having
to implement its own per-conn serialization queue.

Cross-conn parallelism is unconstrained: events for conn A and conn B
can dispatch and reply in any interleaving.

System-dispatched `Action`s (alarm-originated, `conn: None`) are
unordered with respect to each other and with respect to conn-scoped
events.

### Adapter-driven connection disconnect

During shutdown, adapters need to control *when* connections disconnect
relative to their own cleanup work (e.g. TS's reference order is
`onSleep → disconnect non-hibernatable → save`). Core exposes:

```rust
impl ActorContext {
    /// Tears down a single conn's transport. Does NOT fire an
    /// `ActorEvent::ConnectionClosed` through the mailbox — the caller
    /// is responsible for running any disconnect-notification logic.
    /// Returns after the transport is closed.
    pub async fn disconnect_conn(&self, conn: ConnId) -> Result<()>;

    /// Tears down every conn for which `predicate(&ConnHandle)` returns
    /// true. Same semantics as `disconnect_conn`: no events fired.
    /// Typical use: `ctx.disconnect_conns(|c| !c.is_hibernatable()).await`.
    pub async fn disconnect_conns(
        &self,
        predicate: impl Fn(&ConnHandle) -> bool + Send + Sync,
    ) -> Result<()>;

    /// Iterate current conns. Cheap snapshot.
    pub fn conns(&self) -> impl Iterator<Item = ConnHandle> + '_;
}
```

These let the adapter interleave disconnect with its own drain / callback
logic during `Sleep` / `Destroy`. Core's automatic post-Sleep disconnect
is removed — if the adapter doesn't disconnect, non-hibernatable conns
stay up until the actor process exits.

**Why no mailbox event on adapter-driven disconnect.** Client-initiated
disconnects (transport dies) still fire `ActorEvent::ConnectionClosed`
normally, so the adapter's standard event dispatch runs. But during the
adapter's `Sleep`/`Destroy` arm it's already past the receive loop's
dispatch for these conns — routing adapter-initiated disconnects back
through the mailbox would force a second drain loop between
`disconnect_conns` and the final reply. Cleaner: adapter invokes its
disconnect handler inline, then tells core to tear down transports.

### Queue

Queue messages stay pull-based via `ctx.queue().recv().await`. They are not part of `ActorEvent`. The actor composes them with the event loop using `tokio::select!`:

```rust
loop {
    tokio::select! {
        Some(event) = events.recv() => match event { .. },
        Some(msg)   = ctx.queue().recv() => { /* handle */ },
    }
}
```

Rationale: queue reads are optional (not every actor uses them), may be selective (by queue name), and may be batched. Forcing them through the single event channel would bottleneck all of these.

### Counter example (end-user API, inside `rivetkit-core`)

```rust
use std::io::Cursor;
use anyhow::{Result, anyhow};
use ciborium::{from_reader, into_writer};
use rivetkit_core::{
    ActorConfig, ActorEvent, ActorEvents, ActorFactory, ActorStart,
    CoreRegistry, Reply,
};

async fn run(start: ActorStart) -> Result<()> {
    let ActorStart { ctx, snapshot, mut events, .. } = start;

    let mut count: i64 = match snapshot {
        Some(bytes) => from_reader(Cursor::new(bytes))?,
        None => 0,
    };
    let mut state_dirty = false;

    while let Some(event) = events.recv().await {
        match event {
            ActorEvent::Action { name, args, reply, .. } => match name.as_str() {
                "increment" => {
                    let delta: i64 = from_reader(Cursor::new(args)).unwrap_or(1);
                    count += delta;
                    state_dirty = true;
                    ctx.request_save(false);
                    ctx.broadcast("count_changed", &encode(&count)?);
                    reply.send(Ok(encode(&count)?));
                }
                "get" => reply.send(Ok(encode(&count)?)),
                other => reply.send(Err(anyhow!("unknown action `{other}`"))),
            },

            ActorEvent::SerializeState { reason, reply } => {
                let deltas = build_deltas(&count, &mut state_dirty, reason)?;
                reply.send(Ok(deltas));
            }
            ActorEvent::Sleep { reply } => {
                // Sleep: persist one final time if dirty, then exit.
                if state_dirty {
                    let deltas = build_deltas(&count, &mut state_dirty, SerializeStateReason::Save)?;
                    ctx.save_state(deltas).await?;
                }
                reply.send(Ok(()));
                break;
            }
            ActorEvent::Destroy { reply } => {
                // Destroy: same, but in a real actor we might skip persistence.
                if state_dirty {
                    let deltas = build_deltas(&count, &mut state_dirty, SerializeStateReason::Save)?;
                    ctx.save_state(deltas).await?;
                }
                reply.send(Ok(()));
                break;
            }

            _ => {}
        }
    }
    Ok(())
}

fn encode(n: &i64) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    into_writer(n, &mut out)?;
    Ok(out)
}

fn build_deltas(count: &i64, dirty: &mut bool) -> Result<Vec<StateDelta>> {
    if !*dirty {
        return Ok(Vec::new());
    }
    *dirty = false;
    Ok(vec![StateDelta::ActorState(encode(count)?)])
}

fn counter_factory() -> ActorFactory {
    ActorFactory::new(ActorConfig::default(), |start| Box::pin(run(start)))
}
```

## Mapping from `ActorInstanceCallbacks`

Only callbacks that correspond to events the core runtime actually drives are
listed here. Callbacks that are purely lifecycle wrappers (`on_create`,
`on_migrate`, `on_wake`, `create_vars`, `create_conn_state`,
`on_before_actor_start`, `on_state_change`, `on_before_action_response`) are
expected to be emulated by whatever adapter (NAPI, V8, pure Rust wrapper) sits
on top of this API. They have no core-level correspondence; core neither fires
them nor enforces ordering between them. See
`rivetkit-napi-receive-loop-adapter.md` for the NAPI-side emulation.

| Old callback | New mechanism | Notes |
|---|---|---|
| `on_sleep` | `ActorEvent::Sleep` | Reply is `Reply<()>`. Adapter runs its full shutdown sequence (`onSleep`, disconnects, etc.) and calls `ctx.save_state(deltas)` explicitly for any final persistence before replying `Ok(())`. |
| `on_destroy` | `ActorEvent::Destroy` | Same shape as `Sleep`. |
| `on_request` | `ActorEvent::HttpRequest` | 1:1. |
| `on_websocket` | `ActorEvent::WebSocketOpen` | 1:1. |
| `on_before_connect` + `on_connect` | `ActorEvent::ConnectionOpen` | Core fires one event. Adapters that need the today-style two-phase split (pre-conn validation then post-conn setup) run them sequentially inside their `ConnectionOpen` handler and reply once. Reject via `reply.send(Err(..))`. |
| `on_disconnect` | `ActorEvent::ConnectionClosed` | 1:1, no reply. Also fires for each conn disconnected via `ctx.disconnect_conn` / `ctx.disconnect_conns`. |
| `on_before_subscribe` | `ActorEvent::SubscribeRequest` | **New capability.** No reference behavior in feat/sqlite-vfs-v2 — subscriptions are added programmatically today. Introduced to give adapters a per-event access-control gate (analogous to a hypothetical `canSubscribe`). Reply `Err(..)` to reject. |
| `actions` map | `ActorEvent::Action` | Dispatch moves from runtime `HashMap` lookup to user-side `match name.as_str()`. Alarms dispatch as `Action { conn: None, .. }`. |
| `run` | *adapter concern* | Not a core event. Adapters spawn their own long-running handler alongside the receive loop and own its lifecycle (restart on crash, cancel on shutdown, etc.). |
| `get_workflow_history` | `ActorEvent::WorkflowHistoryRequested` | Actor replies with serialized history or `None`. |
| `replay_workflow` | `ActorEvent::WorkflowReplayRequested` | `entry_id: Option<String>` matches workflow-engine `entry.id` stored at KV `[4, entry_id]`. `None` = replay from start. |

### Not in core (adapter emulation)

- `on_create` / `create_state` / `create_vars` / `create_conn_state` — first-create preamble; adapter runs before entering the receive loop, gated on `ActorStart.snapshot.is_none()`.
- `on_migrate` — adapter-level wrapper if the adapter's public API keeps it. Core has no migration concept.
- `on_wake` — adapter runs before entering the loop when `ActorStart.snapshot.is_some()`.
- `on_before_actor_start` — driver-level hook; adapter runs before the receive loop.
- `on_state_change` — adapter-level notification. The TypeScript adapter fires it from its `@rivetkit/on-change` handler synchronously on mutation; core never sees it.
- `on_before_action_response` — adapter-level wrapper around action dispatch. If defined, adapter invokes it with `(ctx, name, args, result)` before sending the result as its `Reply`.
- `run` — adapter-spawned task; adapter chooses its supervision policy (feat/sqlite-vfs-v2 logs errors and keeps the actor alive, supports `restartRunHandler()`).

## Runtime changes (internal, not user-facing)

- `ActorFactory::create` returns the `BoxFuture<Result<()>>` of the actor's lifetime instead of returning `ActorInstanceCallbacks`. The factory function owns spawning the future; `ActorTask` owns holding the join handle and driving events into the mailbox.
- `ActorInstanceCallbacks` is deleted. `ActorTask` holds an `mpsc::Sender<ActorEvent>` instead of a callbacks `Arc`.
- `ActorTask`'s dispatch logic becomes a translation layer: incoming `DispatchCommand` variants (today: `Action`, `Http`, `OpenWebSocket`) are translated into `ActorEvent` variants and pushed into the mailbox. The reply `oneshot` from the dispatch command becomes the `Reply<T>` in the event.
- Per-conn event causality (see above): `ActorTask` maintains a per-conn gate:

```rust
struct PerConnGate {
    in_flight: bool,
    pending: VecDeque<ConnScopedEvent>,
}
// ActorTask:
per_conn_gates: scc::HashMap<ConnId, PerConnGate>
```

On conn-scoped event arrival for conn X: if the gate's `in_flight` is false, flip it to true and push the event onto the mailbox. If true, push onto `pending` and wait. Each `Reply<T>` for a conn-scoped event carries a hidden hook fired on send (whether explicit or via drop-guard); that hook pops the next `pending` event onto the mailbox or flips `in_flight = false` if `pending` is empty. `scc::HashMap` is used per `CLAUDE.md` (no `Mutex<HashMap>`). ConnId entries are removed on `ActorEvent::ConnectionClosed` or when `ctx.disconnect_conn` completes.
- **`ActorTask.children`, `run_handler_abort`, `pending_replies`, `set_run_handler_active`, `restart_run_handler`, `abort_remaining_children`, and `wait_for_run_handler` are all deleted.** The runtime no longer spawns or supervises user tasks — the actor's entry future is the only user task, and it owns whatever it spawns. `Reply<T>` drop-guards handle the "forgot to reply" case without a runtime-side tracking vector.
- Shutdown sequencing in `ActorTask::shutdown_for_sleep` / `shutdown_for_destroy` becomes: send `ActorEvent::Sleep` (or `Destroy`) into the mailbox, await the `Reply<Vec<StateDelta>>`, write the deltas atomically, then run the existing disconnect flow and await the entry future. No separate `save_state(immediate: true)` final flush — the delta reply *is* the final flush.
- The existing `actor_channel_overloaded_error` / `try_send_*` helpers for bounded channels are reused for the new event mailbox.
- `ctx.abort_signal()` and `ctx.aborted()` are removed from `ActorContext`. Their callers inside `ActorTask` (`abort_signal().cancel()` during shutdown) are also removed — there is nothing for the runtime to cancel. Actors that need cross-task cancellation use their own `CancellationToken`.
- Panic isolation: the entry fn is `catch_unwind`'d in `ActorTask`. A panic surfaces as `Err(..)` on any outstanding `Reply<T>` via the drop-guard and terminates the actor. No per-event panic recovery — if a user wants that, they wrap their own match arms.

The net result: `ActorTask` shrinks from ~1100 LOC to an event pump + shutdown sequencer. Most of the current concurrency machinery (child tasks, run-handler supervision, pending replies, abort signals, scoped shutdown waits) disappears because the receive-loop model pushes all of it into the actor's own entry future.

## Open questions

None currently blocking. All initial open questions have been resolved (see below).

## Resolved (design decisions locked in by author review)

- **Persistence drive**: bidirectional — actor calls `ctx.request_save(immediate: bool)` (cheap dirty notification), runtime fires `ActorEvent::SaveTick { reply: Reply<Vec<StateDelta>> }` when the debounce elapses, actor replies with deltas, runtime writes atomically. `ctx.save_state(deltas).await` is the synchronous bypass the TypeScript `saveState({ immediate: true })` adapter uses.
- **Inspector state reads**: overlay model on top of KV. Core tracks last-written bytes; when an inspector attaches, it serves the KV base immediately and fires `ActorEvent::SerializeState { reason: Inspector }` whenever the dirty flag flips (debounced by `inspector_serialize_state_interval`). Adapter replies without clearing dirty flags. Attaching inspector sees near-realtime overlay without any KV write cost; zero cost when no inspector is attached.
- **Concurrency / task ownership**: the runtime owns zero user-level tasks. `ActorTask.children`, `run_handler_abort`, `pending_replies`, and `abort_remaining_children` are deleted. Actors spawn their own tasks; if they need cancellation they use their own `CancellationToken`.
- **`ctx.abort_signal()` / `ctx.aborted()`**: removed. They existed to broadcast cancellation to runtime-spawned children; with no runtime-spawned children, they have no purpose.
- **Action concurrency**: runtime does not parallelize actions. Parallelism is an actor implementation choice via `tokio::spawn`. No `ctx.spawn_scoped` in core.
- **Unknown action names**: runtime keeps no action registry. Unknown names reach the actor and the actor replies `Err(..)`.
- **`Reply<T>` drop semantics**: always fails with `ActorLifecycle::DroppedReply`. No escape hatch.
- **`ActorStart.input`**: typed as `Option<Vec<u8>>`, matching today's `FactoryRequest.input`.
- **Workflow hooks**: add `ActorEvent::WorkflowHistoryRequested { reply: Reply<Option<Vec<u8>>> }` and `ActorEvent::WorkflowReplayRequested { entry_id: Option<String>, reply: Reply<Option<Vec<u8>>> }`. Keeps the event-stream abstraction uniform.
- **Hibernatable WebSockets**: unified with the delta mechanism. Per-conn hibernation bytes flow through `StateDelta::ConnHibernation { conn, bytes }` (upsert to KV `[2] + conn_id`) and `StateDelta::ConnHibernationRemoved(conn)` (delete) on every `SerializeState` / `Sleep` / `Destroy`. This preserves today's KV layout and TypeScript-runtime parity, enables continuous persistence of hibernation state (so crashes no longer drop per-conn mutations), and removes the need for a separate `SleepSnapshot` type. On wake the runtime filters to live conns and exposes them as `ActorStart.hibernated: Vec<(ConnHandle, Vec<u8>)>`.
- **`on_before_subscribe`**: kept as `ActorEvent::SubscribeRequest { conn, event_name, reply: Reply<()> }`. Load-bearing for per-event access control (`canSubscribe`); reply `Err(..)` to reject.
- **Back-pressure**: single `mpsc::Receiver<ActorEvent>` mailbox sized from `ActorConfig::lifecycle_event_inbox_capacity` (or equivalent tuning knob). Keeping one channel is the simplest path; if a real workload demonstrates that dispatch floods can starve `Sleep` / `SerializeState` enqueues, we split into two internal channels with a biased `select!` then — not before. The existing `actor_channel_overloaded_error` / `try_send_*` helpers apply.
- **Access control**: `rivetkit-core` has no access-control concept of its own. The actor inspects the materials it receives on each event (`ConnectionOpen.params` + `request` headers, `Action.conn` + `name`, `HttpRequest.request`, `SubscribeRequest.conn` + `event_name`) and replies `Err(..)` to reject. Engine-level auth and the inspector token remain outside the actor event stream, unchanged by this spec.
- **Preamble callbacks are adapter concerns**: Core's ActorStart gives the adapter `ctx`, `input`, `snapshot`, `hibernated`, and `events`. Everything today's `ActorInstance` does before entering its main loop (`on_create` / `create_state` / `create_vars` / `create_conn_state` / `on_migrate` / `on_wake` / `on_before_actor_start`) is the adapter's responsibility. Core provides no hooks for these and no ordering guarantees between them.
- **`run` handler is an adapter concern**: Core has no notion of a `run` callback. Adapters that expose one spawn it as a detached task and choose their own supervision policy. `restartRunHandler()` is likewise adapter-level.
- **`Action.conn` is `Option<ConnHandle>`**: `None` for alarm-originated or otherwise system-dispatched actions; `Some(..)` for client-originated actions. Adapters that want a synthetic "system conn" create one adapter-side and pass `Some(synthetic)`.
- **Per-conn causality**: Core serializes enqueue of conn-scoped events per-conn, enabling adapters to spawn handlers concurrently without losing ordering within a conn. Cross-conn events are unordered. System-dispatched actions are unordered.
- **Adapter-driven disconnect**: `ctx.disconnect_conn` and `ctx.disconnect_conns` let the adapter interleave disconnects with its own Sleep/Destroy sequencing. Core does not auto-disconnect after `Sleep`.
- **`maxWait` for state saves**: exposed as `ctx.request_save_within(ms)`. Same debounced `SerializeState` machinery, with an upper bound on the delay. Load-bearing for hibernatable WebSocket ack state.

## Migration path

Not addressed in this spec beyond the sketch below. A separate spec should own the migration sequencing if this proposal is accepted.

- Keep `ActorInstanceCallbacks` working in parallel.
- Provide an adapter: `ActorFactory::from_callbacks(config, callbacks)` internally runs a default entry fn that forwards each `ActorEvent` to the matching callback.
- Removed hooks (`on_state_change`, `on_before_action_response`, runtime-driven `run` supervision, `ctx.abort_signal`) are **adapter responsibilities**, not core responsibilities. For example, the TypeScript adapter can wrap its write-through `state` proxy so every mutation fires the user's `onStateChange` callback inline and then calls `ctx.request_save(false)` on the Rust side. Core stays minimal and event-based; adapter layers provide the callback sugar for users who expect it.
- Delete `ActorInstanceCallbacks` in a follow-up change once in-tree consumers migrate.

## What this spec does not commit to

- Exact byte layouts of `Reply<T>` / `ActorEvents` internals — implementation detail.
- Whether `ActorEntryFn` takes an `Arc<ActorStart>` or `ActorStart` by value — both work; decide during implementation.
- Whether the `Actor` convenience trait from the earlier draft (`trait Actor { async fn run(self, ctx, events) -> Result<()>; }`) lands. Strictly additive; can be added after the core surface settles. Not part of this spec.
