# Spec: KV and Native Bridge Remediation Plan

## Status

Draft

## Scope

This plan covers the following performance issues:

1. Cache `get_for_kv` actor metadata lookups
2. Move WebSocket serialization out from under `ws_tx` lock
3. Parallelize sequential start-command enrichment in `pegboard-envoy`
4. Remove unnecessary per-KV-operation `tokio::spawn` in `rivetkit-native`, or justify a shared-runtime alternative
5. Replace base64 and JSON binary transport in the N-API bridge with typed `Buffer`-based events
6. Remove avoidable JS-side KV buffer copies and document unavoidable Rust-side ownership copies
7. Reduce costly KV key clones in pegboard KV put paths

## Goals

- Reduce repeated database work on hot KV paths.
- Shorten lock hold time on outbound envoy sends.
- Increase concurrency when enriching actor start commands.
- Remove unnecessary runtime scheduling overhead in the native KV bridge.
- Eliminate repeated base64 encode and decode work for HTTP and WebSocket payloads across the N-API boundary.
- Remove avoidable JS buffer copies without weakening ownership or safety guarantees.
- Reduce repeated key cloning during KV writes.

## Non-Goals

- No protocol version bump unless a wire format actually changes.
- No changes to published `*.bare` runner protocol schemas.
- No broad refactor of unrelated envoy-client or guard-core behavior.

## Constraints From The Current Code

- `engine/packages/pegboard/src/ops/actor/get_for_kv.rs` already has a TODO for caching.
- `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs` performs `get_for_kv` on every KV request before namespace validation.
- `engine/sdks/rust/envoy-client/src/connection.rs` currently serializes while holding `shared.ws_tx.lock().await`, which also preserves outbound send ordering across concurrent callers.
- `engine/packages/pegboard-envoy/src/tunnel_to_ws_task.rs` mutates `command_wrappers` in place after sequential async lookups.
- `engine/packages/pegboard-envoy/src/conn.rs` also sequentially fetches `preloaded_kv` for missed `CommandStartActor` entries during connection init.
- `engine/packages/pegboard-runner/src/tunnel_to_ws_task.rs` has two separate sequential start-command enrichment loops, one for mk2 and one for mk1.
- `rivetkit-typescript/packages/rivetkit-native/src/lib.rs` creates a dedicated Tokio runtime with `Runtime::new()`.
- `rivetkit-typescript/packages/rivetkit-native/src/bridge_actor.rs` uses one `ThreadsafeFunction<serde_json::Value>` for all callback events and base64-encodes binary payloads.
- `rivetkit-typescript/packages/rivetkit-native/wrapper.js` assumes those JSON envelopes and performs the matching base64 decode and JS buffer conversions.
- `rivetkit-typescript/packages/rivetkit-native/src/envoy_handle.rs` still resolves callback responses through `respond_callback(response_id, serde_json::Value)`, so the response path is just as JSON-shaped as the event path.
- `engine/packages/pegboard/src/keys/actor_kv.rs` currently models `KeyWrapper` and `EntryValueChunkKey` as owning key bytes, so the current clone sites are structural rather than accidental.

## Delivery Strategy

Implement this in four phases so the highest-value, lowest-risk changes land first and the larger bridge refactor lands behind passing coverage.

## Phase 1: Cheap Hot-Path Wins

### 1. Cache `get_for_kv` actor metadata lookups

Files:

- `engine/packages/pegboard-envoy/src/conn.rs`
- `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs`
- `engine/packages/pegboard/src/ops/actor/get_for_kv.rs`

Plan:

- Add a connection-scoped `scc::HashMap<Id, KvActorInfo>` cache on `Arc<Conn>` in pegboard-envoy rather than inside the operation itself.
- Store `namespace_id` and `name` by `actor_id`.
- On KV request handling, check the cache first. Only call `pegboard::ops::actor::get_for_kv` on miss.
- Preserve the existing namespace guard by validating the cached `namespace_id` against `conn.namespace_id`.
- Make the cache reachable from both `ws_to_tunnel_task.rs` and `tunnel_to_ws_task.rs` so lifecycle handling can invalidate the same map the read path uses.
- Rely on connection teardown for the primary eviction boundary, then add explicit actor stop and destroy invalidation only where the signal path is real on that connection object.
- Do not negative-cache missing actors until destroy semantics are verified, because a transient miss is materially different from a permanently destroyed actor.

Why this shape:

