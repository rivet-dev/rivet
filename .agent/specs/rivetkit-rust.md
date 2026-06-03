# RivetKit Rust SDK Spec

## Overview

Two-layer Rust SDK for writing Rivet Actors in Rust, mirroring the TypeScript RivetKit lifecycle 1:1. Everything except workflows moves to Rust.

- **`rivetkit-core`** — Dynamic, language-agnostic crate. All lifecycle logic lives here. TypeScript (via NAPI) and Rust both call into this. Callbacks are closures with named param structs. All data is opaque bytes (CBOR at boundaries). The primary value: lifecycle state machine, sleep logic, shutdown sequencing, state persistence, action dispatch, event broadcast, queue management, and schedule system are implemented once in Rust and shared across language runtimes.
- **`rivetkit`** — Typed Rust-native wrapper. `Actor` trait, `Registry` builder, ergonomic context types. Thin layer that delegates to `rivetkit-core`.

Both crates sit on top of the existing `envoy-client` (`engine/sdks/rust/envoy-client/`), which handles the wire protocol (BARE serialization, WebSocket to engine, KV request/response matching, SQLite protocol dispatch, tunnel routing).

The only thing remaining in TypeScript is workflows. The ~65KB `ActorInstance` class is replaced by calls into `rivetkit-core`.

## Package Locations

- `rivetkit-rust/packages/rivetkit-core/` — core crate
- `rivetkit-rust/packages/rivetkit/` — high-level crate
- `rivetkit-typescript/packages/rivetkit-napi/` — NAPI bridge (renamed from `rivetkit-native`)

## Goals

1. Mirror the TypeScript actor lifecycle exactly (same hooks, same sleep behavior, same shutdown sequencing).
2. Enable TypeScript to call into `rivetkit-core` via NAPI, moving lifecycle logic from TS to Rust.
3. Provide an ergonomic Rust-native API via `rivetkit` for writing actors purely in Rust.
4. CBOR serialization at all boundaries (actions, events, state, queues, connections) for cross-language compatibility.
5. KV API must be stable. No breaking ABI changes.

---

## rivetkit-core API

### Two-Phase Actor Construction

Actors are constructed in two phases because the full set of instance callbacks (actions, lifecycle hooks) can only be wired up after the actor instance exists.

```rust
/// Stored in the registry. Knows how to create an actor instance.
struct ActorFactory {
    config: ActorConfig,
    /// Creates an ActorInstanceCallbacks. Called once per actor lifecycle (start or wake).
    create: Box<dyn Fn(FactoryRequest) -> BoxFuture<'static, Result<ActorInstanceCallbacks>> + Send + Sync>,
}

struct FactoryRequest {
    pub ctx: ActorContext,
    pub input: Option<Vec<u8>>,   // CBOR-encoded input (None if waking from sleep)
    pub is_new: bool,             // true = first boot, false = wake from sleep
}
```

### ActorInstanceCallbacks

All callbacks for a running actor instance. Closures capture the actor instance (via `Arc`) so all futures are `'static`.

```rust
struct ActorInstanceCallbacks {
    // Lifecycle
    on_wake: Option<Box<dyn Fn(OnWakeRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
    on_sleep: Option<Box<dyn Fn(OnSleepRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
    on_destroy: Option<Box<dyn Fn(OnDestroyRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
    on_state_change: Option<Box<dyn Fn(OnStateChangeRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,

    // Network
    on_request: Option<Box<dyn Fn(OnRequestRequest) -> BoxFuture<'static, Result<Response>> + Send + Sync>>,
    on_websocket: Option<Box<dyn Fn(OnWebSocketRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,

    // Connections
    on_before_connect: Option<Box<dyn Fn(OnBeforeConnectRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
    on_connect: Option<Box<dyn Fn(OnConnectRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
    on_disconnect: Option<Box<dyn Fn(OnDisconnectRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,

    // Actions (dynamic dispatch by name)
    actions: HashMap<String, Box<dyn Fn(ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>>,

    // Action response transform hook
    on_before_action_response: Option<Box<dyn Fn(OnBeforeActionResponseRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>>,

    // Background work
    run: Option<Box<dyn Fn(RunRequest) -> BoxFuture<'static, Result<()>> + Send + Sync>>,
}
```

### Request Types

```rust
struct OnWakeRequest { pub ctx: ActorContext }
struct OnSleepRequest { pub ctx: ActorContext }
struct OnDestroyRequest { pub ctx: ActorContext }
struct OnStateChangeRequest { pub ctx: ActorContext, pub new_state: Vec<u8> }
struct OnRequestRequest { pub ctx: ActorContext, pub request: Request }
struct OnWebSocketRequest { pub ctx: ActorContext, pub ws: WebSocket }
struct OnBeforeConnectRequest { pub ctx: ActorContext, pub params: Vec<u8> }
struct OnConnectRequest { pub ctx: ActorContext, pub conn: ConnHandle }
struct OnDisconnectRequest { pub ctx: ActorContext, pub conn: ConnHandle }
struct ActionRequest { pub ctx: ActorContext, pub conn: ConnHandle, pub name: String, pub args: Vec<u8> }
struct OnBeforeActionResponseRequest { pub ctx: ActorContext, pub name: String, pub args: Vec<u8>, pub output: Vec<u8> }
struct RunRequest { pub ctx: ActorContext }
```

