# rivetkit-core / napi / typescript Adversarial Review — Synthesis

Findings consolidated from 5 original review agents (API parity, SQLite v2 soundness, test quality, lifecycle conformance, code quality) plus 3 spec-review agents on the proposed shutdown redesign. Each finding has been challenged by a follow-up verification pass; verdicts annotated inline. Each finding now ends with a **Desired behavior** section describing what the fix should be.

## Layer glossary

Used throughout to disambiguate claims:

- **core** = `rivetkit-rust/packages/rivetkit-core/` — Rust lifecycle / state / dispatch state machine. Owns `ActorTask`, `ActorContext`, `run_handle: Option<JoinHandle>`, `SleepState`, lifecycle events, grace timers, counters.
- **napi** = `rivetkit-typescript/packages/rivetkit-napi/` — Rust NAPI bindings between core and the JS runtime. Owns `ActorContextShared` (JS-callable ctx), event-loop task that processes `ActorEvent` and dispatches to JS callbacks, cancel-token registry.
- **typescript** = `rivetkit-typescript/packages/rivetkit/` — TypeScript runtime consumed by user code. Owns user-defined callbacks (`run`, `onSleep`, `onDestroy`, `onDisconnect`, `onStateChange`, `serializeState`), Zod validation, `AbortSignal` surface (`c.abortSignal`, `c.aborted`), workflow engine, client library, `@rivetkit/actor` API shape.
- **engine** = `engine/packages/*` — orchestrator (pegboard-envoy, sqlite-storage, actor2 workflow, etc). Outside the three layers above; referenced when findings touch SQLite v2 migration or engine-side state.

User-defined lifecycle hooks (`run`, `onSleep`, `onDestroy`, `onDisconnect`, `onStateChange`, `serializeState`) are **defined in typescript** as user code. They are **dispatched from core** via `ActorEvent` messages (e.g. `ActorEvent::RunGracefulCleanup`) that traverse the napi event channel and trigger the napi adapter to call the typescript callback. Lifecycle accounting (when to fire, when to consider complete) lives in **core**.

## Challenger verdict legend

- **REAL** — verified on current branch.
- **REAL (narrow)** — verified but bounded by mitigating mechanism.
- **INTENTIONAL** — verified but the behavior is deliberate, not a bug.
- **UNCERTAIN** — factually present but the "bug" framing is debatable.

## Architectural invariants (important context)

1. **One Stop command per actor generation.** The engine's actor2 workflow sends exactly one `Stop` command per generation — either `Sleep` or `Destroy`, never both, never multiples. Any "concurrent Stop" or "Stop upgrade" scenario is unreachable by construction.
2. **One actor instance running, cluster-wide.** At any moment, exactly one physical copy of an actor is running across the entire cluster. Failover transitions ownership atomically via engine assignment. Any finding that depends on "two envoys running the same actor concurrently" is infeasible under this invariant.

These invariants narrow several findings below.

---

## Blockers

### F3. User's `run` cleanly exits → the one engine Stop no-ops → `onSleep`/`onDestroy` never fire — REAL

**Layer.** user `run` and `onSleep`/`onDestroy` are **typescript** callbacks; lifecycle-state transitions happen in **core**; dispatch of the callbacks is via **napi** receiving `ActorEvent`s from core.

**Claim.** If user's typescript `run` handler returns cleanly before the (sole, guaranteed-to-arrive) Stop command arrives, core transitions to `Terminated` in `handle_run_handle_outcome` (`task.rs:1303-1328`), and `begin_stop` on `Terminated` replies `Ok` without emitting grace events (`task.rs:773-776`). The one Stop per generation lands on a dead lifecycle. Hooks never dispatch.

**Verdict.** Real.

**User impact.** A typescript actor whose `run` naturally completes (e.g. a task-tree that finishes) never gets its user `onSleep` / `onDestroy` hook invoked, even though the engine does send exactly one Stop.

