# Driver Test Uncommitted Changes Review

Reviewed: 2026-04-18
Branch: feat/sqlite-vfs-v2
State: 20 files, +1127/-293, all unstaged

## Medium Issues

- **Unbounded `tokio::spawn` for action dispatch** — `registry.rs` `handle_actor_connect_websocket` spawns action dispatch without `JoinSet`/`AtomicUsize` tracking. Sleep checks can't read in-flight count and shutdown can't abort/join. Per CLAUDE.md, envoy-client HTTP fetch work should use `JoinSet` + `Arc<AtomicUsize>`.

- **Duplicated action timeout in TS** — `native.ts` adds `withTimeout` wrapper for action execution, but `rivetkit-core` already implements action timeout in `actor/action.rs`. Double enforcement risks mismatched defaults and confusing error messages. Should be consolidated into rivetkit-core per layer constraints.

- **Duplicated message size enforcement in TS** — `native.ts` enforces `maxIncomingMessageSize`/`maxOutgoingMessageSize`, but `rivetkit-core` already has this in `registry.rs`. Same double-enforcement concern.

## Low Issues

- **`find()` vs `strip_prefix()` in error parsing** — `actor_factory.rs` changed `parse_bridge_rivet_error` from `strip_prefix()` to `find()`. More permissive, could match prefix mid-string in nested error messages.

- **Hardcoded empty-vec in `connect_conn`** — `actor_context.rs` passes `async { Ok(Vec::new()) }` as third arg to `connect_conn_with_request`. Embeds empty-response policy in NAPI layer rather than letting core decide.

- **Unused serde derives on protocol structs** — `registry.rs` protocol types (`ActorConnectInit`, `ActorConnectActionResponse`, etc.) derive `Serialize`/`Deserialize` but encoding uses hand-rolled BARE codec. Dead derives could mislead.

- **`_is_restoring_hibernatable` unused** — `registry.rs` `handle_actor_connect_websocket` accepts but ignores this param. Forward-compatible, but should eventually wire to connection restoration.

## Observations (Not Issues)

- BARE codec in registry.rs is ~230 lines of hand-rolled encoding/decoding. Works correctly with overflow checks and canonical validation, but will need extraction if other modules need BARE.
- No tests for `Request` propagation through connection lifecycle callbacks (verifying `onBeforeConnect`/`onConnect` actually receive the request).
- No tests for message size limit enforcement at runtime.