### ActorContext

Internally `Arc`-backed. `Clone` is safe. All clones share the same runtime state.

```rust
impl ActorContext {
    // State (CBOR-encoded bytes)
    fn state(&self) -> Vec<u8>;
    fn set_state(&self, state: Vec<u8>);
    async fn save_state(&self, opts: SaveStateOpts) -> Result<()>;

    // Vars (transient, not persisted, recreated every start)
    fn vars(&self) -> Vec<u8>;
    fn set_vars(&self, vars: Vec<u8>);

    // KV
    fn kv(&self) -> &Kv;

    // SQLite
    fn sql(&self) -> &SqliteDb;

    // Schedule (dispatches to actions)
    fn schedule(&self) -> &Schedule;

    // Queue
    fn queue(&self) -> &Queue;

    // Events
    fn broadcast(&self, name: &str, args: &[u8]);

    // Connections
    fn conns(&self) -> Vec<ConnHandle>;

    // Actor-to-actor client
    fn client(&self) -> &Client;

    // Sleep control
    fn sleep(&self);          // Defers to next tick. Does NOT fire abort signal.
    fn destroy(&self);        // Defers to next tick. Fires abort signal immediately.
    fn set_prevent_sleep(&self, prevent: bool);
    fn prevent_sleep(&self) -> bool;

    // Background work tracking
    fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static);

    // Actor info
    fn actor_id(&self) -> &str;
    fn name(&self) -> &str;
    fn key(&self) -> &ActorKey;
    fn region(&self) -> &str;

    // Shutdown
    fn abort_signal(&self) -> &CancellationToken;
    fn aborted(&self) -> bool;
}

struct SaveStateOpts {
    pub immediate: bool,
}

type ActorKey = Vec<ActorKeySegment>;
enum ActorKeySegment {
    String(String),
    Number(f64),
}
```

### State Persistence

Core manages state persistence with dirty tracking and throttled saves.

- `set_state(bytes)` marks dirty, schedules throttled save.
- Throttle: `max(0, save_interval - time_since_last_save)`.
- `save_state({ immediate: true })` bypasses throttle.
- On shutdown: flush all pending saves.
- `on_state_change` fires after `set_state`. Not called during init or from within itself (prevents recursion). Errors logged, not fatal.

Persisted format (BARE-encoded in KV):
```rust
struct PersistedActor {
    input: Option<Vec<u8>>,
    has_initialized: bool,
    state: Vec<u8>,
    scheduled_events: Vec<PersistedScheduleEvent>,
}
```

Config: `state_save_interval: Duration` (default: 1s).

### Vars (Transient State)

Vars are non-persisted ephemeral state, recreated on every start (both new and wake). Useful for caches, computed values, runtime handles.

- `vars()` / `set_vars()` on `ActorContext`, opaque bytes like state.
- The `ActorFactory::create` callback is responsible for initializing vars.
- Lost on sleep. Recreated via the factory on next wake.

Config: `create_vars_timeout: Duration` (default: 5s).

### Actions

Actions are string-keyed RPC handlers. Args and return values are CBOR-encoded bytes.

```rust
// In ActorInstanceCallbacks:
actions: HashMap<String, Box<dyn Fn(ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>>
```

Action dispatch flow:
1. Client sends `ActionRequest { id, name, args }` over connection.
2. Core looks up handler by name.
3. Wraps with `action_timeout` deadline.
4. On success: send `ActionResponse { id, output }`.
5. If `on_before_action_response` is set, call it to transform output before sending.
6. On error: send error response with group/code/message.
7. After completion: trigger throttled state save.

Config: `action_timeout: Duration` (default: 60s).

### Schedule (dispatches to actions)

Matches TS behavior: schedule calls invoke actions by name.

```rust
impl Schedule {
    // Schedule an action invocation. Fire-and-forget (matches TS void return).
    // Errors in persistence are logged, not returned.
    fn after(&self, duration: Duration, action_name: &str, args: &[u8]);
    fn at(&self, timestamp_ms: i64, action_name: &str, args: &[u8]);
}

struct PersistedScheduleEvent {
    pub event_id: String,       // UUID (internal, for dedup)
    pub timestamp_ms: i64,
    pub action: String,
    pub args: Vec<u8>,          // CBOR-encoded
}
```

Cancellation/introspection (`cancel`, `next_event`, `all_events`, `clear_all`) are internal to the `ScheduleManager`, not exposed on the public `Schedule` API. This matches TS where `Schedule` only has `after` and `at`.

