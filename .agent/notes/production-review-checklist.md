# Production Review Checklist

Consolidated from deep review (2026-04-19) + existing notes. Re-verified against HEAD `7764a15fd` on 2026-04-21. Fixed/stale items removed.

---

## CRITICAL — Data Corruption / Crashes

- [ ] **C1: Connection hibernation encoding mismatch** — `gateway_id`/`request_id` are fixed 4-byte in TS BARE v4 (`bare.readFixedData(bc, 4)`) but variable-length `Vec<u8>` in Rust serde_bare (length-prefixed). Wire format incompatibility confirmed. Actors persisted by TS and loaded by Rust (or vice versa) get corrupted connection metadata. Fix: change Rust to `[u8; 4]` with custom serde. (`rivetkit-core/src/actor/connection.rs:58-69`)

- [ ] **C2: Missing on_state_change idle wait during shutdown** — Action dispatch waits for `on_state_change` idle, but sleep and destroy shutdown do not. In-flight `on_state_change` callback (flag at `state.rs:72`) can race with final `save_state`. Fix: add `wait_for_on_state_change_idle().await` with deadline after `set_started(false)` in both paths. (Lifecycle migrated to `rivetkit-core/src/actor/task.rs`: `shutdown_for_sleep:720`, `shutdown_for_destroy:782`.)

---

## HIGH — Real Issues Worth Fixing

- [ ] **H2: Action timeout/size enforcement in wrong layer** — TS `native.ts` enforces `withTimeout()` and message size for HTTP actions. Rust `handle_fetch` (`registry.rs:692`) dispatches `DispatchCommand::Http` with no timeout or size enforcement. Different execution paths (not double enforcement), but HTTP path lacks Rust-side enforcement. Should consolidate into Rust.

- [ ] **H3: Mutex\<HashMap\> violations (remaining after US-218)** — CLAUDE.md forbids this. Replace with `scc::HashMap` (preferred) or `DashMap`. Remaining locations: `client/src/connection.rs:70` (in_flight_rpcs), `client/src/connection.rs:72` (event_subscriptions). Test-only (low priority): `rivetkit-sqlite/src/vfs.rs:1632,1633` under `#[cfg(test)]`.

---

## MEDIUM — Pre-existing TS Issues (Not Regressions)

These existed before the Rust migration. Tracked here for visibility but are not caused by the migration.

- [ ] **M1: Traces exceed KV value limits** — `DEFAULT_MAX_CHUNK_BYTES = 1MB`, KV max value = 128KB. (`rivetkit-typescript/packages/traces/src/traces.ts:63`)

- [ ] **M3: Workflow persistence unsplit write arrays** — `storage.flush` builds unbounded writes, calls `driver.batch(writes)` once. (`rivetkit-typescript/packages/workflow-engine/src/storage.ts:316`)

- [ ] **M4: Workflow flush clears dirty flags before write success** — If batch fails, dirty markers lost. (`rivetkit-typescript/packages/workflow-engine/src/storage.ts:266,278`)

- [ ] **M7: Traces write queue poison after KV failure** — `writeChain` promise chain has no rejection recovery. (`rivetkit-typescript/packages/traces/src/traces.ts:169,560,792`)

- [ ] **M11: v2 actor dispatch requires ~5s delay after metadata refresh** — Engine-side issue. (`v2-metadata-delay-bug.md`)

---

## LOW — Code Quality / Cleanup

- [ ] **L1: BARE codec extraction** — ~380 lines of hand-rolled BARE across `registry.rs` (~257 lines) and `client/src/protocol/codec.rs` (~123 lines). Should be replaced by generated protocol crate.

- [ ] **L2: registry.rs is 3668 lines** — Biggest file by far. Needs splitting.

- [ ] **L3: Metrics registry panic** — `expect()` on prometheus gauge creation. Should fallback to no-op. (`rivetkit-core/src/actor/metrics.rs:62-77`)

- [ ] **L4: Response map orphaned entries (NAPI)** — Minor: on error paths, response_id entry not cleaned up from map. Cleaned on actor stop. (`rivetkit-napi/src/bridge_actor.rs:200-223`)

- [ ] **L5: Unused serde derives on protocol structs** — `registry.rs` protocol types derive Serialize/Deserialize but use hand-rolled BARE.

- [ ] **L6: _is_restoring_hibernatable unused** — `registry.rs` accepts but ignores this param.

- [ ] **L7: Shared-counter waiters need wakeups** — Review every shared counter with async awaiters for a paired `Notify`, `watch`, or permit. Decrement-to-zero sites must wake waiters, and waiters must arm before re-checking the counter.
