# Workflow Replay Code Review: Confirmed Findings & Proposals

## Confirmed: No Action Needed

### `loadActor` is idempotent (review item #2)
All three driver implementations (file-system, engine, cloudflare) cache the actor instance in a map keyed by actor ID. Subsequent calls return the cached instance immediately. The double `loadActor` in `inspectorAuth` + route handler is cosmetically redundant but has zero performance impact. No fix needed.

### `replayWorkflowFromStep` inclusive deletion is correct (review item #6)
`ordered.slice(targetIndex)` deletes the boundary entry itself. This is correct for "re-execute from step X" semantics. If the target is inside a loop, `findReplayBoundaryEntry` walks up to the enclosing loop entry, which is also deleted so the entire loop re-executes. Intentional behavior.

### `getWorkflowInspector` / `createWorkflowInspectorAdapter` complexity is load-bearing
The WeakMap + closure-based factory pattern exists because:
- The adapter must survive across run handler restarts (inspector connection outlives the run handler)
- `update` and `setReplayFromStep` are internal mutation handles that must not be exposed on the public `WorkflowInspectorAdapter` interface
- WeakMap prevents memory leaks when actor instances are GC'd

No simplification recommended.

### `inspectorAuth` return pattern and diff
The function returns `undefined` (auth passed) or `Response` (401). This is standard Hono middleware-as-function pattern. The diff (commit `6486370f6`) changed the control flow from "reject if no global token configured" to "fall through to per-actor token check." The old code would 401 when no global token was set, blocking the standalone Inspector UI which uses per-actor tokens. The new code correctly tries both. No issue.

### `getReplayState` visibility logic (review item #10)
The logic is correct and compact. When `disabledBecauseRunning` is true for the current step with no retries, `isVisible` becomes `true` to show a disabled button. This is intentional UX: "you could replay this, but not right now." No change needed.

### `loadStorage` eager metadata loading (review item #7)
Added in commit `6486370f6` ("feat(rivetkit): rerun workflows from inspector"). The inspector needs complete metadata to show workflow history (status, retry counts, timestamps) immediately after actor wake. Without eager loading, `getWorkflowHistoryJson()` would have no metadata until steps actually ran. The trade-off is small: metadata is a few bytes per entry. Only matters for workflows with hundreds of steps. No change needed for now.

### Unchecked `c.req.json()` in inspector endpoints (review item #8)
Inspector endpoints use bare `c.req.json<T>()` with no validation. Public-facing endpoints (actions, queue send) in `router-endpoints.ts` use zod schemas (`HttpActionRequestSchema`, `HttpQueueSendRequestSchema`) with proper error types. The difference is appropriate: inspector endpoints are internal/debug-only, authenticated, and called only by the inspector UI. No change needed.

### KV driver duplication (review item #3)
`ActorWorkflowControlDriver` is deleted by Action Item 1, removing the duplication entirely. No separate work needed.

---

## Action Items

### 1. Move replay into the workflow engine via `handle.replay()` with looping `run()`

**Problem:** Replay currently operates outside the workflow engine. It requires `restartRunHandler`, `ActorWorkflowControlDriver`, `NoopWorkflowMessageDriver`, handle lifetime management, and `isRunHandlerActive()` guards. This creates race conditions and makes it impossible to replay during sleep (the most common workflow state).

**Solution:** Add a `replay(entryId?: string)` method to `WorkflowHandle` and make `run()` in `mod.ts` loop across replays instead of exiting.

#### 1a. `handle.replay()` in the workflow engine

In `rivetkit-typescript/packages/workflow-engine/src/index.ts`, add to `WorkflowHandle`:

```typescript
async replay(entryId?: string): Promise<WorkflowHistorySnapshot> {
    // Evict current execution. This causes executeLiveWorkflow to
    // catch EvictedError and resolve resultPromise, which unblocks
    // whoever is awaiting handle.result (i.e. run() in mod.ts).
    abortController.abort(new EvictedError());
    await resultPromise.catch(() => {});

    // Mutate storage: delete entries from target forward, reset
    // state to "sleeping", clear output/error. Uses the same driver
    // the engine already has, no separate control driver needed.
    // The force flag skips the "running entries" safety check since
    // we just evicted and entry metadata may still show "running."
    const snapshot = await replayWorkflowFromStep(
        workflowId, driver, entryId, { force: true },
    );

    // Restart the internal execution loop with a fresh abort
    // controller. The old abort controller is dead (already aborted).
    // run() in mod.ts will see wasReplayed=true on the handle and
    // loop back to await the new resultPromise.
    abortController = new AbortController();
    liveRuntime = createLiveRuntime();
    wasReplayed = true;
    resultPromise = executeLiveWorkflow(
        workflowId, workflowFn, input, driver, messageDriver,
        abortController, liveRuntime, onHistoryUpdated, onError, logger,
    );

    return snapshot;
}
```

