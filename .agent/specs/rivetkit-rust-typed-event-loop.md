# RivetKit Rust — Typed Event-Loop API (v2)

## Overview

A rewrite of the high-level `rivetkit-rust/packages/rivetkit/` crate as a thin, typed, event-loop-based layer over the new task-model `rivetkit-core`. The user writes the actor's receive loop themselves and gets typed helpers for state, actions, connections, and persistence — no proc macros, serde only.

This replaces the current callback-based `Actor` trait (ten lifecycle methods, action registration, method-on-trait config) with a surface that matches `rivetkit-core`'s `ActorFactory` entry function 1:1 and gets out of the way.

Primary goals:
- Mirror `rivetkit-core`'s contract: the user's `run` fn drives `ActorEvents::recv()` and replies through drop-guarded `Reply<T>` handles.
- Typed generics for `Input`, `ConnParams`, `ConnState`, `Action` carried on an `Actor` trait.
- State is use-site generic (not on the trait), held in the user's local `let mut state` inside the loop.
- No proc macros. Serde (CBOR) for all encode/decode.
- Config passed at `Registry::register_with(..)`, not on the trait.
- Let the user spawn tasks from inside the loop the same way core does (`tokio::spawn`, `ctx.wait_until`).

Non-goals for v1:
- Typed broadcast events (planned; single string-named `ctx.broadcast` is enough for v1).
- Typed queue stream helpers (users can call `ctx.queue().next()` directly with serde for now).
- Migration tooling for the old callback-based `Actor` trait — this is a hard cutover.

## Prerequisites (core-side)

This spec is written against the **post-US-004** shape of `rivetkit-core`. Specifically it assumes:
- PRD `US-001` — `ActorEvent::SaveTick` renamed to `ActorEvent::SerializeState { reason: SerializeStateReason, .. }`.
- PRD `US-002` — `ActorEvent::Sleep.reply` and `ActorEvent::Destroy.reply` are `Reply<()>`; `ActorEvent::Action.conn` is `Option<ConnHandle>`.
- PRD `US-003` — `ActorContext::request_save_within(ms)`, `disconnect_conn(id)`, `disconnect_conns(predicate)`, `conns()` iterator accessor, `on_request_save(hook)`.
- PRD `US-004` — not directly exposed to rivetkit users, but inspector attach/detach + broadcast fan-out are available for future use.

If any of the prerequisite stories slip, the matching wrapper on the rivetkit side should be gated on landing them — do not build rivetkit against the pre-US-002 Sleep/Destroy shape.

## Package Layout

- `rivetkit-rust/packages/rivetkit-core/` — unchanged event-loop core (source of `ActorFactory`, `ActorContext`, `ActorEvent`, `Reply<T>`, `StateDelta`, etc.)
- `rivetkit-rust/packages/rivetkit/` — **rewritten** typed layer described by this spec

Re-exports from `rivetkit-core` stay: `ActorConfig`, `Kv`, `SqliteDb`, `Queue`, `Schedule`, `ConnHandle`, `ConnId`, `WebSocket`, `Request`, `Response`, `WsMessage`, `StateDelta`, `ActorKey`, `ActorKeySegment`, `ListOpts`, `SaveStateOpts`, `EnqueueAndWaitOpts`, `QueueWaitOpts`, `QueueMessage`, `CanHibernateWebSocket`, `ServeConfig`, `SerializeStateReason`.

## `Actor` Trait

Type-binding only — no methods, no defaults, no state.

```rust
pub trait Actor: Send + 'static {
    type Input:      DeserializeOwned + Send + 'static;
    type ConnParams: DeserializeOwned + Send + Sync + 'static;
    type ConnState:  Serialize + DeserializeOwned + Send + Sync + Clone + 'static;
    type Action:     DeserializeOwned + Send + 'static;
}
```

Notes:
- `Sync` is required on `ConnParams` and `ConnState` because typed accessors hand out shared references that may cross `.await` points in user-spawned tasks.
- `Sized` is not required on the trait itself — leave it to the `impl`.
- No `Default` bound on `Input`. Missing input is handled by `Start<A>::input` being a decoder handle (see below), not by silently defaulting.
- Actors that do not use connections set `type ConnParams = ()` and `type ConnState = ()`.
- Actors that do not use the typed-action dispatcher set `type Action = rivetkit::action::Raw` (a unit type whose `decode()` is a no-op that forces the user to fall back to `action.name()` / `action.raw_args()`).

## Entry Signature

