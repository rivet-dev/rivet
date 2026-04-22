# rivetkit-core / napi / typescript Adversarial Review — Synthesis

Findings consolidated from 5 original review agents (API parity, SQLite v2 soundness, test quality, lifecycle conformance, code quality) plus 3 spec-review agents that ran on the proposed shutdown redesign.

Each finding below includes the citation the original agent provided. **Subject to verification** — agents may have been wrong.

---

## Blockers

### F1. Engine-Destroy doesn't fire `c.aborted` in `onDestroy`

**Claim.** When the engine sends `Stop { Destroy }`, `run_shutdown` never calls `ctx.cancel_abort_signal_for_sleep()`. The abort signal only fires for Sleep (because `cancel_abort_signal_for_sleep` runs in `start_sleep_grace`) and for self-initiated destroy via `c.destroy()` (because `mark_destroy_requested` calls it at `context.rs:466`). Engine-initiated Destroy bypasses both paths.

**Evidence.** `task.rs:1496-1676` (`run_shutdown` body) shows no abort-signal cancel. `task.rs:1497-1519` shows abort-signal cancel is only in `start_sleep_grace`. `context.rs:461-467` shows self-destroy path calls cancel.

**User-visible impact.** User code in `onDestroy` that checks `c.aborted` sees `false`. Contradicts `lifecycle.mdx:932` which says the abort signal fires before `onDestroy` runs.

**Source.** Lifecycle agent (N-11, claimed as new confirmed bug not previously filed).

### F2. 2× `sleepGracePeriod` wall-clock budget

**Claim.** `start_sleep_grace` at `task.rs:1376` computes `deadline = now + sleep_grace_period` for the idle wait. After grace exits, `run_shutdown` at `task.rs:1508-1518` computes a **fresh** `deadline = now + effective_sleep_grace_period()`. Total wall-clock from grace entry to save start can be up to 2× `sleepGracePeriod`.

**User-visible impact.** Users set 15s and actor can take up to 30s to shut down.

**Source.** Lifecycle agent, independently confirmed by me during spec drafting.

### F3. `onSleep` silently doesn't run when `run` already returned

**Claim.** `request_begin_sleep` at `task.rs:2170-2173` early-returns if `run_handle.is_none()`. So if user's `run` handler exited cleanly before Stop arrived, `ActorEvent::BeginSleep` never enqueues, and the adapter's `onSleep` spawn path at `napi_actor_events.rs:566-575` is never triggered.

**Source.** Lifecycle agent.

### F4. `run_handle` awaited at end of `run_shutdown`, after hooks

**Claim.** Doc contract (`lifecycle.mdx:838-843`): step 2 waits for `run`, step 3 runs `onSleep`. Actual code: `onSleep` spawns from `BeginSleep` at grace entry, and `run_handle.take()` + select-with-sleep happens at `task.rs:1657-1680` (end of `run_shutdown`, after drain/disconnect).

**User-visible impact.** `onSleep` runs concurrently with user's `run` handler instead of after it.

**Source.** Lifecycle agent + spec drafting.

### F5. Self-initiated `c.destroy()` bypasses grace under the new design

**Claim.** `handle_run_handle_outcome` at `task.rs:1337-1349` sees `destroy_requested` flag when `run` returns, and jumps to `LiveExit::Shutdown` directly. Under the proposed grace-based design, this path skips the grace window entirely, so `onDestroy` never fires for self-initiated destroy.

**Source.** Spec correctness agent (B3).

---

## High-priority

### F6. SQLite v1→v2 has no cross-process migration fence

**Claim.** `SQLITE_MIGRATION_LOCKS` at `engine/packages/pegboard-envoy/src/sqlite_runtime.rs:24` is a `OnceLock<HashMap<...>>` local to one pegboard-envoy process. Two envoy processes hitting the same actor concurrently (failover, scale-out, split-brain) both pass the origin-None check at `:141-155` and both call `prepare_v1_migration` (`takeover.rs:64`) which wipes chunks/pidx each time.

**Source.** SQLite agent.

### F7. `prepare_v1_migration` resets generation on every call

**Claim.** `takeover.rs:99` builds `DBHead::new(now_ms)` which hardcodes `generation: 1` (`types.rs:51`). If a stale `MigratingFromV1` exists at generation 5, prepare overwrites to 1. Generation fence in `commit_stage_begin` (`commit.rs:200-206`) cannot distinguish concurrent prepare reset.