Key implementation details:
- `resultPromise` and `abortController` change from `const` to `let` since `replay()` reassigns them.
- `result` on the handle changes from a data property to a getter (`get result() { return resultPromise; }`) so that `run()` in mod.ts always sees the current promise after replay.
- Add `wasReplayed: boolean` flag and `clearReplayFlag()` method to the handle.
- Add `{ force?: boolean }` option to `replayWorkflowFromStep` that skips the `metadata.status === "running"` check (line 792-801).
- `onHistoryUpdated` is NOT called inside `replay()` to avoid double notification. The caller (`setReplayFromStep` in mod.ts) calls `workflowInspector.update(snapshot)`, and the new `executeLiveWorkflow` will also emit updates as steps execute.

**Changes to `WorkflowHandle` type** in `types.ts`:
```typescript
replay(entryId?: string): Promise<WorkflowHistorySnapshot>;
readonly wasReplayed: boolean;
clearReplayFlag(): void;
```

#### 1b. Looping `run()` in mod.ts

In `rivetkit-typescript/packages/rivetkit/src/workflow/mod.ts`, `run()` must loop to survive across replays:

```typescript
async function run(runCtx): Promise<void> {
    const actor = /* ... */;
    const workflowInspector = getWorkflowInspector(actor);
    const driver = new ActorWorkflowDriver(actor, runCtx);

    const handle = runWorkflow(actor.id, /* ... */, driver, { mode: "live", /* ... */ });

    // Register replay callback AFTER handle creation so we can
    // reference the handle directly. setReplayFromStep is called
    // inside run(), which re-runs on each run handler invocation,
    // so the callback always has the current handle.
    workflowInspector.setReplayFromStep(async (entryId) => {
        const snapshot = await handle.replay(entryId);
        workflowInspector.update(snapshot);
        return workflowInspector.adapter.getHistory();
    });

    const onAbort = () => { handle.evict(); };
    runCtx.abortSignal.addEventListener("abort", onAbort, { once: true });

    try {
        // Loop across replays. handle.replay() evicts the current
        // execution (resolving handle.result), sets wasReplayed=true,
        // and starts a new executeLiveWorkflow. This loop detects
        // the replay flag and re-awaits the new execution instead
        // of exiting. This keeps run() alive so error handling,
        // abort cleanup, and #runHandlerActive all work correctly.
        while (true) {
            try {
                await handle.result;
            } catch (error) {
                if (runCtx.abortSignal.aborted) return;
                if (shouldRethrowWorkflowError(error)) {
                    runCtx.log.error({ msg: "workflow run failed", error: stringifyError(error) });
                    throw error;
                }
                runCtx.log.warn({ msg: "workflow failed and will sleep until woken", error: stringifyError(error) });
            }

            if (!handle.wasReplayed) break;
            handle.clearReplayFlag();
        }
    } finally {
        runCtx.abortSignal.removeEventListener("abort", onAbort);
    }
}
```

#### 1c. Rollback-aware replay

When `replayWorkflowFromStep` is called and the workflow state is `rolling_back`, it abandons the rollback. The existing logic already handles this: state is set to `"sleeping"` (line 819) and entries are deleted. No special handling needed in the engine.

The frontend communicates this to the user (see Action Item 5).

#### 1d. What this eliminates

Delete from the codebase:
- `ActorWorkflowControlDriver` class — `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts`
- `NoopWorkflowMessageDriver` class — `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts`
- `restartRunHandler()` method — `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts` (line 486-496). Only caller was the replay callback. Remove the method entirely.
- `isRunHandlerActive()` method — `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts` (line 498-500). Only caller was the replay callback. The private `#runHandlerActive` field is still used internally for sleep timer logic and must be kept.
- Import of `ActorWorkflowControlDriver` in `mod.ts`