```rust
pub struct Start<A: Actor> {
    pub ctx: Ctx<A>,
    pub input: Input<A>,              // decoder handle, not a decoded value
    pub snapshot: Snapshot,           // opaque, decoded on demand
    pub hibernated: Vec<Hibernated<A>>,
    pub events: Events<A>,
}
```

### `Input<A>`

```rust
pub struct Input<A: Actor> { /* Option<Vec<u8>>, PhantomData */ }

impl<A: Actor> Input<A> {
    pub fn is_present(&self) -> bool;                               // true iff core gave Some(..) bytes
    pub fn decode(&self) -> Result<A::Input>;                       // errors if missing or malformed
    pub fn decode_or<F: FnOnce() -> A::Input>(&self, f: F) -> Result<A::Input>;
    pub fn decode_or_default(&self) -> Result<A::Input> where A::Input: Default;
    pub fn raw(&self) -> Option<&[u8]>;
}
```

### `Snapshot`

```rust
pub struct Snapshot { /* Option<Vec<u8>> */ }

impl Snapshot {
    pub fn is_new(&self) -> bool;                                   // true iff no persisted state
    pub fn decode<S: DeserializeOwned>(&self) -> Result<Option<S>>; // None iff is_new()
    pub fn decode_or_default<S: DeserializeOwned + Default>(&self) -> Result<S>;
    pub fn raw(&self) -> Option<&[u8]>;
}
```

### `Hibernated<A>`

```rust
pub struct Hibernated<A: Actor> {
    pub conn: ConnCtx<A>,
    // state bytes are still inside the ConnHandle; decode via conn.state()
}
```

### `Events<A>`

Wraps the core `ActorEvents` and yields typed `Event<A>` variants. `Events<A>: !Sync` (single-consumer). Dropping `Events<A>` closes the channel — drop only when the user intends to stop processing events (i.e. about to return from `run`).

**Contract:** the user's `run` fn MUST drain `events.recv()` until `None` for core to consider the actor shut down cleanly. Exiting early (returning before the stream ends) is allowed but will be logged at `warn!` by core and may time out on the shutdown path.

```rust
impl<A: Actor> Events<A> {
    pub async fn recv(&mut self) -> Option<Event<A>>;
    pub fn try_recv(&mut self) -> Option<Event<A>>;
}
```

## `Registry`

```rust
impl Registry {
    pub fn new() -> Self;

    pub fn register<A, F, Fut>(&mut self, name: &str, entry: F) -> &mut Self
    where
        A: Actor,
        F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static;

    pub fn register_with<A, F, Fut>(
        &mut self, name: &str, config: ActorConfig, entry: F,
    ) -> &mut Self where /* same bounds */;

    pub async fn serve(self) -> Result<()>;
    pub async fn serve_with_config(self, config: ServeConfig) -> Result<()>;
}
```

Internally `register_with` builds a `CoreActorFactory::new(config, move |core_start| { Box::pin(async move { entry(wrap_start::<A>(core_start)?).await }) })` and calls `CoreRegistry::register(name, factory)`. `wrap_start` converts `ActorStart { ctx, input, snapshot, hibernated, events }` from core into our typed `Start<A>`, turning raw byte fields into `Input<A>` / `Snapshot` / `Vec<Hibernated<A>>` / `Events<A>` handles.

Usage:

```rust
let mut reg = Registry::new();
reg.register::<Chat>("chat", run_chat);
reg.register_with::<Counter>("counter", counter_cfg, run_counter);
reg.serve().await?;
```

## `Event<A>`

```rust
#[must_use = "dropping an Event<A> without replying sends actor/dropped_reply"]
pub enum Event<A: Actor> {
    Action(Action<A>),
    Http(HttpCall),
    WebSocketOpen(WsOpen<A>),
    ConnOpen(ConnOpen<A>),
    ConnClosed(ConnClosed<A>),
    Subscribe(Subscribe<A>),
    SerializeState(SerializeState<A>),
    Sleep(Sleep<A>),
    Destroy(Destroy<A>),
    WorkflowHistory(WfHistory),
    WorkflowReplay(WfReplay),
}
```

Every variant except `ConnClosed` holds a core `Reply<T>`. All wrapper structs are `#[must_use]`. Dropping without replying causes core's drop-guard to send `actor/dropped_reply` — same guarantee as core. Each wrapper's `Drop` logs at `warn!` with variant name + any identifying field (action name, conn id) before the guard fires so "silent dropped reply" is never silent.

### `Action<A>`

