# Production Review Complaints

Tracking issues and complaints about the rivetkit Rust implementation for production readiness.

Re-verified 2026-04-21 against HEAD `7764a15fd`. Fixed items removed.

---

## TS/NAPI Cleanup & Routing (fix first)

28. **Unify HTTP routing in core** — Framework routes are split across two layers with no clean boundary. Rust `handle_fetch` owns `/metrics` and `/inspector/*`. TS `on_request` callback owns `/action/*` and `/queue/*` via regex matching in `maybeHandleNativeActionRequest` and `maybeHandleNativeQueueRequest`. Path parsing happens twice (Rust prefix checks, then TS regex). The `on_request` callback became a fallback router instead of a user handler. Fix: core should own all framework routes (`/metrics`, `/inspector/*`, `/action/*`, `/queue/*`), and only delegate unmatched paths to the user's `on_request` callback.

25. **Move HTTP action/queue dispatch from TS to core** — TS `native.ts` owns HTTP action dispatch (`maybeHandleNativeActionRequest`, ~lines 2656-2871) and queue dispatch (`maybeHandleNativeQueueRequest`, ~lines 2873-3041) with routing, timeout enforcement, message size limits, and response encoding. Core already dispatches actions via WebSocket (`ActionInvoker::dispatch()` in `action.rs`). Move HTTP routing + dispatch + timeout + size checks into core's `handle_fetch()`. Schema validation stays in TS (pre-validated before calling core). Fixes checklist item H2 and enables Rust runtime parity.

10. **Action timeout/size enforcement lives in TS instead of Rust** — `native.ts` enforces `withTimeout()` and `maxIncomingMessageSize`/`maxOutgoingMessageSize` for HTTP actions. Rust `handle_fetch` in `registry.rs` bypasses these checks entirely. WebSocket path enforces them in Rust. Consolidate into Rust.

27. **Remove `AsyncMutex` action serialization from TS native bridge** — Rust core action lock was REMOVED (verified 2026-04-21: `action.rs` is now 23 lines with no mutex, `context.rs` has zero `action_lock`). TS side still serializes via `AsyncMutex actionMutex` at `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:130,224,3055`. The original TS runtime had NO serialization. Fix: remove the `AsyncMutex` from the native bridge to restore concurrent action dispatch per actor.

13. **Delete `openDatabaseFromEnvoy` and its supporting caches** — `rivetkit-typescript/packages/rivetkit-napi/src/database.rs:189-221` plus the `sqlite_startup_map` and `sqlite_schema_version_map` on `JsEnvoyHandle` (`src/envoy_handle.rs:32-33, 55-68`) and the matching insert/remove sites in `src/bridge_actor.rs:27-30, 44-45, 84-99, 143-148`. Verified: zero callers in `rivetkit-typescript/packages/rivetkit/`. The production path goes through `ActorContext::sql()` which already has the schema version + startup data via `RegistryCallbacks::on_actor_start`.

14. **Delete `BridgeCallbacks` JSON-envelope path** — `rivetkit-typescript/packages/rivetkit-napi/src/bridge_actor.rs` (entire file) plus `start_envoy_sync_js` / `start_envoy_js` entry points in `src/lib.rs:80-156` and the `wrapper.js` adapter layer (`startEnvoySync`/`startEnvoy`/`wrapHandle` ~lines 36-174). Production uses `NapiActorFactory` + `CoreRegistry` via direct rivetkit-core callbacks, not this JSON-envelope bridge. ~700 lines of Rust + ~490 lines of JS removable.

15. **Delete standalone `SqliteDb` wrapper** — `rivetkit-typescript/packages/rivetkit-napi/src/sqlite_db.rs`. Verified: production sql access goes through `JsNativeDatabase` via `ctx.sql()`, not this class.

16. **Delete `JsEnvoyHandle::start_serverless` method** — `rivetkit-typescript/packages/rivetkit-napi/src/envoy_handle.rs:378-387`. Verified dead: serverless support was removed from the TypeScript routing stack and `Runtime.startServerless()` in `rivetkit/runtime/index.ts:117` already throws `removedLegacyRoutingError`. The NAPI method is unreachable.

17. **Drop the `wrapper.js` adapter layer once items 13-14 land** — `rivetkit-typescript/packages/rivetkit-napi/wrapper.js` exists to translate JSON envelopes back into `EnvoyConfig` callbacks for the dead BridgeCallbacks path. After deletion, rivetkit can import `index.js` directly and the wrapper module disappears.

---

## Core Architecture

5. **`context.rs` passes `ActorConfig::default()` to Queue and ConnectionManager** — `build()` receives a `config` param but ignores it for Queue (line 145) and ConnectionManager (line 152) and SleepController. Possible bug: these subsystems get default timeouts instead of the actor's configured values.

6. **`sleep()` spawns fire-and-forget task with untracked JoinHandle** — `context.rs:286-297`. Spawned task persists connections and requests sleep. Not tracked, can be orphaned on destroy.

