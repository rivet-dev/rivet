# Rollback Behavior

Rollback handlers let you undo completed steps when a workflow fails with an unrecoverable error. Throwing `RollbackError` or `CriticalError` from a step forces rollback immediately.

## When Rollback Runs

Rollback runs only when the workflow encounters an unrecoverable error (any error that is not a retryable step failure). It does not run on evictions or cancellations. Rollback is disabled until you call `ctx.rollbackCheckpoint()`.

## Ordering and Persistence

- Rollback handlers execute in reverse order of step completion.
- Each rollback is recorded, so restarts resume unfinished rollbacks instead of repeating completed ones.
- Steps without a rollback handler are skipped.
- Rollback handlers require a prior `rollbackCheckpoint`.
- Loop rollbacks only replay the last retained iteration because loop history is trimmed.

## Rollback Context

Rollback handlers receive a `RollbackContextInterface` with `abortSignal` and `isEvicted()` for cooperative shutdown. A checkpoint must be registered before any rollback handlers.

```ts
await ctx.rollbackCheckpoint("billing");

await ctx.step({
  name: "charge",
  run: async () => chargeCard(orderId),
  rollback: async (rollbackCtx, receipt) => {
    if (rollbackCtx.abortSignal.aborted) {
      return;
    }
    await refundCharge(receipt);
  },
});
```

## Failure Behavior

If a rollback handler throws, the workflow fails and preserves the rollback error in metadata. Subsequent retries or reruns will attempt remaining rollback steps again. If a rollback handler is configured without a checkpoint, the workflow fails with `RollbackCheckpointError`.

```ts
await ctx.step({
  name: "charge",
  run: async () => chargeCard(orderId),
  rollback: async () => {
    await refundCharge(orderId);
  },
});
// Throws RollbackCheckpointError because no checkpoint was set.
```

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:139` for rollback examples.
- `rivetkit-typescript/packages/workflow-engine/tests/rollback.test.ts:19` for rollback ordering.