**Files to modify:**
- `rivetkit-typescript/packages/workflow-engine/src/index.ts` — add `replay()`, `wasReplayed`, `clearReplayFlag()` to handle; change `result` to getter; add `force` option to `replayWorkflowFromStep`; change `const` to `let` for `resultPromise`/`abortController`/`liveRuntime`
- `rivetkit-typescript/packages/workflow-engine/src/types.ts` — add `replay`, `wasReplayed`, `clearReplayFlag` to `WorkflowHandle` interface
- `rivetkit-typescript/packages/rivetkit/src/workflow/mod.ts` — looping `run()`, simplified `setReplayFromStep`, move registration after handle creation, remove `ActorWorkflowControlDriver` import
- `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts` — delete `ActorWorkflowControlDriver` and `NoopWorkflowMessageDriver`
- `rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts` — delete `restartRunHandler()` and `isRunHandlerActive()`

### 2. Delete `syncWorkflowHistoryAfterReplay` (review item #1)

**Current behavior:** Fires a void promise with hardcoded polling delays `[250, 1_000, 2_500, 5_000]` to refetch workflow history and actor state after replay. No abort mechanism. Multiple chains can stack.

**Finding:** Redundant. The existing realtime WebSocket channel pushes `WorkflowHistoryUpdated` events as steps execute. `StateUpdated` events fire when actor state is saved during workflow `batch()` calls.

**Proposed fix:** Delete `syncWorkflowHistoryAfterReplay` and its call site. Add a single post-replay invalidation of `actorState` (not polled).

```typescript
const handleReplay = async (entryId?: string) => {
    try {
        const result = await replayMutation.mutateAsync(entryId);
        queryClient.setQueryData(
            actorInspectorQueriesKeys.actorWorkflowHistory(actorId),
            { history: result.history, isEnabled: result.isEnabled },
        );
        queryClient.setQueryData(
            actorInspectorQueriesKeys.actorIsWorkflowEnabled(actorId),
            result.isEnabled,
        );
        // Single refetch of state (not polled)
        queryClient.invalidateQueries({
            queryKey: actorInspectorQueriesKeys.actorState(actorId),
            exact: true,
        });
        toast.success(
            entryId
                ? "Workflow replay scheduled from selected step."
                : "Workflow replay scheduled from the beginning.",
        );
    } catch (error) {
        toast.error(
            error instanceof Error
                ? error.message
                : "Failed to replay workflow step.",
        );
    }
};
```

**Files to modify:**
- `frontend/src/components/actors/workflow/actor-workflow-tab.tsx` — delete `syncWorkflowHistoryAfterReplay`, simplify `handleReplay`

### 3. Add workflow cancel endpoint and UI

#### 3a. Backend: cancel endpoint

Add `POST /inspector/workflow/cancel` to `rivetkit-typescript/packages/rivetkit/src/actor/router.ts`:

```typescript
router.post("/inspector/workflow/cancel", async (c) => {
    const authResponse = await inspectorAuth(c);
    if (authResponse) return authResponse;

    const actor = await actorDriver.loadActor(c.env.actorId);
    await actor.inspector.cancelWorkflow();
    return c.json({ ok: true });
});
```

Add `cancelWorkflow()` to the actor inspector interface. This calls `handle.cancel()` on the workflow handle.

Wire through the inspector adapter in `rivetkit-typescript/packages/rivetkit/src/workflow/inspector.ts`:

```typescript
export interface WorkflowInspectorAdapter {
    getHistory: () => inspectorSchema.WorkflowHistory | null;
    onHistoryUpdated: (listener: (history: inspectorSchema.WorkflowHistory) => void) => () => void;
    replayFromStep: (entryId?: string) => Promise<inspectorSchema.WorkflowHistory | null>;
    cancel: () => Promise<void>;  // NEW
}
```

In `mod.ts`, wire the cancel callback similarly to replay:

```typescript
workflowInspector.setCancel(async () => {
    await handle.cancel();
});
```

Also add the WebSocket handler for `WorkflowCancelRequest` in `rivetkit-typescript/packages/rivetkit/src/inspector/handler.ts`.

Update driver test at `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-inspector.ts` to cover the cancel endpoint.