```rust
#[must_use]
pub struct Action<A: Actor> { /* name, raw_args, Reply<Vec<u8>>, Option<ConnCtx<A>>, PhantomData */ }

impl<A: Actor> Action<A> {
    pub fn name(&self) -> &str;
    pub fn conn(&self) -> Option<&ConnCtx<A>>;          // None for alarm-originated actions (US-002)
    pub fn raw_args(&self) -> &[u8];
    pub fn decode(&self) -> Result<A::Action>;          // serde-backed decode of (name, args)
    pub fn decode_as<T: DeserializeOwned>(&self) -> Result<T>; // when A::Action is not used
    pub fn ok<T: Serialize>(self, value: &T);           // CBOR-encodes, sends Ok
    pub fn err(self, err: anyhow::Error);               // sends Err
}
```

#### (name, args) → `A::Action` decoding

Core hands the adapter `(name: String, args: Vec<u8>)`. `A::Action` is an externally-tagged serde enum in the user's code:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChatAction {
    Send { text: String },
    History,
    Kick { user_id: String },
}
```

`decode()` implements a small `Deserializer` that drives `deserialize_enum` directly:

- `deserialize_enum(name, variants, visitor) -> visitor.visit_enum(Access { name, args })`.
- `EnumAccess::variant_seed` returns the variant identifier by deserializing the name through `BorrowedStrDeserializer::new(self.name)` — this is what serde-derive expects; feeding anything else yields "invalid type: string, expected variant identifier" at runtime.
- `VariantAccess::unit_variant` — only accepts `args` that is empty (zero bytes) or a single CBOR null (`0xf6`). Any other shape returns `de::Error::custom(..)`. This is the canonical empty encoding; document for callers.
- `VariantAccess::newtype_variant_seed` — constructs a **fresh** `ciborium::de::Deserializer` over `args` and forwards to `seed.deserialize(fresh)`.
- `VariantAccess::tuple_variant` — expects `args` to decode as a CBOR array whose length matches `len`. Feeds the outer visitor a fresh ciborium sequence deserializer.
- `VariantAccess::struct_variant` — expects `args` to decode as a CBOR map whose keys match the declared fields. Feeds the outer visitor a fresh ciborium map deserializer.
- Unknown variant name: return `de::Error::custom(format!("unknown action variant: {}", name))` — the runtime variant list isn't available to a generic decoder.

Forward all non-`deserialize_enum` methods to a forward-to-deserialize-any implementation that returns a "use deserialize_enum instead" error; we only support enum targets in `decode()`.

For non-enum action payloads or dynamic dispatch cases, use `action.decode_as::<T>()` which just decodes `raw_args()` directly through `ciborium::from_reader` and ignores the name.

### `SerializeState<A>`

```rust
#[must_use]
pub struct SerializeState<A: Actor> { /* reason: SerializeStateReason, Reply<Vec<StateDelta>>, PhantomData */ }

impl<A: Actor> SerializeState<A> {
    pub fn reason(&self) -> SerializeStateReason;      // Save | Inspector
    pub fn save<S: Serialize>(self, state: &S);         // single ActorState delta
    pub fn save_with(self, deltas: Vec<StateDelta>);    // escape hatch
    pub fn save_state_and_conns<S: Serialize>(
        self,
        state: &S,
        conn_hibernation: Vec<(ConnId, Vec<u8>)>,
        conn_hibernation_removed: Vec<ConnId>,
    );
    pub fn skip(self);                                  // empty deltas
}
```

Covers both periodic `Save` ticks and `Inspector` overlay writes (the latter never lands in KV — core fans it through the inspector broadcast channel). Users can ignore the reason unless they need Inspector-specific behavior.

### `Sleep<A>` and `Destroy<A>`

Post-US-002 these reply with `Reply<()>`. Persistence is not bundled into the reply — the user calls `ctx.save_state(deltas).await` explicitly before replying when they want state saved.

```rust
#[must_use]
pub struct Sleep<A: Actor>   { /* Reply<()>, PhantomData */ }
#[must_use]
pub struct Destroy<A: Actor> { /* Reply<()>, PhantomData */ }

