# RivetKit Core WebAssembly Support Proposal

## Goal

Support a WebAssembly build of RivetKit core while keeping the existing native NAPI/runtime behavior intact. This splits into:

- Phase 1: add remote SQLite SQL execution for runtimes that cannot load native SQLite.
- Phase 2: make `rivetkit-core` and the Rust envoy client compile and run behind WebAssembly-compatible runtime and WebSocket interfaces.

This proposal is intentionally gate-oriented: implementation should not start until the parity, rollout, and failure-mode criteria below are accepted.

## Implemented Invariants

- Remote SQL is an envoy protocol v4-only capability. Older protocol targets reject remote SQL request and response serialization with `ProtocolCompatibilityError { feature: RemoteSqliteExecution, required_version: 4, ... }`, and runtime callers map unsupported remote SQL to `sqlite.remote_unavailable` instead of a BARE decode failure.
- Wasm uses remote SQLite only. The valid SQLite driver cells are native/local/all encodings, native/remote/all encodings, and wasm/remote/all encodings; wasm/local is invalid and covered by a targeted unavailable assertion.
- Shared SQLite bind, value, result, and route types live in `rivetkit-sqlite-types`. Native and remote execution both route through the shared SQLite execution layer so statement classification, writer stickiness, migrations, `execute_write`, and manual transactions stay aligned.
- `pegboard-envoy` creates at most one remote SQL executor per active `(actor_id, sqlite_generation)`. The executor is created lazily on the first accepted SQL request, reused for that generation, and removed when the actor closes or the connection shuts down.
- Remote SQL work runs outside the pegboard-envoy WebSocket read loop in bounded workers. Accepted SQL is tracked per `(actor_id, sqlite_generation)`; actor stop rejects new SQL after stopping begins and waits for already accepted SQL before closing storage.
- Sent remote SQL requests fail with `sqlite.remote_indeterminate_result` if the WebSocket disconnects before the response arrives. Only unsent requests may be sent after reconnect.
- `rivet-envoy-client` owns native versus wasm WebSocket transport selection through mutually exclusive `native-transport` and `wasm-transport` features. `rivetkit-core` selects transport by enabling `native-runtime` or `wasm-runtime`.
- The wasm binding strategy is direct `wasm-bindgen` in `@rivetkit/rivetkit-wasm`. Native NAPI and wasm bindings both adapt into the shared TypeScript `CoreRuntime` interface; raw binding imports stay inside approved runtime adapter files.
- `scripts/cargo/check-rivetkit-core-wasm.sh` is the canonical wasm dependency gate for `rivetkit-core`.

## Current State

- `rivetkit-core` owns `ActorContext::sql()` and currently routes `exec`, `query`, `run`, `execute`, and `execute_write` through `SqliteDb`.
- With `sqlite-local`, `SqliteDb` opens `rivetkit-sqlite::NativeDatabaseHandle`, which uses native `libsqlite3-sys` plus a custom VFS.
- With `sqlite-remote`, `SqliteDb` sends SQL through envoy remote execution. With no usable backend, the public API returns explicit structured unavailable errors.
- The envoy SQLite protocol now includes both the page/storage path and v4 SQL execution requests for `exec`, `execute`, and `execute_write`.
- `pegboard-envoy` validates remote SQL namespace, actor, generation, request size, and response size before executing through the shared SQLite execution layer.
- The canonical wasm compile and dependency check is `scripts/cargo/check-rivetkit-core-wasm.sh`.

## Phase 1: Remote SQLite SQL Execution

### Summary

Add a second SQLite execution backend in `rivetkit-core`: local native SQLite when compiled with native SQLite support, and remote SQL execution through the envoy protocol when compiled without it or explicitly configured to use remote execution.

The important pushback: this is not just dynamic routing over the current protocol. The current protocol only remotes page reads/writes. SQL execution still happens in the actor process. To run SQLite on `pegboard-envoy`, we need new protocol messages and a server-side SQL executor.

### Proposed Shape

Introduce a core-level SQLite backend enum behind `SqliteDb`:

```rust
enum SqliteBackend {
	LocalNative(LocalNativeSqlite),
	RemoteEnvoy(RemoteSqlite),
	Unavailable,
}
```

Keep the public `SqliteDb` methods unchanged:

- `exec(sql) -> QueryResult`
- `query(sql, params) -> QueryResult`
- `run(sql, params) -> ExecResult`
- `execute(sql, params) -> ExecuteResult`
- `execute_write(sql, params) -> ExecuteResult`
- `close()`

Add a compile/runtime selection layer:

- Native default: `local-native` when `rivetkit-core/sqlite` is enabled.
- Wasm/default-no-native: `remote-envoy` only when the runtime explicitly declares remote SQLite support. Other no-native builds keep returning `sqlite.unavailable`.
- Explicit override: config/env/feature for forcing remote SQLite in native builds so we can test phase 1 without wasm.

### Protocol Additions

Add a new envoy protocol version rather than mutating an existing `*.bare` version.

New BARE types should mirror core/native result shapes:

- `SqliteBindParam`: `Null | Integer(i64) | Float(f64) | Text(str) | Blob(data)`
- `SqliteColumnValue`: `Null | Integer(i64) | Float(f64) | Text(str) | Blob(data)`
- `SqliteExecuteKind`: `Exec | Execute | ExecuteWrite`
- `SqliteExecuteSqlRequest`: `actorId`, `generation`, `kind`, `sql`, `params`
- `SqliteExecuteSqlOk`: `columns`, `rows`, `changes`, `lastInsertRowId`, `route`
- `SqliteExecuteSqlResponse`: `Ok | FenceMismatch | ErrorResponse`
- `ToRivetSqliteExecuteSqlRequest`
- `ToEnvoySqliteExecuteSqlResponse`

`query` and `run` can be client-side projections over `Execute`, just like TypeScript now does over the native `execute` path. `exec` remains separate because it supports multi-statement compatibility.

Protocol implementation must include:

- A new `engine/sdks/schemas/envoy-protocol/v4.bare`.
- Regenerated Rust/TypeScript protocol code and updated stringifiers.
- `engine/sdks/rust/envoy-protocol/src/versioned.rs` guards that reject remote SQL messages on protocol versions older than v4 with explicit errors.
- Compatibility tests for old core/new pegboard-envoy, new core/old pegboard-envoy, old core/old pegboard-envoy, and new core/new pegboard-envoy.
- A user-facing `sqlite.remote_unavailable` or equivalent structured error when the runtime selects remote SQLite but the connected pegboard-envoy protocol cannot serve it.

