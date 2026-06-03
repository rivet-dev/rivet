# ActorTask Shutdown / Lifecycle / State Save Review

Three-agent review (2026-04-22) of `ActorTask::run`, `run_live`, `run_shutdown`, and state save interactions after US-105 (commit `1fbaf973b`) collapsed the boxed-future shutdown state machine into an inline async function.

- Files reviewed:
  - `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs`
  - `rivetkit-rust/packages/rivetkit-core/src/actor/context.rs`
  - `rivetkit-rust/packages/rivetkit-core/src/actor/state.rs`
  - `rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs`
  - `rivetkit-rust/packages/rivetkit-core/src/actor/connection.rs`
- Docs checked: `website/src/content/docs/actors/lifecycle.mdx`, `docs-internal/engine/rivetkit-core-state-management.md`, `docs-internal/engine/rivetkit-core-internals.md`, `rivetkit-rust/packages/rivetkit-core/CLAUDE.md`.

Each issue has a **Status** field: `unverified` until an adversarial agent confirms or refutes. Retracted items remain with a note explaining why.

---

## F-1 — Self-initiated shutdown bypasses `run_shutdown` (RETRACTED in common case, residual race)

**Status:** SHIPPED in `d61ce3144` (2026-04-22, US-109). `handle_run_handle_outcome` now returns `LiveExit::Shutdown` for self-requested sleep/destroy so `run_shutdown` runs without waiting for an inbound Stop. Covered by core self-initiated sleep/destroy tests plus TS driver regressions for `run` handlers that call `c.sleep()`/`c.destroy()` and return.

**Claim:** When `ctx.sleep()`/`ctx.destroy()` is called, the code transitions directly to SleepFinalize/Destroying via `handle_run_handle_outcome` and never runs `run_shutdown`, silently skipping disconnect/save/hooks.

**Why partially retracted:** `ctx.sleep()` (`context.rs:418-438`) only sets a flag and notifies envoy; it returns immediately. Envoy sends `Stop(Sleep)` back, which goes through `begin_stop(Sleep, Started)` → SleepGrace → `LiveExit::Shutdown { Sleep }` → `run_shutdown`. Same for destroy. So the normal path is correct.

**Residual concern:** If the user's run closure *returns on its own* (or panics) before envoy round-trips the Stop, `handle_run_handle_outcome` (`task.rs:1314-1346`) sets lifecycle to SleepFinalize/Destroying. `should_terminate()` (`task.rs:2115`) only matches Terminated, so `run_live` keeps spinning. When `Stop` later arrives, `begin_stop`'s `SleepFinalize | Destroying` arm (`task.rs:794-803`) acks `Ok(())` without running `run_shutdown`.

**Adversary task:** Prove or refute whether the race is reachable from real user code. Look at the TS bridge (`rivetkit-typescript/packages/rivetkit/src/registry/native.ts`) and the foreign-runtime `run` handler contract. Can a user's run closure return on its own before shutdown? What do tests do?

---

## F-2 — `run_live` is not wrapped in `catch_unwind`

**Status:** PARTIAL (2026-04-22, adversary A). Claim is narrowly correct but exposure is small: no `.unwrap()`, `.expect()`, indexing, or `panic!` calls in `run_live`'s handlers. User run closure panics are caught at `spawn_run_handle` (`task.rs:1243-1251`). Remaining panic sources are dependency bugs (tracing, scc, tokio), arithmetic overflow in debug, or OOM. Low-severity defense-in-depth gap; registry caller (`mod.rs:806-808`) propagates `JoinError` but doesn't run cleanup if it fires.

**Claim:** `task.rs:535-556` (`run`) wraps `run_shutdown` and the user factory in `AssertUnwindSafe(...).catch_unwind()` but NOT `run_live`. A panic inside the live loop body (ciborium, `handle_event`, `handle_dispatch`, `on_sleep_tick`, `schedule_state_save`, inspector broadcast) unwinds straight out of `run`. Any pending shutdown reply oneshot is dropped; destroy cleanup, KV flush, `disconnect_all_conns`, `mark_destroy_completed` never run.