**Desired behavior.** Because exactly one Stop arrives per generation, the correct fix is: clean `run` exit while `Started` must **not** transition to `Terminated`. Instead, stay in a waiting substate (or `Started`) until the Stop arrives. When the Stop arrives, `begin_stop` enters `SleepGrace`/`DestroyGrace` and hooks fire via the normal grace path. `Terminated` should mean "lifecycle fully complete, including hooks." Invariant to enforce: `onSleep` or `onDestroy` fires exactly once per generation, regardless of how `run` returned.

---

## High-priority

### F7. `prepare_v1_migration` resets generation to 1 on every call — REAL (negligible)

**Layer.** **engine** — `sqlite-storage`.

**Background for reviewers.** `generation` is sqlite-storage's optimistic concurrency fence for commits. Under the "one actor instance cluster-wide" invariant, there are no concurrent writers, so the fence is protecting against stale retries within a single process, not against two processes.

**Claim.** `engine/packages/sqlite-storage/src/takeover.rs:99` hardcodes `generation: 1` when preparing a v1 migration.

**Verdict.** Real but negligible. Under the one-instance invariant, `prepare_v1_migration` is always the first v2 write for the actor, so `generation: 1` is correct. There's no concurrent writer that could hold a higher generation and get rewound.

**Desired behavior.** Leave as-is. The "preserve existing generation" hardening is defense-in-depth against a scenario that can't happen under current architectural guarantees.

### F8. Truncate leaks PIDX + DELTA entries above new EOF — REAL

**Layer.** **engine** — shared by `rivetkit-sqlite` (VFS host) and `sqlite-storage` (KV backend).

**Claim.** `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs:1403-1413` `truncate_main_file` updates `state.db_size_pages` but doesn't delete entries for `pgno > new_size`. `engine/packages/sqlite-storage/src/commit.rs:222` sets the new size; `takeover.rs:258-269` `build_recovery_plan` ignores `pgno`. Compaction (`compaction/shard.rs`) folds stale pages into shards rather than freeing them.

**Verdict.** Real, unmitigated.

**User impact.** Every `VACUUM`/`DROP TABLE` shrink permanently leaks KV space in the actor's sqlite subspace. Billable against actor storage quota (`sqlite_storage_used` never decrements for the leaked pages).

**Desired behavior.** The commit path should enumerate and delete all `pidx_delta_*` and `pidx_shard_*` entries for `pgno >= new_db_size_pages` whenever `db_size_pages` shrinks. `build_recovery_plan` should also filter orphan entries by `pgno >= head.db_size_pages`. `sqlite_storage_used` must decrement to reflect freed space. Compaction should reclaim (delete) truncated pages, not fold them into shards.

### F9. V1 KV data never cleaned up after successful migration — INTENTIONAL

**Layer.** **engine** — `pegboard-envoy`.

**Claim.** `engine/packages/pegboard-envoy/src/sqlite_runtime.rs:124-250` (`maybe_migrate_v1_to_v2`) never calls `actor_kv::delete_all`, `delete_range`, or similar on the `0x08` prefix after finalize. `engine/packages/pegboard/src/actor_kv/mod.rs:497` has the `delete_all` helper but it's not wired in.

**Verdict.** Factually accurate, but the behavior is **intentional**. V1 data is preserved after migration as a safety net in case migration corruption is detected after the fact — operators can fall back to the v1 bytes. A future version may add opt-in cleanup once the v2 path has soaked long enough to trust.

**User impact.** Post-migration storage is roughly doubled for the affected actors. Trade-off is deliberate: storage cost vs. corruption-recovery safety net.

**Desired behavior.** No change now. Future work: once v2 has soaked long enough to trust, add an opt-in retention policy (config flag or per-actor age threshold) that triggers `delete_all` on the `0x08` prefix for actors whose v2 meta has been stable for N days / writes. Keep the current behavior as the default until then.