**Files to modify:**
- `rivetkit-typescript/packages/rivetkit/src/actor/router.ts` — new endpoint
- `rivetkit-typescript/packages/rivetkit/src/workflow/inspector.ts` — add `cancel` to adapter interface
- `rivetkit-typescript/packages/rivetkit/src/workflow/mod.ts` — wire cancel callback
- `rivetkit-typescript/packages/rivetkit/src/inspector/actor-inspector.ts` — add `cancelWorkflow` method
- `rivetkit-typescript/packages/rivetkit/src/inspector/handler.ts` — add WebSocket handler
- `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-inspector.ts` — add test

#### 3b. Frontend: cancel button

Add a "Cancel Workflow" button in the workflow tab header area (not the step detail panel). This is a workflow-level action.

In `frontend/src/components/actors/workflow/actor-workflow-tab.tsx`:

- Add a cancel mutation using the inspector's cancel endpoint
- Render a button in the header area, visible when the workflow state is `running`, `sleeping`, `rolling_back`, or `failed`
- Hide/disable when the workflow is `completed` or `cancelled`
- Show a confirmation dialog before cancelling ("This will permanently stop the workflow. Are you sure?")
- After cancel, the `WorkflowHistoryUpdated` WebSocket event will update the UI

**Files to modify:**
- `frontend/src/components/actors/workflow/actor-workflow-tab.tsx` — cancel button and mutation
- `frontend/src/components/actors/actor-inspector-context.tsx` — add cancel mutation option if needed

### 4. `routeTree.gen.ts` formatting noise (review item #9)

Auto-generated by TanStack Router. Manual formatting fixes will be overwritten on next route change. The correct fix is to update the TanStack Router formatter config to match the project, or add the file to the formatter ignore list.

**Files to modify:**
- TanStack Router config in `frontend/apps/inspector/` (not the generated file itself)

### 5. Workflow state indicator and rollback-aware replay UI

#### 5a. Workflow state indicator

The workflow-level state (`running`, `sleeping`, `failed`, `rolling_back`, `completed`, `cancelled`) is not displayed anywhere in the UI. Add a status badge in the workflow tab header.

In `frontend/src/components/actors/workflow/actor-workflow-tab.tsx` or `workflow-visualizer.tsx`:

- Display a badge/pill showing the current workflow state
- Use consistent colors: green for completed, blue for running, amber for sleeping, red for failed/cancelled, pink/magenta for rolling_back
- For `rolling_back`, show "Rolling Back" with a spinner or animated indicator

#### 5b. Rollback-aware replay button text

In `frontend/src/components/actors/workflow/workflow-visualizer.tsx`, update the replay button:

- When workflow state is `rolling_back`: change button text from "Replay from this step" to "Abandon rollback and replay from this step"
- When workflow state is NOT `rolling_back`: keep the current "Replay from this step" text
- The `onReplayStep` callback and `getReplayState` function need access to the workflow state. Pass `workflowState` through to the visualizer.

The `getReplayState` function should also allow replay when the workflow is in `rolling_back` state (currently it may be blocked by the `hasRunningStep` check).

**Files to modify:**
- `frontend/src/components/actors/workflow/actor-workflow-tab.tsx` — pass workflow state to visualizer
- `frontend/src/components/actors/workflow/workflow-visualizer.tsx` — workflow state badge, conditional button text
- `frontend/src/components/actors/workflow/workflow-types.ts` — may need to add `workflowState` to node data or visualizer props

### 6. Add test coverage for replay edge cases

**New tests to add:**

#### In `rivetkit-typescript/packages/workflow-engine/tests/replay.test.ts`:

