# Spec: Rust Envoy Client (1:1 port of TypeScript)

Port `engine/sdks/typescript/envoy-client/` (~3600 LOC) to a production Rust crate at `engine/sdks/rust/envoy-client/`.

---

## Crate Setup

- [ ] Create `engine/sdks/rust/envoy-client/` with `Cargo.toml`
- [ ] Add to workspace members and workspace dependencies
- [ ] Dependencies: `tokio`, `tokio-tungstenite`, `rivet-envoy-protocol`, `tracing`, `anyhow`, `uuid`, `serde`, `serde_bare`, `rand`, `futures-util`
- [ ] Re-export `rivet-envoy-protocol` as `protocol` from crate root (mirrors TS `export * as protocol`)

---

## Config (`config.rs`) — mirrors `config.ts` (170 LOC)

- [ ] Define `EnvoyConfig` struct with fields:
  - `endpoint: String`
  - `namespace: String`
  - `envoy_key: String`
  - `pool_name: String`
  - `token: Option<String>`
  - `prepopulated_actors: Vec<PrepopulatedActor>` (name, tags)
  - `auto_restart: bool`
  - `metadata: HashMap<String, String>`
- [ ] Define callback traits/closures for:
  - `on_actor_start(handle, actor_id, generation, config, preloaded_kv) -> Result<()>`
  - `on_actor_stop(handle, actor_id, generation, reason) -> Result<()>`
  - `on_shutdown()`
  - `fetch(handle, actor_id, gateway_id, request_id, request) -> Result<Response>`
  - `websocket(handle, actor_id, ws, gateway_id, request_id, request, path, headers, is_hibernatable, is_restoring) -> Result<()>`
  - `can_hibernate(actor_id, gateway_id, request_id, request) -> bool`

---

## Shared Context (`context.rs`) — mirrors `context.ts` (27 LOC)

- [ ] Define `SharedContext` holding shared state:
  - WebSocket sender (`Option<SplitSink>` behind `Arc<Mutex>` or similar)
  - `shutting_down: AtomicBool`
  - `protocol_metadata: Option<ProtocolMetadata>`
  - Actors map: `scc::HashMap<String, ActorEntry>`

---

## Handle (`handle.rs`) — mirrors `handle.ts` (100 LOC)

- [ ] Define `EnvoyHandle` with methods:
  - `shutdown(immediate: bool)`
  - `get_protocol_metadata() -> Option<ProtocolMetadata>`
  - `get_envoy_key() -> String`
  - `started() -> impl Future` (wait for init)
  - `get_actor(actor_id, generation) -> Option<ActorEntry>`
  - `sleep_actor(actor_id, generation)`
  - `stop_actor(actor_id, generation, error)`
  - `destroy_actor(actor_id, generation)`
  - `set_alarm(actor_id, alarm_ts, generation)`
- [ ] KV operations (all async, send request and await response):
  - `kv_get(actor_id, keys) -> Vec<Option<Vec<u8>>>`
  - `kv_list_all(actor_id, opts) -> Vec<(Vec<u8>, Vec<u8>)>`
  - `kv_list_range(actor_id, start, end, exclusive, opts) -> Vec<(Vec<u8>, Vec<u8>)>`
  - `kv_list_prefix(actor_id, prefix, opts) -> Vec<(Vec<u8>, Vec<u8>)>`
  - `kv_put(actor_id, entries) -> ()`
  - `kv_delete(actor_id, keys) -> ()`
  - `kv_delete_range(actor_id, start, end) -> ()`
  - `kv_drop(actor_id) -> ()`
- [ ] Tunnel/WebSocket operations:
  - `restore_hibernating_requests(actor_id, meta_entries)`
  - `send_hibernatable_ws_message_ack(gateway_id, request_id, client_message_index)`
  - `start_serverless_actor(payload)`

---

## Utils (`utils.rs`) — mirrors `utils.ts` (222 LOC)