### F10. 5-minute migration lease blocks crash-recovery restart — REAL (narrow)

**Layer.** **engine** — `pegboard-envoy`.

**Background for reviewers.** The migration lease is a wall-clock fence that says "the actor cannot restart a v1→v2 migration for N minutes after the last stage was begun." Under the "one actor instance cluster-wide" invariant, the lease is not protecting against concurrent migrations (impossible by construction); it's a conservative "the prior attempt probably didn't finish cleanly, wait before retrying" timeout after a crash.

**Claim.** `engine/packages/pegboard-envoy/src/sqlite_runtime.rs:34` `SQLITE_V1_MIGRATION_LEASE_MS = 5 * 60 * 1000`. If the owning envoy crashes between `commit_stage_begin` and `commit_finalize`, the new owner's restart within 5 min is rejected.

**Verdict.** Real but narrow. Requires crash during the stage window (typically milliseconds to a few seconds for 128 MiB actors).

**User impact.** Affected actors are non-startable for up to 5 min after a rare envoy crash. No data loss — once the lease expires, migration restarts from scratch.

**Desired behavior.** Shorten the lease to reflect real stage-window duration (30-60s), *and* add a production path (not test-only) to invalidate the stale in-progress marker when a new engine `Allocate` assigns the actor. Since only one instance runs cluster-wide, a fresh `Allocate` is authoritative evidence the prior attempt is dead — no need to wait for a wall-clock timeout.

### F12. `Registry.handler()` / `Registry.serve()` throw at runtime — REAL

**Layer.** **typescript** — `@rivetkit/actor` package surface.

**Claim.** `rivetkit-typescript/packages/rivetkit/src/registry/index.ts:75-95` throws `removedLegacyRoutingError`. Reference branch (`feat/sqlite-vfs-v2`) returned a real `Response`.

**Verdict.** Real. Intentional per commit `US-035`; error message names replacement (`Registry.startEnvoy()`).

**User impact.** `export default registry.serve()` / `registry.handler(c.req.raw)` in user typescript code throws on first request. No type-level signal.

**Desired behavior.** A custom routing layer is needed to replace the old `handler`/`serve` surface. Until that lands, add `@deprecated` jsdoc annotations pointing at `Registry.startEnvoy()` so users see a compile-time / editor warning before hitting the runtime throw. Document the removal in CHANGELOG.md with a migration example. The custom routing layer is the real long-term fix; the deprecation shim is a stopgap during the gap.

### F13. ~48 typed error classes removed from typescript `./errors` subpath — INTENTIONAL

**Layer.** **typescript**.

**Claim.** `git show feat/sqlite-vfs-v2:rivetkit-typescript/packages/rivetkit/src/actor/errors.ts` exported 48 classes (`QueueFull`, `ActionTimedOut`, etc.). Current `actor/errors.ts` exports only `RivetError`, `UserError`, alias `ActorError = RivetError`, plus 7 factory helpers.

**Verdict.** Factually accurate, but the behavior is **intentional**. The new design has no concrete error classes; users discriminate via `group`/`code` on `RivetError` using helpers like `isRivetErrorCode(e, "queue", "full")`. The collapse was deliberate.

**User impact.** User code doing `catch (e) { if (e instanceof QueueFull) … }` breaks. Migration is one-line per site: replace with `isRivetErrorCode(e, "queue", "full")`.

**Desired behavior.** No restoration. Document the migration in CHANGELOG.md so users have a clear path. Include the most common `group`/`code` pairs in the migration guide.

### F14. typescript package `exports` subpaths removed — REAL (split)

**Layer.** **typescript** — `@rivetkit/actor` package.json exports map.

**Claim.** `rivetkit-typescript/packages/rivetkit/package.json` dropped `./dynamic`, `./driver-helpers`, `./test`, `./inspector`, `./db`, `./db/drizzle`, `./sandbox/*` and more. Reference had all of these.

