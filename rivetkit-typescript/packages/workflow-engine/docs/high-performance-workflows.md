# High Performance Workflows

The workflow engine is designed to minimize writes and replay costs, but you can still tune performance for high-throughput workflows. This document covers the user-facing knobs that reduce storage churn and runtime overhead.

## Ephemeral Steps

Set `ephemeral: true` on a step to delay flushing its entry to storage. The step still records history, but the flush is deferred until the next non-ephemeral operation.

```ts
await ctx.step({
  name: "prepare",
  ephemeral: true,
  run: async () => preparePayload(),
});

await ctx.step({
  name: "send",
  ephemeral: true,
  run: async () => sendPayload(),
});

// This step flushes all pending entries
await ctx.step("finalize", async () => finalizeBatch());
```

Use ephemeral steps for idempotent work where you want to batch writes. Do not use them for critical side effects that must be persisted immediately.

## Batch Writes Intentionally

- Group short-lived steps between durable checkpoints.
- Use non-ephemeral steps to ensure important state changes are flushed to storage.
- Prefer `ctx.loop()` with `commitInterval`, `historyEvery`, and `historyKeep` to control persistence and history retention.

## Parallelism Without Extra Workers

`ctx.join()` and `ctx.race()` let you run async work in parallel inside a single workflow instance. This keeps history consistent while taking advantage of concurrency.

## Deterministic Hot Paths

- Move nondeterministic work into `ctx.step()` callbacks so replay is fast.
- Avoid CPU-heavy work outside of steps; replay executes that code again.
- Use stable names to prevent `HistoryDivergedError` on replays.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:100` for ephemeral step usage.