Flow:
1. Actor calls `ctx.schedule().after(duration, "action_name", args)`.
2. Core creates event, inserts sorted, persists to KV.
3. Core sends `EventActorSetAlarm { alarm_ts: soonest }` to engine.
4. On alarm: find events with `timestamp_ms <= now`, execute each via `invoke_action_by_name`. Each wrapped in `internal_keep_awake`. Events removed after execution (at-most-once).
5. Events survive sleep/wake.

### Events/Broadcast

```rust
impl ActorContext {
    fn broadcast(&self, name: &str, args: &[u8]);  // CBOR-encoded args
}

impl ConnHandle {
    fn send(&self, name: &str, args: &[u8]);  // To single connection
}
```

Core tracks event subscriptions per connection.

### Connections

```rust
impl ConnHandle {
    fn id(&self) -> &str;                     // UUID
    fn params(&self) -> Vec<u8>;              // CBOR-encoded
    fn state(&self) -> Vec<u8>;               // CBOR-encoded
    fn set_state(&self, state: Vec<u8>);
    fn is_hibernatable(&self) -> bool;
    fn send(&self, event_name: &str, args: &[u8]);
    async fn disconnect(&self, reason: Option<&str>) -> Result<()>;
}

type ConnId = String;
```

Connection lifecycle:
1. Client connects. Core calls `on_before_connect(params)`. Rejection on error.
2. Create connection state (via factory or default).
3. Core calls `on_connect(conn)`.
4. On disconnect: remove from tracking, clear subscriptions, call `on_disconnect(conn)`.
5. Hibernatable connections: persisted to KV on sleep, restored on wake.

Config:
- `on_before_connect_timeout: Duration` (default: 5s)
- `on_connect_timeout: Duration` (default: 5s)
- `create_conn_state_timeout: Duration` (default: 5s)

### Queues

```rust
impl Queue {
    // Enqueue a message.
    async fn send(&self, name: &str, body: &[u8]) -> Result<()>;

    // Blocking receive. Returns None on timeout.
    async fn next(&self, opts: QueueNextOpts) -> Result<Option<QueueMessage>>;

    // Batch receive.
    async fn next_batch(&self, opts: QueueNextBatchOpts) -> Result<Vec<QueueMessage>>;

    // Non-blocking variants.
    fn try_next(&self, opts: QueueTryNextOpts) -> Option<QueueMessage>;
    fn try_next_batch(&self, opts: QueueTryNextBatchOpts) -> Vec<QueueMessage>;
}

struct QueueNextOpts {
    pub names: Option<Vec<String>>,
    pub timeout: Option<Duration>,
    pub signal: Option<CancellationToken>,
    pub completable: bool,
}

struct QueueNextBatchOpts {
    pub names: Option<Vec<String>>,
    pub count: u32,
    pub timeout: Option<Duration>,
    pub signal: Option<CancellationToken>,
    pub completable: bool,
}

struct QueueTryNextOpts {
    pub names: Option<Vec<String>>,
    pub completable: bool,
}

struct QueueTryNextBatchOpts {
    pub names: Option<Vec<String>>,
    pub count: u32,
    pub completable: bool,
}

// Non-completable message. Returned when completable=false.
struct QueueMessage {
    pub id: u64,
    pub name: String,
    pub body: Vec<u8>,         // CBOR-encoded
    pub created_at: i64,
}

// Completable message. Returned when completable=true.
// Must call complete() before next receive. Enforced at runtime.
struct CompletableQueueMessage {
    pub id: u64,
    pub name: String,
    pub body: Vec<u8>,
    pub created_at: i64,
    completion: CompletionHandle,
}

impl CompletableQueueMessage {
    fn complete(self, response: Option<Vec<u8>>) -> Result<()>;
}
```

Queue persistence: messages stored in KV with auto-incrementing IDs. Metadata (next_id, size) stored separately.

Sleep interaction: `active_queue_wait_count` tracks callers blocked on `next()`. The `can_sleep()` check allows sleep if the run handler is only blocked on a queue wait.

Config:
- `max_queue_size: u32` (default: 1000)
- `max_queue_message_size: u32` (default: 65536)

### WebSocket

Callback-based API matching envoy-client's `WebSocketHandler`.

```rust
struct WebSocket { /* internal */ }

impl WebSocket {
    pub fn send(&self, msg: WsMessage);
    pub fn close(&self, code: Option<u16>, reason: Option<String>);
}

enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}
```

### KV

Stable API. No breaking changes.

```rust
impl Kv {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &[u8]) -> Result<()>;
    async fn delete_range(&self, start: &[u8], end: &[u8]) -> Result<()>;
    async fn list_prefix(&self, prefix: &[u8], opts: ListOpts) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    async fn list_range(&self, start: &[u8], end: &[u8], opts: ListOpts) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>>;
    async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()>;
    async fn batch_delete(&self, keys: &[&[u8]]) -> Result<()>;
}

struct ListOpts {
    pub reverse: bool,
    pub limit: Option<u32>,
}
```

### Registry (core level)