Rollout order:

| Runtime | Pegboard-envoy | Expected behavior |
|---|---|---|
| Old native | New | Existing page-storage SQLite path works unchanged. |
| New native local | Old | Local native SQLite works unchanged. Remote override fails fast with remote-unavailable. |
| New native remote | New | Remote SQL path works and passes parity tests. |
| New wasm remote | Old | Startup or first SQL call fails with remote-unavailable, not `sqlite.unavailable` and not a protocol decode failure. |
| New wasm remote | New | Remote SQL path works and passes wasm smoke tests. |

Remote SQL is a v4-only capability. `versioned.rs` must fail serialization for `ToRivetSqliteExecRequest`, `ToRivetSqliteExecuteRequest`, `ToRivetSqliteExecuteWriteRequest`, and their `ToEnvoy` responses when the negotiated protocol version is v1, v2, or v3. That failure uses a typed `ProtocolCompatibilityError` with `feature = RemoteSqliteExecution`, `required_version = 4`, and the attempted target version, so runtime code can convert the failure into a user-facing remote-unavailable error instead of leaking a BARE decode failure.

### Server-Side Execution

Do not duplicate SQL classification and connection-routing behavior in `pegboard-envoy`. Prefer extracting reusable native SQLite execution from `rivetkit-sqlite`:

- Move `BindParam`, `ColumnValue`, `ExecResult`, `QueryResult`, `ExecuteResult`, `ExecuteRoute`, statement classification, and connection manager behavior behind a storage backend abstraction.
- Keep the current actor-side VFS backend backed by `EnvoyHandle`.
- Add a pegboard-envoy VFS/storage backend that adapts the same execution layer to `Arc<sqlite_storage::SqliteEngine>`. Direct engine access is not enough; the adapter must preserve connection manager setup, PRAGMAs, classification, reader authorizer, and read/write routing.
- Reuse the same PRAGMA setup, query-only reader authorizer, transaction/write-mode handling, and read-pool policy on both local and remote execution.
- Create the pegboard-envoy SQL executor lazily on first SQL use for an active `(actor_id, generation)`. Declaring or constructing a database handle in the actor runtime must not eagerly create a server-side database executor.
- Drop the pegboard-envoy SQL executor when the owning actor generation closes. Client-side `SqliteDb::close()` releases the actor-side handle only; server-side executor cleanup is tied to actor lifecycle.

This keeps native and remote behavior aligned. A concrete example: `BEGIN`, `SAVEPOINT`, schema writes, and unknown classification currently force the writer path. Remote execution must make exactly the same route choice or user code will pass in Node and fail in wasm.

### Concurrency And Lifecycle

- `pegboard-envoy` should hold at most one SQL executor per `(actor_id, generation)`. It is created on the first accepted remote SQL request and closed/removed on `ActorStateStopped` or the equivalent actor-close path.
- Actors that declare SQLite but never execute SQL should never create a pegboard-envoy SQL executor.
- The first remote SQL call should perform the same namespace, generation, and local-open validation as existing SQLite storage requests before creating the executor.
- Server-side SQL must not run inline on the pegboard-envoy WebSocket read loop. Dispatch SQL work to bounded per-connection workers, keep per-actor serialization where required by the connection manager, and continue processing pings, stops, tunnel traffic, and later messages while SQL is running.
- Track in-flight remote SQL per `(actor_id, generation)`. Accepted SQL runs in bounded per-connection workers. Once an actor enters stopping, new remote SQL is rejected; actor close waits up to the actor stop budget for already-running SQL and must not close SQLite while work is still in flight.
- Serverful actors can rely on the existing pegboard exclusivity invariant: one active actor generation owns SQLite access.
- Serverless flows already call `ensure_local_open`; remote execution should use the same generation fencing before each query.
- Remote `close()` from actor core should release the client-side handle only. Final server-side cleanup should be driven by actor stop so leaked JS/Rust handles cannot keep the database alive forever.
- Long-running SQL must count as actor activity from core's point of view. Awaited SQL inside action/run work is covered by the user task; detached/background SQL must use a first-class SQL activity counter or mandatory `keep_awake` wrapping so sleep finalize cannot close under it.
- Remote SQL requests are not blindly retried after a WebSocket disconnect. If a request may have reached pegboard-envoy and the response is lost, the actor-side envoy client fails it with `sqlite.remote_indeterminate_result`. Only requests that were still unsent may be sent after reconnect.
- Manual transaction sequences spanning calls must remain sticky to the writer connection for the same client-side `SqliteDb` handle, matching native write-mode behavior.

### Payload Limits

Remote SQL must enforce concrete limits before execution and before sending responses:

- SQL text bytes.
- Bind parameter count and total serialized bind bytes.
- Row count, column count, cell bytes, and total serialized response bytes.
- Maximum execution time or cancellation deadline.

The serialized response limit should default to `ProtocolMetadata.maxResponsePayloadSize`; requests that exceed limits return structured SQLite errors without sending oversized WebSocket frames.

### Errors

- SQL errors should cross the protocol as `SqliteErrorResponse` and be converted by core into `RivetError` where possible.
- Do not expose raw internal engine errors to TypeScript as canonical `RivetError` unless they came from the universal error system.
- Preserve existing TypeScript error enrichment behavior for KV/VFS failures where useful, but rename it once remote execution is not actually native VFS I/O.

### Tests

Core/unit:

- `SqliteDb` routes to local native when enabled and remote when forced.
- Remote request serialization supports null/int/float/text/blob bindings, plus TypeScript wrapper normalization for named params, booleans, bigint, `Uint8Array`, and blob-like values.
- `query`, `run`, `execute`, `execute_write`, and `exec` preserve existing result shapes.
- `execute_write` forces writer route even for read-looking SQL.
- `writeMode` and `db({ onMigrate })` run through the remote path with the same writer stickiness and migration ordering as native.
- Native-to-remote error mapping preserves structured `RivetError` sanitization and does not leak internal engine errors.

Pegboard-envoy:

