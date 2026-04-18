# Driver Test Fix Audit

Audited: 2026-04-18
Updated: 2026-04-18
Scope: All uncommitted changes on feat/sqlite-vfs-v2 used to pass the driver test suite
Method: Compared against original TS implementation (ref 58b217920) across 5 subsystems

## Verdict: No test-overfitting found. 3 parity gaps fixed, 1 architectural debt item remains intentionally unchanged.

---

## Issues Found

### BARE-only encoding on actor-connect WebSocket (fixed)

The Rust `handle_actor_connect_websocket` in `registry.rs` rejects any encoding that isn't `"bare"` (line 1242). The original TS implementation accepted `json`, `cbor`, and `bare` via `Sec-WebSocket-Protocol`, defaulting to `json`. Tests only exercise BARE, so this passed. Production JS clients that default to JSON encoding will fail to connect.

**Severity**: High (production-breaking for non-BARE clients)
**Type**: Incomplete port, not overfit

### Error metadata dropped on WebSocket error responses (fixed)

`action_dispatch_error_response` in `registry.rs` hardcodes `metadata: None` (line 3247). `ActionDispatchError` in `actor/action.rs` lacks a `metadata` field entirely, so it's structurally impossible to propagate. The TS implementation forwarded CBOR-encoded metadata bytes from `deconstructError`. Structured error metadata from user actors is silently lost on WebSocket error frames.

**Severity**: Medium (error context lost, but group/code preserved)
**Type**: Incomplete port

### Workflow inspector stubs (fixed)

`NativeWorkflowRuntimeAdapter` has two stubs:
- `isRunHandlerActive()` always returns `false` — disables the safety guard preventing concurrent replay + live execution
- `restartRunHandler()` is a no-op — inspector replay computes but never takes effect

Normal workflow execution (step/sleep/loop/message) works. Inspector-driven workflow replay is broken on the native path.

**Severity**: Low (inspector-only, not user-facing)
**Type**: Known incomplete feature

### Action timeout/size enforcement in wrong layer (left as-is)

TS `native.ts` adds `withTimeout()` and byte-length checks for actions. Rivetkit-core also has these in `actor/action.rs` and `registry.rs`. However, the native HTTP action path bypasses rivetkit-core's event dispatch (`handle_fetch` instead of `actor/event.rs`), so TS enforcement is the pragmatic correct location. Not duplicated at runtime for the same request, but the code exists in both layers.

**Severity**: Low (correct behavior, architectural debt)
**Type**: Wrong layer, but justified by current routing

---

## Confirmed Correct Fixes

- **Stateless actor state gating** — Config-driven, matches original TS behavior
- **KV adapter key namespacing** — Uses standard `KEYS.KV` prefix, matches `ActorKv` contract
- **Error sanitization** — Uses `INTERNAL_ERROR_DESCRIPTION` constant and `toRivetError()`, maps by group/code pairs
- **Raw HTTP void return handling** — Throws instead of silently converting to 204, matches TS contract
- **Lifecycle hooks conn params** — Fixed in client-side `actor-handle.ts`, correct layer
- **Connection state bridging** — `createConnState`/`connState` properly wired, fires even without `onConnect`
- **Sleep/lifecycle/destroy timing** — `begin_keep_awake`/`end_keep_awake` tracked through `ActionInvoker.dispatch()`, no timing hacks
- **BARE codec** — Correct LEB128 varint, canonical validation, `finish()` rejects trailing bytes
- **Actor key deserialization** — Faithful port of TS `deserializeActorKey` with same escape sequences
- **Queue canPublish** — Real `NativeConnHandle` via `ctx.connectConn()` with proper cleanup

## Reviewed and Dismissed

- **`tokio::spawn` for WS action dispatch** — Not an issue. Spawned tasks call `invoker.dispatch()` which calls `begin_keep_awake()`/`end_keep_awake()`, so sleep is properly blocked. The CLAUDE.md `JoinSet` convention is about `envoy-client` HTTP fetch, not rivetkit-core action dispatch.
- **`find()` vs `strip_prefix()` in error parsing** — Intentional. Node.js can prepend context to NAPI error messages, so `find()` correctly locates the bridge prefix mid-string. Not a bug, it's a fix for errors being missed.
- **Hardcoded empty-vec in `connect_conn`** — Correct value for internally-created connections (action/queue HTTP contexts) which have no response body to send.

## Minor Notes

- `rearm_sleep_after_http_request` helper duplicated in `event.rs` and `registry.rs` — intentional per CLAUDE.md (two dispatch paths), but could be extracted
- `_is_restoring_hibernatable` parameter accepted but unused in `handle_actor_connect_websocket`
- Unused `Serialize`/`Deserialize` derives on protocol structs (hand-rolled BARE used instead)
- No tests for `Request` propagation through connection lifecycle callbacks
- No tests for message size limit enforcement at runtime