- [ ] `calculate_backoff(attempt, base, max, jitter) -> Duration`
- [ ] `inject_latency(ms)` (debug-only sleep)
- [ ] `parse_ws_close_reason(reason) -> Option<ParsedCloseReason>`
- [ ] Wrapping u16 arithmetic: `wrapping_add_u16`, `wrapping_sub_u16`, `wrapping_gt_u16`, `wrapping_lt_u16`, `wrapping_gte_u16`, `wrapping_lte_u16`
- [ ] `BufferMap<T>` — hash map keyed by `Vec<Vec<u8>>` (equivalent of TS `BufferMap`)
- [ ] `id_to_str(id: &[u8]) -> String` (hex encoding)
- [ ] `stringify_error(err) -> String`
- [ ] `EnvoyShutdownError` error type

---

## Logger (`log.rs`) — mirrors `log.ts` (11 LOC)

- [ ] Use `tracing` crate (already standard in the codebase)
- [ ] No Pino equivalent needed; just use `tracing::info!`, `tracing::warn!`, etc. with structured fields

---

## Stringify (`stringify.rs`) — mirrors `stringify.ts` (300 LOC)

- [ ] Debug formatting for protocol messages (for tracing output)
- [ ] `stringify_to_rivet(msg) -> String`
- [ ] `stringify_to_envoy(msg) -> String`
- [ ] Format each message variant with key fields for readability

---

## WebSocket Transport (`websocket.rs`) — mirrors `websocket.ts` (349 LOC)

- [ ] Wrapper around `tokio-tungstenite` WebSocket
- [ ] `EnvoyWebSocket` struct wrapping the split sink/stream
- [ ] Send binary messages (encode via `protocol::encode_to_rivet`)
- [ ] Receive binary messages (decode via `protocol::decode_to_envoy`)
- [ ] Optional latency injection for testing (`latency_ms: Option<u64>`)
- [ ] Close handling with reason parsing

---

## Connection Task (`connection.rs`) — mirrors `connection.ts` (228 LOC)

- [ ] `start_connection()` — spawns tokio task running `connection_loop()`
- [ ] `connection_loop()` — retry loop with exponential backoff
- [ ] `single_connection()` — one WebSocket connection attempt:
  - Build URL: `{endpoint}/envoys/connect?protocol_version=...&namespace=...&envoy_key=...&pool_name=...`
  - Convert http/https to ws/wss
  - Set subprotocols: `["rivet"]` and optionally `["rivet_token.{token}"]`
  - Forward received messages to envoy task via channel
  - Handle close/error and determine reconnect vs shutdown
- [ ] Backoff reset after 60s stable connection
- [ ] "Lost threshold" timer — stop all actors if disconnected for N seconds (from protocol metadata)

---

## Envoy Core (`envoy.rs`) — mirrors `envoy/index.ts` (730 LOC)

- [ ] `start_envoy(config) -> EnvoyHandle` (async)
- [ ] `start_envoy_sync(config) -> EnvoyHandle` (spawns task, returns handle immediately)
- [ ] Envoy loop:
  - Receive messages from connection task
  - Route `ToEnvoyInit` → store metadata, mark started
  - Route `ToEnvoyCommands` → dispatch to command handler
  - Route `ToEnvoyAckEvents` → clean up acknowledged events
  - Route `ToEnvoyKvResponse` → resolve pending KV requests
  - Route `ToEnvoyTunnelMessage` → route to actor task
  - Route `ToEnvoyPing` → reply with `ToRivetPong`
- [ ] On connection close: buffer tunnel messages, track disconnection time
- [ ] On reconnect: resend unacknowledged events, send `ToRivetInit`
- [ ] Graceful shutdown: send `ToRivetStopping`, wait for actors, close WS

---

## Commands (`commands.rs`) — mirrors `envoy/commands.ts` (94 LOC)

- [ ] `handle_command(cmd)` dispatcher
- [ ] `CommandStartActor` → create actor entry, spawn actor task, call `on_actor_start`
- [ ] `CommandStopActor` → signal actor to stop
- [ ] Acknowledge commands back to server via `ToRivetAckCommands`

---

## Events (`events.rs`) — mirrors `envoy/events.ts` (84 LOC)