- SQL request validation rejects invalid actor id, namespace mismatch, stale generation, oversized SQL, oversized params, and oversized result.
- SQL executor creation is lazy: an actor that never uses SQL creates no server-side SQL executor, the first accepted SQL call creates exactly one executor, and repeated calls reuse it for that actor generation.
- Actor close removes the server-side SQL executor. A later actor wake creates a fresh executor for the new generation while persisted SQLite data remains in storage.
- Server-side executor returns fence mismatch when generation does not match.
- Concurrent remote reads/writes follow the same read-pool/write-mode behavior as native.
- A long SQL query does not block the WebSocket read loop from handling ping/pong, stop, and tunnel traffic.
- Actor stop rejects new remote SQL, waits up to the actor stop budget for already-running SQL, and never closes storage under an executing query.
- Old protocol versions reject remote SQL messages cleanly.
- Lost response during write SQL returns the selected indeterminate-result or deduped response behavior.

Driver/parity:

- Extend `rivetkit-typescript/packages/rivetkit/tests/driver/shared-matrix.ts` beyond the current encoding-only matrix to cover SQLite backend and runtime dimensions.
- Run the existing raw SQLite driver suite across `sqliteBackend = local | remote`, `runtime = native | wasm`, and `encoding = bare | cbor | json`.
- Treat `runtime = wasm` plus `sqliteBackend = local` as an invalid cell. It should be skipped by matrix construction or asserted unavailable, because wasm has no local SQLite backend.
- Required valid cells are native/local/all encodings, native/remote/all encodings, and wasm/remote/all encodings.
- SQLite-specific driver tests are the only tests that must multiply by SQLite backend. Non-SQLite driver tests should continue to run across their existing registry/encoding coverage, and may add the runtime dimension only when the wasm runtime is ready.
- The SQLite driver suite means the existing `rivetkit-typescript/packages/rivetkit/tests/driver/actor-db*.test.ts` coverage plus any database-specific helper suites added for this work.
- Add deterministic tests for reconnect during write SQL, stale-generation SQL, duplicate command replay around SQL, result-size rejection, shutdown during SQL, and manual transaction sequences spanning calls.
- Add a wasm/no-native smoke gate once phase 2 exists, then promote wasm/remote/all-encoding SQLite tests into the normal driver matrix.

Required SQLite driver matrix:

| Runtime | SQLite backend | Encodings | Required phase | Expected behavior |
|---|---|---|---|---|
| native | local | `bare`, `cbor`, `json` | Phase 1 | Existing native local SQLite behavior passes unchanged. |
| native | remote | `bare`, `cbor`, `json` | Phase 1 | Native runtime forced to remote SQL passes the same database API tests. |
| wasm | remote | `bare`, `cbor`, `json` | Phase 2 | Wasm runtime with no local SQLite passes the same database API tests through pegboard-envoy. |
| wasm | local | none | Phase 2 | Invalid combination. Matrix construction must not run it, and a targeted assertion should prove local SQLite is unavailable in wasm. |

Required test controls:

- Add explicit config fields equivalent to `runtime: "native" | "wasm"` and `sqliteBackend: "local" | "remote"` in the shared driver config.
- Native remote tests must force remote SQL through a single stable knob such as `RIVETKIT_SQLITE_BACKEND=remote` or an equivalent driver config field.
- Wasm tests must run with no local SQLite dependency compiled in.
- The matrix builder should name each dimension in test output so failures show registry, runtime, SQLite backend, and encoding.
- Before phase 2 lands, wasm/remote/all-encoding tests may be present as skipped or smoke-only coverage. Once phase 2 acceptance is claimed, wasm/remote/all-encoding tests are a required normal driver gate.

### Acceptance Criteria

- Existing native SQLite driver tests pass unchanged with local native SQLite.
- The same public database API passes the driver SQLite tests with `RIVETKIT_SQLITE_BACKEND=remote` or equivalent.
- The driver suite has explicit matrix dimensions for SQLite backend, runtime, and encoding. The valid SQLite matrix is native/local/all encodings, native/remote/all encodings, and wasm/remote/all encodings.
- The driver matrix excludes wasm/local from normal execution and includes a targeted assertion that wasm local SQLite is unavailable.
- SQLite-specific driver tests multiply by SQLite backend. Non-SQLite driver tests do not multiply by SQLite backend unless they explicitly need database behavior.
- Test output names the registry, runtime, SQLite backend, and encoding for every SQLite driver cell.
- Pegboard-envoy creates the server-side SQL executor lazily on first SQL use and removes it when the actor generation closes.
- Tests prove that an actor that never executes SQL does not create a remote SQL executor, and that reopening the actor after close creates a fresh executor while preserving persisted database contents.
- `rivetkit-core` can be built with no native SQLite dependency and still execute SQL remotely.
- `pegboard-envoy` owns server-side SQLite query execution and enforces actor namespace/generation validation.
- Protocol v4 is added, generated clients are updated, old-version guards are tested, and rollout matrix behavior is implemented.
- No existing published `*.bare` protocol version is modified.
- Remote SQLite is opt-in for no-native builds until the runtime has confirmed pegboard-envoy support.

## Phase 2: WebAssembly Compilation

### Summary

Make a wasm target a first-class runtime target for `rivetkit-core`, not a special TypeScript-only side path. The core move is to split native runtime concerns from pure actor runtime concerns.

`wasm_bindgen` can expose JavaScript host APIs, and `web-sys` can drive standard WebSockets, but the current Rust envoy client is native: `tokio-tungstenite`, `mio`, native rustls setup, native process management, and `reqwest`/pooling dependencies all need to be behind target-specific features or abstractions.

### Proposed Crate/Feature Shape

Add explicit features:

- `native-runtime`: current default native transport, process management, native HTTP helpers.
- `sqlite-local`: current `rivetkit-sqlite` path.
- `sqlite-remote`: phase 1 remote SQL path.
- `wasm-runtime`: wasm-safe timers, spawning, WebSocket transport, panic/log setup, and JS bindings.

For the direct wasm-bindgen path on `wasm32-unknown-unknown`, default to:

- no `sqlite-local`
- yes `sqlite-remote`
- no native engine process spawning
- no native `tokio-tungstenite`
- no native `reqwest` client construction in core

Do not use NAPI-RS wasm for the production edge-host binding. The spike showed its async/callback path is not compatible with workerd's no-threading runtime.

The feature work must include a target-specific dependency graph. The wasm graph must not depend on the workspace `tokio` with `full`, `mio`, `tokio-tungstenite`, `nix`, native `reqwest` pooling, `rivet-pools`, or `rivet-util` paths that pull native networking. This is a dependency-level requirement, not just a source-level `cfg`.