**Source.** SQLite agent.

### F8. Truncate leaks PIDX + DELTA entries above new EOF

**Claim.** `vfs.rs:1403-1413` `truncate_main_file` updates `state.db_size_pages` but does not mark pages `>= new_size` for deletion. `commit.rs:222` sets `head.db_size_pages` but doesn't clear `pidx_delta_key(pgno)` for `pgno > new_db_size_pages`. `build_recovery_plan` (`takeover.rs:222-278`) only filters by `txid > head.head_txid`.

**User-visible impact.** Permanent KV-space leak on every shrink.

**Source.** SQLite agent.

### F9. V1 data never cleaned up after successful migration

**Claim.** After `commit_finalize` sets origin to `MigratedFromV1` (`sqlite_runtime.rs:234`), the V1 KV entries under `0x08` prefix (`:26`) are left in place. `mod.rs` has `delete_all`, `delete_range` helpers but neither is called from the migration path.

**User-visible impact.** Storage doubles per migrated actor, forever.

**Source.** SQLite agent.

### F10. 5-minute migration lease blocks legitimate crash recovery

**Claim.** `sqlite_runtime.rs:34, 149-152`. If pegboard-envoy crashes between `commit_stage_begin` and `commit_finalize`, the next start within 5 minutes returns `"sqlite v1 migration for actor ... is already in progress"`. Actor can't start for 5 minutes.

**Source.** SQLite agent.

### F11. Every actor start probes `sqlite_v1_data_exists`

**Claim.** `actor_kv/mod.rs:46-71` issues a range scan with `limit:1` under a fresh transaction even for actors that never had v1 data. Extra UDB RTT on hot actor-start path, forever.

**Source.** SQLite agent.

### F12. `Registry.handler()` and `Registry.serve()` throw at runtime

**Claim.** `rivetkit-typescript/packages/rivetkit/src/registry/index.ts:76, 89-94` throws `"removedLegacyRoutingError"`. Old branch (`feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/registry/index.ts:75-77`) returned a real `Response`.

**User-visible impact.** `export default registry.serve()` breaks instantly. No deprecation notice.

**Source.** API parity agent.

### F13. ~45 typed error classes deleted from `@rivetkit/*` `./errors` subpath

**Claim.** Reference (`feat/sqlite-vfs-v2`) `actor/errors.ts` exported ~45 concrete subclasses: `InternalError`, `Unreachable`, `ActionTimedOut`, `ActionNotFound`, `InvalidEncoding`, `IncomingMessageTooLong`, `OutgoingMessageTooLong`, `MalformedMessage`, `InvalidStateType`, `QueueFull`, `QueueMessageTooLarge`, etc. Current exports only `RivetError`, `UserError`, `ActorError` alias plus factory functions.

**User-visible impact.** `catch (e) { if (e instanceof QueueFull) … }` breaks — `QueueFull` undefined.

**Source.** API parity agent.

### F14. Package `exports` subpaths removed

**Claim.** `rivetkit-typescript/packages/rivetkit/package.json:25-99` dropped: `./dynamic`, `./driver-helpers`, `./driver-helpers/websocket`, `./topologies/coordinate`, `./topologies/partition`, `./test`, `./inspector`, `./inspector/client`, `./db`, `./db/drizzle`, `./sandbox`, `./sandbox/client`, `./sandbox/computesdk`, `./sandbox/daytona`, `./sandbox/docker`, `./sandbox/e2b`, `./sandbox/local`, `./sandbox/modal`, `./sandbox/sprites`, `./sandbox/vercel`.

**User-visible impact.** `import "rivetkit/test"`, `import "rivetkit/db/drizzle"`, etc. all resolve to nothing.

**Source.** API parity agent.

### F15. `ActorError.__type` silently changed

**Claim.** Reference `actor/errors.ts:17`: `class ActorError extends Error { __type = "ActorError"; … }`. Current `actor/errors.ts:209`: `ActorError = RivetError` whose `__type = "RivetError"`. Tag comparison `err.__type === "ActorError"` stops matching.

**Source.** API parity agent.

### F16. Signal-primitive mismatch: `notify_one` vs `notify_waiters`

**Claim.** `AsyncCounter::register_change_notify(&activity_notify)` at `sleep.rs:615` wires counter changes through `notify_waiters()` at `async_counter.rs:79` (no permit storage). The spec wants `notify_one` semantics (stores permit). Mixed shapes cause lost wakes when a counter fires while no waiter is registered (i.e., main loop is inside `.await`).

