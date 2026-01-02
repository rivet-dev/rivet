# Long-Running Workflows

Long-running workflows can pause, sleep, and resume across process restarts. This behavior is powered by durable history and the workflow driver scheduler.

## Yielding Execution

Use sleep and listen helpers to yield control while waiting:

```ts
await ctx.sleep("wait-5-min", 5 * 60 * 1000);
const message = await ctx.listen<string>("wait-approval", "approval");
```

When a workflow yields, `runWorkflow` returns a `WorkflowResult` with `state: "sleeping"`. The driver alarm or a message wake-up triggers the next run.

## Short vs. Long Sleeps

- Short sleeps (< `driver.workerPollInterval`) wait in memory.
- Longer sleeps set an alarm via the driver and return control to the scheduler.

This allows workflows to pause for hours or days without tying up worker memory.

## Checkpointing Loop State

`ctx.loop()` persists loop state every `commitInterval` iterations. Loop history is trimmed every `historyEvery` iterations, keeping the most recent `historyKeep` iterations. Rollback only replays the last retained iteration, and long-running loops do not accumulate unbounded history.

## Handling Eviction

Workers can be evicted for scaling or deployments. Use `ctx.abortSignal` or `ctx.isEvicted()` to stop work safely:

```ts
await ctx.step("long-task", async () => {
  while (!ctx.isEvicted()) {
    await doChunkOfWork();
  }
});
```

Evictions save state and return control to the scheduler, allowing the workflow to resume elsewhere.

## Driver Considerations

The `EngineDriver` provides scheduling via `setAlarm` and `clearAlarm`. For long-running workflows, ensure your driver implementation persists alarms reliably and returns due alarms to the runner.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:193` for sleep behavior.
