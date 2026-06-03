# Spec: Rust Client Parity with TypeScript Client

## Goal

Bring the Rust `rivetkit-client` crate to feature parity with the TypeScript client, and add actor-to-actor communication via `c.client()` in the `rivetkit` Rust crate.

## Deviation Summary

| Feature | TypeScript | Rust | Gap |
|---------|-----------|------|-----|
| Encoding: bare | Yes (default) | No | Missing |
| Encoding: cbor | Yes | Yes (default) | OK |
| Encoding: json | Yes | Yes | OK |
| `handle.send()` (queue) | Yes | No | Missing |
| `handle.fetch()` (raw HTTP) | Yes | No | Missing |
| `handle.webSocket()` (raw WS) | Yes | No | Missing |
| `handle.reload()` (dynamic) | Yes | No | Missing |
| `handle.getGatewayUrl()` | Yes | No | Missing |
| `conn.on()` / `conn.once()` | Yes (`on`/`once`) | Yes (`on_event`) | Partial (no `once`) |
| `conn.onError` | Yes | No | Missing |
| `conn.onOpen` | Yes | No | Missing |
| `conn.onClose` | Yes | No | Missing |
| `conn.onStatusChange` | Yes | No | Missing |
| `conn.connStatus` | Yes | No | Missing |
| `conn.dispose()` | Yes | `disconnect()` | Name differs |
| `client.dispose()` | Yes | `disconnect()` | Name differs |
| `handle.action()` typed proxy | Yes (Proxy) | No | Missing |
| AbortSignal support | Yes | No | Missing |
| Token auth | Yes | Yes | OK |
| Namespace config | Yes | Yes (via endpoint) | Partial |
| Custom headers | Yes | No | Missing |
| `maxInputSize` config | Yes | No | Missing |
| `disableMetadataLookup` | Yes | No | Missing |
| `getParams` lazy resolver | Yes | No | Missing |
| `c.client()` actor-to-actor | Yes | No | Missing |
| Error metadata (CBOR) | Yes | Yes | OK |
| Auto-reconnect | Yes | Yes | OK |

## Detailed Deviations

### 1. Missing BARE encoding

The TS client defaults to BARE encoding. The Rust client only supports JSON and CBOR (`EncodingKind::Json | Cbor`). BARE is the most efficient encoding and what the actor-connect protocol uses natively.

**Fix:** Add `EncodingKind::Bare` using the same BARE codec from rivetkit-core's `registry.rs` (or extract the hand-rolled `BareCursor`/`bare_write_*` into a shared module).

### 2. Missing queue send on handle

TS exposes `handle.send(name, body, opts)` for queue operations with optional `wait` and `timeout`. Rust `ActorHandleStateless` has no queue methods.

**Fix:** Add `send()` and `send_and_wait()` on `ActorHandleStateless`. Requires HTTP POST to `/queue/{name}` endpoint with versioned request encoding.

### 3. Missing raw HTTP fetch on handle

TS exposes `handle.fetch(input, init)` for raw HTTP requests to the actor's `/request` endpoint. Rust has no equivalent.

**Fix:** Add `fetch(path, method, headers, body)` on `ActorHandleStateless` that proxies to the actor gateway.

### 4. Missing raw WebSocket on handle

TS exposes `handle.webSocket(path, protocols)` for raw (non-protocol) WebSocket connections. Rust has no equivalent.

**Fix:** Add `web_socket(path, protocols)` that returns a raw WebSocket handle without the client protocol framing.

### 5. Missing connection lifecycle callbacks

TS `ActorConn` exposes `onError`, `onOpen`, `onClose`, `onStatusChange`, and `connStatus`. Rust `ActorConnection` has none of these.

**Fix:** Add callback registration methods and a `ConnectionStatus` enum (`Idle`, `Connecting`, `Connected`, `Disconnected`). Use `tokio::sync::watch` for status changes.

### 6. Missing `once` event subscription

TS supports both `on(event, cb)` and `once(event, cb)`. Rust only has `on_event`.

**Fix:** Add `once_event()` that auto-unsubscribes after first delivery.

### 7. Missing gateway URL builder

TS exposes `handle.getGatewayUrl()` which returns the gateway URL for direct access. Useful for sharing actor endpoints.

**Fix:** Add `gateway_url()` on `ActorHandleStateless`.

### 8. Missing typed action proxy

TS uses JavaScript `Proxy` to allow `handle.actionName(args)` syntax. Rust can't do runtime proxying but can provide a macro or trait-based approach.

