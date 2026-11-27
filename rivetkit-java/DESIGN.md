# RivetKit Java SDK: design

Status: v2. Java 25, built on the Rust core through FFM. This file is for people and coding
agents changing the SDK. Read §1–4 before touching anything, and §5–6 before changing the boundary.

## 1. Principles

1. **Rust does the work, Java maps it.** Every actor behaviour (lifecycle order, timeouts,
   shutdown, connections, HTTP and WebSocket streams, SQL, schedules) lives in
   `rivetkit-core` and `rivetkit-host-adapter`. Java turns events into calls on user objects
   and back. If logic could be shared with another language, it belongs in the adapter.
2. **Mistakes fail to compile.** Wrong event types, wrong connection params, misspelled hooks,
   wrong schedule or async arguments, and an implementation of the wrong API are all javac
   errors (`CompileTimeSafetyTest`). What javac cannot see (overloads, unsupported wire types)
   fails at `Registry.Builder.build()`, never at call time.
3. **No locks in user code.** Actor code runs in turns (§4).
4. **The boundary is narrow and safe.** There are 7 C functions and `u64` handles, never
   pointers. Rust has exactly one `unsafe` block, and Java has no `sun.misc.Unsafe` or JNI.
5. **Nothing grows without bound.** Mailboxes, event queues and stream windows are all
   bounded.
6. **Deprecated features are absent.** Actor state, connection state, KV, queues, a plain
   `run` and prevent-sleep are deprecated in 3.0 (`docs/actors/content/docs/deprecated/`).
   Durable data goes in SQLite; in-memory data goes in fields.

## 2. Public API (`dev.rivet.kit`)

```java
// The contract, shared by client and actor.
public interface ChatRoom extends Actor<ChatRoom.Event, ChatRoom.Params> {
    Message send(String text);                       // an action: typed args and result
    List<Message> history(int limit);
    int purgeOlderThan(long cutoff);
    record Params(String user, String token) {}      // connection params
    record Message(String sender, String text, long at) {}
    sealed interface Event {}                        // what the actor broadcasts
    record NewMessage(Message message) implements Event {}
}

// The actor: the API plus the hooks it needs, each a one-method interface.
public final class ChatRoomActor implements ChatRoom, OnConnect<ChatRoom.Params>, OnWake {
    private final ActorContext<ChatRoom, Event> ctx;
    private final Map<String, String> userByConn = new HashMap<>();   // a var: no locks needed

    public ChatRoomActor(ActorContext<ChatRoom, Event> ctx) { this.ctx = ctx; }   // runs on every start

    @Override public void onWake() {
        ctx.cron().every("purge", Duration.ofHours(1), ChatRoom::purgeOlderThan, 0L);   // method ref, not a string
    }
    @Override public void onConnect(ActorConn<Params> conn) { userByConn.put(conn.id(), conn.params().user()); }
    @Override public Message send(String text) {
        Message m = ctx.db().queryOne(Message.class,
                "INSERT INTO messages (sender, text, at) VALUES (?, ?, ?) RETURNING sender, text, at",
                "ann", text, System.currentTimeMillis()).orElseThrow();
        ctx.broadcast(new NewMessage(m));            // javac: must be a ChatRoom.Event
        return m;
    }
    // ...
}

Registry registry = Registry.builder()
    .actor(ChatRoom.class, ChatRoomActor::new, a -> a.migrate(db -> db.script(SCHEMA)))
    .build();                                          // validates every API
try (Registry.Serving serving = registry.serve()) { serving.ready().join(); /* ... */ }

try (Client client = Client.of("http://127.0.0.1:6420")) {
    ChatRoom room = client.getOrCreate(ChatRoom.class, "general");     // blocking typed proxy
    room.send("hi");
    client.handle(ChatRoom.class, "general").async(ChatRoom::history, 10);   // CompletableFuture
    try (var conn = client.connect(ChatRoom.class, () -> new ChatRoom.Params("ann", token), "general")) {
        conn.on(ChatRoom.NewMessage.class, e -> show(e.message()));   // javac: must be a ChatRoom.Event
    }
}
```

