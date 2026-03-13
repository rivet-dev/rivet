# Control Flow

Workflow control flow is expressed through deterministic helpers on `WorkflowContextInterface`. These helpers create durable history entries, so the workflow can resume after crashes and restarts without re-running completed work.

## Overview

- Every control-flow operation has a durable name. Names must be unique within the current scope.
- Code outside of `ctx.step()` must be deterministic. Use steps for I/O and side effects.
- Avoid native loops; use `ctx.loop()` so iterations are replayable and checkpointed.

## Loops

Use `ctx.loop()` for repeatable logic with durable state, periodic checkpoints, and bounded history.

```ts
import { Loop, type WorkflowContextInterface } from "@rivetkit/workflow-engine";

async function processBatches(ctx: WorkflowContextInterface) {
  return await ctx.loop({
    name: "process-batches",
    state: { cursor: null as string | null, processed: 0 },
    historyPruneInterval: 20,
    run: async (loopCtx, state) => {
      const batch = await loopCtx.step("fetch", () => fetchBatch(state.cursor));

      if (batch.items.length === 0) {
        return Loop.break(state.processed);
      }

      await loopCtx.step("process", () => processBatch(batch.items));

      return Loop.continue({
        cursor: batch.nextCursor,
        processed: state.processed + batch.items.length,
      });
    },
  });
}
```

Loop state is persisted every `historyPruneInterval` iterations. Old iterations beyond `historySize` (defaults to `historyPruneInterval`) are pruned, so rollback only replays the last retained iterations.

## Join (Wait for All)

`ctx.join()` runs named branches in parallel and waits for all of them to complete. Branch errors are collected into a `JoinError`.

```ts
const results = await ctx.join("fetch-all", {
  user: { run: async (ctx) => ctx.step("user", () => fetchUser(id)) },
  posts: { run: async (ctx) => ctx.step("posts", () => fetchPosts(id)) },
});
```

- Branches run concurrently on the same workflow instance.
- All branches settle before `join` returns.
- Failures raise `JoinError` with per-branch errors.

## Race (First Wins)

`ctx.race()` runs branches in parallel and returns the first successful result. Losing branches are cancelled via `AbortSignal`.

```ts
const { winner, value } = await ctx.race("timeout", [
  { name: "work", run: async (ctx) => ctx.step("do-work", doWork) },
  { name: "timeout", run: async (ctx) => { await ctx.sleep("wait", 30000); return null; } },
]);
```

- The winner is persisted for replay.
- Losing branches receive `ctx.abortSignal` and typically throw `CancelledError`.
- If all branches fail, `RaceError` is thrown.

## Handling Terminal Failures as Data

Use `ctx.tryStep()` when a single step failure should become a value instead of failing the workflow:

```ts
const charge = await ctx.tryStep({
  name: "charge-card",
  maxRetries: 3,
  run: async () => chargeCard(orderId),
});

if (!charge.ok) {
  return {
    status: "manual-review",
    reason: charge.failure.error.message,
  };
}
```

Use `ctx.try()` when you want a named scope that can recover from terminal `step`, `join`, or `race` failures:

```ts
const payment = await ctx.try("payment-flow", async (blockCtx) => {
  const auth = await blockCtx.step("authorize", () => authorize(orderId));
  const capture = await blockCtx.step("capture", () => captureFunds(orderId));
  return { auth, capture };
});
```

- `ctx.tryStep()` and `ctx.try()` only catch terminal failures. Retry backoff, sleeps, queue waits, eviction, and history divergence still rethrow.
- `RollbackError` is not caught by default. Opt in with `catch: ["rollback"]` when you want rollback failures returned as data.
- `ctx.try()` still needs a stable name because the enclosed control-flow history is nested under that block.

## Messages in Control Flow

`ctx.queue.next()` pauses for one required message. `ctx.queue.nextBatch()` supports optional / batch waits. Queue wait names are part of history, so keep them stable and unique.

```ts
const approval = await ctx.queue.next<string>("approval", {
  names: ["approval-granted"],
});
```

Messages are loaded at workflow start. If a message arrives during execution, the workflow yields and picks it up on the next run.

## Best Practices

- Use stable names for steps, loops, joins, races, and queue waits.
- Keep all nondeterministic work inside steps.
- Use loop state to avoid native `while`/`for` loops.
- Handle cancellation via `ctx.abortSignal` in long-running branches.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:155` for loop usage.
- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:207` for queue waits.