**Fix:** Provide a `#[derive(ActorClient)]` macro that generates typed action methods from an actor definition. Or accept that Rust uses `handle.action("name", args)` — this is idiomatic Rust.

### 9. Missing AbortSignal/cancellation support

TS supports `AbortSignal` on actions and connections. Rust has no cancellation token.

**Fix:** Accept `Option<tokio_util::sync::CancellationToken>` or `Option<tokio::sync::oneshot::Receiver<()>>` on action calls. Or rely on `tokio::select!` / dropping futures for cancellation (idiomatic Rust).

### 10. Missing client config options

TS `ClientConfigInput` has: `headers`, `maxInputSize`, `disableMetadataLookup`, `getUpgradeWebSocket`, `devtools`, `poolName`. Rust `Client::new` only takes endpoint, token, transport, encoding.

**Fix:** Add a `ClientConfig` builder struct:

```rust
pub struct ClientConfig {
    pub endpoint: String,
    pub token: Option<String>,
    pub namespace: Option<String>,
    pub pool_name: Option<String>,
    pub encoding: EncodingKind,
    pub transport: TransportKind,
    pub headers: Option<HashMap<String, String>>,
    pub max_input_size: Option<usize>,
    pub disable_metadata_lookup: bool,
}
```

### 11. Missing `c.client()` actor-to-actor communication

TS actors access `c.client()` which returns a fully-typed client configured with the actor's own endpoint and token. This enables actor-to-actor RPC, queue sends, and connections.

The Rust `Ctx<A>` has no client. rivetkit-core has a stub `client_call()` that errors with "not configured".

**Fix:** In the `rivetkit` Rust crate (not core):

```rust
impl<A: Actor> Ctx<A> {
    pub fn client(&self) -> Client {
        // Build client from actor's own endpoint + token
        // Cached after first call
    }
}
```

The client factory reads endpoint/token from the actor's `EnvoyHandle` config (same as TS reads from `RegistryConfig`). The returned `Client` is the same external client type, just pre-configured for internal use.

rivetkit-core should provide a `client_endpoint()` and `client_token()` on `ActorContext` so the typed wrapper can construct the client without reaching into envoy internals.

### 12. Naming inconsistencies

| TypeScript | Rust | Recommendation |
|-----------|------|----------------|
| `dispose()` | `disconnect()` | Keep `disconnect()` (more descriptive in Rust) |
| `ActorConn` | `ActorConnection` | Keep `ActorConnection` (Rust convention) |
| `on(event, cb)` | `on_event(event, cb)` | Keep `on_event` (avoids keyword clash) |

## New Protocol Crates

### Problem

The client protocol BARE schemas currently live only in TypeScript (`rivetkit-typescript/packages/rivetkit/src/common/bare/client-protocol/`), and Rust duplicates them in two places: the server-side codec in `registry.rs` plus the client-side codec in `rivetkit-rust/packages/client/src/protocol/codec.rs`. Together these are ~380 lines of hand-rolled `BareCursor`/`bare_write_*`/encode/decode code that should come from generated protocol types instead. The inspector protocol schemas are also TS-only (`schemas/actor-inspector/v1-v4.bare`).

### Solution

Create proper Rust protocol crates following the engine pattern (`runner-protocol`, `envoy-protocol`):

**`rivetkit-rust/packages/client-protocol/`** — Client-actor wire protocol
```
rivetkit-rust/packages/client-protocol/
├── Cargo.toml              # vbare-compiler in [build-dependencies]
├── build.rs                # vbare_compiler::process_schemas_with_config()
├── schemas/
│   ├── v1.bare             # Moved from TS (Init with connectionToken)
│   ├── v2.bare             # Init without connectionToken
│   └── v3.bare             # + HttpQueueSend request/response
├── src/
│   ├── lib.rs              # pub use generated::v3::*; pub const PROTOCOL_VERSION: u16 = 3;
│   ├── generated/          # Auto-generated by build.rs
│   └── versioned.rs        # Version migration (v1→v2→v3 converters)
```

Schemas cover:
- WebSocket: `ActionRequest`, `SubscriptionRequest`, `Init`, `Error`, `ActionResponse`, `Event`
- HTTP: `HttpActionRequest`, `HttpActionResponse`, `HttpQueueSendRequest`, `HttpQueueSendResponse`, `HttpResponseError`

