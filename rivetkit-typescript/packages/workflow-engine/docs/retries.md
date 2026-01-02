# Retries

Step retries provide fault tolerance for transient failures. The workflow engine tracks attempts per step and retries with deterministic backoff.

## Automatic Retry Behavior

- Errors thrown from a step callback are treated as retryable by default.
- Each failure increments the step’s attempt counter.
- The engine computes deterministic exponential backoff and yields until the retry time.
- Once attempts exceed `maxRetries`, a `StepExhaustedError` is thrown.

## Configuring Retries

```ts
await ctx.step({
  name: "external-api",
  maxRetries: 5,
  retryBackoffBase: 200,
  retryBackoffMax: 60000,
  timeout: 10000,
  run: async () => callExternalApi(),
});
```

- `maxRetries` defaults to 3.
- `retryBackoffBase` and `retryBackoffMax` control exponential delay.
- `timeout` limits how long the step can run; timeouts are treated as critical failures.

## Unrecoverable Errors

Use `CriticalError` or `RollbackError` for failures that should not retry. Rollback requires a checkpoint:

```ts
import { CriticalError, RollbackError } from "@rivetkit/workflow-engine";

await ctx.step("validate", async () => {
  if (!isValid(input)) {
    throw new CriticalError("Invalid input");
  }
});

await ctx.rollbackCheckpoint("rollback");

await ctx.step("halt", async () => {
  throw new RollbackError("Stop and roll back");
});
```

`StepTimeoutError` is also treated as critical, so timeouts bypass retries.

## Exhaustion and Recovery

When a step exhausts retries, the workflow fails with `StepExhaustedError`. You can reset exhausted steps using the workflow handle:

```ts
await handle.recover();
```

`recover()` clears retry metadata, removes the workflow error, and schedules the workflow to run again.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:101` for step configuration defaults.