- **"handle.replay() evicts running workflow and replays"** — Create a workflow in live mode with a long-running step. Call `handle.replay(entryId)`. Verify the running step is interrupted, storage is truncated, and the workflow re-executes from the target step.
- **"handle.replay() works when workflow is sleeping"** — Create a workflow that sleeps. Call `handle.replay(entryId)`. Verify the sleep is interrupted and the workflow re-executes.
- **"handle.replay() works when workflow is completed"** — Complete a workflow. Call `handle.replay(entryId)`. Verify completed state is cleared and the workflow re-executes.
- **"handle.replay() without entryId replays from beginning"** — Complete a workflow. Call `handle.replay()`. Verify all entries are deleted and the workflow re-executes from scratch.
- **"handle.replay() during rollback abandons rollback"** — Run a workflow that enters rolling_back state. Call `handle.replay()`. Verify state is reset to sleeping and the workflow re-executes forward.
- **"concurrent handle.replay() calls are safe"** — Call `handle.replay()` twice simultaneously. Verify the second call evicts the first replay's execution and starts fresh.
- **"replay after workflow error clears error state"** — Run a workflow that fails. Call `handle.replay()`. Verify `storage.error` is cleared and the workflow re-executes.
- **"handle.cancel() permanently stops workflow"** — Cancel a running workflow. Verify state is set to cancelled and re-execution throws EvictedError.
- **"run() loops correctly across replays"** — Verify that after replay, the run() function continues awaiting the new execution and handles errors from the replayed workflow.

#### In `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-inspector.ts`:

- **"POST /inspector/workflow/replay succeeds during running workflow"** — Use `workflowRunningStepActor`, call replay, verify it succeeds instead of returning 500.
- **"POST /inspector/workflow/replay succeeds during rollback"** — Trigger rollback, call replay, verify it abandons rollback and replays.
- **"POST /inspector/workflow/replay works with per-actor token"** — Authenticate using the per-actor inspector token, verify replay works.
- **"POST /inspector/workflow/cancel cancels workflow"** — Cancel a running workflow via the endpoint, verify state is cancelled.
- **"POST /inspector/workflow/cancel is idempotent"** — Cancel an already-cancelled workflow, verify no error.

#### Test fixture actors to add in `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/workflow.ts`:

- **`workflowErrorActor`** — Actor whose workflow throws a `CriticalError` on a specific step, for testing replay-after-error.
- **`workflowRollbackActor`** — Actor whose workflow has steps with rollback handlers, where a later step fails, triggering rollback. For testing replay during rollback.

**Files to modify:**
- `rivetkit-typescript/packages/workflow-engine/tests/replay.test.ts`
- `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-inspector.ts`
- `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/workflow.ts`

---

## Adversarial Review Findings

### Addressed by this spec

- **Handle reference lifetime bug** — Eliminated. `setReplayFromStep` is registered inside `run()` after `handle` creation. Each `run()` invocation gets its own handle.
- **No timeout on `waitForRunHandler`** — Eliminated. No `waitForRunHandler` needed.
- **Concurrent replay race condition** — `replay()` evicts + awaits before mutating. Natural serialization.
- **`syncWorkflowHistoryAfterReplay` loses state updates** — Addressed by single state invalidation.
- **`run()` exits after eviction, orphaning new execution** — Fixed by looping `run()`. The `while(true)` loop checks `handle.wasReplayed` and re-awaits.
- **`result` is a data property, not a getter** — Spec requires changing to getter.
- **`Object.assign` on liveRuntime** — Eliminated. `replay()` creates a fresh `liveRuntime` via assignment.
- **Double inspector notification** — `replay()` does NOT call `onHistoryUpdated`. Only the caller in mod.ts and the new execution emit updates.
- **Replay during rollback bricks workflow** — Replay abandons rollback. UI communicates this.
- **Replay on cancelled workflow** — `handle.cancel()` is exposed as a deliberate user action. Replay on a cancelled workflow will re-execute (the state is reset to sleeping by `replayWorkflowFromStep`). This is intentional: if the user explicitly replays a cancelled workflow, they want it to run again.
- **KV base class no longer needed** — `ActorWorkflowControlDriver` is deleted.

### Not addressed by this spec (lower priority, separate work)

- **Pre-auth actor loading in global middleware** (`router.ts:69-77`). Unauthenticated requests still load actors. Low priority, `loadActor` is cached.
- **Dev mode auth bypass is broad** (`router.ts:169-174`). Pre-existing, not from this PR.
- **WebSocket/HTTP auth inconsistency**. WS requires per-actor token; HTTP accepts global OR per-actor. Pre-existing.
- **`selectedNode` holds stale data in frontend**. Unrelated to replay.
- **`fitView` fires on every render**. Unrelated to replay.
- **`onReplayStep` inline function breaks `useMemo`**. Unrelated to replay.
- **Actor destroy during replay**. Inherent limitation of non-transactional KV.
- **WebSocket protocol header crash** (`router-websocket-endpoints.ts:132`). Pre-existing.