```rust
struct CoreRegistry {
    factories: HashMap<String, ActorFactory>,
}

impl CoreRegistry {
    fn new() -> Self;
    fn register(&mut self, name: &str, factory: ActorFactory);
    async fn serve(self) -> Result<()>;
}
```

`serve()` creates a single `EnvoyCallbacks` dispatcher:
1. On `on_actor_start`: extract name from `protocol::ActorConfig`, look up `ActorFactory`, call `factory.create(...)` to get `ActorInstanceCallbacks`, store in `scc::HashMap<(actor_id, generation), ActorInstanceCallbacks>`.
2. Route `fetch`, `websocket`, etc. to the correct instance callbacks.

### Actor Config

All timeouts use `Duration`.

```rust
struct ActorConfig {
    // Display
    pub name: Option<String>,
    pub icon: Option<String>,

    // WebSocket hibernation
    pub can_hibernate_websocket: bool,                    // default: false

    // State persistence
    pub state_save_interval: Duration,                    // default: 1s

    // Lifecycle timeouts
    pub create_vars_timeout: Duration,                    // default: 5s
    pub create_conn_state_timeout: Duration,              // default: 5s
    pub on_before_connect_timeout: Duration,              // default: 5s
    pub on_connect_timeout: Duration,                     // default: 5s
    pub on_sleep_timeout: Duration,                       // default: 5s
    pub on_destroy_timeout: Duration,                     // default: 5s
    pub action_timeout: Duration,                         // default: 60s
    pub run_stop_timeout: Duration,                       // default: 15s

    // Sleep behavior
    pub sleep_timeout: Duration,                          // default: 30s
    pub no_sleep: bool,                                   // default: false
    pub sleep_grace_period: Option<Duration>,             // default: None

    // Connection liveness
    pub connection_liveness_timeout: Duration,            // default: 2.5s
    pub connection_liveness_interval: Duration,           // default: 5s

    // Queue limits
    pub max_queue_size: u32,                              // default: 1000
    pub max_queue_message_size: u32,                      // default: 65536

    // Preload budgets
    pub preload_max_workflow_bytes: Option<u64>,
    pub preload_max_connections_bytes: Option<u64>,

    // Driver overrides (driver can cap these by taking min)
    pub overrides: Option<ActorConfigOverrides>,
}

struct ActorConfigOverrides {
    pub sleep_grace_period: Option<Duration>,
    pub on_sleep_timeout: Option<Duration>,
    pub on_destroy_timeout: Option<Duration>,
    pub run_stop_timeout: Option<Duration>,
}
```

`sleep_grace_period` fallback (mirrors TS):
- If explicitly set: use it (capped by override if present).
- If `on_sleep_timeout` was explicitly customized from its default: `effective_on_sleep_timeout + 15s`.
- Otherwise: 15s (DEFAULT_SLEEP_GRACE_PERIOD).

---

## rivetkit (High-Level Rust API)

### Actor Trait

All async methods return `impl Future + Send + 'static`. The actor instance is stored as `Arc<A>` internally. Each method receives `self: &Arc<Self>` so the returned future is `'static` and can be boxed for the core callbacks. All methods receive `&Ctx<Self>` (typed context), not the raw core `ActorContext`.

```rust
#[async_trait]
trait Actor: Send + Sync + Sized + 'static {
    type State: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;
    type ConnParams: DeserializeOwned + Send + Sync + 'static;
    type ConnState: Serialize + DeserializeOwned + Send + Sync + 'static;
    type Input: DeserializeOwned + Send + Sync + 'static;
    type Vars: Send + Sync + 'static;

    // State initialization (called on first boot only, before on_create)
    async fn create_state(ctx: &Ctx<Self>, input: &Self::Input) -> Result<Self::State>;

    // Vars initialization (called on every start, both new and wake)
    async fn create_vars(ctx: &Ctx<Self>) -> Result<Self::Vars> {
        // Default impl only available when Vars = ()
        unimplemented!("must implement create_vars if Vars is not ()")
    }

    // Connection state initialization (called per connection, after actor exists)
    async fn create_conn_state(self: &Arc<Self>, ctx: &Ctx<Self>, params: &Self::ConnParams) -> Result<Self::ConnState>;

    // Construction (called once on first boot, after state + vars init)
    async fn on_create(ctx: &Ctx<Self>, input: &Self::Input) -> Result<Self>;

    // Called on every start (new AND wake), after vars init
    async fn on_wake(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> { Ok(()) }

    async fn on_sleep(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> { Ok(()) }
    async fn on_destroy(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> { Ok(()) }
    async fn on_state_change(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> { Ok(()) }

    // Network
    async fn on_request(self: &Arc<Self>, ctx: &Ctx<Self>, request: Request) -> Result<Response> {
        Ok(Response::not_found())
    }
    async fn on_websocket(self: &Arc<Self>, ctx: &Ctx<Self>, ws: WebSocket) -> Result<()> { Ok(()) }

    // Connections
    async fn on_before_connect(self: &Arc<Self>, ctx: &Ctx<Self>, params: &Self::ConnParams) -> Result<()> { Ok(()) }
    async fn on_connect(self: &Arc<Self>, ctx: &Ctx<Self>, conn: ConnCtx<Self>) -> Result<()> { Ok(()) }
    async fn on_disconnect(self: &Arc<Self>, ctx: &Ctx<Self>, conn: ConnCtx<Self>) -> Result<()> { Ok(()) }

    // Background work
    async fn run(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> { Ok(()) }

    fn config() -> ActorConfig { ActorConfig::default() }
}
```