**Fix:** wrap `run_live()` in `catch_unwind`; on `Err(_)` synthesize `LiveExit::Shutdown { Destroy }` and a panic error so `run_shutdown` still runs.

**Adversary task:** Prove or refute that a panic in `run_live` is catastrophic. Is something upstream (task spawner, registry) already catching it? Does tokio's JoinHandle propagate panics in a way that doesn't lose cleanup, given the rest of the code assumes completion? Look at where `ActorTask::run` is spawned and what the caller does with `JoinError`.

---

## F-3 — Dirty state can be lost at shutdown

**Status:** REFUTED (2026-04-22, adversary B). TS proxy unconditionally calls `this.#ctx.requestSave({immediate: false})` on every state mutation (`native.ts:2721`). `request_save_with_revision` calls `notify_request_save_hooks` BEFORE the `already_requested` early-return (`state.rs:168`), so the adapter's `on_request_save` hook sets `dirty=true` even for duplicates. Both `handle_sleep_event` and `handle_destroy_event` end with `maybe_shutdown_save` (`napi_actor_events.rs:696-719`) which serializes + saves if dirty. `state_save_deadline = None` at `task.rs:1529` only cancels the deferred tick; actual save happens via `maybe_shutdown_save` regardless. Destroy DOES save (`napi_actor_events.rs:663`). Mutations in `onStop`/`onDestroy` flow through the normal dirty-bit path before the save.

**Claim:** `run_shutdown` clears `state_save_deadline = None` at `task.rs:1528` without first flushing pending saves. Core only emits `save_state(Vec::new())` for hibernatable conns (`task.rs:1749`); the actor-state flush relies entirely on the foreign runtime's `FinalizeSleep`/`Destroy` reply producing deltas. If the runtime adapter no-ops, or the user mutated in `onStop`/`onDestroy` without `request_save`, the mutation is lost. The Destroy path has NO explicit actor-state save at all.

**Fix:** before draining, if `ctx.save_requested()` is true, issue a synchronous `SerializeState { Save }` + `save_state_with_revision` bounded by `deadline`.

**Adversary task:** Prove or refute by tracing the full save path end-to-end, including what the TS runtime adapter does inside `FinalizeSleep`/`Destroy` handlers in `native.ts`. Does the adapter reliably flush user state deltas? Is there a code path I missed? Check the driver-test-suite shutdown tests — do they verify state-after-destroy or only state-after-sleep?

---

## F-4 — `runStopTimeout` is never applied to the run-handle wait

**Status:** SHIPPED in `f2e9167da` (2026-04-22, US-110). `runStopTimeout` now flows from TS actor options through NAPI `JsActorConfig` into core `ActorConfigInput`, and `run_shutdown` applies `effective_run_stop_timeout()` as the per-run-handler join budget bounded by the outer shutdown deadline. Covered by core timeout regression plus TS driver coverage for a run handler that ignores abort.

**Claim:** `run_shutdown`'s run-handle join at `task.rs:1640` uses `remaining_shutdown_budget(deadline)` where `deadline` is `sleepGracePeriod` (Sleep) or `on_destroy_timeout` (Destroy). `lifecycle.mdx:289,825-826` promises `runStopTimeout` controls this.

**Fix:** wire `factory.config().run_stop_timeout` at that join, or remove the option from docs.

**Adversary task:** Check the actual semantics documented. Is `runStopTimeout` meant to be the overall budget (= sleepGracePeriod/on_destroy_timeout) or a separate per-run-handle budget? What does the TS-facing API expose? Is this a naming misunderstanding or a real bug?

---

## F-5 — Accepting dispatch during `SleepGrace` without waking