Current blockers to remove or gate:

- `rivetkit-core` has unconditional `nix`, `reqwest`, `rivet-pools`, and `rivet-util` dependencies.
- `rivet-envoy-client` has an unconditional `tokio-tungstenite` dependency.
- Workspace `tokio` currently enables native-heavy features through dependent crates.
- `rivetkit-core/src/lib.rs` exports native-only modules like `engine_process` and `serverless` unconditionally.

### WebSocket Ownership And Branching

The envoy WebSocket implementation is defined by `rivet-envoy-client`, not by `pegboard-envoy`.

Current ownership:

- `rivetkit-core/src/registry/mod.rs` calls `rivet_envoy_client::envoy::start_envoy(...)` with `EnvoyConfig`.
- `engine/sdks/rust/envoy-client/src/connection.rs` builds the envoy connection URL, chooses `tokio-tungstenite`, sends the `rivet` and `rivet_token.*` subprotocols, reconnects, serializes `ToRivet`, deserializes `ToEnvoy`, and forwards messages into the envoy loop.
- `engine/packages/pegboard-envoy` accepts the WebSocket and speaks the envoy protocol. It should not care whether the actor host used `tokio-tungstenite`, `web-sys::WebSocket`, or a future host transport.
- `rivetkit-core/src/registry/websocket.rs` and `rivetkit-core/src/websocket.rs` are a different WebSocket surface: actor/user WebSockets tunneled through envoy. They do not choose the actor-host-to-pegboard-envoy transport.

Desired ownership:

- `rivet-envoy-client` owns transport selection and the transport implementation.
- `rivetkit-core` owns runtime feature selection and passes a normal `EnvoyConfig` into `start_envoy`.
- `pegboard-envoy` owns protocol validation, close semantics, and actor lifecycle. It does not branch on wasm/native client transport.

Branching should be compile-time, not a runtime `if wasm` check:

| Build | `rivetkit-core` feature | `rivet-envoy-client` feature | Envoy transport |
|---|---|---|---|
| Native NAPI/Rust | `native-runtime` | `native-transport` | `tokio-tungstenite` |
| Wasm JS host | `wasm-runtime` | `wasm-transport` | Host WebSocket API, normally `web-sys::WebSocket` |

The branch should work like this:

- `rivetkit-rust/packages/rivetkit-core/Cargo.toml` maps `native-runtime` to `rivet-envoy-client/native-transport`.
- `rivetkit-rust/packages/rivetkit-core/Cargo.toml` maps `wasm-runtime` to `rivet-envoy-client/wasm-transport`.
- `engine/sdks/rust/envoy-client/Cargo.toml` makes `tokio-tungstenite`, native rustls setup, and any native-only HTTP/WebSocket dependencies optional behind `native-transport`.
- `engine/sdks/rust/envoy-client/Cargo.toml` adds optional `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, and `web-sys` dependencies behind `wasm-transport`.
- `engine/sdks/rust/envoy-client/src/connection/mod.rs` exposes the stable API used by the envoy loop, for example `start_connection(shared)`.
- `engine/sdks/rust/envoy-client/src/connection/native.rs` contains the current `tokio-tungstenite` implementation moved out of `connection.rs`.
- `engine/sdks/rust/envoy-client/src/connection/wasm.rs` contains the `web-sys::WebSocket` implementation.
- `connection/mod.rs` uses `#[cfg(feature = "native-transport")]` and `#[cfg(feature = "wasm-transport")]` to re-export exactly one implementation.
- Invalid feature combinations should fail at compile time: wasm target plus `native-transport`, native target plus `wasm-transport` unless explicitly supported, both transports enabled, or no transport enabled.

The wasm transport must preserve the behavior of the current native `connection.rs`:

- Same `ws_url(...)` query parameters: `protocol_version`, `namespace`, `envoy_key`, `version`, and `pool_name`.
- Same subprotocol auth shape: `rivet` plus `rivet_token.{token}` when a token exists.
- Same initial `ToRivetMetadata` send after connection open.
- Same `ToRivet` vbare serialization and `ToEnvoy` vbare deserialization.
- Same ping/pong handling, close-reason parsing, reconnect backoff, and shutdown close behavior.

The important browser constraint is that `web-sys::WebSocket` cannot set arbitrary WebSocket upgrade headers such as `Host`, `Connection`, `Upgrade`, or `Sec-WebSocket-Key`. This is acceptable only if `pegboard-envoy` authentication continues to work through URL parameters and `Sec-WebSocket-Protocol`.

### Runtime Abstractions

Introduce small core traits rather than threading `#[cfg(target_arch = "wasm32")]` through lifecycle code:

- `RuntimeSpawner`: spawn local futures, spawn Send futures on native, and surface abort handles.
- `RuntimeClock`: sleep, intervals, deadlines.
- `EnvoyTransport`: connect, send binary messages, receive binary messages, close, observe close reason.
- `HttpClient`: only for the few places core fetches runner config or probes local engine health.
- `ProcessManager`: native-only engine binary spawning, unavailable in wasm with explicit errors.

The existing actor lifecycle, queues, schedule, sleep, state persistence, registry dispatch, and SQLite routing should stay in core. Only environmental I/O moves behind interfaces.

The callback surface also needs a wasm-local design. Current callback traits require `Send + Sync + 'static` and `BoxFuture + Send`, which does not fit browser/worker JS promises and closures. Phase 2 must either:

- Add wasm-local callback traits that use local futures and JS-owned closures.
- Or keep Rust callback traits native-only and expose a JS host wrapper that converts JS promises into core events without requiring `Send`.

This decision is a blocker for WebAssembly feature parity; a spawn helper alone is not enough.

### WebSocket Transport

Create a wasm transport using `web-sys::WebSocket` and `wasm_bindgen` closures:

- Build the URL and subprotocol list to match native authentication semantics.
- Set binary type to `ArrayBuffer`.
- Convert JS `MessageEvent` data into `Vec<u8>`.
- Feed decoded `ToEnvoy` messages into the existing envoy loop.
- Map close frames into the same close reason parser used by native.
- Reconnect with the same backoff policy.

This should live in the envoy client layer, but selected by `wasm-runtime`, so `rivetkit-core` does not import `web_sys` directly unless we intentionally create a wasm facade crate.