**Source.** Spec concurrency agent (§1).

### F17. `handle_run_handle_outcome` emits no notify when clearing `run_handle`

**Claim.** `task.rs:1322` writes `self.run_handle = None` but doesn't call `reset_sleep_timer` or notify `activity_notify`. Under the grace-drain predicate `can_finalize_sleep() && run_handle.is_none()`, grace would silently degrade to deadline path whenever `run` exits after the last tracked task.

**Source.** Spec concurrency agent (§2).

### F18. Actor-lifecycle state lives in napi, not core

**Claim.** `rivetkit-typescript/packages/rivetkit-napi/src/actor_context.rs:58-70, 505-522, 770-787` stores `ready: AtomicBool`, `started: AtomicBool` on `ActorContextShared` and exposes `mark_ready`, `mark_started`, `is_ready`, `is_started` through NAPI. No equivalent in core. A future V8 runtime would have to re-implement.

**Source.** Code quality agent.

### F19. Inspector logic duplicated in TS

**Claim.** `rivetkit-typescript/packages/rivetkit/src/inspector/actor-inspector.ts:141-475` implements `ActorInspector` with `patchState`, `executeAction`, `getDatabaseSchema`, `getQueueStatus`, `replayWorkflowFromStep` directly in TS. Core has `src/inspector/` and `registry/inspector.rs` (775 lines) + `inspector_ws.rs` (447 lines) that duplicate surface area.

**Source.** Code quality agent.

### F20. Shutdown-save orchestration duplicated in napi

**Claim.** `rivetkit-typescript/packages/rivetkit-napi/src/napi_actor_events.rs:624-719` implements `handle_sleep_event`, `handle_destroy_event`, `notify_disconnects_inline`, `maybe_shutdown_save` — sequencing callbacks + conn-disconnect + state-save. The ordering is lifecycle logic that a V8 runtime would re-implement verbatim.

**Source.** Code quality agent.

---

## Medium-priority

### F21. 50ms polling loop in TypeScript

**Claim.** `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:2405-2415` uses `setInterval(..., 50)` to poll `this.#isDispatchCancelled(cancelTokenId)` even though a native `on_cancelled` TSF callback already exists at `rivetkit-napi/src/cancellation_token.rs:47-73`.

**Source.** Code quality agent.

### F22. Banned mock patterns

**Claim.** `vi.spyOn(Runtime, "create").mockResolvedValue(createMockRuntime())` at `rivetkit-typescript/packages/rivetkit/tests/registry-constructor.test.ts:30-32, :52`. Same for `vi.spyOn(Date, "now").mockImplementation(...)` in `packages/traces/tests/traces.test.ts:184-187, :365`.

**Source.** Test quality agent.

### F23. `createMockNativeContext` factory fakes the whole NAPI

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/native-save-state.test.ts:14-59, :73, :237, :250` produces full fake `NativeActorContext` objects via `vi.fn()`. Tests the TS adapter against fakes, never exercises real NAPI.

**Source.** Test quality agent.

### F24. `expect(true).toBe(true)` sentinel after race iterations

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-lifecycle.test.ts:118` asserts `expect(true).toBe(true)` after 10 create/destroy iterations with comment "If we get here without errors, the race condition is handled correctly."

**Source.** Test quality agent.

### F25. 10 skipped tests in `actor-sleep-db.test.ts` without tracking

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-sleep-db.test.ts:219, 260, 292, 375, 522, 572, 617, 739, 895, 976` — 10 `test.skip` covering `onDisconnect` during sleep shutdown, async websocket close DB writes, action dispatch during sleep shutdown, new-conn rejection, double-sleep no-op, concurrent WebSocket DB handlers. No tracking ticket on any.

**Source.** Test quality agent.

### F26. `test.skip("onDestroy is called even when actor is destroyed during start")`

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/driver/actor-lifecycle.test.ts:142`. Real invariant silently disabled. No tracking link.

**Source.** Test quality agent.

### F27. Flake fixes papering over races

**Claim.** `.agent/notes/flake-conn-websocket.md:45-47` proposes bumping wait. `driver-test-progress.md:57, :68` notes "passes on retry" with no regression test added. `actor-sleep-db.test.ts:198-208` wraps assertions in `vi.waitFor({ timeout: 5000, interval: 50 })` with no explanation of why polling is needed.

**Source.** Test quality agent.