### `Ctx<A>` — Typed Actor Context

`Ctx<A>` is a high-level typed wrapper around the core `ActorContext`. It is NOT the same type. It provides cached state deserialization, typed vars, typed connections, and typed event serialization.

```rust
struct Ctx<A: Actor> {
    inner: ActorContext,
    state_cache: Arc<Mutex<Option<Arc<A::State>>>>,
    vars: Arc<A::Vars>,
}

impl<A: Actor> Ctx<A> {
    /// Returns cached deserialized state. Cache populated on first access,
    /// invalidated by set_state.
    fn state(&self) -> Arc<A::State>;

    /// Serializes to CBOR, updates core, invalidates cache, marks dirty.
    fn set_state(&self, state: &A::State);

    /// Typed vars (concrete, not serialized). Transient, recreated each start.
    fn vars(&self) -> &A::Vars;

    // Delegates to core ActorContext
    fn kv(&self) -> &Kv;
    fn sql(&self) -> &SqliteDb;
    fn schedule(&self) -> &Schedule;
    fn queue(&self) -> &Queue;
    fn client(&self) -> &Client;
    fn actor_id(&self) -> &str;
    fn name(&self) -> &str;
    fn key(&self) -> &ActorKey;
    fn region(&self) -> &str;
    fn abort_signal(&self) -> &CancellationToken;
    fn aborted(&self) -> bool;
    fn sleep(&self);
    fn destroy(&self);
    fn set_prevent_sleep(&self, prevent: bool);
    fn prevent_sleep(&self) -> bool;
    fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static);

    // Typed event broadcast
    fn broadcast<E: Serialize>(&self, name: &str, event: &E);

    // Typed connections
    fn conns(&self) -> Vec<ConnCtx<A>>;
}

/// Typed connection handle. Wraps core ConnHandle with CBOR serde.
struct ConnCtx<A: Actor> {
    inner: ConnHandle,
    _phantom: PhantomData<A>,
}

impl<A: Actor> ConnCtx<A> {
    fn id(&self) -> &str;
    fn params(&self) -> A::ConnParams;          // Deserializes from CBOR
    fn state(&self) -> A::ConnState;            // Deserializes from CBOR
    fn set_state(&self, state: &A::ConnState);  // Serializes to CBOR
    fn is_hibernatable(&self) -> bool;
    fn send<E: Serialize>(&self, name: &str, event: &E);
    async fn disconnect(&self, reason: Option<&str>) -> Result<()>;
}
```

### Action Registration

Actions registered via builder. Uses closures (not `fn` pointers) to support `async fn`.

```rust
impl Registry {
    fn register<A: Actor>(&mut self, name: &str) -> ActorRegistration<A>;
}

struct ActorRegistration<'a, A: Actor> { /* ... */ }

impl<'a, A: Actor> ActorRegistration<'a, A> {
    fn action<Args, Ret, F, Fut>(
        &mut self,
        name: &str,
        handler: F,
    ) -> &mut Self
    where
        Args: DeserializeOwned + Send + 'static,
        Ret: Serialize + Send + 'static,
        F: Fn(Arc<A>, Ctx<A>, Args) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Ret>> + Send + 'static;

    fn done(&mut self) -> &mut Registry;
}
```

The bridge clones `Arc<A>` and moves it into each action closure. CBOR deserialization of `Args` and serialization of `Ret` handled automatically.

### Registry

```rust
struct Registry { /* wraps CoreRegistry */ }

impl Registry {
    fn new() -> Self;
    fn register<A: Actor>(&mut self, name: &str) -> ActorRegistration<A>;
    async fn serve(self) -> Result<()>;
}
```

### Usage Example

