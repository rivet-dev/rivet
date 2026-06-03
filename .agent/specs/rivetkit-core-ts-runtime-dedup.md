# rivetkit-core / TS native runtime — duplication fixes

Consolidate runtime logic currently duplicated between `rivetkit-core` (Rust) and `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` (TS native runtime). Per `CLAUDE.md`, lifecycle, dispatch, state, queue, schedule, inspector, and metrics logic must live in core. TS owns types, Zod schema validation, workflow engine, agent-os, and client library — nothing runtime.

## Principle

For every user-configured runtime setting or boundary check:

1. **Enforcement lives in core.** A single timer, a single size check, a single cancellation token — no parallel implementation in TS.
2. **Errors cross the boundary structured.** `RivetError { group, code, message, metadata }` is the contract. Bare `anyhow!` at a boundary is a bug.
3. **TS callbacks are cooperative.** When core abandons a deadline it drops the reply receiver; the JS promise may still be running. Any work that mutates state must receive a `CancellationToken` through the TSF payload so it can short-circuit.
4. **Tracking state (sets, maps, counters) lives in core.** TS reads through accessors, never maintains parallel bookkeeping.

## Findings

Ordered by severity. Each finding gives current state, target state, and the concrete change.

### F1 — Action timeout race [critical, blocks `action-features` driver tests]

**Current**

- TS: `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:3083-3118`, action handler is wrapped in `withTimeout(..., options.actionTimeoutMs ?? 60_000, ...)` that rejects with a structured `{ __type: "ActorError", public: true, statusCode: 408, group: "actor", code: "action_timed_out", message: "Action timed out" }`. Caught by `buildNativeRequestErrorResponse` which produces a proper 408 HTTP response.
- Rust NAPI: `rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs:302-309` wraps the `onRequest` TSF call in `spawn_reply_with_timeout(..., config.on_request_timeout, ...)`. Duration comes from `actor_factory.rs:336-339` which falls back to `action_timeout_ms` when `on_request_timeout_ms` is not set. `with_timeout` at `napi_actor_events.rs:757-772` returns `anyhow!("callback \`{name}\` timed out after {} ms", ...)` — a bare anyhow with no `RivetError` metadata.
- Rust core: when the NAPI reply carries that anyhow, `rivetkit-core/src/registry.rs:739-743` calls `inspector_anyhow_response` which runs `RivetError::extract(&error)` and falls through to the engine default `INTERNAL_ERROR` schema (`engine/packages/error/src/lib.rs:9-10`) emitting `group=core, code=internal_error, message="An internal error occurred"`.

The Rust `tokio::time::timeout` consistently wins the race against the TS `setTimeout + Promise.race + NAPI response marshalling`, so the client always sees the wrong error for HTTP-dispatched actions that exceed `actionTimeout`.

**A parallel path exists for scheduled/alarm-driven actions.** `ActorEvent::Action` at `napi_actor_events.rs:255-295` applies `config.action_timeout` around `call_action` — for TS native actors this fires when a schedule or alarm triggers an action directly (no HTTP path). When that timer fires, the same anyhow → `core/internal_error` leak happens. No TS-side `withTimeout` here; the handler runs bare.

**Target**

- TS no longer enforces the timeout. `maybeHandleNativeActionRequest` drops the `withTimeout` wrapper and the `actionTimeoutMs` option.
- Core owns the deadline via `on_request_timeout` (HTTP path) and `action_timeout` (scheduled path), both derived from `action_timeout_ms` in `from_js_config`.
- Core's `with_timeout` helper emits a structured `RivetError("actor", "action_timed_out", "Action timed out")` (plus HTTP status metadata) instead of bare anyhow. Add a timed-spawn variant that takes `(group, code, message)`.
- TS action handlers receive a `CancellationToken` on the TSF payload so long-running JS work can short-circuit when core has already given up.

**Concrete change**

1. New helper in `napi_actor_events.rs`:
   ```rust
   async fn with_structured_timeout<T, F>(
       group: &'static str,
       code: &'static str,
       message: &'static str,
       duration: Duration,
       future: F,
   ) -> Result<T>
   where F: Future<Output = Result<T>>
   ```
   Internally uses `tokio::time::timeout` and on `Elapsed` returns `Err(anyhow::Error::new(RivetError::new(group, code, message)))` so `RivetError::extract` recovers it upstream. The existing `with_timeout(callback_name, duration, future)` stays as a thin wrapper that calls `with_structured_timeout("actor", "callback_timed_out", format!("callback `{callback_name}` timed out"), ...)` for non-action lifecycle callbacks.