- A module-global cache would need more invalidation machinery and cross-connection namespace reasoning.
- The TODO in `get_for_kv.rs` suggests caching, but the call site is where request locality exists and where eviction can be tied to connection lifecycle.
- The existing TODO mentions purging on runner changes. For a call-site cache that only stores `namespace_id` and `name`, runner changes are not part of the cached value and should be documented as intentionally out of scope.

Verification:

- Add a targeted test or instrumentation proving repeated KV requests for one actor trigger at most one metadata lookup per connection until eviction.
- Confirm that a stale cache entry cannot cross namespaces because namespace validation still occurs on the cached value.

### 2. Move WebSocket serialization outside the `ws_tx` lock

Files:

- `engine/sdks/rust/envoy-client/src/connection.rs`
- `engine/sdks/rust/envoy-client/src/context.rs`

Plan:

- Do not land the naive “serialize first, then take the lock” rewrite by itself, because it can reorder outbound frames across concurrent callers.
- Preserve ordering explicitly. The preferred design is a single-writer outbound queue or task that owns the actual sender, accepts already-encoded bytes, and preserves enqueue order.
- If that queue refactor is too large for this batch, keep the current ordering semantics and defer this item rather than trading correctness for lock hold time.
- Add a small benchmark note or trace timing if useful to confirm lock hold time dropped after the ordering-safe design is in place.

Follow-up decision:

- Evaluate replacing `Arc<Mutex<Option<UnboundedSender<WsTxMessage>>>>` with a lock-free read path such as `ArcSwapOption<_>` only if message ordering remains explicit somewhere else.
- Do not do that in the same patch unless the ordering story is obviously simpler than today.

Verification:

- Existing envoy-client tests should still pass.
- Add or update a focused test if one exists around disconnected `ws_tx` behavior to confirm the `None` path is unchanged.
- Add a concurrency-focused test or stress harness proving outbound frame order remains stable under concurrent callers.

## Phase 2: Concurrency Fixes In Command And Native KV Paths

### 3. Parallelize start-command enrichment in `pegboard-envoy`

Files:

- `engine/packages/pegboard-envoy/src/tunnel_to_ws_task.rs`
- `engine/packages/pegboard-envoy/src/conn.rs`
- `engine/packages/pegboard-runner/src/tunnel_to_ws_task.rs`

Plan:

- Collect owned `CommandStartActor` work items first instead of awaiting inside the mutation loop.
- For each work item, concurrently fetch hibernating request IDs and preloaded KV when needed.
- Preserve output ordering by storing results by original wrapper index, then applying them back in a second pass.
- Mirror the same treatment in all live variants:
  - `pegboard-envoy/src/tunnel_to_ws_task.rs`
  - `pegboard-envoy/src/conn.rs` for missed command replay
  - `pegboard-runner/src/tunnel_to_ws_task.rs` mk2 path
  - `pegboard-runner/src/tunnel_to_ws_task.rs` mk1 path
- Do not leave any protocol path to drift.

Preferred implementation:

- Use `futures_util::stream::iter(work_items).map(...).buffer_unordered(N)` or `FuturesUnordered` so concurrency can be bounded.
- Avoid unconstrained `join_all` if a large command batch is possible.
- Extract the owned fields needed for async work up front. Do not hold mutable borrows into `command_wrappers` across `.await`.

Verification:

- Add or update a test proving multiple `CommandStartActor` entries are enriched correctly and keep their original order.
- Confirm that a failure in one enrichment still aborts the enclosing operation exactly as before.
- Add coverage for missed command replay during connection init, not just live tunnel handling.

### 4. Remove unnecessary per-operation `tokio::spawn` in `rivetkit-native`

Files:

- `rivetkit-typescript/packages/rivetkit-native/src/lib.rs`
- `rivetkit-typescript/packages/rivetkit-native/src/envoy_handle.rs`

Plan:

- First verify runtime ownership. `start_envoy_sync_js` creates and enters a dedicated Tokio runtime at startup, but that does not prove later N-API async methods execute on that same runtime.
- Before removing any `spawn(...).await`, verify whether `#[napi] async fn` bodies run on a separate executor and whether `EnvoyHandle` methods are runtime-agnostic enough to await from there.
- If the handle methods are runtime-agnostic, replace `self.runtime.spawn(async move { ... }).await` with direct `self.handle.<op>(...).await` for `started`, all KV methods, and `start_serverless`.
- If they are not runtime-agnostic, keep the hop onto the owned runtime or explicitly consolidate the runtimes first.
- Do not remove the stored `Arc<Runtime>` from `JsEnvoyHandle` until the executor story is proven by code inspection or instrumentation.