**Verdict.** Real. Per commits `US-035`, `US-036`: deliberate feature-surface deletions.

**Desired behavior.** Split decision:

- **Accepted removals (keep gone):** `./dynamic`, `./sandbox/*`. These feature surfaces are not coming back. Document in CHANGELOG.
- **Evaluate per subpath:** `./driver-helpers`, `./test`, `./inspector`, `./db`, `./db/drizzle`, `./topologies/*`, `./driver-helpers/websocket`. For each, decide whether it makes sense to restore given the post-rewrite architecture. If yes, bring back. If no, document in CHANGELOG why and provide a migration note.

Don't do blanket restoration; evaluate each removed subpath against the current architecture and restore only the ones that still make sense.

### F18. Actor-ready / actor-started state lives in napi, not core — REAL

**Layer.** layer violation — **core** vs **napi**.

**Claim.** Core's `SleepState::ready` and `SleepState::started` AtomicBools (`sleep.rs:39-40`) already feed `can_arm_sleep_timer`. napi *also* has its own `ready`/`started` AtomicBools on `ActorContextShared` (`actor_context.rs:68-69`) with parallel `mark_ready`/`mark_started` logic — including a "cannot start before ready" precondition (`:783-794`). The two are not wired to each other.

**Verdict.** Real. Duplicate state machine. A future V8 runtime would need to reimplement napi's version and separately coordinate with core's.

**Desired behavior.** Deduplicate the state without changing core semantics. Make napi's `ready`/`started` accessors read through to core's existing `SleepState::ready`/`started`. napi's `mark_ready`/`mark_started` become thin forwarders to core's setters. **Do not alter core's current semantics or gating behavior** — this is a pure refactor to remove the parallel copy, not an opportunity to change when/how readiness flips. If the "cannot start before ready" precondition exists in napi for a JS-side ordering reason (TSF callback registration, etc.), keep it on the napi side as a precondition check, still forwarding the state read to core. Net: one source of truth (core), napi is transport; no behavior change.

### F19. Inspector logic duplicated in typescript — REAL

**Layer.** layer violation — **typescript** duplicates **core**.

**Claim.** `rivetkit-typescript/packages/rivetkit/src/inspector/actor-inspector.ts:141-475` implements `patchState`, `executeAction`, `getQueueStatus`, `getDatabaseSchema` in typescript. Core's `registry/inspector.rs:385` + `inspector_ws.rs:222, 369` handle the same surface.

**Verdict.** Real. Two parallel implementations of inspector state patching, action execution, queue introspection, schema introspection.

**Desired behavior.** Move **all** inspector logic into core. After the move, there should be **nothing left** for the inspector in the typescript layer — no `ActorInspector` class, no `patchState`/`executeAction`/`getQueueStatus`/`getDatabaseSchema` implementations. Inspector is entirely core-owned. If any TS-specific concerns exist (e.g., user-schema-aware state patching), resolve them by having core call back into TS for the narrow piece that needs user schemas, not by leaving a parallel TS implementation.

---

## Medium-priority

### F21. 50 ms polling loop in typescript `native.ts` — REAL

**Layer.** **typescript** (`registry/native.ts`) with the intended fix in **napi**.

**Claim.** typescript `native.ts:2405-2415` uses `setInterval(..., 50)` to poll `#isDispatchCancelled`. napi already has a TSF `on_cancelled` callback (`cancellation_token.rs:47-73`) that should replace the poll.

**Verdict.** Real. typescript uses the BigInt-registry version of the cancel token (`cancel_token.rs`) instead of the NAPI class with TSF callbacks — related to F31.

**Desired behavior.** Delete the `setInterval` poll. Subscribe to napi's `on_cancelled` TSF callback via the NAPI `CancellationToken` class. Dispatch cancellation becomes event-driven: napi invokes the JS callback exactly once when the token is cancelled, typescript awakens, no polling. Tied to F31's consolidation — once the TSF-callback version is canonical, the BigInt registry and its polling consumer both disappear.