JavaScript-host WebSockets cannot set arbitrary upgrade headers such as `Host`, `Connection`, `Upgrade`, or `Sec-WebSocket-Key`. The real compatibility gate is subprotocol-token auth working from the chosen host's WebSocket API.

### Tokio And Futures

The current direct `tokio::spawn` usage assumes `Send` futures and native runtime features. Wasm should use `wasm_bindgen_futures::spawn_local` or a wasm-compatible local executor through `RuntimeSpawner`.

Likely migration pattern:

- Replace `tokio::spawn(...)` in core-owned lifecycle code with a core spawn helper.
- Keep `tokio::sync` where it compiles, but gate `tokio` features so `net`, process, and full native runtime are not pulled into wasm.
- Avoid `spawn_blocking` in wasm. Phase 1 remote SQLite removes the current SQLite preload-hint `spawn_blocking` path for wasm.

### Native-Only Code To Gate

- `rivetkit-core/src/engine_process.rs`: native-only. Wasm should return explicit configuration errors.
- `rivetkit-core/src/serverless.rs`: split pure request handling from HTTP/client validation and native streaming assumptions.
- `rivetkit-core/src/registry/runner_config.rs`: move HTTP fetch behind a wasm-safe client abstraction.
- `rivet-envoy-client/src/connection.rs`: split `tokio-tungstenite` native transport from wasm `web-sys` transport.
- `rivetkit-sqlite`: never compiled for wasm.
- `rivet-pools`, `rivet-util`, and `rivet-metrics`: stop pulling broad engine/native dependencies into `rivetkit-core` for wasm.

### File-Level Change Plan

Phase 1 remote SQLite changes:

| File or package | Change |
|---|---|
| `engine/sdks/schemas/envoy-protocol/v4.bare` | Add remote SQL request/response messages and SQL value/result types. |
| `engine/sdks/rust/envoy-protocol/src/versioned.rs` | Wire v4 and reject remote SQL against older protocol versions with explicit structured errors. |
| `engine/sdks/rust/envoy-client/src/envoy.rs` | Add a `ToEnvoyMessage` variant for remote SQL execution requests and cleanup behavior on shutdown. |
| `engine/sdks/rust/envoy-client/src/sqlite.rs` | Add request ID tracking and response matching for remote SQL execution, separate from existing page/VFS requests. |
| `engine/sdks/rust/envoy-client/src/handle.rs` | Add a handle method used by core to send remote SQL requests and await responses. |
| `engine/sdks/rust/envoy-client/src/stringify.rs` | Add stringifiers for the new v4 SQL execution messages. |
| `rivetkit-rust/packages/rivetkit-core/src/actor/sqlite.rs` | Add `SqliteBackend::{LocalNative, RemoteEnvoy, Unavailable}` routing and preserve existing public `SqliteDb` methods. |
| `rivetkit-rust/packages/rivetkit-core/src/actor/context.rs` | Select the SQLite backend when `ActorContext::sql()` constructs the database handle. Remote selection requires an envoy handle and remote capability. |
| `rivetkit-rust/packages/rivetkit-core/Cargo.toml` | Split `sqlite-local` from `sqlite-remote`; keep native local SQLite optional and unavailable for wasm. |
| `rivetkit-rust/packages/rivetkit-sqlite/` | Extract reusable execution/result/classification pieces so pegboard-envoy and actor-local native SQLite share behavior. |
| `engine/packages/pegboard-envoy/src/sqlite_runtime.rs` | Add the lazy per-`(actor_id, generation)` SQL executor registry, first-use creation, in-flight accounting, and actor-close cleanup. |
| `engine/packages/pegboard-envoy/src/conn.rs` and related message dispatch files | Dispatch new remote SQL messages to `sqlite_runtime` without blocking the WebSocket read loop. |
| `engine/packages/pegboard-envoy/src/errors.rs` | Add structured errors for remote SQLite unavailable, stale generation, size limits, and indeterminate lost-response behavior. |
| `rivetkit-typescript/packages/rivetkit/tests/driver/shared-types.ts` | Add matrix fields for `runtime` and `sqliteBackend` alongside `encoding`. |
| `rivetkit-typescript/packages/rivetkit/tests/driver/shared-matrix.ts` | Generate native/local/all encodings, native/remote/all encodings, and wasm/remote/all encodings. Exclude wasm/local. |
| `rivetkit-typescript/packages/rivetkit/tests/driver/actor-db*.test.ts` | Run existing SQLite coverage across the new valid matrix and add lazy-create/cleanup assertions. |

Phase 2 wasm transport and build changes:

| File or package | Change |
|---|---|
| `engine/sdks/rust/envoy-client/Cargo.toml` | Add `native-transport` and `wasm-transport` features. Make `tokio-tungstenite` optional. Add optional wasm dependencies. |
| `engine/sdks/rust/envoy-client/src/connection.rs` | Replace with a module wrapper or move to `connection/mod.rs`. Keep the public `start_connection(shared)` API stable. |
| `engine/sdks/rust/envoy-client/src/connection/native.rs` | Move the current `tokio-tungstenite` implementation here with minimal behavior changes. |
| `engine/sdks/rust/envoy-client/src/connection/wasm.rs` | Implement `web-sys::WebSocket` transport, ArrayBuffer decoding, subprotocol auth, close parsing, reconnect, and metadata send. |
| `engine/sdks/rust/envoy-client/src/context.rs` | Keep shared protocol state transport-agnostic. Replace native-only channel assumptions if wasm needs local channels. |
| `engine/sdks/rust/envoy-client/src/utils.rs` | Keep backoff and close-reason parsing shared by native and wasm transports. Gate native-only helpers. |
| `rivetkit-rust/packages/rivetkit-core/Cargo.toml` | Add `native-runtime`, `wasm-runtime`, `sqlite-local`, and `sqlite-remote` feature mappings. Gate native dependencies. |
| `rivetkit-rust/packages/rivetkit-core/src/lib.rs` | Gate exports for `engine_process`, native serverless helpers, and any module that cannot compile on wasm. |
| `rivetkit-rust/packages/rivetkit-core/src/engine_process.rs` | Native-only behind `native-runtime`; wasm path returns explicit unsupported configuration errors. |
| `rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs` | Keep lifecycle logic in core, but construct envoy with runtime-selected features and no direct dependency on `web_sys`. |
| `rivetkit-rust/packages/rivetkit-core/src/registry/runner_config.rs` | Move HTTP fetches behind `HttpClient` so wasm can use a JS/fetch-backed implementation or fail explicitly. |
| `rivetkit-rust/packages/rivetkit-core/src/serverless.rs` | Split pure request/response parsing from native HTTP/client assumptions. |
| `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs` and lifecycle spawn sites | Replace direct native spawn assumptions with `RuntimeSpawner` or an equivalent core helper. |
| `rivetkit-typescript/packages/rivetkit-napi/` | Current native Node binding package. Keep native-only for production edge support. The NAPI-RS wasm spike only supports sync calls in workerd and fails on async/callback-style surfaces. |
| `rivetkit-typescript/packages/rivetkit-wasm/` | New wasm binding package that wraps `rivetkit-core` through direct `wasm-bindgen` on `wasm32-unknown-unknown`. |
| `rivetkit-typescript/packages/rivetkit/src/registry/core-runtime-interface.ts` | New bridge-neutral TypeScript interface implemented by both NAPI and wasm bindings. Exact file name can change, but the boundary must exist. |
| `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` | Refactor NAPI-specific loading and NAPI object adaptation behind the shared core-runtime interface instead of serving as the only runtime glue. |
| `rivetkit-typescript/packages/rivetkit/src/registry/wasm.ts` | New wasm-specific loader/adaptor that imports the selected wasm binding package and implements the same core-runtime interface. |

