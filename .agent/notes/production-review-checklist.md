# Production Review Checklist

Consolidated from deep review (2026-04-19) + existing notes. Verified against actual code 2026-04-19.

---

## CRITICAL — Data Corruption / Crashes

- [ ] **C1: Connection hibernation encoding mismatch** — `gateway_id`/`request_id` are fixed 4-byte in TS BARE v4 (`bare.readFixedData(bc, 4)`) but variable-length `Vec<u8>` in Rust serde_bare (length-prefixed). Wire format incompatibility confirmed. Actors persisted by TS and loaded by Rust (or vice versa) get corrupted connection metadata. Fix: change Rust to `[u8; 4]` with custom serde. (`rivetkit-core/src/actor/connection.rs:58-69`)

- [ ] **C2: Missing on_state_change idle wait during shutdown** — Action dispatch waits for `on_state_change` idle (`action.rs:98`), but sleep and destroy shutdown do not. In-flight `on_state_change` callback can race with final `save_state`. Fix: add `wait_for_on_state_change_idle().await` with deadline after `set_started(false)` in both paths. (`rivetkit-core/src/actor/lifecycle.rs:215` sleep, `:303` destroy)

- [ ] **C3: NAPI string leaking via Box::leak()** — `leak_str()` in `parse_bridge_rivet_error` leaks every unique error group/code/message as `&'static str`. Bounded by error message uniqueness in practice (group/code are finite, but message can include user context). (`rivetkit-napi/src/actor_factory.rs:889-903`)

---

## HIGH — Real Issues Worth Fixing

- [ ] **H1: Scheduled event panic not caught** — `run` handler is wrapped in `catch_unwind`, but scheduled event dispatch (`invoke_action_by_name`) is not. Low practical risk since actions go through serialization boundaries, but a defensive gap. (`rivetkit-core/src/actor/schedule.rs:199-264`)

- [ ] **H2: Action timeout/size enforcement in wrong layer** — TS `native.ts` enforces `withTimeout()` and message size for HTTP actions. Rust `handle_fetch` bypasses these. Different execution paths (not double enforcement), but HTTP path lacks Rust-side enforcement. Should consolidate into Rust.

- [ ] **H3: Mutex\<HashMap\> violations (5 instances)** — CLAUDE.md forbids this. Replace with `scc::HashMap` (preferred) or `DashMap`. Locations: `rivetkit-core/src/actor/queue.rs:105` (completion_waiters), `client/src/connection.rs:70` (in_flight_rpcs), `client/src/connection.rs:72` (event_subscriptions), `rivetkit-sqlite/src/vfs.rs:1632` (stores), `rivetkit-sqlite/src/vfs.rs:1633` (op_log)

---

## MEDIUM — Pre-existing TS Issues (Not Regressions)

These existed before the Rust migration. Tracked here for visibility but are not caused by the migration.

- [ ] **M1: Traces exceed KV value limits** — `DEFAULT_MAX_CHUNK_BYTES = 1MB`, KV max value = 128KB. (`rivetkit-typescript/packages/traces/src/traces.ts:63`)

- [ ] **M2: SQLite VFS unsplit putBatch/deleteBatch** — Can exceed 128 entries and/or 976KB payload. (`rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts:856,908,979`)

- [ ] **M3: Workflow persistence unsplit write arrays** — `storage.flush` builds unbounded writes, calls `driver.batch(writes)` once. (`rivetkit-typescript/packages/workflow-engine/src/storage.ts:270,346`)

- [ ] **M4: Workflow flush clears dirty flags before write success** — If batch fails, dirty markers lost. (`rivetkit-typescript/packages/workflow-engine/src/storage.ts:296,308`)

- [ ] **M5: State persistence can exceed batch limits** — `savePersistInner` aggregates actor + all changed connections into one batch. (`rivetkit-typescript/packages/rivetkit/src/actor/instance/state-manager.ts:422,503`)

- [ ] **M6: Queue batch delete can exceed limits** — Removes all selected messages in one `kvBatchDelete(keys)`. (`rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:520,530`)

- [ ] **M7: Traces write queue poison after KV failure** — `writeChain` promise chain has no rejection recovery. (`rivetkit-typescript/packages/traces/src/traces.ts:545,767`)

- [ ] **M8: Queue metadata mutates before storage write** — Enqueue increments `nextId`/`size` before `kvBatchPut`. If write fails, in-memory metadata drifts. (`rivetkit-typescript/packages/rivetkit/src/actor/instance/queue-manager.ts:163,168,523`)

- [ ] **M9: Connection cleanup swallows KV delete failures** — Stale connection KV may remain. (`rivetkit-typescript/packages/rivetkit/src/actor/instance/connection-manager.ts:372,379`)

- [ ] **M10: Cloudflare driver KV divergence** — No engine-equivalent limit validation. (`rivetkit-typescript/packages/cloudflare-workers/src/actor-kv.ts:14`)