### F22. `vi.spyOn(...).mockImplementation/mockResolvedValue` in typescript tests — REAL (with caveat)

**Layer.** **typescript** — test code.

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/registry-constructor.test.ts:30-32, :52` uses `vi.spyOn(Runtime, "create").mockResolvedValue(createMockRuntime())`. `packages/traces/tests/traces.test.ts:184-187, :365` spies on `Date.now` and `console.warn` with `mockImplementation`.

**Verdict.** Real. CLAUDE.md bans `vi.mock`/`vi.doMock`/`jest.mock` explicitly and allows `vi.fn()` for callback tracking. `vi.spyOn` with `mockImplementation` replaces implementation — violates the "real infrastructure" spirit. `Runtime.create` swap is the clearer violation; `Date.now` is more defensible for time tests.

**Desired behavior.** Rewrite `registry-constructor.test.ts` to use a real `Runtime` built via a test-infrastructure helper (same pattern as driver-test-suite). Delete the `Runtime.create` spy. For time-dependent tests, replace `vi.spyOn(Date, "now")` with `vi.useFakeTimers()` + `vi.setSystemTime()` — vitest's built-in deterministic clock. `console.warn` silencing is acceptable as a test-hygiene measure; keep it.

### F23. `createMockNativeContext` fakes the whole napi surface in typescript tests — REAL

**Layer.** **typescript** — tests fake the **napi** boundary.

**Claim.** `rivetkit-typescript/packages/rivetkit/tests/native-save-state.test.ts:14-59` builds full fake `NativeActorContext` via `vi.fn()` for 10+ methods, cast as `unknown as NativeActorContext`. Never exercises real napi.

**Verdict.** Real.

**Desired behavior.** Delete `createMockNativeContext`. Move the save-state test coverage into the driver-test-suite (`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`) so it runs against real napi + real core. If the specific logic being tested is a pure typescript adapter transformation independent of napi, refactor to extract that logic into a pure function and unit-test the function directly without needing a `NativeActorContext` at all.

### F24. `expect(true).toBe(true)` sentinel after race iterations in typescript tests — REAL

**Layer.** **typescript** test.

**Claim.** `tests/driver/actor-lifecycle.test.ts:118` asserts `expect(true).toBe(true)` after 10 create/destroy iterations with comment "If we get here without errors, the race condition is handled correctly."

**Verdict.** Real. Test has no real assertion; race could be broken and test would pass.

**Desired behavior.** Replace with a concrete observable assertion. Options: (a) count successful destroy callbacks (`expect(destroyCount).toBe(10)`), (b) capture all thrown exceptions across iterations and assert `expect(errors).toEqual([])`, (c) track the final actor state and assert cleanup completed. Whatever invariant the test is actually supposed to verify — encode it.

### F25. 10 skipped tests in typescript `actor-sleep-db.test.ts` without tracking — REAL

**Layer.** **typescript** tests.

**Claim.** `tests/driver/actor-sleep-db.test.ts:219, 260, 292, 375, 522, 572, 617, 739, 895, 976` have `test.skip` on shutdown-lifecycle invariants. 9 of 10 have no TODO/issue reference.

**Verdict.** Real.

**Desired behavior.** For each of the 10 skipped tests: either root-cause the underlying ordering/race issue and un-skip, or file a tracking ticket and annotate the skip with the ticket ID in a comment (e.g. `test.skip("...", /* TODO(RVT-123): task-model shutdown ordering race */ ...)`). Unannotated `test.skip` should not pass code review. Once policy is established, add a lint rule or CI check that rejects bare `test.skip`.

### F26. `test.skip("onDestroy is called even when actor is destroyed during start")` — REAL

**Layer.** **typescript** test; verifies an invariant over user `onDestroy` (typescript callback) scheduling by core.

**Claim.** `tests/driver/actor-lifecycle.test.ts:196` (not `:142` as originally cited).

**Verdict.** Real.

**Desired behavior.** Same as F25 — fix the underlying invariant (core should call `onDestroy` even when destroy arrives during actor start, i.e. the `Loading` lifecycle state should still dispatch `onDestroy`) or file a tracking ticket with the skip comment pointing at it.

### F27. Flake "fixes" paper over races in typescript tests — REAL

**Layer.** **typescript** tests (and notes in `.agent/notes/`).

**Claim.** `.agent/notes/flake-conn-websocket.md:47` proposes "longer wait"; `actor-sleep-db.test.ts:198-208` wraps assertions in `vi.waitFor({ timeout: 5000, interval: 50 })` with no explanation; `driver-test-progress.md:57, :68` notes "passes on retry" with no regression test.

**Verdict.** Real.

**Desired behavior.** Ban retry-loop workarounds for production-path flakes. When a flake is found: (1) root-cause the race, (2) write a deterministic repro using `vi.useFakeTimers()` or event-ordered `Promise` resolution, (3) fix the underlying ordering in core/napi/typescript, (4) delete the flake-workaround note. `vi.waitFor` is acceptable for legitimate "wait for an async event" coordination but never as a retry-until-success masking layer. Every existing `vi.waitFor` call should have a one-line comment explaining why polling rather than direct awaiting is necessary.

### F28. Driver test suites feature-gated off by default — REAL

**Layer.** **typescript** tests, gated on driver feature flags from test harness.

**Claim.** `tests/driver/hibernatable-websocket-protocol.test.ts:140` `describe.skipIf(!driverTestConfig.features?.hibernatableWebSocketProtocol)` → all 6 tests skipped in default driver. Likely other suites are similarly gated.

**Verdict.** Real.

**Desired behavior.** Compare driver test-feature flags against `feat/sqlite-vfs-v2`: any test suite that was enabled there should be enabled now. Audit the driver test config on both branches and re-enable every suite that was running on the reference branch. Zero runtime coverage regressions from the rewrite.

### F30. Plain `Error` thrown in typescript `native.ts` on required path — REAL

**Layer.** **typescript**.

**Claim.** `registry/native.ts:2654` throws `new Error("native actor client is not configured")` instead of `RivetError`.

**Verdict.** Real. CLAUDE.md says errors at boundaries must be `RivetError`.

**Desired behavior.** Replace with a `RivetError` using an appropriate group/code, e.g. `throw new RivetError("native", "not_configured", "native actor client is not configured")`. Audit `native.ts` for other `new Error(...)` throws on required paths and fix them all at once.

### F31. Two cancel-token modules in napi — REAL (subjective)

**Layer.** **napi**.

**Claim.** Both `cancellation_token.rs` (82-line NAPI class with TSF `on_cancelled` callback) and `cancel_token.rs` (BigInt-keyed `SccHashMap` registry) exist and serve different call patterns.

**Verdict.** Real. Consolidation is a judgment call but the duplication is factual. Tied to F21 — typescript picks the wrong one for the dispatch-cancel path.

**Desired behavior.** Canonical module is `cancellation_token.rs` (NAPI class + TSF `on_cancelled` callback). Migrate typescript's dispatch-cancel path (`native.ts:2405`) to use the NAPI class directly — this also fixes F21. Once no typescript code uses the BigInt-registry pattern, delete `cancel_token.rs` entirely. One cancel-token concept per actor, event-driven.

### F32. Module-level actor-keyed maps in typescript `native.ts` — REAL

**Layer.** **typescript** — file-level process-global state.

**Claim.** `registry/native.ts:114-149` declares `nativeSqlDatabases`, `nativeDatabaseClients`, `nativeActorVars`, `nativeDestroyGates`, `nativePersistStateByActorId` as `new Map<string, ...>` keyed on `actorId`.

**Verdict.** Real. Actor-scoped state lives on file-level globals instead of on the actor context.

**Desired behavior.** Take the cleanest approach at whichever layer fits best. If there's a natural per-actor object in typescript to hang the state on, move it there. If the cleanest solution is to move the accounting into core (and have napi expose it via the ctx), do that instead. The goal is to eliminate the actorId-keyed module-global maps; the right destination is whatever produces the simplest lifecycle management with the least cross-layer plumbing.

### F33. Core's `request_save` silently degrades error to `warn!` — UNCERTAIN

**Layer.** **core**.

**Claim.** `state.rs:141-145` catches "lifecycle channel overloaded" and only `tracing::warn!`s. Required lifecycle path returns `Ok(())` semantics for a failed save request.

**Verdict.** Uncertain. Public signature is `fn request_save(&self, opts) -> ()` — no Result. All callers use fire-and-forget. The `request_save_and_wait` variant returns `Result<()>`. If fire-and-forget is the design choice, warn-and-continue is consistent. If not, the API itself needs a return type change.

**Desired behavior.** Decide intent in a doc-comment on `request_save`. Two options: (a) Confirm fire-and-forget is intended: document explicitly that callers do not handle overload, that `warn!` is the sole signal, and that `request_save_and_wait` is the error-aware alternative. (b) Reject fire-and-forget: change signature to `fn request_save(&self, opts) -> Result<()>` and propagate the overload error. Callers update to handle or explicitly `.ok()`. Do not leave the current ambiguous state.

### F34. typescript `ActorContext.key` type widened to `(string | number)[]` — REAL

**Layer.** **typescript**.

**Claim.** `actor/config.ts:289` declares `readonly key: Array<string | number>`. Reference was `string[]`. `client/query.ts:15-17` still declares `ActorKeySchema = z.array(z.string())`.

**Verdict.** Real. Latent type inconsistency between typescript ctx surface and typescript query schema — a number-containing key cannot round-trip through the query path. Likely unintentional.

**Desired behavior.** Narrow `key` back to `readonly key: string[]` to match `ActorKeySchema`. If numeric keys are intentionally supported end-to-end, widen `ActorKeySchema = z.array(z.union([z.string(), z.number()]))` and audit every consumer of `ActorKey` for numeric-safety. Fix the inconsistency one direction or the other; don't leave `key` wider than what can round-trip.

### F35. typescript `ActorContext` gained `sql` but `./db` subpath dropped — REAL

**Layer.** **typescript**.

**Claim.** `actor/config.ts:283-284` has `readonly sql: ActorSql; readonly db: InferDatabaseClient<TDatabase>;`. Reference had only `db`. `./db/drizzle` package export is gone.

**Verdict.** Real. `db` is dead surface (no drizzle provider path); `sql` is new surface.

**Desired behavior.** Keep the old exports surface. Remove `sql` from `ActorContext`, restore `./db/drizzle` subpath as the way users configure the drizzle backing driver. `db` remains the typed drizzle client on ctx — no dual API.

### F36. ~20 root exports removed from typescript `actor/mod.ts` — REAL (split)

**Layer.** **typescript**.

**Claim.** Reference exported `PATH_CONNECT`, `PATH_WEBSOCKET_PREFIX`, `ActorKv`, `KV_KEYS`, `ActorInstance`, `ActorRouter`, `createActorRouter`, `routeWebSocket`, all `*ContextOf` type helpers. Current exports none (39-line `actor/mod.ts`). `actor/contexts/index.ts` directory removed entirely.

**Verdict.** Real per commits `US-038`, `US-040`.

**Desired behavior.** Split decision:

- **Keep removed** (no longer relevant in post-rewrite architecture): `PATH_CONNECT`, `PATH_WEBSOCKET_PREFIX`, `KV_KEYS`, `ActorKv`, `ActorInstance`, `ActorRouter`, `createActorRouter`, `routeWebSocket`. These were tied to the old routing/kv surfaces that don't exist anymore. Document in CHANGELOG that they're gone permanently.
- **Restore**: all `*ContextOf` type helpers (`ActionContextOf`, `ConnContextOf`, `CreateContextOf`, `SleepContextOf`, `DestroyContextOf`, `WakeContextOf`, etc.). These are user-facing type utilities with zero runtime cost; dropping them breaks `type MyCtx = ActionContextOf<typeof actor>` patterns for no architectural reason. Recreate `actor/contexts/index.ts` (or equivalent) as a type-only module.

Update `rivetkit-typescript/CLAUDE.md` to either restore the sync rule (if `actor/contexts/index.ts` comes back) or remove the stale reference.

---

## Low-priority

### F38. Inline `use` inside function body in core — REAL

**Layer.** **core**.

**Claim.** `rivetkit-core/src/registry/http.rs:1003` has `use vbare::OwnedVersionedData;` inside a `#[test] fn`.