**`rivetkit-rust/packages/inspector-protocol/`** — Inspector debug protocol
```
rivetkit-rust/packages/inspector-protocol/
├── Cargo.toml
├── build.rs
├── schemas/
│   ├── v1.bare             # Moved from TS schemas/actor-inspector/
│   ├── v2.bare
│   ├── v3.bare
│   └── v4.bare
├── src/
│   ├── lib.rs              # pub use generated::v4::*;
│   ├── generated/
│   └── versioned.rs
```

### What Changes

- **rivetkit-core `registry.rs`**: Delete hand-rolled `BareCursor`/`bare_write_*` (~230 lines). Import from `rivetkit-client-protocol` instead. Server-side encoding/decoding uses the generated types with `serde_bare`.
- **rivetkit-core `inspector/protocol.rs`**: Replace manual JSON-based protocol types with generated BARE types from `rivetkit-inspector-protocol`.
- **rivetkit-client `src/protocol/codec.rs`**: Delete hand-rolled `BareCursor` (~123 lines). Import from `rivetkit-client-protocol` instead. Client-side encoding/decoding uses the generated types.
- **rivetkit-client**: Import from `rivetkit-client-protocol` for BARE encoding support. Default to `EncodingKind::Bare`.
- **TypeScript**: `build.rs` generates TS codecs from the same `.bare` files (same pattern as runner-protocol). The vendored `src/common/bare/client-protocol/` and `src/common/bare/inspector/` files get replaced by the generated output.

### Schema Inventory

| Schema Set | Current Location | New Crate | Versions |
|-----------|-----------------|-----------|----------|
| client-protocol | TS `src/common/bare/client-protocol/v1-v3.ts` (generated, vendored) | `rivetkit-rust/packages/client-protocol/` | v1-v3 |
| inspector | TS `schemas/actor-inspector/v1-v4.bare` | `rivetkit-rust/packages/inspector-protocol/` | v1-v4 |
| actor-persist | TS `src/common/bare/actor-persist/v1-v4.ts` (generated, vendored) | Stay in TS (TS-only persistence) | v1-v4 |
| transport/workflow | TS `src/common/bare/transport/v1.ts` | Stay in TS (workflow engine is TS-only) | v1 |
| traces | TS `packages/traces/schemas/v1.bare` | Stay in TS (OTLP export is TS-only) | v1 |

Only client-protocol and inspector-protocol move to Rust. The rest are TS-only consumers.

## Migration Steps

1. **Create `rivetkit-client-protocol` crate** — Move `.bare` schemas, set up `build.rs` with `vbare_compiler`, generate Rust + TS codecs, add `versioned.rs`
2. **Create `rivetkit-inspector-protocol` crate** — Same pattern for inspector schemas
3. **Delete hand-rolled BARE in `registry.rs` and `client/src/protocol/codec.rs`** — Replace with generated types from `rivetkit-client-protocol`
4. **Replace vendored TS codecs** — Point TS imports at build-generated output from the new crates
5. **Add `ClientConfig` builder** — Replace positional constructor args with config struct
6. **Add BARE encoding to client** — Import from `rivetkit-client-protocol`, add `EncodingKind::Bare`, make it default
7. **Add queue send** — `send()` and `send_and_wait()` on `ActorHandleStateless`
8. **Add raw HTTP/WS** — `fetch()` and `web_socket()` on `ActorHandleStateless`
9. **Add connection lifecycle** — Status enum, callbacks, `once_event()`
10. **Add gateway URL** — `gateway_url()` on `ActorHandleStateless`
11. **Add missing config options** — headers, max_input_size, disable_metadata_lookup
12. **Add `c.client()` to rivetkit Rust** — Client factory on `Ctx<A>` reading from envoy config
13. **Wire rivetkit-core** — Add `client_endpoint()`/`client_token()` accessors on `ActorContext`
14. **Cancellation** — Document idiomatic Rust cancellation via `tokio::select!` / drop

## Non-Goals

- **Typed action proxy via macros** — Defer. `handle.action("name", args)` is fine for now. A `#[derive(ActorClient)]` macro is a separate effort.
- **SSE transport** — Already declared in Rust (`TransportKind::Sse`) but unimplemented in both TS and Rust. Not a parity gap.
- **Devtools** — TS-only feature, not applicable to Rust client.
- **`getUpgradeWebSocket`** — Cloudflare Workers concern, not applicable to Rust.
- **actor-persist, transport/workflow, traces schemas** — Stay in TS. No Rust consumer.