2. Swap `ActorEvent::HttpRequest` and `ActorEvent::Action` dispatch call sites to use it with `("actor", "action_timed_out", "Action timed out")`. The 11 other lifecycle callbacks (F6 below) get the generic `actor/callback_timed_out` path.
3. Delete the TS `withTimeout` in `maybeHandleNativeActionRequest` and the `actionTimeoutMs` parameter. The surrounding `try/catch` stays for schema / serialization failures.
4. **Cancellation token bridge, primitive-only.** Follow the pattern already used by `enqueue_and_wait` (per CLAUDE.md "use primitives or JS-side polling instead of trying to pass a `#[napi]` class instance through an object field"). Concretely:
   - Rust maintains an `scc::HashMap<u64, CancellationToken>` keyed by a monotonic token ID.
   - `ActionPayload` / `HttpRequestPayload` get a new `cancel_token_id: Option<u64>` plain-number field.
   - TS side exposes a JS-level primitive on the action `ctx` (e.g. `ctx.abortSignal()`) that polls via a NAPI function call `rivetkit_napi.pollCancelToken(tokenId)` or subscribes through a sync-only `ThreadsafeFunction` notification. The exact bridge shape is a F1 sub-decision; constraint is **no `#[napi]` class instance in the payload**.
   - When Rust's `with_structured_timeout` fires, it cancels the matching token and drops the payload's map entry.

### F2 — Incoming/outgoing message size checks [high]

**Current**

- TS: `registry/native.ts:3018-3032` and `3154-3168` check `maxIncomingMessageSize` / `maxOutgoingMessageSize` for HTTP action bodies, emitting `message/incoming_too_long` and `message/outgoing_too_long`.
- Rust core: `rivetkit-core/src/registry.rs:1436-1454` enforces the same limits on WebSocket events, closing with code 1011 and reason `message.outgoing_too_long`.

Two independent implementations. HTTP-path limits live in TS (after body is already fully read), WS-path limits live in core. Config names match by convention; any drift silently breaks parity.

**Target**

Core enforces both limits at the request boundary for HTTP and WS alike. TS stops reading `maxIncomingMessageSize` / `maxOutgoingMessageSize` for size enforcement; if it needs them at all it's for client-side hints, not server-side rejection.

**Concrete change**

1. Add size enforcement to `handle_fetch` in `rivetkit-core/src/registry.rs` before spawning the HTTP dispatch. Reject oversize request bodies with `message/incoming_too_long`, emit a proper `HttpResponse` with status 400.
2. After the reply returns, check response body length; if it exceeds `max_outgoing_message_size`, replace with `message/outgoing_too_long`.
3. Delete the size checks in `registry/native.ts:3015-3032` and `3154-3168`. Drop `maxIncomingMessageSize` / `maxOutgoingMessageSize` from the `maybeHandleNativeActionRequest` options object.
4. Core error helper emits the `message/*` group so wire format is identical to today.

### F3 — Queue `waitForNames` AbortSignal polling [medium]

**Current**

- TS: `registry/native.ts:1500-1557` runs a 100ms timeout-slicing poll loop around native `queue.waitForNames(...)` to service `AbortSignal`. Rust queue wait doesn't accept a cancel token, so TS fakes cancellation with short slices.
- CLAUDE.md explicitly notes this pattern is "safe for receive-style" — i.e. a workaround, not the target design.

**Target**

Rust `waitForNames` accepts a `CancellationToken` like `enqueue_and_wait` already does. TS passes the signal-backed token and does a single await.

**Concrete change**