**Status:** PARTIAL / mostly REFUTED (2026-04-22, adversary C). Engine-level routing is correct: actor2 workflow clears `ConnectableKey` via `SetSleepingInput` (`runtime.rs:841-857`) on `ActorIntentSleep`, so new external dispatches can't reach a sleeping/grace actor. The docs permit pre-existing-connection actions to continue during grace (`lifecycle.mdx:850`). Remaining gap: a successfully-processed action during grace does not re-wake the actor, but this is not claimed by docs either. Not a clear bug. Close without action.

**Claim:** `accepting_dispatch()` (`task.rs:1824`) returns true for `Started | SleepGrace`. `lifecycle.mdx:852` says "New requests that arrive during shutdown are held until the actor wakes up again." Code neither holds nor wakes — it processes the action under SleepGrace and then shuts down anyway. No `SleepGrace → Started` transition exists.

**Fix direction:** wake+cancel the grace, or reject with a retry hint. Today it's silently the worst of both.

**Adversary task:** Is there a wake/cancel mechanism I missed? Does new dispatch activity-notify into `reset_sleep_timer` in a way that effectively aborts grace? What does the engine actor2 workflow do when a Dispatch arrives for an actor it just asked to Sleep? Is the docs language about "held until wakes up" describing a different scenario (full-sleep request routing) rather than grace-window dispatch?

---

## F-6 — `request_save` dropped in "already requested" window

**Status:** REFUTED (2026-04-22, adversary B). Race is closed by two independent revision checks: (1) `finish_save_request` (`state.rs:821-833`) only clears `save_requested` if `save_request_revision.load() == passed_in_revision` — a concurrent `request_save` bumps the revision via `fetch_add` (`state.rs:167`), so equality fails and flag stays true. (2) `on_state_save_tick` (`task.rs:1947`) re-checks `ctx.save_requested()` after save and re-schedules if still true. `state.rs:351-353` also independently guards `state_dirty` against concurrent mutation. Concrete interleaving traced; data is not lost.

**Claim:** `state.rs:199-204` — if a mutation lands between "save tick dispatched" and "save finishes (`finish_save_request`)", `already_requested` is still true so no new `SaveRequested` event is enqueued. Same class of bug as US-098 (workflow dirty-flag ordering), unresolved for actor state.

**Fix:** re-check dirty after clearing `already_requested`, or flip the order (clear flag → enqueue → re-check).

**Adversary task:** Trace the exact sequence. Does `apply_state_deltas` (`state.rs:268`) re-snapshot after the guard? Does the revision check at `state.rs:351-353` cover the mutation-mid-write case? Is the race actually closed by some mechanism I didn't see? Produce a concrete interleaving that loses data, or show why it can't.

---

## F-7 — Destroy during SleepGrace silently ignored

**Status:** REFUTED (2026-04-22, adversary C). Engine actor2 workflow (`mod.rs:990-1042` `Main::Destroy`) explicitly checks `Transition::SleepIntent` and does NOT emit a fresh `CommandStopActor` ("Stop command was already sent" comment). Only `Transition::Running` emits new Stop. Engine guarantees one Stop per actor instance generation. Root CLAUDE.md also labels `pegboard-envoy` as part of the trusted internal boundary. Core's `debug_assert!` + ack Ok is correct. Close without action.

**Claim:** `handle_sleep_grace_lifecycle` (`task.rs:694-706`) hits `debug_assert!(false)` + ack Ok. In production (asserts off), a client `destroy()` during the grace window never takes effect on that instance. Matches the engine's one-Stop contract but engine↔pegboard-envoy is an untrusted boundary per root `CLAUDE.md`.

**Fix direction:** either escalate (abort grace → `LiveExit::Shutdown { Destroy }`) or explicitly document the "engine must not send Destroy during grace" invariant in both code and spec.

**Adversary task:** Check the engine pegboard-envoy code for actor2. Does it guarantee one-Stop-per-instance? Can a client `destroy()` call arrive at pegboard-envoy for an actor already in sleep grace? If so, how does envoy route it? Prove or refute "untrusted boundary means defense-in-depth is needed here" vs. "envoy normalizes this, core is fine trusting it."

---

## F-8 — `transition_to` has no source-state guard