**Verdict.** Real. CLAUDE.md: imports must be at top of file.

**Desired behavior.** Move `use vbare::OwnedVersionedData;` to the top of `http.rs`'s test module (`#[cfg(test)] mod tests { use …; }`).

### F39. No `antiox` usage in typescript — RESOLVED (rule retired)

**Layer.** **typescript**.

**Claim.** Zero `antiox` imports; hand-rolled primitives like `Lock<T>` (`utils.ts:65`) in use.

**Verdict.** Resolved by retiring the rule.

**Desired behavior.** CLAUDE.md's "TypeScript Concurrency" section has been removed. If any speculative `antiox` imports were added in anticipation of the rule, remove them. Existing hand-rolled primitives stay as-is.

### F41. Dead BARE code in typescript — AUDIT TASK

**Layer.** **typescript**.

**Claim.** Post-rewrite, typescript may have BARE-protocol code that's no longer exercised by any current caller.

**Verdict.** User-reported; concrete dead surface not yet enumerated.

**Desired behavior.** The task is the audit itself. Enumerate every BARE type / codec / helper in `rivetkit-typescript/packages/`, trace each to confirm it has a live caller, and record the list of dead symbols. Do not delete as part of the audit; produce the list of candidates for removal first. Removal is a follow-up decision.