7. **`Default` impl creates empty context with `actor_id: ""`** — `context.rs:997-1001`. Footgun for any code calling `ActorContext::default()`.

11. **`registry.rs` is 3668 lines** — Now the biggest file by far. Needs splitting.

18. **Review all `tokio::spawn` and replace with JoinSets** — Audit every `tokio::spawn` in rivetkit-core and rivetkit-sqlite for untracked fire-and-forget tasks. Replace with `JoinSet` so shutdown can abort and join all spawned tasks cleanly. Ensure JoinSets are cancelled/aborted on actor completion (sleep, destroy) so no orphaned tasks outlive the actor.

26. **Merge `active_instances` and `stopping_instances` maps** — Registry tracks actors across 4 concurrent maps (`starting_instances`, `active_instances`, `stopping_instances`, `pending_stops`). `active_instances` and `stopping_instances` both store `ActiveActorInstance` (same type). Merge into a single `SccHashMap<String, ActorInstanceState>` with an enum `{ Active(ActiveActorInstance), Stopping(ActiveActorInstance) }`. Eliminates the multi-map lookup in `active_actor()` which currently searches both maps sequentially. `starting_instances` (Arc\<Notify\>) and `pending_stops` (PendingStop) have different value types and should stay separate. (`rivetkit-core/src/registry.rs:78-81`)

25b. **Remove `ActorContext::new_runtime`, make `build` pub(crate)** — `new_runtime` is a misleading name ("runtime" isn't a concept in the system). It's just the fully-configured constructor vs the test-only `new`/`new_with_kv` convenience constructors. Delete `new_runtime`, make `build()` `pub(crate)`, and have callers use `build()` directly. (`rivetkit-core/src/actor/context.rs:110-128`)

---

## Wire Compatibility

23. **`gateway_id`/`request_id` must be `[u8; 4]`, not `Vec<u8>`** — Runner protocol BARE schema defines `type GatewayId data[4]` and `type RequestId data[4]` (fixed 4-byte). Rust `PersistedConnection` uses `Vec<u8>` which serializes with a length prefix, breaking wire compatibility with TS. Fix: change to `[u8; 4]` with fixed-size serde. This is NOT the same as the engine `Id` type (which is 19 bytes). (`rivetkit-core/src/actor/connection.rs:58-69`, `engine/sdks/schemas/runner-protocol/v7.bare:8-9`)

12. **Use native `Id` type from engine instead of `Vec<u8>` for IDs** — Connection `gateway_id`/`request_id` and other IDs use `Vec<u8>` instead of the engine's native `Id` type. Should switch to the proper type.

---

## Code Quality

1. **Actor key ser/de should be in its own file** — Currently in `types.rs` alongside unrelated types. Move to `utils/key.rs`.

2. **Request and Response structs need their own file** — Currently in `actor/callbacks.rs` (364 lines, 19 structs). Move to a dedicated file.

3. **Rename `callbacks` to `lifecycle_hooks`** — `actor/callbacks.rs` should be `actor/lifecycle_hooks.rs`.

4. **Rename `FlatActorConfig` to `ActorConfigInput`** — Add doc comment: "Sparse, serialization-friendly actor configuration. All fields are optional with millisecond integers instead of Duration. Used at runtime boundaries (NAPI, config files). Convert to ActorConfig via ActorConfig::from_input()." Rename `from_flat()` to `from_input()`.

8. **Remove all `#[allow(dead_code)]`** — 57 instances across rivetkit-core. All decorated methods are actually called from external modules. Attributes are unnecessary cargo-cult suppressions. Safe to remove all.

9. **Move `kv.rs` and `sqlite.rs` out of top-level `src/`** — They're actor subsystems. Move to `src/actor/kv.rs` and `src/actor/sqlite.rs`.

---

## Safety & Correctness

19. **Review inspector security** — General audit of inspector endpoints in `registry.rs:704-900`. Check auth is enforced on all paths, no unintended state mutations, and that the TS and Rust inspector surfaces match.

20. **No panics unless absolutely necessary** — rivetkit-core, rivetkit, and rivetkit-napi should never panic. There are ~146 `.expect("lock poisoned")` calls that should be replaced with non-poisoning locks (e.g. `parking_lot::RwLock`/`Mutex`) or proper error propagation. Audit all `unwrap()`, `expect()`, and `panic!()` calls across these three crates and eliminate them.

22. **Standardize error handling with rivetkit-core** — Investigate whether errors across rivetkit-core, rivetkit, and rivetkit-napi are consistently using `RivetError` with proper group/code/message. Look for places using raw `anyhow!()` or string errors that should be structured `RivetError` types instead.

---

## Investigation

21. **Investigate v1 vs v2 SQLite wiring** — Need to understand how v1 and v2 VFS are dispatched, whether both paths are correctly wired through rivetkit-core, and if there are any gaps in the v1-to-v2 migration path.