### Build Targets

Support Supabase Edge Functions and Cloudflare Workers as first-class wasm hosts.

Host requirements from docs:

| Host | Documented wasm model | Implication |
|---|---|---|
| Supabase Edge Functions | Deno-based functions can load wasm generated with `wasm-pack --target deno`. | The wasm package must work in Deno and avoid Node-only NAPI assumptions. |
| Cloudflare Workers | Workers can import/instantiate `.wasm` modules, but threading is not possible in Workers and WASI support is experimental. | The wasm package must not require wasm threads, SharedArrayBuffer/Atomics, or a full WASI runtime. |

Target direct wasm-bindgen on `wasm32-unknown-unknown`, packaged/tested for both Deno/Supabase and Cloudflare Workers. Browser main thread, Node wasm, and WASI are follow-up targets unless they pass the same Supabase and Cloudflare host contract explicitly.

Expected packages:

- `rivetkit-core` wasm library.
- `@rivetkit/rivetkit-wasm`, a direct wasm-bindgen binding package over `rivetkit-core`.
- Separate native NAPI package remains unchanged.

### TypeScript Runtime Boundary

The TypeScript glue must be a separate layer from `rivetkit-core`. `rivetkit-core` should expose Rust runtime primitives; it should not contain TypeScript package loading, JS promise conversion, NAPI-specific public API design, or wasm-bindgen-specific public API design. The wasm binding belongs in a separate `rivetkit-wasm` package, equivalent in role to `rivetkit-napi`.

Recommended package shape:

| Layer | Responsibility |
|---|---|
| `rivetkit-core` | Shared Rust actor runtime, lifecycle, state, sleep, queue, schedule, KV/SQLite handles, and envoy integration. No NAPI, NAPI-RS wasm, or wasm-bindgen exports. |
| `rivetkit-napi` | Node N-API binding over `rivetkit-core`. Native-only. Owns N-API object wrappers, ThreadsafeFunction bridging, Node buffers, and native Tokio interop. |
| `rivetkit-wasm` | Wasm binding over `rivetkit-core`. Owns wasm-bindgen classes/functions, JS Promise conversion, `Uint8Array`/ArrayBuffer conversion, wasm-local callback handling, and host setup for Supabase Edge Functions and Cloudflare Workers. |
| `rivetkit` TypeScript package | Public TypeScript actor API. Chooses a runtime binding and adapts it through a shared TypeScript interface. |

The current TypeScript NAPI glue in `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` should not be duplicated wholesale for wasm. It should be split into:

- Runtime-independent TypeScript actor adaptation: actor definition lookup, schema validation, action/request/connection callback adaptation, state serialization, vars, workflow/agent-os integration, client construction, and error decoding.
- Runtime-specific binding adaptation: loading `@rivetkit/rivetkit-napi` or `@rivetkit/rivetkit-wasm`, converting JS values to that binding's ABI, cancellation token wiring, buffer conversion, and host-specific callback scheduling.

Define a shared TypeScript interface first, then make both bindings implement it. The local `JsNativeDatabaseLike` and `NativeDatabaseProvider` shapes are already a small example of this pattern, but the broader runtime interface should be flatter than the current generated NAPI class graph.

Use explicit methods with opaque handles. Do not expose a command bus, and do not expose rich binding classes across the shared TypeScript boundary. The NAPI and wasm adapters may internally wrap generated classes, but the rest of `rivetkit` should see only the flat interface.

Keep the handle set small:

- `RegistryHandle`
- `ActorFactoryHandle`
- `ActorContextHandle`
- `ConnHandle`
- `WebSocketHandle`
- `CancellationTokenHandle`

Do not expose separate shared-interface handles for KV, SQLite, queue, or schedule unless a later implementation proves it is necessary. Route those operations through `ActorContextHandle`.

Initial interface sketch:

```ts
type RegistryHandle = unknown;
type ActorFactoryHandle = unknown;
type ActorContextHandle = unknown;
type ConnHandle = unknown;
type WebSocketHandle = unknown;
type CancellationTokenHandle = unknown;

interface CoreRuntimeBindings {
	createCancellationToken(): CancellationTokenHandle;
	cancelTokenCancel(token: CancellationTokenHandle): void;

	createRegistry(): Promise<RegistryHandle>;
	registryRegisterActor(
		registry: RegistryHandle,
		name: string,
		factory: ActorFactoryHandle,
	): Promise<void>;
	registryServe(
		registry: RegistryHandle,
		config: CoreServeConfig,
	): Promise<void>;
	registryShutdown(registry: RegistryHandle): Promise<void>;

	createActorFactory(
		callbacks: CoreActorCallbacks,
		config: CoreActorConfig,
	): Promise<ActorFactoryHandle>;

	registryHandleServerlessRequest?(
		registry: RegistryHandle,
		request: CoreServerlessRequest,
		onStreamEvent: CoreServerlessStreamCallback,
		cancelToken: CancellationTokenHandle,
		config: CoreServeConfig,
	): Promise<CoreServerlessResponseHead>;

	actorId(ctx: ActorContextHandle): string;
	actorState(ctx: ActorContextHandle): Uint8Array;
	actorRequestSave(
		ctx: ActorContextHandle,
		opts?: CoreRequestSaveOpts,
	): Promise<void>;

	kvBatchGet(
		ctx: ActorContextHandle,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]>;
	kvBatchPut(
		ctx: ActorContextHandle,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void>;
	kvBatchDelete(ctx: ActorContextHandle, keys: Uint8Array[]): Promise<void>;
	kvDeleteRange(
		ctx: ActorContextHandle,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void>;

	sqliteExec(ctx: ActorContextHandle, sql: string): Promise<CoreSqliteExecResult>;
	sqliteExecute(
		ctx: ActorContextHandle,
		sql: string,
		params: CoreSqliteBindParam[] | null,
		mode: "readWrite" | "forceWrite",
	): Promise<CoreSqliteExecuteResult>;
	sqliteClose(ctx: ActorContextHandle): Promise<void>;

	queueSend(
		ctx: ActorContextHandle,
		queue: string,
		message: Uint8Array,
	): Promise<void>;

	scheduleSetAlarm(
		ctx: ActorContextHandle,
		timestamp: number,
	): Promise<void>;

	webSocketSend(
		ws: WebSocketHandle,
		data: Uint8Array,
		binary: boolean,
	): Promise<void>;
	webSocketClose(
		ws: WebSocketHandle,
		code?: number,
		reason?: string,
	): Promise<void>;
}
```

This interface is the cleanup point. Native NAPI and wasm may expose different raw generated bindings, but `rivetkit` should only depend on the normalized `CoreRuntimeBindings` contract. That keeps the public TypeScript actor API unified while allowing each binding to use the ABI that fits its host.

Boundary rules:

- `rivetkit-core` must not depend on `napi`, `wasm-bindgen`, `web-sys`, `js-sys`, Node buffers, or TypeScript package-loading behavior. Host bindings wrap core; core does not wrap hosts.
- `rivetkit-napi` and `rivetkit-wasm` are the only packages that may expose generated host ABI classes/functions.
- `rivetkit-typescript/packages/rivetkit` must not import raw generated binding classes outside the runtime adapters. Direct imports of `@rivetkit/rivetkit-napi` belong in the NAPI adapter; direct imports of `@rivetkit/rivetkit-wasm` belong in the wasm adapter.
- The shared TypeScript runtime boundary uses explicit methods plus opaque handles, not generated classes and not a generic command bus.
- Runtime-independent actor glue should live behind the shared interface: actor definition lookup, schema validation, callback adaptation, state serialization, vars, workflow/agent-os integration, client construction, and error decoding.
- Runtime-specific adapter code owns ABI conversion: Node `Buffer` vs `Uint8Array`, NAPI errors vs wasm thrown values, cancellation token wrappers, callback scheduling, and host-specific startup.
- Add type tests proving both adapters satisfy `CoreRuntimeBindings`.
- Add a static guard or lint check that fails if raw generated binding imports appear outside the approved adapter files.

### Wasm Binding Strategy Decision

Use direct wasm-bindgen on `wasm32-unknown-unknown` for the production edge-host path.

The NAPI-RS wasm spike in `/home/nathan/misc/napi-rs-wasm-test/` proved that sync-only NAPI-RS wasm can run through a custom workerd loader, but async/callback-style surfaces fail in workerd:

```text
failed to spawn thread: Error { kind: Unsupported, message: "operation not supported on this platform" }
RuntimeError: unreachable
```

That failure is decisive for RivetKit because the real boundary needs async methods, callback dispatch, cancellation, and JS promise interop. NAPI-RS wasm remains useful as a Node fallback/playground path, but not as the mainline Supabase/Cloudflare binding strategy.

Why disabling the worker pool is not enough:

- The generated NAPI-RS browser loader uses shared wasm memory, `asyncWorkPoolSize: 4`, and `new Worker(...)` by default.
- The custom workerd loader can set `asyncWorkPoolSize: 0`, which makes emnapi use a single-thread mock for `napi_create_async_work` / `napi_queue_async_work`.
- That does not affect NAPI-RS `#[napi] async fn` plumbing. The generated wrapper calls `execute_tokio_future_with_finalize_callback`, and the non-`tokio_unstable` wasm path uses `std::thread::spawn(|| block_on(inner))`.
- In workerd, that `std::thread::spawn` panics because Workers do not support threads.
- Making NAPI-RS viable would require upstream-level support for a single-thread wasm target/loader, host-event-loop-driven Rust futures, and non-threaded callback/deferred resolution. This is not a RivetKit config toggle.

Decision record:

| Option | Shape | Main benefit | Main risk |
|---|---|---|---|
| NAPI-RS wasm | Reuse a NAPI-shaped binding surface compiled to wasm, likely as a separate wasm package. | Sync-only exports can run in local workerd with a custom loader. | Async/callback exports try to spawn threads and fail in workerd. Generated loader also is not directly Cloudflare-compatible. Not viable for RivetKit's edge runtime boundary. |
| Direct wasm-bindgen | Create a separate wasm binding package over `rivetkit-core`. | Supabase/Cloudflare-compatible ABI with direct `Promise`, `Uint8Array`, and standard host WebSocket patterns. | More binding code to write and maintain beside `rivetkit-napi`. |

Prior art checked:

- `napi-rs` supports building N-API projects for WebAssembly. The current docs say the default support is `wasm32-wasip1-threads`, aimed at Node fallback/playgrounds/browser repros, and browser usage relies on `SharedArrayBuffer`/Atomics headers.
- `emnapi` can emulate Node-API on WebAssembly and supports browser execution, but it preserves the Node-API programming model and can introduce thread/SAB constraints that do not match a clean browser-compatible Web Worker target.
- `wasm-bindgen` is the standard Rust-to-JS wasm binding path and can generate TypeScript-facing JS classes/functions, but it is not N-API-compatible.

Conclusion: keep `rivetkit-core` binding-agnostic, keep `rivetkit-napi` native-only for production, and build `rivetkit-wasm` as a direct wasm-bindgen binding. `rivetkit` TypeScript must consume a normalized `CoreRuntimeBindings` interface so the public actor API stays unified.

### TypeScript API Parity Matrix

Feature parity means the wasm package preserves these public TypeScript surfaces or explicitly fails the phase:

| Surface | Parity requirement |
|---|---|
| Actor lifecycle and actions | Same start, run, action, stop, sleep, destroy, and error sanitization behavior as NAPI. |
| State | Same persisted state serialization and `onStateChange` semantics. |
| Vars | Same TypeScript-owned vars behavior in the wasm wrapper. Do not move vars into core. |
| KV | Same batch operations, ordering, errors, and reconnect behavior as native. |
| SQLite | Remote SQL passes the phase 1 parity suite, including migrations and `writeMode`. |
| Schedule/alarms | Same persisted alarm dedup, wake, and local alarm behavior. |
| Queue | Same enqueue, receive, completion, cancellation, and sleep gating behavior. |
| Connections/WebSockets | Same hibernatable connection persistence, raw WebSocket callbacks, close handler gating, and ack behavior where supported by the host. |
| Client | Actor-to-actor client construction works or fails with explicit unsupported errors for surfaces impossible in the selected host. |
| `abortSignal`, `keepAwake`, `waitUntil` | Same shutdown and sleep gating semantics. |
| Inspector | Protocol support remains compatible. Serving inspector HTTP inside wasm is not required for the first host unless the package claims full embedded serving. |
| Workflow/agent-os | Must be either implemented by the wasm TypeScript wrapper or explicitly declared out of scope before phase 2 starts. |

### Acceptance Criteria

- `cargo check -p rivetkit-core --target wasm32-unknown-unknown --no-default-features --features wasm-runtime,sqlite-remote` passes.
- The above check uses a wasm dependency graph that excludes native networking/process crates, verified with `cargo tree --target wasm32-unknown-unknown`.
- `rivet-envoy-client` has a wasm transport that can connect to pegboard-envoy using browser/Web Worker WebSocket APIs.
- The wasm build does not include `rivetkit-sqlite`, `libsqlite3-sys`, `tokio-tungstenite`, `mio`, `nix`, native `reqwest` pooling, or engine process spawning.
- A wasm runtime can start an actor, receive a command from pegboard-envoy, run an action, persist state, use KV, and execute SQLite remotely.
- Deterministic wasm parity tests cover reconnect during action, reconnect during remote write SQL, actor stop with in-flight SQL, stale-generation SQL, duplicate command replay, KV failure sanitization, and sleep finalization blocked by remote SQL.
- Native persisted actor state can round-trip native to wasm to native for state, schedule, queue, hibernatable connection metadata, and inspector-visible fields.
- Existing native NAPI tests continue to pass.
- Wasm smoke tests run in both Supabase Edge Functions/Deno and Cloudflare Workers and verify subprotocol-token WebSocket auth.

### Build Gate

Run `scripts/cargo/check-rivetkit-core-wasm.sh` before claiming wasm dependency hygiene. It performs the checked `rivetkit-core` wasm compile, scans the normal wasm dependency tree for native-only crates, verifies the wasm feature graph excludes `native-runtime` and `native-transport`, and asserts native envoy/core runtime feature selections fail for `wasm32-unknown-unknown`.

## Questions And Decisions

- Decision: remote SQLite is the only SQLite backend for wasm in phase 1/2. A wasm SQLite VFS can be reconsidered later.
- Decision: remote SQL execution uses the existing envoy WebSocket because it already has actor lifecycle, namespace validation, reconnect, and generation fencing.
- Decision: no streaming result rows in phase 1. Match the existing `execute` API and reject oversized results.
- Decision: Supabase Edge Functions/Deno and Cloudflare Workers are first-class wasm hosts.
- Decision: direct wasm-bindgen on `wasm32-unknown-unknown` is the wasm binding path for Supabase and Cloudflare.
- Decision: NAPI-RS wasm is not viable for the mainline edge-host binding because the spike showed async/callback surfaces fail in workerd when Rust tries to spawn a thread.
- Open: exact numeric defaults for SQL text, bind bytes, row count, cell bytes, response bytes, and execution timeout.
- Decision: sent remote SQL requests fail with `sqlite.remote_indeterminate_result` on lost responses; durable request ID deduplication is a future enhancement.
- Decision: remote SQL calls already accepted before actor stopping may finish, but new calls after stopping begins are rejected.
- Open: whether workflow/agent-os are in scope for the first wasm package or deferred as explicit non-goals.
- Decision: Node wasm and WASI are follow-up targets. They do not replace Supabase and Cloudflare acceptance.
- Do we need inspector HTTP handled inside wasm? I recommend no for the first wasm milestone; preserve inspector protocol support but leave HTTP serving to the host.

## Concerns

- Remote SQL performance will add one network round trip per query unless batching is added later. This is acceptable for wasm enablement, but benchmarks should set expectations.
- Server-side SQL execution must not bypass actor exclusivity. The safest model is one SQL executor per active `(actor_id, generation)` owned by the same pegboard-envoy connection that owns the actor.
- If we duplicate SQL execution code between `rivetkit-sqlite` and `pegboard-envoy`, route drift is likely. Extracting the VFS/backend abstraction is more work up front but less risky.
- Wasm support will expose broad dependency hygiene problems. `rivetkit-core` currently depends on engine utility crates that pull in native networking and metrics trees. Phase 2 should aggressively narrow those dependencies.
- `tokio::spawn` and `Send` assumptions may be more invasive than the WebSocket binding itself. Treat spawn/timer abstraction as a first-class part of the wasm work.
- `rand::thread_rng` and `getrandom` paths may need target-specific features for wasm randomness.

## External References Checked

- wasm-bindgen WebSocket example: https://rustwasm.github.io/docs/wasm-bindgen/examples/websockets.html
- wasm-bindgen futures `spawn_local`: https://docs.rs/wasm-bindgen-futures/latest/wasm_bindgen_futures/fn.spawn_local.html
- wasm-bindgen TypeScript custom sections: https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-rust-exports/typescript_custom_section.html
- emnapi overview: https://emnapi-docs.vercel.app/
- emnapi FAQ on browser/WebAssembly differences from native Node-API: https://toyobayashi.github.io/emnapi-docs/guide/faq.html
- Supabase Edge Functions WebAssembly guide: https://supabase.com/docs/guides/functions/wasm
- Cloudflare Workers WebAssembly API docs: https://developers.cloudflare.com/workers/runtime-apis/webassembly/
- reqwest crate docs for WebAssembly support: https://docs.rs/reqwest/latest/reqwest/
- Local compile gate: `scripts/cargo/check-rivetkit-core-wasm.sh`.