| Type | Role |
|---|---|
| `Actor<E, P>` | Base of every API interface. `E` is the events (a sealed interface of records, one record, or `None`). `P` is the connection params (or `None`). |
| `@Name`, `@Concurrent` | `@Name` pins a wire name for an actor, action or event. `@Concurrent` runs an action outside the turn. |
| `OnCreate<I>`, `OnWake`, `OnSleep`, `OnDestroy`, `OnFork`, `OnBeforeConnect<P>`, `OnConnect<P>`, `OnDisconnect<P>`, `OnSubscribe<E>`, `OnRequest`, `OnWebSocket` | Hooks, each a one-method interface: implement only the ones you need. |
| `ActorContext<A, E>` | (`A` the actor's API, `E` its events; `schedule()` and `cron()` take only `A`'s methods) `broadcast`, `send`, `conn()`, `conns()`, `db()`, `schedule()`, `cron()`, `client()`, `keepAwake`, `waitUntil`, `spawn`, `aborted`, `blocking`, `exclusive`, `sleep`, `destroy`, plus the actor's id, name, key and region. |
| `Db` | `query` / `queryOne` map rows to records (by column name; `snake_case` columns also match camelCase fields) or scalars. Also `exec`, `script`, `batch`, `transaction`. |
| `Schedule`, `Cron` | Calls to this actor's own actions, written as method references. |
| `Request`, `Response`, `FetchResponse`, `WebSocket` | Raw HTTP (streaming bodies both ways) and raw WebSockets. An actor's `onRequest` returns a `Response` (`Bytes` or `Streaming`); a client's `fetch` returns a `FetchResponse`. |
| `Registry` | `builder().actor(...)`, then `build()`, then `serve()` (returns `Serving`). Also `validate()` for tests. |
| `Client`, `Handle<A>`, `Conn<A, E>` | Typed proxies; `async` / `run` by method reference; connections with events, status and `onError`; `fetch` and `webSocket` for raw access. |
| `RivetException`, `UserError` | Errors carry group, code, message and metadata. Throw `UserError` to send the client a specific error; other exceptions reach clients as `core/internal_error` and are logged. |

**Wire names.** The actor name defaults to the interface name with a lowercase first letter
(`chatRoom`). An action's name is the method name. An event's name is the record name with a
lowercase first letter (`newMessage`), and its arguments are the record's components in order.
Arguments travel as a positional CBOR array, the same as from TypeScript. The Rust client's
single struct argument (`[{amount: 5}]`) is also accepted; matching it by parameter name needs
`javac -parameters`, which the build sets.

**Hooks and stateless calls.** Every stateless call (`client.getOrCreate(...).send(...)`) runs
through a short-lived connection with no params. So `OnBeforeConnect` and `OnConnect` see
`params == null` for those calls (see `ChatRoomActor`).

**Testing.** The `testing` module's `RivetTest.start(registry)` serves on a fresh pool against a
local engine and gives you a client. `retry(...)` covers the first call while the actor is
being scheduled.

## 3. Architecture

```
user code       ChatRoom (interface) + ChatRoomActor (class)
dev.rivet.kit   public API
  .internal     Native · Instance/Turn/Calls · ApiModel/ImplModel · Codec · Recorder · ClientImpl · Streams
  .ffi.internal jextract output (generated)
──────── C ABI: include/rivetkit.h (generated by cbindgen) ────────
rivetkit-ffi            7 exports + the upcall transport + log bridge (~400 lines, the one unsafe block)
rivetkit-host-adapter   event loop, hooks, timeouts, HTTP/WS streams, and the wire protocol (wire/)
rivetkit-core · rivetkit-client
```

`rivetkit-host-adapter` is binding-independent and forbids `unsafe`:
- `event_loop.rs` is the napi actor loop without state. It calls a `HostInstance` trait.
- `wire/` implements that trait as bytes over a `Transport`: event kinds, ops, CBOR headers,
  the handle table, leases and streams.