**Status:** REFUTED (2026-04-22, adversary D). `handle_run_handle_outcome` checks `destroy_requested` first (`task.rs:1334-1342`), so destroy always wins if both flags are set. Engine then sends exactly one Stop (Destroy). Final state is well-defined: destroy path supersedes sleep. `debug_assert!` would be defensive but existing logic is correct. Close without action.

**Claim:** `task.rs:2128-2148` accepts any transition including `Terminated → Started`. Combined with `handle_run_handle_outcome` reading `destroy_requested`/`sleep_requested` atomic flags non-atomically (`task.rs:1314-1346`), simultaneous `ctx.sleep()` + `ctx.destroy()` before the run handle exits has no assertion protecting against bad state writes.

**Fix:** add an allow-list `debug_assert!`.

**Adversary task:** Can the flags actually be set in parallel in practice? `ctx.sleep()` and `ctx.destroy()` aren't `&mut self` — can user code race them? Even if they can, does the current logic produce a valid final state? Is there already a guard I didn't see (e.g., first call wins)?

---

## F-9 — Destroy has no `BeginDestroy` pre-event

**Status:** REFUTED (2026-04-22, adversary D). `lifecycle.mdx:846` describes user-visible shutdown steps, not internal `ActorEvent::BeginSleep`/`BeginDestroy` signaling. `BeginSleep` is an internal inspector event, not a lifecycle hook. Destroy's equivalent signal is `abort_signal.cancel()` in `mark_destroy_requested` (`context.rs:467`), which is stronger than Sleep's begin event. No drift. Close without action.

**Claim:** Sleep emits `ActorEvent::BeginSleep` via `request_begin_sleep()` at `task.rs:2150-2163` when grace starts. Destroy has no symmetric "grace began, actor still functional" notification even though `lifecycle.mdx:846` claims parity.

**Fix direction:** either emit `BeginDestroy` or clarify the docs.

**Adversary task:** Read `lifecycle.mdx:846` and surrounding context. Does the docs actually claim parity, or does it clearly distinguish? Does the TS runtime need a `BeginDestroy` signal, or does it already handle this via other means (e.g., the `abort_signal` cancellation in `mark_destroy_requested`)?

---

## F-10 — `handle_stop` is a second, test-only state machine

**Status:** PARTIAL (2026-04-22, adversary D). Not dead — 16+ test call sites at `tests/modules/task.rs:740..3446` exercise `run_shutdown` + `deliver_shutdown_reply` + terminate transitions in isolation. Divergence from `run`/`run_live` (spins `poll_sleep_grace` inline rather than via `select!`) is a real maintenance risk. Test infrastructure, not production dead code. Consider helper consolidation.

**Claim:** `task.rs:716` with `#[cfg_attr(not(test), allow(dead_code))]` drives SleepGrace by spinning `poll_sleep_grace` inline, diverging from `run_live`. Maintenance trap.

**Fix direction:** share a helper or delete if genuinely unused.

**Adversary task:** What tests actually call `handle_stop`? Does their coverage justify the maintenance cost, or is it exercising behavior `run`/`run_live` tests already cover? Is `handle_stop` actually dead even under `cfg(test)`?

---

## F-11 — Connection state mutations during shutdown disconnect not captured by core

**Status:** REFUTED — correct by design, not luck (2026-04-22, adversary B). Load-bearing ordering: (a) adapter `handle_sleep_event` fires `onDisconnect` only for non-hibernatable conns (`napi_actor_events.rs:633` with filter `|conn| !conn.is_hibernatable()`), so hibernatable conns' disconnect callbacks never fire during Sleep. (b) Adapter `maybe_shutdown_save` runs after onDisconnect but before replying — captures state mutations via `on_request_save`-driven dirty bit. (c) Adapter event loop exits before `task.rs:1758` runs, so no concurrent user code. (d) `request_hibernation_transport_save` at `task.rs:1755` queues ALL preserved hibernatables unconditionally via `pending_hibernation_updates`; `save_state(Vec::new())` serializes their current in-memory state without relying on dirty tracking. On Destroy, all conns are disconnected inside `handle_destroy_event` (`napi_actor_events.rs:662`) before `maybe_shutdown_save`.