- [ ] **M11: v2 actor dispatch requires ~5s delay after metadata refresh** — Engine-side issue. (`v2-metadata-delay-bug.md`)

---

## LOW — Code Quality / Cleanup

- [ ] **L1: BARE codec extraction** — ~380 lines of hand-rolled BARE across `registry.rs` (~257 lines) and `client/src/protocol/codec.rs` (~123 lines). Should be replaced by generated protocol crate.

- [ ] **L2: registry.rs is 3668 lines** — Biggest file by far. Needs splitting.

- [ ] **L3: Metrics registry panic** — `expect()` on prometheus gauge creation. Should fallback to no-op. (`rivetkit-core/src/actor/metrics.rs:62-77`)

- [ ] **L4: Response map orphaned entries (NAPI)** — Minor: on error paths, response_id entry not cleaned up from map. Cleaned on actor stop. (`rivetkit-napi/src/bridge_actor.rs:200-223`)

- [ ] **L5: Unused serde derives on protocol structs** — `registry.rs` protocol types derive Serialize/Deserialize but use hand-rolled BARE.

- [ ] **L6: _is_restoring_hibernatable unused** — `registry.rs` accepts but ignores this param.

---

## SEPARATE EFFORTS (not blocking ship)

- [ ] **S1: Workflow replay refactor** — 6 action items in `workflow-replay-review.md`.

- [ ] **S2: Rust client parity** — Full spec in `.agent/specs/rust-client-parity.md`.

- [ ] **S3: WASM shell shebang** — Blocks agentOS host tool shims. (`.agent/todo/wasm-shell-shebang.md`)

- [ ] **S4: Native bridge bugs (engine-side)** — WebSocket guard + message_index conflict. (`native-bridge-bugs.md`)

---

## REMOVED — Verified as Not Issues

Items from original checklist that were verified as bullshit or already fixed:

- ~~Ready state vs connection restore race~~ — OVERSTATED. Microsecond window, alarms gated by `started` flag.
- ~~Queue completion waiter leak~~ — BULLSHIT. Rust drop semantics clean up when Arc is dropped.
- ~~Unbounded HTTP body size~~ — OVERSTATED. Envoy/engine enforce limits upstream.
- ~~BARE-only encoding~~ — ALREADY FIXED. Accepts json/cbor/bare.
- ~~Error metadata dropped~~ — ALREADY FIXED. Metadata field exists and is passed through.
- ~~Action timeout double enforcement~~ — BULLSHIT. Different execution paths, not overlapping.
- ~~Lock poisoning pattern~~ — BULLSHIT. Standard Rust practice with `.expect()`.
- ~~State lock held across I/O~~ — BULLSHIT. Data cloned first, lock released before I/O.
- ~~SQLite startup cache leak~~ — BULLSHIT. Cleanup exists in on_actor_stop.
- ~~WebSocket callback accumulation~~ — BULLSHIT. Callbacks are replaced via `configure_*_callback(Some(...))`, not accumulated.
- ~~Inspector DB access~~ — BULLSHIT. No raw SQL in inspector.
- ~~Raw WS outgoing size~~ — BULLSHIT. Enforced at handler level.
- ~~Unbounded tokio::spawn~~ — BULLSHIT. Tracked via keep_awake counters.
- ~~Error format changed~~ — SAME AS TS. Internal bridge format, not external.
- ~~Queue send() returns Promise~~ — SAME AS TS. Always was async.
- ~~Error visibility forced~~ — SAME AS TS. Pre-existing normalization.
- ~~Queue complete() double call~~ — Expected behavior, not breaking.
- ~~Negative queue timeout~~ — Stricter validation, unlikely to break real code.
- ~~SQLite schema version cached~~ — Required by design, not a bug.
- ~~Connection state write-through proxy~~ — Unclear claim, unverifiable.
- ~~WebSocket setEventCallback~~ — Internal API, handled by adapter.
- Code quality items (actor key file, Request/Response file, rename callbacks, rename FlatActorConfig, context.rs issues, #[allow(dead_code)], move kv.rs/sqlite.rs) — Moved to `production-review-complaints.md`.

---

## VERIFIED OK

- Architecture layering: CLEAN
- Actor state BARE encoding v4: compatible
- Queue message/metadata BARE encoding: compatible
- KV key layout (prefixes [1]-[7]): identical
- SQLite v1 chunk storage (4096-byte chunks): compatible
- BARE codec overflow/underflow protection: correct
- WebSocket init/reconnect/close: correct
- Authentication (bearer token on inspector): enforced
- SQL injection: parameterized queries, read-only enforcement
- Envoy client bugs B1/B2: FIXED
- Envoy client perf P1-P6: FIXED
- Driver test suite: all fast+slow tests PASS (excluding agent-os, cross-backend-vfs)
