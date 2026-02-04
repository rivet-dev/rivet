# Cancellation

Workflows can be stopped in two ways: eviction (graceful) and cancellation (permanent). Both are initiated through the workflow handle.

## Eviction (Graceful Stop)

`handle.evict()` requests the workflow to stop gracefully. The workflow receives an aborted `AbortSignal` and should exit at the next yield point.

```ts
const handle = runWorkflow("wf-1", workflow, input, driver);
handle.evict();
```

Use `ctx.abortSignal` or `ctx.isEvicted()` to detect eviction inside steps.

## Cancellation (Permanent Stop)

`handle.cancel()` marks the workflow as cancelled and clears any alarms. Future runs with the same workflow ID will throw `EvictedError`.

```ts
await handle.cancel();
const state = await handle.getState(); // "cancelled"
```

## Race Branch Cancellation

In `ctx.race()`, losing branches are cancelled via `AbortSignal` and typically surface as `CancelledError`. Use `ctx.abortSignal` to exit promptly.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:323` for eviction and cancellation details.