impl<A: Actor> Sleep<A> {
    pub fn ok(self);                                    // reply Ok(())
    pub fn err(self, e: anyhow::Error);                 // reply Err
}
impl<A: Actor> Destroy<A> {
    pub fn ok(self);
    pub fn err(self, e: anyhow::Error);
}
```

Helper free functions in `rivetkit::persist`:

```rust
pub fn state_delta<S: Serialize>(state: &S) -> Result<StateDelta>;
pub fn state_deltas<S: Serialize>(state: &S) -> Result<Vec<StateDelta>>;
pub fn conn_hibernation_delta(conn: ConnId, bytes: Vec<u8>) -> StateDelta;
pub fn conn_hibernation_removed_delta(conn: ConnId) -> StateDelta;
```

Typical Sleep handler:

```rust
Event::Sleep(s) => {
    s.ctx_ref(&s.ctx); // stub: user holds their own ctx from Start
    ctx.save_state(rivetkit::persist::state_deltas(&state)?).await?;
    s.ok();
}
```

(The user's `ctx` is the `Ctx<A>` from `Start<A>`, already in scope.)

### `ConnOpen<A>` / `ConnCtx<A>` / `ConnClosed<A>` / `Subscribe<A>`

```rust
#[must_use]
pub struct ConnOpen<A: Actor> { /* ConnHandle, params: Vec<u8>, request: Option<Request>, Reply<()> */ }

impl<A: Actor> ConnOpen<A> {
    pub fn params(&self) -> Result<A::ConnParams>;
    pub fn request(&self) -> Option<&Request>;
    pub fn conn(&self) -> &ConnCtx<A>;
    pub fn accept(self, state: A::ConnState);           // encodes, sets, replies Ok(())
    pub fn accept_default(self) where A::ConnState: Default;
    pub fn reject(self, err: anyhow::Error);
}

pub struct ConnCtx<A: Actor> { /* ConnHandle, PhantomData<fn() -> A> */ }

impl<A: Actor> ConnCtx<A> {
    pub fn id(&self) -> &str;
    pub fn is_hibernatable(&self) -> bool;
    pub fn params(&self) -> Result<A::ConnParams>;
    pub fn state(&self) -> Result<A::ConnState>;
    pub fn set_state(&self, state: &A::ConnState) -> Result<()>;
    pub fn send<E: Serialize>(&self, name: &str, event: &E) -> Result<()>;
    pub async fn disconnect(&self, reason: Option<&str>) -> Result<()>;
    pub fn inner(&self) -> &ConnHandle;
}

pub struct ConnClosed<A: Actor> { pub conn: ConnCtx<A> }

#[must_use]
pub struct Subscribe<A: Actor> { /* ConnCtx<A>, event_name: String, Reply<()> */ }

impl<A: Actor> Subscribe<A> {
    pub fn conn(&self) -> &ConnCtx<A>;
    pub fn event_name(&self) -> &str;
    pub fn allow(self);
    pub fn deny(self, err: anyhow::Error);
}
```

### `HttpCall` / `WsOpen<A>` / `WfHistory` / `WfReplay`

```rust
#[must_use]
pub struct HttpCall { /* Request, Reply<Response> */ }
impl HttpCall {
    pub fn request(&self) -> &Request;
    pub fn request_mut(&mut self) -> &mut Request;
    pub fn into_request(self) -> (Request, HttpReply);
    pub fn reply(self, response: Response);
    pub fn reply_status(self, status: u16);
    pub fn reply_err(self, err: anyhow::Error);
}

#[must_use]
pub struct WsOpen<A: Actor> { /* WebSocket, Option<Request>, Reply<()>, PhantomData */ }
impl<A: Actor> WsOpen<A> {
    pub fn websocket(&self) -> &WebSocket;
    pub fn request(&self) -> Option<&Request>;
    pub fn accept(self);
    pub fn reject(self, err: anyhow::Error);
}

#[must_use]
pub struct WfHistory { /* Reply<Option<Vec<u8>>> */ }
impl WfHistory {
    pub fn reply<T: Serialize>(self, history: Option<&T>);
    pub fn reply_raw(self, bytes: Option<Vec<u8>>);
    pub fn reply_err(self, err: anyhow::Error);
}

#[must_use]
pub struct WfReplay { /* entry_id: Option<String>, Reply<Option<Vec<u8>>> */ }
impl WfReplay {
    pub fn entry_id(&self) -> Option<&str>;
    pub fn reply<T: Serialize>(self, value: Option<&T>);
    pub fn reply_raw(self, bytes: Option<Vec<u8>>);
    pub fn reply_err(self, err: anyhow::Error);
}
```

All wrapper types implement `Debug` (sufficient detail for `tracing::debug!(?event)`).

## `Ctx<A>`

```rust
pub struct Ctx<A: Actor> { inner: ActorContext, _p: PhantomData<fn() -> A> }