Risk to watch:

- Do not hold N-API-managed borrowed data across `.await`. Convert arguments into owned Rust values before awaiting, as the current code already does.
- Do not assume “same process” means “same runtime”. Cross-runtime channel usage can be fine, but runtime-local APIs such as spawned tasks and timers must be validated.

Verification:

- Run the targeted native envoy tests that exercise KV and lifecycle methods.
- Confirm there is no regression in startup or shutdown behavior.
- Add one smoke test that exercises the N-API entry points from JavaScript after startup and confirms the direct-await version does not deadlock or panic under the actual addon executor.

## Phase 3: Native Bridge Binary Transport Refactor

### 5. Replace JSON plus base64 binary transport with typed `Buffer` events

Files:

- `rivetkit-typescript/packages/rivetkit-native/src/bridge_actor.rs`
- `rivetkit-typescript/packages/rivetkit-native/src/types.rs`
- `rivetkit-typescript/packages/rivetkit-native/src/lib.rs`
- `rivetkit-typescript/packages/rivetkit-native/src/envoy_handle.rs`
- `rivetkit-typescript/packages/rivetkit-native/wrapper.js`
- `rivetkit-typescript/packages/rivetkit-native/index.d.ts`

Plan:

- Replace the single `ThreadsafeFunction<serde_json::Value>` event channel with typed N-API objects for each event family that currently carries binary payloads.
- Keep a lightweight tagged event shape at the JS boundary, but move binary fields to `Buffer` instead of base64 strings.
- Start with these event objects:
  - `ActorStartEvent` with `input: Option<Buffer>`
  - `HttpRequestEvent` with `message_id: Buffer` and `body: Option<Buffer>`
  - `WebSocketOpenEvent` with `message_id: Buffer`
  - `WebSocketMessageEvent` with `message_id: Buffer`, `data: Buffer`, and `binary: bool`
  - `WebSocketCloseEvent` with `message_id: Buffer`
- Redesign the response path in parallel with the event path. `respond_callback(response_id, serde_json::Value)` cannot remain the only bridge response API if the goal is to eliminate JSON and base64 round-trips.
- Define typed callback response objects for HTTP callback results so Rust no longer decodes `body` from base64.
- Update `wrapper.js` to consume these typed objects directly and to return `Buffer` for HTTP response bodies.

Recommended boundary design:

- Use one N-API callback per event family if that keeps the Rust and JS code simple.
- If one callback must remain, use a tagged union object whose fields are already typed at the N-API layer rather than packed into `serde_json::Value`.
- Keep the public JS wrapper contract stable unless there is a deliberate breaking change. `index.d.ts` is generated from the N-API layer, so the spec must assume those signatures will move when the Rust callback surface changes.

Why this is a separate phase:

- This is the largest behavioral refactor in the set.
- It changes both sides of the native bridge and touches callback typing, `index.d.ts`, and the response path.

Verification:

- Add targeted tests for actor start input delivery, HTTP request and response bodies, WebSocket binary messages, and WebSocket text messages.
- Confirm `wrapper.js` no longer contains base64 encode and decode for these event paths after the refactor.

### 6. Remove avoidable JS-side KV buffer copies

Files:

- `rivetkit-typescript/packages/rivetkit-native/wrapper.js`
- `rivetkit-typescript/packages/rivetkit-native/src/envoy_handle.rs`

Plan:

- In `wrapper.js`, avoid copying inputs that are already `Buffer` instances by using `Buffer.isBuffer(value) ? value : Buffer.from(value)`.
- For return values, document the contract difference clearly:
  - If the public TS surface requires `Uint8Array`, `new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength)` can share memory instead of copying.
  - If consumers tolerate `Buffer`, return `Buffer` directly because it is already a `Uint8Array` subclass in Node.
- Prefer preserving the existing wrapper contract by default. If returning `Buffer` directly changes observable behavior for callers that expect a plain `Uint8Array`, document that as a compatibility decision rather than a hidden optimization.
- On the Rust side, keep the copy into owned `Vec<u8>` where requests cross async boundaries and ownership must survive after the N-API frame ends.
- Reassess after Phase 3 whether some Rust-side copies can be reduced by accepting owned `Buffer` and converting once at the edge, but do not assume zero-copy across async ownership boundaries.

Verification:

- Add focused JS tests that pass both `Buffer` and `Uint8Array` inputs and confirm semantics stay identical.
- If returning shared-memory `Uint8Array` views, add a regression test that ensures the returned bytes are correct and stable for the caller’s expected lifetime.

