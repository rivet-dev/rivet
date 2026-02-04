# RivetKit Actor Workflow Integration Plan

This document captures the full end-to-end plan for integrating the Rivet workflow engine into RivetKit actors. It merges the initial high-level approach with the clarifications provided afterwards.

## Goals

- Allow `actor({ run: workflow(async ctx => { … }) })` syntax.
- Execute a single workflow per actor, using the workflow as the actor’s long-running `run` handler.
- Preserve deterministic replay by keeping the workflow engine in control of all persistent effects (KV, queue, sleeps, retries).
- Make the workflow operate in “live” mode so it keeps running in the actor process and reacts immediately to queue messages or alarms.
- Share logging context with the actor’s Pino logger.
- Provide comprehensive tests that exercise the workflow capabilities through the actor drivers (focus on the filesystem driver suite).

## Non-goals

- No workflow-specific configuration knobs for now.
- No secondary workflow queue/storage beside the actor’s queue.
- No changes to external developer ergonomics outside the new `workflow()` helper and logging visibility.

## Open Questions (Resolved)

1. **Workflow multiplicity** – exactly one workflow per actor. The `workflow()` helper returns a closure for `run`.
2. **Context determinism** – the workflow context can see actor state/vars/etc only inside workflow steps. Accessing them elsewhere throws at runtime.
3. **Message delivery** – workflows rely entirely on the existing actor queue persistence. The workflow engine must use that queue for `listen` APIs; no separate workflow message store.
4. **Message entry** – other contexts/actions send workflow input by writing to the actor queue.
5. **Sleeping** – workflow sleep/sleepUntil calls translate to actor-native alarms.
6. **Completion semantics** – keep workflow-engine behavior; when it finishes the actor’s `run` promise will resolve and crash the actor as today.
7. **Resume on wake** – when the actor restarts, we create the workflow driver with the same ID so it resumes from workflow persistence.
8. **Alarms** – fine if alarms remain scheduled; workflow retry/backoff must still use alarms.
9. **Config knobs** – none needed initially.
10. **Logging** – integrate workflow-engine logging with the actor’s Pino logger/context.
11. **Driver coverage** – write extensive actor-workflow tests, executed at least against the filesystem driver (other drivers inherit via their own suites).

## Architecture Overview

1. **Workflow KV Subspace**
   - Introduce a workflow-specific prefix in `ActorInstance` keys (e.g., `KEYS.WORKFLOW`), plus helpers to translate between workflow keys and actor KV keys.
   - All workflow-engine KV reads/writes go through this subspace to avoid clobbering user KV data.
   - The subspace must also hold workflow metadata (state, output, history) and any alarm bookkeeping needed for retries.

2. **ActorWorkflowDriver**
   - Implement `EngineDriver` by bridging to the actor’s driver and queue manager.
   - KV operations map to prefixed actor KV calls. `list` must sort keys lexicographically just like the workflow engine expects.
   - `setAlarm` calls the actor driver’s alarm API and persists the desired wake time under the workflow subspace. `clearAlarm` can be a no-op since actor alarms can remain scheduled safely.
   - Queue integration:
     - Replace workflow-engine message handling with calls into the actor queue so `listen`/`listenN` drain from queue storage rather than workflow-owned messages.
     - Persist consumed messages in workflow history for deterministic replay.
     - Ensure new queue writes wake the actor/wake runtime as needed.
  - Every workflow operation that performs active work—steps, loops, joins, races, rollbacks, queue drains, KV reads/writes—must be wrapped in `runCtx.keepAwake` via `await c.keepAwake(promise)` so the actor stays awake while work is happening.
  - The only operations that should *not* be wrapped are explicit workflow sleeps/listen waits (and other idle waits) so the actor can safely hibernate until something wakes it.

3. **Workflow Context Wrapper**
  - Add `ActorWorkflowContext` that wraps a `WorkflowContextInterface`.
  - Forward all workflow operations while injecting:
     - Deterministic guards: accessing `state`, `vars`, `db`, etc. outside a step throws.
    - Step wrappers: inside `ctx.step`, temporarily expose those actor properties, wrap the step run in `keepAwake`, and re-hide them afterward.
     - Queue bridging for `listen` methods.
     - Logging hooks that send workflow-engine logs through the actor’s logger.
  - Provide a `keepAwake` helper so workflow authors can keep the actor awake for specific promises; workflow internals automatically call `keepAwake` for every active operation, but users can still wrap their own async work as needed.