impl<A: Actor> Ctx<A> {
    // Identity
    pub fn actor_id(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn key(&self) -> &ActorKey;
    pub fn region(&self) -> &str;

    // Core handles (pass-through)
    pub fn kv(&self) -> &Kv;
    pub fn sql(&self) -> &SqliteDb;
    pub fn queue(&self) -> &Queue;
    pub fn schedule(&self) -> &Schedule;

    // Persistence signaling (user owns state in-loop; these just arm the debounce)
    pub fn request_save(&self, immediate: bool);
    pub fn request_save_within(&self, ms: u32);                     // US-003
    pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()>;

    // Lifecycle signaling (envoy-visible)
    pub fn sleep(&self);
    pub fn destroy(&self);
    pub fn set_prevent_sleep(&self, enabled: bool);
    pub fn prevent_sleep(&self) -> bool;
    pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static);

    // Typed broadcast + connection enumeration
    pub fn broadcast<E: Serialize>(&self, name: &str, event: &E) -> Result<()>;
    pub fn conns(&self) -> ConnIter<'_, A>;                         // US-003 iterator (lazy)
    pub fn conns_vec(&self) -> Vec<ConnCtx<A>>;                     // convenience for owned snapshot

    // Connection-surface control (US-003)
    pub async fn disconnect_conn(&self, id: &ConnId) -> Result<()>;
    pub async fn disconnect_conns<F: Fn(&ConnCtx<A>) -> bool>(&self, pred: F) -> Result<()>;

    // Alarms
    pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()>;

    // Client bridge
    pub fn client(&self) -> Result<rivetkit_client::Client>;

    // Escape hatches
    pub fn inner(&self) -> &ActorContext;
    pub fn into_inner(self) -> ActorContext;
}
```

`Ctx<A>: Clone + Send + Sync` (same as `ActorContext`, which is `Arc<ActorContextInner>`).

State is NOT on `Ctx<A>`. The user holds state in their loop:

```rust
let mut state: ChatState = s.snapshot.decode_or_default()?;
```

## Typed Broadcast

`ctx.broadcast::<E: Serialize>(name: &str, event: &E)` stays name-based in v1 for parity with core. A follow-up may add:

```rust
pub trait BroadcastSet: Serialize + 'static {
    fn event_name(&self) -> &'static str;
}
// ctx.broadcast_typed(&ChatBroadcast::Message { .. })
```

Not required to land v1.

## Wire/Serde Conventions

- All cross-language payloads use CBOR (matches the rest of the repo, matches core's contract with NAPI).
- `ciborium` is the preferred CBOR crate (already in workspace).
- `Action::decode()` uses the hand-rolled `Deserializer` described above — see the decoding notes for the unit-variant, tuple-variant, and newtype rules.
- Unit-variant action args MUST be empty bytes or CBOR null (`0xf6`). Clients that can't control encoding should use a newtype or struct variant instead.
- Tuple-variant args MUST be a CBOR array of the variant's tuple length.
- Struct-variant args MUST be a CBOR map keyed by field names.
- State, connection state, input, and broadcast event bodies use standard `ciborium::{from_reader, into_writer}`.
- Connection params decode from the raw bytes core provides (no envelope).

## Deleted Surface

The current `rivetkit` crate exposes a callback-shaped `Actor` trait with ten methods, a per-action registration builder, and a wrapped `CoreRegistry` built on the old NAPI `ActorInstanceCallbacks` / `FactoryRequest` shape. All of that goes away:

- Delete `src/bridge.rs` in full.
- Delete `src/actor.rs` (replace with new minimal `trait Actor`).
- Rewrite `src/context.rs`: keep `Ctx<A>`/`ConnCtx<A>` names and the clone/arc shape, but drop state-caching (no `A::State` field) and drop all method-on-trait hooks.
- Rewrite `src/registry.rs` to the new `Registry` + `register` / `register_with`.
- Drop `src/validation.rs` if nothing still uses `catch_unwind_result`/`encode_cbor`/`decode_cbor`; otherwise pare it down to the shared encode/decode helpers reused by the new wrappers.
- Keep the `client` re-export (`rivetkit_client as client`) and the prelude re-exports.

There is no backward-compat path. Any Rust actor written against the old trait must be ported to the new event-loop pattern.

## Public Example

```rust
use rivetkit::prelude::*;
use serde::{Deserialize, Serialize};

struct Chat;

