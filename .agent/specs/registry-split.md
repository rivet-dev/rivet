# Registry Split Plan

## Goal
- Split `rivetkit-core/src/registry.rs` into a `registry/` module tree without behavior changes.
- Keep each resulting `rivetkit-core/src/registry/*.rs` file under 1000 lines.

## Proposed Modules
- `registry/mod.rs` (~900 lines): public `CoreRegistry`, dispatcher state structs, actor start/stop lifecycle, context construction, and shared module wiring.
- `registry/envoy_callbacks.rs` (~260 lines): `EnvoyCallbacks` implementation, serve env config, actor-key/preload conversion, and preload tests.
- `registry/http.rs` (~850 lines): HTTP fetch entrypoint, framework `/action/*` and `/queue/*` handlers, metrics route, request/response encoding helpers, auth/header helpers, and HTTP unit tests.
- `registry/inspector.rs` (~700 lines): inspector HTTP routes, inspector response builders, database/query helpers, and shared JSON/CBOR helpers.
- `registry/inspector_ws.rs` (~450 lines): inspector websocket setup, message processing, and push updates.
- `registry/websocket.rs` (~620 lines): websocket entrypoint, actor-connect websocket handling, raw websocket handling, websocket route/header helpers, and hibernatable message ack helper.
- `registry/actor_connect.rs` (~410 lines): actor-connect wire DTOs, JSON/CBOR/BARE encode/decode helpers, bigint compatibility helpers, manual CBOR writers, and close/error helper builders.
- `registry/dispatch.rs` (~160 lines): shared actor dispatch helpers and timeout wrappers used by HTTP, inspector, and websocket routes.

## Visibility Rules
- Parent registry state stays in `mod.rs`; child modules can access parent-private fields directly.
- Helpers used across sibling modules become `pub(super)`.
- No public API changes: exported surface remains `CoreRegistry` and `ServeConfig` from `crate::registry`.

## Verification
- Run `cargo test -p rivetkit-core` after the split.