## Phase 4: KV Write Clone Reduction

### 7. Reduce costly key clones in pegboard KV put

Files:

- `engine/packages/pegboard/src/actor_kv/mod.rs`
- `engine/packages/pegboard/src/keys/actor_kv.rs`

Plan:

- Refactor key wrapper types so pack-only paths can borrow key bytes instead of cloning them into owned `KeyWrapper` values repeatedly.
- Introduce borrowed packing helpers rather than converting `KeyWrapper` itself into a self-referential or lifetime-heavy storage type.
- One practical shape is:
  - Keep `KeyWrapper(pub ep::KvKey)` for owned decode results and APIs that need ownership.
  - Add a borrowed `KeyRef<'a>(&'a [u8])` that implements `TuplePack`.
  - Add `EntryValueChunkKeyRef<'a>` and `EntryMetadataKeyRef<'a>` for write-side packing.
- Update `actor_kv::put` to construct one borrowed key reference per entry and reuse it for metadata and chunk writes.

Why not `Arc<Vec<u8>>` first:

- The current hot path mostly pays for repeated cloning into tuple-pack wrapper types, not long-lived cross-task sharing.
- Borrowed packers remove clones without adding refcount churn or changing read-side decode types.

Verification:

- Keep wire layout identical. Add or update tests that compare key encoding before and after the refactor.
- Run the KV write tests and any preload or range-query tests that depend on the same key layout.

## Cross-Cutting Test Plan

- Pegboard KV metadata cache:
  - repeated request hit path
  - eviction on actor lifecycle event
  - namespace mismatch rejection still works
- Envoy-client WebSocket send:
  - disconnected path
  - normal send path
- Start-command enrichment:
  - multiple actors enriched concurrently
  - original wrapper order preserved
  - one failing actor aborts as expected
  - missed command replay path is covered
  - runner mk1 and mk2 paths are both covered
- Envoy-client ordering:
  - concurrent `ws_send` callers preserve frame order
- Native bridge:
  - actor start input binary delivery
  - HTTP request body and response body delivery
  - WebSocket text and binary payload delivery
  - callback response plumbing still resolves the correct request
  - typed response path replaces JSON response plumbing on the hot path
- JS KV wrapper:
  - `Buffer` input does not copy unnecessarily
  - `Uint8Array` input still works
  - contract stability is preserved for callers expecting `Uint8Array` semantics
- Pegboard KV put:
  - exact key encoding compatibility
  - large values still chunk correctly

## Rollout Order

1. Land the `get_for_kv` cache work first because it is the clearest low-risk win in the set.
2. Land Phase 2 next, with `pegboard-envoy` and `pegboard-runner` kept in sync where the code mirrors.
3. Land the `ws_send` change only if the ordering-safe design stays small. If preserving outbound ordering requires a larger single-writer refactor, defer that item behind the simpler cache and command-enrichment patches instead of forcing it into the first batch.
4. Land Phase 3 behind targeted native bridge coverage because it changes both Rust and JS interfaces.
5. Land Phase 4 last unless the bridge work stalls. It is isolated and can land independently.

## Open Questions To Resolve Before Implementation

- Where is the cleanest actor stop and destroy signal in `pegboard-envoy` for cache eviction, and does connection teardown alone give sufficient correctness?
- Should the cache intentionally skip explicit runner-change invalidation because the cached fields are runner-agnostic, and if so should that be documented next to the TODO in `get_for_kv.rs`?
- Should start-command enrichment concurrency be capped, and if so what is the safe default batch size?
- After verifying the addon executor model, is `JsEnvoyHandle.runtime` still needed anywhere outside startup?
- Does the typed N-API bridge need one callback per event family, or is a tagged union object ergonomic enough in both Rust and JS?
- Should the JS wrapper keep returning `Uint8Array`-compatible values for API stability, or can it return `Buffer` directly as an intentional compatibility change?

## Success Criteria

- Repeated KV operations for the same actor no longer trigger repeated metadata fetches on a live connection.
- `ws_send` no longer performs serialization while holding `ws_tx` lock.
- Start-command enrichment no longer awaits actor-by-actor sequentially.
- Native KV and lifecycle calls no longer pay a `spawn(...).await` hop when already on the correct runtime.
- HTTP and WebSocket binary payloads no longer round-trip through base64 across the native bridge.
- JS KV wrapper avoids avoidable copies for already-buffer inputs.
- KV put no longer clones the same key bytes once per chunk write.
