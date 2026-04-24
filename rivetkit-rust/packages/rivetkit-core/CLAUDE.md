# rivetkit-core

## Module layout

- Actor subsystem implementations belong under `src/actor/`; keep root module aliases only for compatibility with existing public callers.

## Sleep state invariants

- Any mutation that changes a `can_sleep` input must call `ActorContext::reset_sleep_timer()` so the `ActorTask` sleep deadline is re-evaluated. Inputs are: `ready`/`started`, `no_sleep`, `active_http_request_count`, `sleep_keep_awake_count`, `sleep_internal_keep_awake_count`, `pending_disconnect_count`, `conns()`, and `websocket_callback_count`. Missing this call leaves the sleep timer armed against stale state and triggers the `"sleep idle deadline elapsed but actor stayed awake"` warning on the next tick.
- `ActorContext::set_prevent_sleep(...)` / `prevent_sleep()` are deprecated no-ops kept for NAPI bridge compatibility. Use `keep_awake(future)` (holds counter while awaited) or `wait_until(future)` (tracked shutdown task) instead. Do not reintroduce a `prevent_sleep` field, a `CanSleep::PreventSleep` variant, or branches that read it.
- `ctx.sleep()` and `ctx.destroy()` return `Result<()>`. They error with `ActorLifecycleError::Starting` when called before startup completes and `ActorLifecycleError::Stopping` if the requested flag has already been set this generation (atomic `swap(true, ...)`). Internal idle-timer paths log and suppress the already-requested error.
- The grace deadline path (`on_sleep_grace_deadline`) aborts the user `run` handle and cancels `shutdown_deadline_token()`. Foreign-runtime adapters running `onSleep` / `onDestroy` must observe that token via `tokio::select!` so SQLite teardown does not race user cleanup work.
- Counter `register_zero_notify(&idle_notify)` hooks only drive shutdown drain waits. They are not a substitute for the activity-dirty notification, so any new sleep-affecting counter must also notify on transitions that change `can_sleep`.
- A clean `run` exit while `Started` is not terminal. Keep the generation alive until the guaranteed `Stop` drives `SleepGrace` or `DestroyGrace`, and only treat `Terminated` as "grace hooks already completed."
- Do not reply to actor startup until the runtime adapter has acknowledged its startup preamble. Otherwise `getOrCreate` can race the first action against `onWake` or `run` startup.
- When forwarding an existing `anyhow::Error` across lifecycle/action replies, preserve structured `RivetError` data with `RivetError::extract` instead of stringifying it.
- `ActorContext::request_save(...)` is intentionally fire-and-forget: lifecycle inbox overloads only emit a warning. Use `request_save_and_wait(...)` when callers need a `Result` and must observe delivery failures.

## Queue invariants

- Register queue completion waiters before publishing messages to KV so fast consumers cannot complete a message before the waiter exists.

## Hibernatable WebSockets

- Raw `onWebSocket` hibernatable connections must create `HibernatableConnectionMetadata` and persist plus ack every inbound message through core before gateway replay state is correct.
- Actor-connect WebSocket setup errors must send a protocol `Error` frame before closing; JSON/CBOR connection-level errors include `actionId: null`.
- Flush the actor-connect `WebSocketSender` after queuing a setup `Error` frame and before closing so the envoy writer handles the error before the close terminates the connection.
- Bound actor-connect websocket setup at the registry boundary as well as inside the actor task. The HTTP upgrade can complete before `connection_open` replies, so a missing reply must still close the socket instead of idling until the client test timeout.

## Run modes

- Two run modes exist for `CoreRegistry`. **Persistent envoy**: `serve_with_config(...)` starts one outbound envoy via `start_envoy` and holds it for the process lifetime; used by standalone Rust binaries and TS `registry.start()`. **Serverless request**: `into_serverless_runtime(...)` returns a `CoreServerlessRuntime` whose `handle_request(...)` lazily starts an envoy on first request via `ensure_envoy(...)` and caches it; used by Node/Bun/Deno HTTP hosts and platform fetch handlers. Both modes end up holding a long-lived `EnvoyHandle`.
- Shutdown is a property of the host's `CoreRegistry` handle, not of whichever entrypoint ran first. Route process-level shutdown (SIGINT/SIGTERM) through a single `CoreRegistry::shutdown()` that trips one shared cancel token observed by `serve_with_config` and calls `CoreServerlessRuntime::shutdown()` on the cached runtime. Never attach the shutdown signal to `serve_with_config`'s config parameter — that misses Mode B entirely.
- `rivetkit-core` and `rivet-envoy-client` must not install process signal handlers (no `tokio::signal::ctrl_c()` in library code). `tokio::signal::ctrl_c()` calls `sigaction(SIGINT, ...)` at the POSIX level and prevents Node from exiting when rivetkit is embedded via NAPI. Signal policy belongs to the host binary or the TS registry layer.
- Per-request `CancellationToken` on `handle_serverless_request` cancels a single in-flight request and does not tear down the cached envoy. Do not overload it with registry shutdown.

## Test harness

- `tests/modules/task.rs` tests that install a tracing subscriber with `set_default(...)` must take `test_hook_lock()` first, or full `cargo test` parallelism makes the log-capture assertions flaky.