4. **`workflow()` Helper**
   - Located at `src/workflow/mod.ts`, exported via the package entry point `rivetkit/workflow` (and optionally re-exported from the root module).
   - Accepts the user’s workflow function (and optional future configuration) and returns the function to use in `run`.
  - On invocation:
    - Derive a deterministic workflow ID (e.g., `actorId`).
    - Build the `ActorWorkflowDriver`, hooking up queue/kv/alarm plumbing and logging.
    - Call `runWorkflow(id, wrappedWorkflowFn, input, driver, { mode: "live" })`.
    - Tie `c.abortSignal` to `handle.evict()`; keep `handle.result` running in the background via `c.keepAwake`/`c.waitUntil` so state flushes cleanly, but never allow the main run promise to resolve.
    - After initialization, park the run handler on a never-resolving promise (e.g., `await new Promise<never>(() => {})`) so the actor runtime never thinks the run loop exited early.
    - Ensure the wrapper automatically calls `c.keepAwake` around every active workflow operation (steps, loops, message writes, etc.) while leaving sleeps/listens unwrapped.
     - Resume automatically on wake by re-instantiating the driver with the same workflow ID when `run` is invoked again.
  - Ensure every active workflow operation (steps, loops, retries, queue drains, etc.) is wrapped in `c.keepAwake`, but never wrap idle waits (sleep/listen) so the actor can sleep in between.

5. **Logging Integration**
   - Extend workflow-engine logging hooks (if necessary) to accept a logger implementation.
   - Pass the actor’s Pino logger (or a child logger) into the workflow context so all workflow logs include the actor metadata.
   - Standardize log messages (lowercase, structured fields) per RivetKit conventions.

6. **Exports & Build**
   - Add `@rivetkit/workflow-engine` as a dependency of `rivetkit`.
   - Update `package.json` exports with `\"./workflow\"`.
   - Include the new module in the tsup build (and types) so it ships with the package.
   - Update documentation snippets to mention the new `workflow` helper.

## Testing Strategy

1. **Test Harness**
   - Reuse the existing `setupTest` fixture with the file-system driver to run actor workflows.
   - Add a dedicated top-level test launcher similar to other driver suites, invoking the new workflow tests against each supported driver (starting with filesystem).

2. **Test Suites**
   - `actor-workflow-basic.test.ts` – sanity checks for steps, persistence, workflow completion, and logging.
   - `actor-workflow-control-flow.test.ts` – loops, joins, races, rollback checkpoints, and deterministic replay (including migrations via `ctx.removed()`).
   - `actor-workflow-queue.test.ts` – `listen`, `listenN`, timeouts, queue back-pressure, and handoff via queue messages from actions.
   - `actor-workflow-sleep.test.ts` – sleep, sleepUntil, retry backoff scheduling using actor alarms, ensuring the actor sleeps between waits.
   - `actor-workflow-eviction.test.ts` – abort handling, eviction, cancellation, and actor wake/resume behavior on restart.
   - `actor-workflow-state-access.test.ts` – verifies that accessing state outside steps throws, but inside steps works and remains deterministic.

3. **Test Utilities**
   - Helpers to send queue messages, fast-forward timers or alarms, and restart actors to validate resume semantics.
  - Use `await c.keepAwake(promise)` in tests wherever a promise must keep the actor alive (e.g., waiting for workflow completion), but avoid wrapping sleep/listen waits so the actor can hibernate naturally.

4. **Automation**
   - Add the workflow test suite to the package’s `pnpm test` (or equivalent) so it runs with the existing driver tests.
   - Ensure the filesystem driver suite covers workflows alongside the existing tests; other drivers inherit coverage via their own runner suites.

## Implementation Steps

1. **Scaffolding**
   - Create `src/workflow/` directory with driver/context/helper modules.
   - Add workflow subspace helpers to `actor/instance/keys.ts`.
   - Add logging hooks in the workflow engine if not already present.

2. **Driver Implementation**
   - Implement `ActorWorkflowDriver` bridging KV, queue, and alarms.
   - Update workflow-engine storage/context (if necessary) to allow external queue backends instead of its internal message array.

3. **Context Wrapper & Deterministic Guards**
   - Implement runtime checks for state/vars access; throw descriptive errors when accessed outside steps.
   - Ensure step execution exposes the necessary actor context and hides it afterward.

4. **`workflow()` Factory**
   - Build the user-facing API plus type exports.
   - Wire into actor `run` lifecycle, including abort handling, `keepAwake`, resuming on wake, logging, and keeping the run promise pending forever (by awaiting a never-resolving promise) so the actor does not crash due to a completed run handler.

5. **Tests**
   - Author the new test suites/files.
   - Update test harness configuration to run them with the filesystem driver.

6. **Documentation & Exports**
   - Update `README.md` or relevant docs with usage examples (matching the requested snippet).
   - Adjust package exports, build config, and type declarations.

7. **Verification**
   - Run workflow tests plus existing suites to ensure no regressions.
   - Manually verify actor sleep/wake behavior with sample actors if needed.

## Future Enhancements (Out of Scope)

- Workflow-specific configuration (custom IDs, poll intervals, etc.).
- GUIs/inspector panels for workflow state.
- Multiple workflows per actor or actor-to-workflow orchestration APIs.
- Integration with additional storage backends before basic functionality lands.

This plan will guide the implementation from scaffolding through testing, ensuring the workflow engine is fully and cleanly integrated into RivetKit actors.