**Claim:** Hibernatable conn `set_state` in `disconnect_managed` (`connection.rs:970-971`) calls `ctx.request_save`, but by then `lifecycle` is `SleepFinalize`/`Destroying` and `schedule_state_save` (`task.rs:1843-1858`) early-returns. The eventual `save_state(Vec::new())` at `task.rs:1749` inside `finish_shutdown_cleanup_with_ctx` does pick up hibernatable dirty state, but only because hibernatable conns are preserved through disconnect.

**Claim sub-question:** Is this correctness coincidental or load-bearing?

**Adversary task:** Trace the exact ordering in `finish_shutdown_cleanup_with_ctx`. Is there a window where a disconnect callback on a hibernatable conn mutates state AFTER the final `save_state(Vec::new())` runs? What if a hibernatable conn's `onDisconnect` handler mutates state synchronously vs. async? Prove or refute data loss for hibernatable conn state.

---

## F-12 — `flush_on_shutdown` bypasses `serializeState`

**Status:** CONFIRMED as docs drift, code correct (2026-04-22, adversary D). `persisted.state` is updated during `apply_state_deltas` (`state.rs:309`), so by the time `flush_on_shutdown` runs, the latest user state delta is already in `persisted.state`. `persist_now_tracked` re-encodes and writes `PERSIST_DATA_KEY`. The doc at `rivetkit-core-state-management.md:27` is too absolute. Fix doc, not code.

**Claim:** `state.rs:614-616` writes the current core-owned `PersistedActor` blob without asking the runtime to serialize user state. Called from `mark_destroy_requested`. Contract doc `rivetkit-core-state-management.md:27` says immediate saves "must not bypass `serializeState`."

**Fix direction:** either update the doc to carve out `flush_on_shutdown`, or route it through the same path.

**Adversary task:** Does `persisted().state` already contain the latest user state at the time of `flush_on_shutdown`? If so, bypassing `serializeState` is fine because the last delta already updated it. Trace the data flow: when is `persisted.state` last updated relative to `mark_destroy_requested`? Is the doc wrong or the code wrong?

---

## F-13 — No core-side KV wipe on destroy

**Status:** REFUTED (2026-04-22, adversary D). `engine/packages/pegboard/src/workflows/actor2/mod.rs:1088` calls `ClearKvInput` activity; `mod.rs:1170-1192` does `tx.clear_subspace_range(&subspace)` on the actor's KV subspace. Envoy also has `actor_kv::delete_all` (`pegboard-envoy/src/ws_to_tunnel_task.rs:353`). KV is wiped by the engine workflow during destroy. Minor doc gap in `rivetkit-core-internals.md`, not a data leak. Close without action.

**Claim:** `mark_destroy_completed` only flips a flag; envoy/engine is assumed to GC KV keys at a higher level. `rivetkit-core-internals.md` does not mention this.

**Adversary task:** Confirm by grepping for KV delete / wipe on destroy paths across core and envoy. Does envoy actually GC? If so, document. If not, data leaks across actor incarnations at the same ID.

---

## F-14 — `deliver_shutdown_reply` drop-on-closed is only logged

**Status:** REFUTED (2026-04-22, adversary D). Only caller is `stop_actor` at `registry/mod.rs:764`, which immediately awaits `reply_rx.await` (line 770). rx cannot be dropped early in any existing caller. Log is sufficient. Close without action.

**Claim:** `task.rs:1475-1493` logs `delivered=false` if the oneshot rx is dropped but doesn't escalate. Registry likely keeps the rx alive, but no assertion.

**Adversary task:** Grep callers of `begin_stop` / shutdown reply path. Is there any caller that drops its rx before getting the reply? Is the log sufficient, or could this mask a real bug?

---

## F-15 — `set_state_initial` marks dirty + bumps revision; not idempotent