1. Add an optional `cancel: Option<CancellationToken>` parameter to `Queue::wait_for_names` in core. When set, `tokio::select!` between the wait and `cancel.cancelled()`.
2. Plumb it through the NAPI layer: `actor_factory.rs` queue message adapter accepts a cancellation-token handle (same primitive-bridge pattern as F1's action cancellation).
3. In TS `registry/native.ts:1500-1557`, replace the polling loop with a single call that forwards the `AbortSignal` as a token. The "no signal" fast path stays as-is.

### F4 — Hibernatable connection removal tracking [medium]

**Current**

- TS: `registry/native.ts` (around `NativeConnAdapter`, ~1042-1161 and `serializeForTick` around 2518-2546) maintains a Proxy-based state mutation tracker, plus a Set `removedHibernatableConnIds` (fields around `native.ts:150,169,198,2531,2536`), then serializes both during each save tick.
- Core has a full connection manager at `rivetkit-core/src/actor/connection.rs:402-649` with `insert_existing`, `remove_existing`, `iter`, `active_count`, `queue_hibernation_update(conn_id)`, `queue_hibernation_removal(conn_id)`, `take_pending_hibernation_changes`, `restore_pending_hibernation_changes`. TS is effectively maintaining a parallel set that duplicates `queue_hibernation_removal` + `take_pending_hibernation_changes`.

**Target**

Core owns the pending-hibernation-removal set via the existing `queue_hibernation_removal` / `take_pending_hibernation_changes` API. TS calls into that instead of maintaining `removedHibernatableConnIds`.

**Concrete change**

1. Expose `queue_hibernation_removal(conn_id)` and `take_pending_hibernation_changes()` through `ActorContext` NAPI surface (likely already plumbed — confirm when implementing).
2. Replace the TS-side `removedHibernatableConnIds` Set with calls into core.
3. `serializeForTick` in TS reads the list from core rather than maintaining its own.
4. State-mutation Proxy on `conn.state` stays in TS — that's a JS idiom — but it pipes into `ctx.requestSave(...)` only; no parallel tracking.

### F5 — Error reclassification at the boundary [low]

**Current**

- TS: `common/utils.ts:201-298` `deconstructError` classifies errors as public/internal and assigns `group/code/statusCode/message`.
- Rust: `rivet_error::RivetError::extract(&anyhow::Error)` does the equivalent on the Rust side.

Different sources (JS throws vs Rust anyhow) so they don't race. But when a Rust-structured error crosses into TS and re-enters `deconstructError` (e.g. via the client path), TS may reclassify it. In practice this is how some `core/internal_error` responses get further muddled.

**Target**

Once an error has `{ group, code, message }`, treat it as canonical. TS `deconstructError` short-circuits for inputs that carry an explicit `RivetError` marker — not via duck-typing.

**Concrete change**

1. Add a fast path at the top of `deconstructError`: if `error instanceof RivetError` OR `error.__type === "RivetError"` (the existing tag at `src/actor/errors.ts:165`), pass through with its own `{ group, code, message, statusCode, public, metadata }`.
2. Do NOT duck-type on property presence (`"group" in error && "code" in error`). A user throwing a plain object with matching property names would accidentally skip classification.
3. Document in `deconstructError` that its job is classifying *unstructured* errors. Structured errors (from core, from user throws that pre-built `RivetError`) do not go through the classifier.

### F6 — Lifecycle callback timeout error shape + dead-code pruning [high]

**Current**

11 lifecycle callbacks in `rivetkit-napi/src/napi_actor_events.rs` use `with_timeout(callback_name, duration, future)` which returns bare `anyhow!("callback \`{name}\` timed out after {} ms")` on elapse. `RivetError::extract` can't recover structured metadata from a bare anyhow, so every timeout decays to `core/internal_error`:

- `create_state` (line 136)
- `on_create` (line 145)
- `create_vars` (line 162)
- `on_migrate` (line 172)
- `on_wake` (line 182)
- `on_before_actor_start` (line 191)
- `create_conn_state` (line 351)
- `on_before_connect` (line 337)
- `on_connect` (line 367)
- `on_sleep` (line 503)
- `on_destroy` (line 531)

None of these race against a TS-side timer — TS handlers are raw — but when they fire, the client sees a generic internal error with no clue which callback stalled.

Dead code (no useful enforcement):
- `workflow_history_timeout` (`actor_factory.rs:340-345`, fires around `napi_actor_events.rs:422-428`): wraps the workflow-history read-only getter. Getters shouldn't have lifecycle timeouts.
- `workflow_replay_timeout` (`actor_factory.rs:346-351`, fires around `napi_actor_events.rs:437-443`): same story.
- `run_stop_timeout` (`actor_factory.rs:352`): defined in `AdapterConfig` but never applied anywhere.

**Target**

- All 11 lifecycle timeouts emit a structured `actor/callback_timed_out` error via F1's `with_structured_timeout`, carrying `{ callback_name, duration_ms }` as metadata.
- Drop the 3 dead-code timeout fields from `AdapterConfig` and their call sites.

**Concrete change**

1. Once F1's `with_structured_timeout` exists, replace all 11 call sites to use it with `("actor", "callback_timed_out", format!("callback `{callback_name}` timed out"))` and metadata `{ callback_name, duration_ms }`.
2. Generate a new `RivetError` JSON artifact for `actor/callback_timed_out` under `rivetkit-rust/engine/artifacts/errors/` (per CLAUDE.md rule on committed error artifacts).
3. Delete `workflow_history_timeout_ms`, `workflow_replay_timeout_ms`, `run_stop_timeout_ms` from `JsActorConfig` / `AdapterConfig` and their `from_js_config` entries.
4. Delete the `TypeScript index.d.ts` fields for the three dead timeouts after the NAPI rebuild regenerates the typings.

### F7 — Inspector token authentication [medium]

**Current**

Two parallel token paths on the TS side, none in core:

- TS env-based: `registry/native.ts:3497-3549` checks `RIVET_INSPECTOR_TOKEN` env var via bearer header.
- TS actor-local: `inspector/actor-inspector.ts:158-183` stores a per-actor token in KV (`loadToken`, `generateToken`, `verifyToken`).
- Rust: `rivetkit-core/src/inspector/protocol.rs` does no token validation; treats the caller as trusted.

Two sources of truth for "is this inspector request authorized," and the core inspector protocol layer is unaware of either.

**Target**

Single `InspectorAuth` module in core that owns both the env-token path and the per-actor KV-token path. TS HTTP route handlers call into core for validation; they don't implement it.

**Concrete change**

1. Add `InspectorAuth` to `rivetkit-core/src/inspector/` with `verify(bearer_token, actor_id) -> Result<()>` that checks env config first, then falls back to per-actor KV.
2. Expose through NAPI as `ctx.verifyInspectorAuth(bearerToken)`.
3. Replace the TS env check at `native.ts:3497-3549` and the actor-inspector methods at `inspector/actor-inspector.ts:158-183` with the NAPI call.
4. HTTP routing in `native.ts` stays in TS (Hono is TS-only), but only the routing — the auth decision moves.

### F8 — Inspector wire-protocol version negotiation [medium]

**Current**

Both TS and Rust implement v1↔v4 version conversion, downgrade stripping, and `inspector.*_dropped` error emission:

- TS: `common/inspector-versioned.ts` defines `TO_SERVER_VERSIONED` / `TO_CLIENT_VERSIONED` with v1→v2→v3→v4 converters and downgrade logic (e.g., v2→v1 strips queue/workflow fields).
- Rust: `rivetkit-core/src/inspector/protocol.rs:214-358` with `decode_v1_message` … `decode_v4_message`, `CURRENT_VERSION=4`, `SUPPORTED_VERSIONS=[1,2,3,4]`.

Per CLAUDE.md: "Inspector WebSocket transport should keep the wire format at v4 for outbound frames, accept v1-v4 inbound request frames, and fan out live updates through `InspectorSignal` subscriptions" — core is the canonical owner.

**Target**

Core owns v1↔v4 conversion exclusively. TS's role is transport + Hono routing; conversion happens across the NAPI boundary.

**Concrete change**

1. Expose `ctx.decodeInspectorRequest(bytes, advertisedVersion)` and `ctx.encodeInspectorResponse(value, targetVersion)` NAPI wrappers that call core's `protocol.rs` routines.
2. Delete `common/inspector-versioned.ts` converters and point every caller at the NAPI wrappers.
3. Keep the CBOR/JSON boundary encoders in TS (HTTP inspector emits JSON; WS inspector emits BARE) — those are transport concerns, not version negotiation.

### F9 — Queue size inspector snapshot [low, but there's a concrete bug]

**Current**

- TS: `inspector/actor-inspector.ts:144,154,186-191` caches `#lastQueueSize`, updated via `updateQueueSize(size)` by the runtime.
- TS HTTP endpoint at `registry/native.ts:3704-3714` returns hardcoded `size: 0` — ignores the cache entirely. This is a bug shipping in the current build.
- Rust: `rivetkit-core/src/inspector/mod.rs:154-158` has `record_queue_updated(queue_size: u32)` with an atomic, and `snapshot()` reads the live value.

TS's cache is already stale relative to core; the HTTP endpoint is even more broken.

**Target**

TS `ActorInspector` stops caching queue size and HTTP endpoint stops hardcoding it. Both read live from core's `Inspector::snapshot()` via NAPI.

**Concrete change**

1. Expose `ctx.inspectorSnapshot()` (or reuse an existing snapshot accessor) through NAPI.
2. Replace `#lastQueueSize` / `updateQueueSize` / `getQueueSize` with direct snapshot reads.
3. Fix the hardcoded `size: 0` at `native.ts:3704-3714` to read from the snapshot.

### F10 — Connection lifecycle failure coordination atomicity [medium]

**Current**

TS `onDisconnect` handler at `registry/native.ts:4294-4306` manually removes the conn from `actorState.connStates` Map and queues a hibernatable removal ID — without reentrant safety. Two racing disconnects (shutdown-triggered + client-triggered) could double-remove or skip cleanup.

Core has the infrastructure (`queue_hibernation_removal` + `take_pending_hibernation_changes` are atomic-compare-exchange backed per `connection.rs:402-649`), but the TS layer does the coordination by hand.

**Target**

All disconnect-triggered cleanup happens in core. TS's `onDisconnect` callback is invoked by core *after* the atomic state change, not before.

**Concrete change**

1. Move the `actorState.connStates` removal and hibernation queue into core's disconnect path.
2. Have core invoke the TS `onDisconnect` callback only after its own bookkeeping is complete and stable.
3. Delete the manual cleanup at `registry/native.ts:4294-4306`. TS handler body becomes pure user-code dispatch.

Related: F4 covers the `removedHibernatableConnIds` Set specifically; F10 covers the broader lifecycle-coordination pattern. Land F4 first because it's narrower; F10 builds on F4's NAPI plumbing.

## Dependencies between findings

- **F1** introduces the `with_structured_timeout` helper and the cancel-token primitive bridge. Both are reused by F3 and F6.
- **F2** also touches the HTTP request boundary in `registry.rs`. Land F2 *before* F1 so F1's timeout-helper swap doesn't collide with F2's boundary rewrite in the same file.
- **F3** reuses F1's cancel-token primitive bridge — depends on F1.
- **F4** reuses core's existing `queue_hibernation_removal` / `take_pending_hibernation_changes` APIs (already present per `connection.rs:402-649`). No cross-dependency beyond NAPI surface work that F1/F3 will have already added.
- **F5** is pure TS cleanup. Land after F1/F6 reduce the number of places errors get reclassified.
- **F6** reuses F1's `with_structured_timeout` helper. Easiest after F1 lands.
- **F7** adds a new core module (`InspectorAuth`). Independent of F1-F6.
- **F8** deletes TS `inspector-versioned.ts` converters. Independent, but easier after F7 lands because both touch inspector NAPI surface.
- **F9** is a small snapshot-reader fix. Depends on NAPI exposure of `inspectorSnapshot`; fold into F8's NAPI pass.
- **F10** builds on F4's NAPI plumbing. Land after F4.

## Migration order

1. **F2 — size limits to core.** Mechanical boundary change. Testable with `raw-http` and `actor-queue` driver tests. Gates F1's registry.rs edits.
2. **F1 — action timeout dedup + structured timeout helper + cancel-token primitive bridge.** Unblocks `action-features` driver tests. Introduces `with_structured_timeout` and the cancel-token plumbing everything else reuses.
3. **F6 — lifecycle timeout error shape + dead-code pruning.** Mechanical once F1's helper exists. Drops 3 unused `AdapterConfig` fields.
4. **F3 — `waitForNames` cancel token.** Reuses F1's cancel-token bridge.
5. **F4 — hibernatable tracking to core.** Wire TS's `removedHibernatableConnIds` removal through `queue_hibernation_removal`.
6. **F10 — connection lifecycle failure atomicity.** Moves `onDisconnect`'s manual cleanup into core. Builds on F4.
7. **F7 — inspector auth unification.** Core `InspectorAuth` module.
8. **F8 — inspector wire-protocol version negotiation.** Delete `common/inspector-versioned.ts`.
9. **F9 — queue size snapshot fix.** Fold into F8's NAPI-surface pass.
10. **F5 — `deconstructError` `instanceof` fast path.** Pure TS cleanup. Last.

## Out of scope

- Moving workflow engine, agent-os, or Zod validation out of TS. Per CLAUDE.md those belong in TS.
- Changing the wire format. Every fix here preserves existing group/code/message strings.
- Performance work on TSF marshalling. The race in F1 is incidental to the timing; fixing the deadline ownership is enough.

## Validation

Each fix lands with:

- The driver test(s) it unblocks or affects explicitly named in the PR description.
- A native-level unit test in `rivetkit-core` or `rivetkit-napi` only if the TS driver test cannot exercise the new Rust code path (per CLAUDE.md anchor rule).
- Rerun the full driver suite via the `driver-test-runner` skill after each fix.

## Open questions

- Should `on_request_timeout` remain as a separate `AdapterConfig` field after F1 collapses the race, or should it be unified with `action_timeout`? Leaning toward keeping them distinct — `onRequest` for raw HTTP may legitimately take longer than an action — but the fallback chain `on_request_timeout_ms.or(action_timeout_ms)` should probably stay with a generous additive buffer (e.g. `+ 1s`) rather than matching it exactly.
- Does `action_timeout` on scheduled-action paths need a TS-visible "abandoned" signal, or is the orphaned-JS-promise side effect acceptable for alarms? Probably the latter for MVP; revisit when we have a concrete complaint.
