# SQLite VFS v2 Spec -- Implementability Review

Reviewed against: codebase at 7c64566fe8, 2026-04-15.

---

## 1. [BLOCKER] Wrong protocol layer

The spec says "runner-protocol v8" (section 4, section 15 items 27-30), referencing `engine/sdks/schemas/runner-protocol/v7.bare`. But KV ops are **not** dispatched through the runner-protocol. They go through the **envoy-protocol** (`engine/sdks/schemas/envoy-protocol/v1.bare`), handled in `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs` via `ToRivetKvRequest`. The runner-protocol (`v7.bare`) is used by the runner-to-engine connection (`engine/sdks/typescript/runner/`), not the envoy-to-engine connection that the VFS talks through.

The sqlite_* ops must be added to the envoy-protocol, not the runner-protocol. This affects: the BARE schema location, the versioned.rs to update, which PROTOCOL_VERSION constants to bump, and the TypeScript codegen target. The entire protocol section (section 4) and checklist items 27-30 need to be rewritten against `envoy-protocol/v2.bare`.

## 2. [BLOCKER] v1/v2 dispatch location is wrong

Section 8 says "the probe runs in pegboard-envoy at actor startup, before VFS registration." But the VFS is registered inside the actor process (`rivetkit-typescript/packages/sqlite-native/src/vfs.rs`), not in pegboard-envoy. Pegboard-envoy is an engine-side service; the actor runs in a separate process (or sandbox). The dispatch decision (v1 vs v2) needs to happen actor-side in `rivetkit-typescript/packages/rivetkit-native/src/database.rs`, not engine-side.

The engine has no mechanism to tell the actor which VFS to register at VFS registration time. The spec needs to define either: (a) a protocol field in the `CommandStartActor` or init handshake that tells the actor which schema version to use, or (b) the actor probes the engine on startup (e.g., via `sqlite_takeover`) and selects the VFS based on the response. Without this, the implementer is stuck.

## 3. [BLOCKER] SqliteStore::transact signature is unimplementable

The spec defines `transact` as taking `Box<dyn FnOnce(&dyn StoreTx) -> Result<()> + Send>` with a synchronous `StoreTx` trait. But universaldb's `run` method takes an async closure receiving a `RetryableTransaction` with async `get`, `set`, `clear_range`, etc. The `StoreTx` trait has synchronous `fn get`, `fn set`, `fn delete` methods. You cannot wrap universaldb's async transaction inside a synchronous trait. The `UdbSqliteStore` impl would need to either: (a) make `StoreTx` async (changing the whole trait), or (b) use `block_on` inside the transaction closure (which deadlocks on the tokio runtime). The implementer needs to redesign `transact` with an async closure signature to match UDB's API.

## 4. [ISSUE] Dependencies: parking_lot and litetx missing

Workspace Cargo.toml has `scc` (3.6.12), `moka` (0.12), and `lz4_flex` (0.11.3). `parking_lot` is not in the workspace dependencies and needs to be added. `litetx` is not in the workspace and the spec itself says it is unmaintained since 2023-09 and should be vendored or forked. The implementer needs to either vendor `litetx` or hand-write LTX encode/decode (the spec acknowledges this in section 3.3 and item 6 of the checklist says "hand-written, ~200 lines"). This is workable but should be called out as a prerequisite task, not assumed.

## 5. [ISSUE] BARE union backwards compatibility not addressed

The envoy-protocol uses a single union version (`v1`). Adding sqlite_* ops to the `ToRivet` and `ToEnvoy` unions creates a `v2.bare` with new variants. A v1 envoy talking to a v2 engine (or vice versa) will fail to deserialize messages with sqlite_* variants. The spec says nothing about backwards compatibility between protocol versions. The existing versioned.rs pattern can handle this, but the spec needs to state that v1 envoys simply cannot use sqlite v2 (which is fine since v1 actors will keep using the KV path). The implementer needs to wire the version negotiation so the engine only sends sqlite_* responses to v2-protocol envoys.