#[derive(Default, Serialize, Deserialize)]
struct ChatState {
    messages: Vec<Msg>,
}
#[derive(Serialize, Deserialize, Clone)]
struct Msg { user: String, text: String }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChatAction {
    Send { text: String },
    History,
    Kick { user_id: String },
}

impl Actor for Chat {
    type Input = ();
    type ConnParams = String;    // username
    type ConnState = String;
    type Action = ChatAction;
}

async fn run(mut s: Start<Chat>) -> anyhow::Result<()> {
    let mut state: ChatState = s.snapshot.decode_or_default()?;

    while let Some(event) = s.events.recv().await {
        match event {
            Event::Action(a) => match a.decode() {
                Ok(ChatAction::Send { text }) => {
                    let user = a.conn()
                        .and_then(|c| c.state().ok())
                        .unwrap_or_default();
                    state.messages.push(Msg { user: user.clone(), text: text.clone() });
                    s.ctx.broadcast("message", &(user, text))?;
                    s.ctx.request_save(false);
                    a.ok(&());
                }
                Ok(ChatAction::History) => a.ok(&state.messages),
                Ok(ChatAction::Kick { user_id }) => {
                    for c in s.ctx.conns() {
                        if c.state().ok().as_deref() == Some(user_id.as_str()) {
                            let _ = c.disconnect(Some("kicked")).await;
                        }
                    }
                    a.ok(&());
                }
                Err(e) => a.err(e.into()),
            },
            Event::ConnOpen(c)          => {
                let username: String = c.params()?;
                c.accept(username);
            }
            Event::ConnClosed(_)        => {}
            Event::Subscribe(s)         => s.allow(),
            Event::SerializeState(p)    => p.save(&state),
            Event::Sleep(s)             => {
                s.ctx // ctx is owned at top-level of run, use it
                    .save_state(rivetkit::persist::state_deltas(&state)?)
                    .await?;
                s.ok();
            }
            Event::Destroy(d) => {
                s.ctx.save_state(rivetkit::persist::state_deltas(&state)?).await?;
                d.ok();
            }
            Event::Http(h)              => h.reply_status(404),
            Event::WebSocketOpen(w)     => w.reject(anyhow!("no websocket support")),
            Event::WorkflowHistory(h)   => h.reply_raw(None),
            Event::WorkflowReplay(r)    => r.reply_raw(None),
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut reg = Registry::new();
    reg.register::<Chat>("chat", run);
    reg.serve().await
}
```

Notes on the example:
- The user's `ctx` is `s.ctx`, cloneable; the Sleep/Destroy arms read it directly.
- `Subscribe::allow` and `WsOpen::reject` are called explicitly — a blanket `_ => {}` would trigger `actor/dropped_reply` on those variants and the Drop warning.
- Alarm-originated actions land in the `Send` arm with `a.conn()` returning `None`; the default username fallback keeps that path working.

## Test Strategy

- Inline `#[cfg(test)]` unit tests per wrapper type covering CBOR round-trips for each serde shape (unit / newtype / tuple / struct variants for `Action::decode`, normal Serialize+Deserialize for `Input`, `ConnParams`, `ConnState`, `Snapshot`).
- Drop-guard tests: construct each wrapper, drop without replying, assert the underlying `Reply<T>` sent `actor/dropped_reply` and the wrapper's `Drop` logged the warning.
- Integration test using a canned `ActorStart` (pumped with a hand-built `mpsc::Sender<ActorEvent>`) that drives the example `run` through a short scripted sequence and asserts replies decode correctly.
- An example actor under `rivetkit-rust/packages/rivetkit/examples/chat.rs` that runs against a local engine (behind `--example chat` and a comment noting the engine requirement). Not part of CI.

## Open Questions

1. Should `type Input` stay on the trait or become use-site generic? Kept for symmetry with `ConnParams`/`ConnState`/`Action` and because `Start<A>::input` autocompletes `A::Input` via `decode()`, but easy to strip if we want a minimum-trait.
2. Should we ship a `BroadcastSet` trait in v1 or defer? Deferred above; revisit once the first real user actor lands.
3. `ctx.keep_awake(promise)`-equivalent helper in v1? `ctx.wait_until` already exists on core; leaving typed variants out of v1 unless a user hits a case it can't cover.

## Out-of-Band Consumer: `open-artifacts`

`~/open-artifacts` (outside this repo) currently depends on an older version of `rivetkit` living in `~/r5`. After this rewrite lands, `open-artifacts` needs its dependencies repointed at `~/r6`'s new typed event-loop API. This is tracked as a follow-up story in `scripts/ralph/prd.json`.