```rust
use rivetkit::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Serialize, Deserialize, Clone)]
struct CounterState { count: i64 }

struct Counter {
    request_count: AtomicU64,
}

#[async_trait]
impl Actor for Counter {
    type State = CounterState;
    type ConnParams = ();
    type ConnState = ();
    type Input = ();
    type Vars = ();

    async fn create_state(_ctx: &Ctx<Self>, _input: &()) -> Result<CounterState> {
        Ok(CounterState { count: 0 })
    }

    async fn on_create(ctx: &Ctx<Self>, _input: &()) -> Result<Self> {
        ctx.sql().exec(
            "CREATE TABLE IF NOT EXISTS log (id INTEGER PRIMARY KEY, action TEXT)",
            [],
        ).await?;
        Ok(Self { request_count: AtomicU64::new(0) })
    }

    async fn on_request(self: &Arc<Self>, ctx: &Ctx<Self>, request: Request) -> Result<Response> {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        let state = ctx.state(); // Arc<CounterState>, cached
        Ok(Response::json(&serde_json::json!({ "count": state.count })))
    }

    async fn run(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
        loop {
            tokio::select! {
                _ = ctx.abort_signal().cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                    ctx.schedule().after(Duration::ZERO, "cleanup", &[]);
                }
            }
        }
        Ok(())
    }
}

impl Counter {
    async fn increment(self: Arc<Self>, ctx: Ctx<Self>, args: (i64,)) -> Result<CounterState> {
        let (amount,) = args;
        let mut state = (*ctx.state()).clone(); // Clone out of Arc to mutate
        state.count += amount;
        ctx.set_state(&state);
        ctx.broadcast("count_changed", &state)?;
        Ok(state)
    }

    async fn get_count(self: Arc<Self>, ctx: Ctx<Self>, _args: ()) -> Result<i64> {
        Ok(ctx.state().count)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut registry = Registry::new();
    registry.register::<Counter>("counter")
        .action("increment", Counter::increment)
        .action("get_count", Counter::get_count)
        .done();
    registry.serve().await
}
```

---

## Actor Lifecycle State Machine

### States

```
Creating -> Ready -> Started -> Sleeping/Destroying -> Stopped
```

### Startup Sequence

1. Load persisted data from KV (or preload). Includes `PersistedActor` with state, scheduled events.
2. Determine create-vs-wake: check `has_initialized` in persisted data.
3. Call `ActorFactory::create(FactoryRequest { is_new, input, ctx })`.
   - For the high-level crate, this calls `create_state` (if new) + `create_vars` + `on_create` (if new), then builds `ActorInstanceCallbacks` wired to the `Arc<A>`.
4. If factory fails: report `ActorStateStopped(Error)`, actor dead.
5. Set `has_initialized = true`, persist.
6. Call `on_wake` (always, both new and restored).
7. Initialize alarms: resync schedule with engine via `EventActorSetAlarm`.
8. Restore hibernating connections from KV.
9. Mark `ready = true`.
10. Driver hook: `onBeforeActorStart`.
11. Mark `started = true`.
12. Reset sleep timer.
13. Start `run` handler in background task.
14. Fire abort signal (entered shutdown context).
15. Process overdue scheduled events immediately.

Note: step 14 is clarification that the abort signal fires at the beginning of `onStop` for BOTH sleep and destroy modes (matches TS `mod.ts:970`). The difference is that `destroy()` fires abort early (on user call), while `sleep()` only fires it when `onStop` begins.

### Graceful Shutdown: Sleep Mode

1. Clear sleep timeout.
2. Cancel local alarm timeouts (events remain persisted).
3. Fire abort signal (if not already fired).
4. Wait for `run` handler (with `run_stop_timeout`).
5. Calculate `shutdown_deadline` from effective `sleep_grace_period`.
6. Wait for idle sleep window (with deadline):
   - No active HTTP requests
   - No active `keep_awake` / `internal_keep_awake` regions
   - No pending disconnect callbacks
   - No active WebSocket callbacks
7. Call `on_sleep` (with remaining deadline budget).
8. Wait for shutdown tasks: `wait_until` futures, WebSocket callback futures, `prevent_sleep` to clear.
9. Disconnect all non-hibernatable connections.
10. Wait for shutdown tasks again.
11. Save state immediately. Wait for all pending KV/SQLite writes.
12. Cleanup database connections.
13. Report `ActorStateStopped(Ok)`.

### Graceful Shutdown: Destroy Mode

Destroy does NOT wait for idle sleep window.

1. Clear sleep timeout.
2. Cancel local alarm timeouts.
3. Fire abort signal (already fired on `destroy()` call).
4. Wait for `run` handler (with `run_stop_timeout`).
5. Call `on_destroy` (with standalone `on_destroy_timeout`).
6. Wait for shutdown tasks.
7. Disconnect all connections.
8. Wait for shutdown tasks again.
9. Save state. Wait for pending writes.
10. Cleanup database connections.
11. Report `ActorStateStopped(Ok)`.

### Sleep Readiness (`can_sleep()`)

ALL must be true:
- `ready` AND `started`
- `prevent_sleep` is false
- `no_sleep` config is false
- No active HTTP requests
- No active `keep_awake` / `internal_keep_awake` regions
- Run handler not active (exception: allowed if only blocked on queue wait)
- No active connections
- No pending disconnect callbacks
- No active WebSocket callbacks

### Error Handling