## 6. [ISSUE] EnvoyHandle KV methods live in the Rust envoy-client

The actor-side `EnvoyKv` impl in `database.rs` calls `self.handle.kv_get()`, `self.handle.kv_put()`, etc. on `EnvoyHandle` from `rivet-envoy-client`. The new `EnvoyV2` protocol impl needs analogous methods on `EnvoyHandle` (e.g., `sqlite_takeover`, `sqlite_get_pages`, `sqlite_commit`). The spec does not mention `engine/sdks/rust/envoy-client/` at all. The implementer needs to add 6 new methods to `EnvoyHandle` and wire them through the envoy-client's WebSocket send/receive machinery. This is significant missing glue code.

## 7. [ISSUE] TypeScript codegen for envoy-protocol not mentioned

The current KV flow from TypeScript uses the runner-protocol codegen (`rivetkit-typescript/packages/engine-runner/src/mod.ts` at `PROTOCOL_VERSION = 7`). But for the envoy path, the Rust envoy-client has its own BARE codegen via `engine/sdks/rust/envoy-protocol/build.rs`. The spec's checklist (items 28-29) says to bump `PROTOCOL_MK2_VERSION` in runner-protocol and `PROTOCOL_VERSION` in engine-runner, which is wrong since the sqlite ops go through the envoy protocol. The implementer needs to bump the envoy-protocol version instead.

## 8. [ISSUE] MemorySqliteStore is insufficient for protocol-level testing

`MemorySqliteStore` tests the `SqliteEngine<S>` layer in isolation. But the critical integration surface -- BARE serialization, WebSocket routing through envoy, and the VFS-to-engine round trip -- is untested. Section 12 does not describe any integration test that exercises the full protocol path. The implementer will need a test harness that stands up a mock envoy WebSocket and round-trips real BARE messages.

## 9. [SUGGESTION] Compaction coordinator should use scc::HashMap, not std::collections::HashMap

Section 7.1 uses `HashMap<String, JoinHandle<()>>` for the coordinator's worker map. Per CLAUDE.md, `Mutex<HashMap<...>>` is forbidden. The coordinator loop is single-threaded (one tokio task), so a plain `HashMap` is fine architecturally, but should be explicitly noted as single-task-owned to avoid review confusion.

## 10. [SUGGESTION] Checklist item 35 path is ambiguous

Item 35 says `EnvoyV2` impl goes in `rivetkit-typescript/packages/rivetkit-native/src/database.rs`. This is where `EnvoyKv` (v1) already lives. The implementer should add the v2 impl alongside it in the same file or a new `database_v2.rs`, but the checklist should be explicit. The napi bindings needed to expose `EnvoyV2` to the TypeScript layer are not mentioned at all.

## 11. [OK] Storage layout and key format

The key format (section 3.1) is clean, the prefix byte scheme (0x01 vs 0x02) for dispatch is sound, and the shard_id computation is straightforward.

## 12. [OK] SqliteStore trait (modulo transact)

The get/batch_get/set/batch_set/delete/delete_range/scan_prefix methods map well to UDB's transaction API. The only problem is the `transact` signature (see item 3).

## 13. [OK] Existing workspace dependencies

`scc`, `moka`, and `lz4_flex` are all present in the workspace at compatible versions. `async-trait`, `bytes`, `tokio`, `tracing`, `serde_bare` are also available.

## 14. [OK] File path conventions

The proposed `engine/packages/sqlite-storage/` follows existing patterns (`engine/packages/pegboard/`, `engine/packages/universaldb/`). The crate structure with `src/`, `tests/`, `benches/` is standard.

---

## Summary

Three blockers prevent starting implementation tomorrow: (1) the spec targets the wrong protocol layer (runner-protocol instead of envoy-protocol), (2) the v1/v2 dispatch mechanism assumes engine-side VFS registration that does not exist, and (3) the `StoreTx` synchronous trait cannot wrap UDB's async transaction API. After fixing these, the main implementation risk is the missing glue code in the envoy-client crate, which is a significant but straightforward engineering task.