Another C-ABI language (Go, Python, C#, Swift) links the same library. napi, wasm or the Rust
SDK could implement `HostInstance` directly. They are deliberately unchanged for now.

`rivetkit-ffi` stays a separate crate on purpose:
- it holds the one `unsafe` block and the process-global runtime, so neither leaks into the
  shared adapter;
- bindings that don't use C don't link it.

Feature coverage:
- **Supported:** actions, input, keys, lifecycle (including fork), vars, SQLite, connections
  and events, schedule and cron, keepAwake/waitUntil/spawn/abort, HTTP `onRequest`, WebSocket
  handlers, actor-to-actor calls, and the inspector (core serves it). Client side: actions,
  connections, raw fetch and raw WebSockets.
- **Declined:** workflows, because the engine is TypeScript-only (core gets `not_configured`).
  `onBeforeActionResponse` is not supported yet.

## 4. Threading: turns

- **One thread at a time.** At most one thread runs an actor's code at a time. Every event is
  a task on its own virtual thread holding a **ticket**. Tickets are reserved on the upcall,
  in arrival order, so tasks run in arrival order (`Turn`). Plain fields need no locks or
  `volatile`.
- **SQL keeps the turn.** `ctx.db()` blocks the virtual thread but does not give the turn up,
  so a read-then-write inside one action is atomic.
- **Outbound waits give the turn up.** `ctx.client()` calls, `ctx.blocking(...)`, client
  WebSocket sends waiting on a slow peer, and HTTP response writes waiting on the client all
  release the turn and take a new ticket afterwards. One exception keeps frame order: a client
  socket's own listener sending on it keeps the turn.
  So actor A can call B, and B can call A, without deadlock (`actorsCanCallEachOtherBack`).
  Fields may have changed after such a call. `ctx.exclusive(...)` holds the turn across such
  waits; it can deadlock.
- **`@Concurrent` actions** take no ticket. They may use `ctx.db()` and immutable or
  concurrent fields only.
- **`ctx` checks the turn.** Every turn-sensitive `ctx` method throws `IllegalStateException`
  if the caller does not hold the turn (for example, a thread you started yourself).
  `ctx.spawn` and `waitUntil` run work as turns; `ScopedValue` carries the current call.
- **Cancellation.** When Rust stops waiting (timeout, caller gone), Java gets `CANCEL`. A
  queued task is dropped; a running one is interrupted. `ABORT` (shutdown started) interrupts
  spawned tasks; `ctx.aborted()` turns true.
- **Bounded queues, one overload policy.** Past `maxWaitingEvents` (default 10,000) waiting
  events, an actor refuses calls and requests with `actor/overloaded`, and closes a WebSocket
  that keeps sending with 1013 (try again later, `actor.overloaded`). Client listeners run on
  one thread per connection or socket (or as turns of the owning actor), at most 1,024 behind:
  past that a connection is closed with an error and a socket with 1013 (`client.overloaded`).
  A client socket's sends wait while a 64-frame queue is full. A connection or socket whose
  owning actor instance stopped is closed (and logged) instead of delivering to it.

## 5. The boundary

| Export | Purpose |
|---|---|
| `rivet_abi_version()` | Java refuses to load on a mismatch (`Wire.ABI_VERSION`). |
| `rivet_init(on_event, on_complete, options)` | Registers the two upcalls. `options` is a CBOR log filter (`-Drivet.log=info`). |
| `rivet_call(target, op, header, body, token)` | Every request from Java. The result comes back through `on_complete(token)`. |
| `rivet_reply(event, status, header, body)` | Answers an event. |
| `rivet_lease_release(lease)` | Frees a leased buffer and returns stream credit. |
| `rivet_release(handle)` | Releases a handle Java owns. |
| `rivet_last_error(token)` | Completes `token` with the error from a failed call. |

Upcalls: `on_event(kind, actor_type, actor, id, header, body, lease)` and
`on_complete(token, status, header, body, lease)`.

**Message shape.**
- Kinds, ops and statuses are `#[repr(u32)]` enums in `wire/mod.rs`, exported to the header
  and read by Java as `RivetFfi.RIVET_*` constants. Renumbering one breaks the Java build.
- **Headers** are small CBOR maps (`wire/messages.rs` ↔ `Wire.java`).
- **Bodies** are raw bytes: user CBOR, HTTP and WebSocket data. They are passed through and
  never re-encoded.

**Copies.**
- **Lent:** a body ≤ 64 KiB is valid only during the callback, so Java copies it once.
- **Leased:** larger bodies, and every stream chunk, stay in Rust's handle table until
  released. Java reads them in place (`MemorySegment`): `Streams.Inbound` copies straight into
  the reader's array.
- **Credit:** releasing a stream chunk returns credit to a 256 KiB window, so a slow reader
  pauses the sender all the way to the network.
- **Outbound bodies:** HTTP response writes complete only once accepted (backpressure).

**Rules.**
- Upcalls only enqueue; user code never runs on a Rust thread.
- No downcall blocks, so virtual threads never pin a carrier.
- Every export runs inside `catch_unwind` (`exports_are_guarded`).
- Completions finish internal futures inline. User-facing futures continue on virtual threads.
- A resource whose answer creates a stream, connection or socket registers its sink inline in
  the completion, before Rust can send its first event. Events Rust has for it before then are
  held back and flushed after the completion (client connection status also adds a snapshot).
- **Order is per resource, in both directions.** Rust → Java: a resource's events follow its
  creation, in order (the holds above, the WebSocket open gate). Java → Rust: ops on one
  response body go into that body's FIFO when called (`wire/body.rs`), so a write, `fin` or
  abort never overtakes an earlier one. Ops on different resources stay concurrent: one
  actor's slow SQL never waits behind another resource's writes.
- **Ownership.** An actor owns what it creates, a client owns its connections, sockets and
  fetch bodies; releasing an owner releases what it owns (`release_owned`). On the Java side
  `Client.close()` closes its connections and sockets, an actor's `ctx.client()` closes when
  the actor sleeps, and an `onRequest` request body is closed when the response is done.
  The end-to-end suite checks that no call is left waiting and no resource is left open.

## 6. Keeping it up to date

| When core changes… | Do this |
|---|---|
| New context operation or feature | Add a header struct to `wire/messages.rs`, then an `Op` value (`wire/mod.rs`) with its dispatch in `wire/ops.rs`, then a fixture in `messages.rs::fixtures`. Then the Java side: a `Wire` record, an `OP_*` constant, and the API method. Additive: no version bump. |
| New lifecycle event | Add an `EventKind` and a `HostInstance` method, call it from `event_loop.rs`, emit it in `wire/host.rs`, then add a Java hook interface and handle it in `Instance.dispatch`. |
| Anything that changes an existing header, kind or rule | Bump `ABI_VERSION` (Rust) and `Wire.ABI_VERSION` (Java) together. |
| napi changes hook behaviour | Port it to `event_loop.rs`. napi's `napi_actor_events.rs` is the reference until napi moves onto the adapter. |

After any change:
1. `mise run bindings` regenerates the header and the jextract bindings.
2. `UPDATE_FIXTURES=1 cargo test -p rivetkit-host-adapter` regenerates the fixtures.
3. Mirror the change in `Wire.java` and in `FixturesTest.TYPES`.

The exhaustive matches in the fixtures module refuse to compile until a new kind or op has a
sample. `FixturesTest` refuses a fixture without a Java type.

## 7. Tooling

`mise.toml` pins Java (Temurin 25), Gradle, Rust, cbindgen and jextract (with a sha256). The
generated header and bindings are committed, so normal builds need neither cbindgen nor jextract.

| Task | What it does |
|---|---|
| `mise run build` | Builds everything. |
| `mise run test` | Rust tests plus all Java tests, including end to end (spawns a local engine). |
| `mise run bindings` | Regenerates the header and the Java bindings. |
| `mise run bindings:check` | Fails if the committed header or bindings are stale. |
| `mise run unsafe:check` | Fails unless there is exactly 1 `unsafe` block, in `abi.rs`. |
| `mise run miri` | Runs the `abi` tests under Miri. |
| `mise run bench:compare` | Benchmarks Rust, Java and TypeScript against the same counter actor. |

Gradle modules:
- `ffi` (generated bindings)
- `core` (the SDK)
- `testing` (`RivetTest`)
- `examples` (example actors, the e2e suite, the bench server)

Gradle builds the Rust library and packages it under `native/<os>-<arch>/`.

## 8. Testing

| Layer | Tests |
|---|---|
| Rust | `wire/tests.rs`: replies, cancel-on-drop, leases (including after the actor stops), stream credit, op errors, handle sweeping. `messages.rs`: fixtures, with an exhaustive match so every kind and op has one. `abi.rs`: input checks, the panic guard, every export guarded; also run under Miri. |
| Java unit | `CodecTest`, `TurnTest`, `ApiModelTest`, `FixturesTest` (every Rust fixture decodes, and re-encodes to the same CBOR), `CompileTimeSafetyTest` (15 mistakes that must not compile, plus one program that must), `RegistryValidationTest` (mistakes javac can't see fail at registration). |
| End to end | Both test classes go through the public API only, against a real engine. `examples/SdkTest` covers the example actors. `examples/FeaturesTest` covers the rest of the API, using the `Lab` test actor: every `ctx` and `Db` call, schedules and cron, every hook, connections and subscriptions, raw HTTP and WebSockets, and client options. |

How the end-to-end suite stays fast:
- **Concurrency.** Tests run concurrently (`junit-platform.properties`, 8 threads). They share one registry and engine (`TestRivet`), and each test uses fresh actor keys.
- **A fresh engine store.** Every run gets an empty store (`RIVETKIT_STORAGE_PATH` under `examples/build/`). An engine started on old data first replays every actor earlier runs left behind, which delayed the first call by about 30 s.
- **Its own engine.** Gradle picks a free port block per run (`rivet.test.endpoint`), so a run never reuses an engine left on 6420 by a developer or an earlier run.
- **No graceful stop.** The engine and its store are thrown away, so the JVM's shutdown hook kills the engine instead of waiting out its storage drain (about 10 s).
- **Hangs fail fast.** Each test has a 20 s timeout on its own thread, its own waits stop at 10 s (so they fail with their message first), the warm-up has a 60 s deadline, and the leak check (no calls left waiting on Rust, no streams, connections or sockets left open) runs once, after every class.

`mise run test` takes about 15 s warm: about 4 s of Rust tests and 11 s of Java.

Coverage: `mise run coverage` writes JaCoCo for `core` (unit plus end-to-end) to
`build/reports/coverage/`; it is about 90% of lines. For Rust coverage under the Java tests:
1. Build `rivetkit-ffi` with `cargo llvm-cov show-env`.
2. Run Gradle with `-Privet.ffi.library=<instrumented .so>` and
   `RIVET_ENGINE_BINARY_PATH` set.
3. Run `cargo llvm-cov report`.

## 9. Open items

- Publishing (Maven Central, native jars for each platform) and Windows: out of scope for now.
- Moving napi, wasm and the Rust SDK onto `rivetkit-host-adapter` (it was designed for that).
- `onBeforeActionResponse`, typed error subclasses (`@Code`), and a validation-only annotation
  processor (compile-time overload checks, `rivet-api.json` for rename diffs and other-language
  clients).
- Tests still to add: a TypeScript client calling a Java actor, and cron actually firing (its
  minimum interval is 5 s).
- The Rust client decodes event args to JSON eagerly. A lazy `raw_args` path would save that
  work.
- A rejected connection keeps retrying, as in the other clients. Java surfaces it through
  `Conn.onError`. A subscription refused by `OnSubscribe` closes the whole connection (core
  sends close code 1011); the client sees `DISCONNECTED` and reconnects.
- The repo's simulation lints (`clippy.toml` bans raw tokio and `HashMap`) are not applied to
  the RivetKit host crates yet; the Rust SDK and napi don't follow them either.

## 10. Notes for agents

- **Don't add logic to `rivetkit-ffi`.** Put it in the adapter.
- **Don't add `unsafe`.** If you think you need it, you need a lease or a handle instead.
- **Every new header needs a fixture,** plus the matching `Wire` record and `FixturesTest` entry.
- **New public API must keep §1.2.** Add a `CompileTimeSafetyTest` case for any new typed
  surface.
- **Upcalls must only enqueue.** Anything slower belongs on a virtual thread.
- **When in doubt about lifecycle behaviour,** do what `napi_actor_events.rs` does, minus state.