- **Factory/`on_create` error**: `ActorStateStopped(Error)`. Actor dead.
- **`on_wake` error**: Same. Actor dead.
- **`on_sleep` / `on_destroy` error**: Logged. Shutdown continues. `ActorStateStopped(Error)`.
- **`on_request` error**: HTTP 500 to caller.
- **`on_websocket` error**: Logged, connection closed.
- **Action error**: Error response to client with group/code/message.
- **`on_state_change` error**: Logged. Not fatal.
- **Schedule event error**: Logged. Event removed (at-most-once). Subsequent events continue.
- **`run` handler error/panic**: Logged. Actor stays alive. Panics caught via `catch_unwind`.
- **`on_before_action_response` error**: Logged. Original output sent as-is.

---

## Envoy-Client Integration

### Required changes (BLOCKING)

1. **In-flight HTTP request visibility**: Detached `tokio::spawn` at `actor.rs:343` drops `JoinHandle`. Core needs an in-flight counter or `JoinSet` per actor for `can_sleep()`.

2. **Graceful shutdown in `on_actor_stop`**: `handle_stop` calls `on_actor_stop` then immediately sends `Stopped` and breaks (`actor.rs:198-199`). Core needs the loop to continue during teardown. `Stopped` must only be sent after core signals completion.

3. **HTTP request lifecycle**: Spawned tasks can outlive actor. Must store `JoinHandle`s and abort/join during shutdown.

### What already works (no changes)
- KV: 100% coverage
- SQLite V2: 100%
- Hibernating WS restore: full
- Sleep/destroy: `EventActorIntent`
- Alarm: `EventActorSetAlarm`
- Multiple actors per process

---

## Proposed Module Structure

### rivetkit-core

```
rivetkit-rust/packages/rivetkit-core/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API re-exports
    ├── actor/
    │   ├── mod.rs                # ActorInstance orchestrator (owns the lifecycle loop)
    │   ├── factory.rs            # ActorFactory, FactoryRequest
    │   ├── callbacks.rs          # ActorInstanceCallbacks, all request/response types
    │   ├── config.rs             # ActorConfig, ActorConfigOverrides, defaults
    │   ├── context.rs            # ActorContext (Arc inner, Clone)
    │   ├── lifecycle.rs          # Startup + shutdown sequences (sleep + destroy)
    │   ├── state.rs              # State dirty tracking, throttled save, PersistedActor
    │   ├── vars.rs               # Vars (transient, recreated each start)
    │   ├── sleep.rs              # can_sleep(), auto-sleep timer, prevent_sleep, keep_awake, internal_keep_awake
    │   ├── schedule.rs           # Schedule API, PersistedScheduleEvent, alarm sync, invoke_action_by_name
    │   ├── action.rs             # Action dispatch, timeout wrapping, on_before_action_response
    │   ├── connection.rs         # ConnHandle, lifecycle hooks, hibernation persistence, subscription tracking
    │   ├── event.rs              # Broadcast + per-connection send
    │   └── queue.rs              # Queue: send, next, nextBatch, tryNext, completable, persistence
    ├── kv.rs                     # Kv wrapper
    ├── sqlite.rs                 # SqliteDb wrapper
    ├── websocket.rs              # WebSocket (callback-based)
    ├── registry.rs               # CoreRegistry, EnvoyCallbacks dispatcher
    └── types.rs                  # ActorKey, ConnId, WsMessage, shared enums
```

### rivetkit (high-level)

```
rivetkit-rust/packages/rivetkit/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API
    ├── prelude.rs                # Common imports
    ├── actor.rs                  # Actor trait, associated types
    ├── context.rs                # Ctx<A>, ConnCtx<A>, state caching
    ├── registry.rs               # Registry, ActorRegistration, action builder
    └── bridge.rs                 # Factory construction: Actor -> ActorFactory + ActorInstanceCallbacks
```

### Dependency chain

```
envoy-client (wire protocol, BARE, WebSocket to engine)
    ^
    |
rivetkit-core (lifecycle, state, actions, events, queues, connections, schedule)
    ^                          ^
    |                          |
rivetkit                   rivetkit-napi
(Actor trait, Ctx<A>,      (NAPI bridge, ThreadsafeFunction wrappers,
 registry, CBOR bridge)     ActorContext as #[napi] class, JS<->Rust callbacks)
```

`rivetkit-napi` (renamed from `rivetkit-native`) is the NAPI bridge that wires `rivetkit-core` callbacks to JavaScript. It replaces the existing `rivetkit-native` package. The rename reflects that its sole purpose is the NAPI boundary, not "native" actor functionality.

---

## Concerns

### 1. envoy-client shutdown is the critical blocker
The entire graceful shutdown sequence depends on envoy-client allowing multi-step teardown. Currently it sends `Stopped` immediately. This must be fixed first.

### 2. NAPI bridge is ~800-1200 lines, not 200
The bridge needs to expose `ActorContext` and all sub-objects (`Kv`, `SqliteDb`, `Schedule`, `Queue`, `ConnHandle`, `WebSocket`) as NAPI classes with method bindings. Each callback type needs a `ThreadsafeFunction` wrapper. The existing NAPI layer (`bridge_actor.rs` + `envoy_handle.rs` + `database.rs` ~1430 lines) is a complete rewrite, not an incremental addition.