### F28. `hibernatable-websocket-protocol.test.ts` skips entire suite

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/driver/hibernatable-websocket-protocol.test.ts:140` skips the whole suite when `!features?.hibernatableWebSocketProtocol`. Per `driver-test-progress.md:47`, "all 6 tests skipped" in default driver config.

**Source.** Test quality agent.

### F29. Silent no-op: `can_hibernate` always returns false

**Claim.** `rivetkit-typescript/packages/rivetkit-napi/src/bridge_actor.rs:371-379` hard-codes `fn can_hibernate(...) -> bool { false }`. Runtime capability check that always returns false.

**Source.** Code quality agent.

### F30. Plain `Error` thrown on required path instead of `RivetError`

**Claim.** `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:2654` throws `new Error("native actor client is not configured")`. CLAUDE.md says errors at boundaries must be `RivetError`.

**Source.** Code quality agent.

### F31. Two near-identical cancel-token modules in napi

**Claim.** `cancellation_token.rs` (NAPI class wrapping `CoreCancellationToken`, 81 lines) and `cancel_token.rs` (BigInt registry with static `SccHashMap`, 176 lines). Registry exists because JS can't hold `Arc<CoreCancellationToken>` directly, but the JS side already has a `CancellationToken` class.

**Source.** Code quality agent.

### F32. Module-level persist maps in TS keyed by `actorId`

**Claim.** `rivetkit-typescript/packages/rivetkit/src/registry/native.ts:114-149` keeps `nativeSqlDatabases`, `nativeDatabaseClients`, `nativeActorVars`, `nativeDestroyGates`, `nativePersistStateByActorId` as process-global `Map`s keyed on `actorId`. Actor-scoped state kept in file-level globals.

**Source.** Code quality agent.

### F33. `request_save` silently degrades error to warn

**Claim.** `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs:140-144` catches "lifecycle channel overloaded" error and only `tracing::warn!`s. Required lifecycle path returns `Ok(())` semantics for failed save request.

**Source.** Code quality agent.

### F34. `ActorContext.key` type widened silently

**Claim.** Ref `actor/contexts/base/actor.ts:208` returned `ActorKey = z.array(z.string())`. Current `rivetkit-typescript/packages/rivetkit/src/actor/config.ts:290` declares `readonly key: Array<string | number>`. Queries still expect `string[]` in `client/query.ts`.

**Source.** API parity agent.

### F35. `ActorContext` gained `sql` without dropping `db`

**Claim.** `rivetkit-typescript/packages/rivetkit/src/actor/config.ts:284` adds `readonly sql: ActorSql`. Previously `sql` was not on ctx. `./db` subpath is dropped but `db` property remains without deprecation.

**Source.** API parity agent.

### F36. Removed ~20 root exports with no migration path

**Claim.** Compared to ref, `actor/mod.ts` current lost: `PATH_CONNECT`, `PATH_WEBSOCKET_PREFIX`, `ActorKv` (class → interface), `ActorInstance` (class removed), `ActorRouter`, `createActorRouter`, `routeWebSocket`, `KV_KEYS`, and all `*ContextOf` type helpers except `ActorContextOf`.

**Source.** API parity agent.

---

## Low-priority

### F37. `std::sync::Mutex` in test harness

**Claim.** `rivetkit-rust/packages/rivetkit-core/tests/modules/context.rs:303, 327, 329, 371-373` uses `std::sync::Mutex` for HashMaps of live tunnel requests, actors, pending hibernation restores. Shared harness.

**Source.** Code quality agent.

### F38. Inline `use` inside function body

**Claim.** `rivetkit-rust/packages/rivetkit-core/src/registry/http.rs:1003` has `use vbare::OwnedVersionedData;` inside a `#[test] fn`. CLAUDE.md says top-of-file imports only.

**Source.** Code quality agent.

### F39. No `antiox` usage

**Claim.** CLAUDE.md says use `antiox` for TS concurrency primitives. `rivetkit-typescript/packages/rivetkit/src/actor/utils.ts:65-85` implements `class Lock<T>` by hand with `_waiting: Array<() => void>` FIFO. No file in `rivetkit-typescript/packages/rivetkit/src/` imports `antiox`.

**Source.** Code quality agent.

### F40. `napi_actor_events.rs` is 2227 lines

**Claim.** ~320-line `dispatch_event` match with 11 repetitive arms using `spawn_reply(tasks, abort.clone(), reply, async move { ... })` scaffold.

**Source.** Code quality agent.