- [ ] Event queue with batching
- [ ] `push_event(event)` — add to pending queue
- [ ] `send_events()` — batch and send via `ToRivetEvents`
- [ ] Track event history until acknowledged by server
- [ ] On `ToEnvoyAckEvents` — remove acknowledged events from history, clean up stopped actors

---

## Tunnel (`tunnel.rs`) — mirrors `envoy/tunnel.ts` (246 LOC)

- [ ] Route `TunnelMessageHttpReqStart` → actor task (create request)
- [ ] Route `TunnelMessageHttpReqChunk` → actor task (append body)
- [ ] Route `TunnelMessageWsOpen` → actor task (open WS)
- [ ] Route `TunnelMessageWsIncomingMessage` → actor task (dispatch WS message)
- [ ] Route `TunnelMessageWsClose` → actor task (close WS)
- [ ] `HibernatingWebSocketMetadata` struct (path, headers, message index)
- [ ] `restore_hibernating_requests()` — recreate WS state from metadata
- [ ] Wrapping u16 message index tracking for hibernatable WS gap/duplicate detection

---

## KV (`kv.rs`) — mirrors `envoy/kv.ts` (114 LOC)

- [ ] KV request/response matching via request ID
- [ ] Pending requests map: `HashMap<RequestId, oneshot::Sender<KvResponse>>`
- [ ] 30 second timeout per request
- [ ] `send_kv_request(actor_id, request) -> Result<KvResponse>`
- [ ] `handle_kv_response(response)` — resolve pending request

---

## Actor Task (`actor.rs`) — mirrors `actor.ts` (871 LOC)

- [ ] Per-actor tokio task managing:
  - HTTP request handling (receive tunnel messages, call `fetch` callback, send response)
  - Streaming request bodies via channel
  - WebSocket lifecycle (open, message, close via `VirtualWebSocket` or Rust equivalent)
  - Hibernatable WebSocket support
  - Stop/sleep/destroy intent handling
  - Alarm setting
- [ ] `ActorEntry` struct:
  - `actor_id: String`
  - `generation: u16`
  - `config: ActorConfig`
  - Actor state (running, stopping, stopped)
  - Active request tracking
  - Event sender channel
- [ ] Request routing:
  - `handle_req_start()` → build Request, call fetch, send HTTP response back
  - `handle_req_chunk()` → append to streaming body
  - `handle_ws_open()` → call websocket callback
  - `handle_ws_message()` → dispatch to virtual WS
  - `handle_ws_close()` → close virtual WS
- [ ] Send tunnel response messages:
  - `TunnelMessageHttpResStart` (status, headers)
  - `TunnelMessageHttpResChunk` (body chunks)
  - `TunnelMessageHttpResEnd`
  - `TunnelMessageWsReady`
  - `TunnelMessageWsOutgoingMessage`
  - `TunnelMessageWsClose`

---

## Latency Channel (`latency_channel.rs`) — mirrors `latency-channel.ts` (39 LOC)

- [ ] Debug-only wrapper that adds configurable delay to channel sends
- [ ] Used for testing reconnection behavior under latency

---

## Migration: Update `test-envoy` to Use New Crate

- [ ] Refactor `test-envoy` to depend on `rivet-envoy-client` instead of inlining envoy logic
- [ ] `test-envoy` becomes a thin wrapper providing `TestActor` implementations via the callback API
- [ ] Verify all existing test-envoy behaviors still work

---

## Key Design Decisions

- Use `tokio` async runtime (matches codebase convention)
- Use `scc::HashMap` for concurrent actor maps (per CLAUDE.md, never `Mutex<HashMap>`)
- Use `tokio::sync::oneshot` for KV request/response
- Use `tokio::sync::mpsc` for inter-task communication
- Use `tracing` for structured logging (not println/eprintln)
- Callbacks via `Arc<dyn Fn(...) + Send + Sync>` or async trait objects
- Error handling via `anyhow::Result`
- Protocol encoding/decoding reuses existing `rivet-envoy-protocol` crate