Key challenges:
- `ActorContext` is a Rust object with `Arc` internals. Must be wrapped as a `#[napi]` class so JS can call methods on it.
- Every `kv.get()` call from JS crosses the NAPI boundary twice (JS->Rust for the method, Rust->envoy for the actual op).
- `wait_until` from JS needs Promise-to-Future conversion (not natively supported by napi-rs, requires custom plumbing).
- `CancellationToken` / `abort_signal` needs an `on_cancelled(callback)` bridge for JS.
- `run` callback produces a long-lived Promise. Cancellation requires cooperative checking in JS.

### 3. State change tracking differs between TS and Rust
- TS: `Proxy`-based auto-detection (`c.state.count++` triggers save)
- Rust: explicit `ctx.set_state(new_state)` call
- Core treats both the same (receives bytes, marks dirty)
- NAPI bridge: JS Proxy handler calls Rust `set_state` on mutation

### 4. CBOR compatibility
Both crates need a CBOR library. Rust: `ciborium`. TS: `cbor-x`. Must produce byte-compatible output. Validate early with cross-language round-trip tests.

### 5. Queue async iteration
TS has `async *iter()`. Rust has no native async generators. Users loop with `while let Some(msg) = queue.next(opts).await`. A `Stream` adapter in the high-level crate is optional.

### 6. Double NAPI boundary crossing
When TS calls a user's `onConnect` handler: Rust calls JS (callback via TSFN) -> JS does user logic -> JS calls Rust (kv.get via NAPI method) -> Rust calls envoy-client -> response returns through all layers. This adds latency vs the current architecture where everything stays in JS. Benchmark this early.

### 7. Inspector system
The TS inspector system includes: inspector token generation and KV persistence, WebSocket-based inspector protocol, HTTP inspector endpoints (mirrored from WebSocket), state change events, connection update events, queue size tracking, and OpenTelemetry trace spans. This must be implemented in `rivetkit-core` so both Rust and TS actors get inspector support. The inspector is deeply integrated into the lifecycle (state changes, connection updates, action invocations all emit inspector events). Implementation should happen after the core lifecycle is stable but before GA.

### 8. `canHibernateWebSocket` function variant
TS allows `boolean | (request: Request) => boolean` for per-request hibernation decisions. Rust core currently uses `bool` only. To reach full parity, add a callback variant to `ActorConfig`:
```rust
pub can_hibernate_websocket: CanHibernateWebSocket,

enum CanHibernateWebSocket {
    Bool(bool),
    Callback(Box<dyn Fn(&Request) -> bool + Send + Sync>),
}
```

### 9. Queue `Stream` adapter
TS has `async *iter()` for queue consumption. Rust has no native async generators. The core exposes `next()` for loop-based consumption. The high-level `rivetkit` crate should provide a `Stream` adapter via `futures::Stream`:
```rust
impl Queue {
    fn stream(&self, opts: QueueStreamOpts) -> impl Stream<Item = QueueMessage>;
}
```

### 10. Schema validation for events and queues
TS validates event payloads and queue messages against StandardSchemaV1 (Zod) schemas defined in actor config. `rivetkit-core` does not perform schema validation (opaque bytes). The language layer is responsible. For the high-level Rust crate, consider integrating with `serde` validation or a schema library. For TS, the existing Zod validation runs in the NAPI callback layer.

### 11. Database provider system
TS has a pluggable database provider pattern (`db` config, `c.db` accessor, `onMigrate` lifecycle hook, database setup/teardown during lifecycle). The spec currently exposes `ctx.sql()` directly. For full parity, add a database provider abstraction to `rivetkit-core` that supports setup, migration, and teardown hooks integrated into the lifecycle (setup before state load, teardown during shutdown).

### 12. Metrics and tracing
TS tracks detailed startup metrics (per-phase timing: `createStateMs`, `onWakeMs`, `createVarsMs`, `dbMigrateMs`, `loadStateMs`, etc.), action metrics (call count, error count, total duration), and OpenTelemetry trace spans for all lifecycle events. `rivetkit-core` should emit equivalent metrics via the `tracing` crate and expose startup timing data.

### 13. Persisted connection format
Hibernatable connections are persisted with BARE-encoded format including: connection ID, params (CBOR), state (CBOR), subscriptions, gateway metadata (gateway_id, request_id, message indices), request path, and headers. The BARE schema must match the TS `CONN_VERSIONED` schema exactly for cross-language compatibility if actors migrate between TS and Rust runtimes.

### 14. `waitForNames` queue method
TS queue manager has `waitForNames()` that blocks until a message with a matching name arrives. Used for coordination patterns. Should be added to the core `Queue` API.

### 15. `enqueueAndWait` queue method
TS queue manager has `enqueueAndWait()` which sends a message and blocks until the consumer calls `complete(response)`. This is a request-response pattern built on queues. Should be added to the core `Queue` API.