### F42. Rust inline `#[cfg(test)] mod tests` blocks in `src/` — NEW POLICY

**Layer.** **core** and **napi** (scope limited; other engine crates not in scope).

**Claim.** The project convention — now added to CLAUDE.md — is that Rust tests live under each crate's `tests/` directory, not inline inside `src/*.rs` files.

**Desired behavior.** Audit `rivetkit-rust/packages/rivetkit-core/` and `rivetkit-typescript/packages/rivetkit-napi/` for inline `#[cfg(test)] mod tests` blocks. Move each to `tests/<module>.rs`. Exceptions (e.g., testing a private internal that can't be reached from an integration test) must have a one-line comment justifying staying inline. CLAUDE.md rule added at `CLAUDE.md:196`. Other engine crates are out of scope for this pass.

---

## Tally

- **Real**: 23 (F3, F7, F8, F10, F12, F14, F18, F19, F21–F28, F30–F32, F34–F36, F38, F39, F41, F42)
- **Real but narrow / negligible**: F7, F10 (2)
- **Intentional (not a bug)**: F9, F13 (2)
- **Uncertain**: F33 (1)
- **Removed**: F1, F2, F4, F5, F6, F11, F15, F16, F17, F20, F29, F37, F40

## Root-cause note

The removed bullshit findings (F1, F2, F5, F16, F17, F20, F29, F40) clustered in lifecycle/core and napi, and mostly relied on stale code citations from before commits `US-067, US-103, US-104, US-105, US-109, US-110` reshaped core's `ActorTask`, `run_shutdown`, grace machinery, and the abort-signal wiring. Future reviews of core should verify citations against the current `task.rs` rather than trusting pre-refactor review output.