**Status:** PARTIAL (2026-04-22, adversary D). Callers at `napi_actor_events.rs:187, 206, 2116` are all boot-time paths (bootstrap install, initial state snapshot, test setup). No runtime path calls it twice in practice. `debug_assert!` would be defensive hygiene but there's no actual stampede. Low priority.

**Claim:** `state.rs:514-526` bumps revision on every call. Contract says "boot-only" but no `debug_assert!` enforces it. Repeated calls stampede saves.

**Adversary task:** Is there any code path that calls `set_state_initial` more than once per actor lifetime? If it's genuinely boot-only by convention, add `debug_assert!(!has_initial_state_set)`. If it's called multiple times on purpose, the doc is wrong.

---

## F-16 — Save ticks fire regardless of in-flight HTTP requests

**Status:** REFUTED — by design, documented (2026-04-22, adversary D). `website/src/content/docs/actors/state.mdx:128-137` explicitly documents the contract: automatic saves happen post-action, WebSocket handlers must call `c.saveState()` explicitly mid-handler, `immediate: true` forces immediate write. No docs claim HTTP requests gate save ticks.

**Claim:** `active_http_request_count` gates sleep but not save. Handlers mid-request that haven't called `request_save` won't be captured until they do.

**Adversary task:** Is this intended (saves should snapshot whatever is committed; in-flight mutations are the user's responsibility to `request_save`), or should HTTP requests gate save to prevent torn reads?

---

## F-17 — Docs drift: `rivetkit-core-internals.md` overstates core's ownership of final state save

**Status:** CONFIRMED as docs drift (2026-04-22, adversary D). `rivetkit-core-internals.md:90, 95, 103` says "Immediate state save" / "Immediate state save + SQLite cleanup" as if core unconditionally flushes user state. In reality `run_shutdown` clears `state_save_deadline` (`task.rs:1320`); the final user-state flush depends on the runtime adapter's `FinalizeSleep`/`Destroy` handler returning deltas + core's `save_state(Vec::new())` draining the hibernatable queue. Core owns the `PersistedActor` blob write via `flush_on_shutdown` but NOT user-state serialization. Doc should clarify the split. Fix docs.

**Claim:** `rivetkit-core-internals.md:95` claims sleep finalize ends with "Immediate state save." `:102-103` claims Destroy does "Immediate state save + SQLite cleanup." But `run_shutdown` + `finish_shutdown_cleanup_with_ctx` does not do an explicit immediate actor-state save — it relies on `save_state(Vec::new())` to flush queued deltas and the runtime adapter's `FinalizeSleep`/`Destroy` handler to emit final deltas.

**Adversary task:** Read the internals doc in full context. Is "immediate state save" referring to what I think (actor-owned user state) or something else (core-owned `PersistedActor` blob)? Is the doc outdated from pre-US-105 or pre-foreign-runtime?

---

## Out-of-scope items (found during review, not bugs to verify)

- **Two-timer save system** (`pending_save` for core-owned fields vs `state_save_deadline` for user-triggered saves) is correct but undocumented. Worth a module-level doc-comment.
- **`on_state_change_in_flight` wait via `wait_for_on_state_change_idle`** before enqueuing FinalizeSleep/Destroy is a subtle but correct guard against losing `onStateChange` mutations.
- **Biased select! ordering** in `run_live` — lifecycle first, then events, sleep-grace, dispatch, run-handle, timers. Reasonable defaults, no starvation surfaced.

---

## Adversarial review protocol

Each finding is assigned to an adversarial sub-agent whose goal is to **disprove** it. The adversary:

1. Reads the full code path, not just the cited lines.
2. Tries to construct a concrete interleaving / user action that demonstrates the claimed bug, OR a mechanism that prevents it.
3. Returns: `CONFIRMED` / `REFUTED` / `PARTIAL` / `INCONCLUSIVE` with specific file:line evidence.

After adversarial review, this file is updated with per-finding verdicts. Confirmed findings graduate to `.agent/todo/` or a new PRD story. Refuted findings are struck through but retained for audit.

---

## Verdict summary (2026-04-22)

**CONFIRMED (real bugs, should fix):**
- **F-1** — Self-initiated shutdown race when user run closure returns before envoy's Stop round-trips. Skips all cleanup, hangs forever. One user line away (`c.sleep(); return;`).
- **F-4** — `runStopTimeout` wiring gap end-to-end. Config plumbed through TS schema but hardcoded to `None` at NAPI layer; `effective_run_stop_timeout()` defined but never called.

**CONFIRMED as docs drift (code correct, docs wrong):**
- **F-12** — `rivetkit-core-state-management.md:27` too absolute about `serializeState`; `flush_on_shutdown` bypass is correct because `persisted.state` already has latest delta.
- **F-17** — `rivetkit-core-internals.md:90, 95, 103` overstates core's ownership of final state save. Runtime adapter drives user-state flush; core drives `PersistedActor` blob flush.

**PARTIAL (narrow or hygienic):**
- **F-2** — `run_live` lacks `catch_unwind` but no reachable panic source. Defense-in-depth only.
- **F-5** — Engine-level routing is correct; the "no wake-on-dispatch during grace" behavior is permitted by docs. Not a bug as claimed.
- **F-10** — `handle_stop` is a maintenance-trap test helper (16+ test callers), not dead. Consider consolidating with `run_shutdown`.
- **F-15** — `set_state_initial` is boot-only by caller convention, no misuse in practice; `debug_assert!` would be hygienic.

**REFUTED (closed, no action):**
- **F-3** — TS proxy always calls `requestSave`; `on_request_save` sets dirty even for duplicates; `maybe_shutdown_save` fires on both Sleep and Destroy paths.
- **F-6** — Two independent revision-check mechanisms close the race (`finish_save_request` + re-check in `on_state_save_tick`).
- **F-7** — Engine actor2 workflow explicitly does not emit a second Stop if in SleepIntent; one-Stop contract is enforced upstream, and pegboard-envoy is trusted per CLAUDE.md boundary.
- **F-8** — Destroy always wins over Sleep in `handle_run_handle_outcome`; final state well-defined.
- **F-9** — `BeginSleep` is an internal inspector event, not a lifecycle hook; `abort_signal.cancel()` is Destroy's equivalent signal.
- **F-11** — Hibernatable conn disconnect callbacks never fire during Sleep; `request_hibernation_transport_save` queues all preserved hibernatables unconditionally.
- **F-13** — Engine workflow `ClearKvInput` activity wipes KV subspace on destroy.
- **F-14** — Only caller awaits rx immediately; dropped-rx scenario is unreachable.
- **F-16** — Post-action save is the documented contract; no drift.

**Scorecard:** 17 findings → 2 real bugs (F-1, F-4), 2 docs-only drifts (F-12, F-17), 4 partial/hygiene (F-2, F-5, F-10, F-15), 9 refuted. Original review over-called by ~4×; adversarial pass caught the over-calls.

**Next actions:**
1. **F-1** → **US-109** filed in `scripts/ralph/prd.json` at priority 1. Fix self-initiated shutdown race; adds driver + Rust unit tests; acceptance requires 5/5 non-flaky runs of the new test plus regressions on `actor-sleep.test.ts`, `actor-lifecycle.test.ts`, `actor-conn-hibernation.test.ts`.
2. **F-4** → **US-110** shipped in `f2e9167da`. Wired `runStopTimeout` end-to-end (NAPI config plumbing → core `effective_run_stop_timeout()` usage in `run_shutdown`); acceptance passed 5/5 non-flaky driver runs.
3. Patch `docs-internal/engine/rivetkit-core-state-management.md:27` and `rivetkit-core-internals.md:90-103` for F-12/F-17 (docs-only, not filed as PRD stories).
4. Optional: F-10 helper consolidation, F-2 defense-in-depth wrap, F-15 `debug_assert!` (not filed).
