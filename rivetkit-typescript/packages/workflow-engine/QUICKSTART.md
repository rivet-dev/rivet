# Workflow Engine Quickstart

A durable execution engine for TypeScript. Write long-running, fault-tolerant workflows as regular async functions that can survive process restarts, crashes, and deployments.

## Overview

The workflow engine enables reliable execution through:

1. **History Tracking** - Every operation is recorded in persistent storage
2. **Replay** - On restart, operations replay from history instead of re-executing
3. **Deterministic Execution** - Same inputs produce the same execution path

## Installation

```bash
npm install @rivetkit/workflow-engine
```

## Quick Example

```typescript
import { runWorkflow, Loop, type WorkflowContextInterface } from "@rivetkit/workflow-engine";

async function orderWorkflow(ctx: WorkflowContextInterface, orderId: string) {
  // Steps are durable - if this crashes after payment, it won't charge twice on restart
  const payment = await ctx.step("process-payment", async () => {
    return await chargeCard(orderId);
  });

  // Wait for shipping confirmation (external message)
  const [tracking] = await ctx.queue.next<string>("wait-shipping", {
    names: ["shipment-confirmed"],
  });

  // Send notification
  await ctx.step("notify-customer", async () => {
    await sendEmail(orderId, tracking);
  });

  return { orderId, payment, tracking };
}

// Run the workflow
const handle = runWorkflow("order-123", orderWorkflow, "order-123", driver);

// Send a message from external system
await handle.message("shipment-confirmed", "TRACK-456");

// Wait for completion
const result = await handle.result;
```

## Core Concepts

### The Driver

The workflow engine requires an `EngineDriver` implementation for persistence and scheduling. Each workflow instance operates on an isolated KV namespace.

```typescript
interface EngineDriver {
  // KV Operations
  get(key: Uint8Array): Promise<Uint8Array | null>;
  set(key: Uint8Array, value: Uint8Array): Promise<void>;
  delete(key: Uint8Array): Promise<void>;
  deletePrefix(prefix: Uint8Array): Promise<void>;
  list(prefix: Uint8Array): Promise<KVEntry[]>;  // MUST be sorted
  batch(writes: KVWrite[]): Promise<void>;       // Should be atomic

  // Scheduling
  setAlarm(workflowId: string, wakeAt: number): Promise<void>;
  clearAlarm(workflowId: string): Promise<void>;
  readonly workerPollInterval: number;
}
```

### Running Workflows

```typescript
import { runWorkflow } from "@rivetkit/workflow-engine";

const handle = runWorkflow(
  "workflow-id",      // Unique ID for this workflow instance
  myWorkflow,         // Your workflow function
  { input: "data" },  // Input passed to workflow
  driver              // Your EngineDriver implementation
);

// The handle provides methods to interact with the running workflow
await handle.result;                    // Wait for completion/yield
await handle.message("name", data);      // Send a message
await handle.wake();                    // Wake immediately
await handle.recover();                 // Reset exhausted retries
handle.evict();                         // Request graceful shutdown
await handle.cancel();                  // Cancel permanently
await handle.getState();                // Get current state
await handle.getOutput();               // Get output if completed
```

## Features

### Steps

Steps execute arbitrary async code. Results are persisted and replayed on restart.

```typescript
// Simple form
const result = await ctx.step("fetch-user", async () => {
  return await fetchUser(userId);
});

// With configuration
const result = await ctx.step({
  name: "external-api",
  maxRetries: 5,                    // Default: 3
  retryBackoffBase: 200,            // Default: 100ms
  retryBackoffMax: 60000,           // Default: 30000ms
  timeout: 10000,                   // Default: 30000ms (0 to disable)
  ephemeral: false,                 // Default: false - batch writes
  run: async () => {
    return await callExternalApi();
  },
});
```

**Retry Behavior:**
- Regular errors trigger automatic retry with exponential backoff
- `CriticalError` and `RollbackError` bypass retry logic for unrecoverable errors
- After exhausting retries, `StepExhaustedError` is thrown

```typescript
import { CriticalError, RollbackError } from "@rivetkit/workflow-engine";

await ctx.step("validate", async () => {
  if (!isValid(input)) {
    throw new CriticalError("Invalid input - no point retrying");
  }
  return processInput(input);
});

await ctx.step("halt", async () => {
  throw new RollbackError("Stop and roll back");
});
```

### Rollback

Rollback handlers run only for steps that completed successfully and registered
`rollback`. They run in reverse completion order, persist across restarts, and
skip handlers that already finished. Rollback is disabled until you set a
checkpoint.

```typescript
await ctx.rollbackCheckpoint("billing");

await ctx.step({
  name: "charge",
  run: async () => chargeCard(orderId),
  rollback: async (_ctx, receipt) => {
    await refundCharge(receipt);
  },
});
```

### Loops

Loops maintain durable state across iterations with periodic checkpointing and bounded history.

```typescript
import { Loop } from "@rivetkit/workflow-engine";

const total = await ctx.loop({
  name: "process-batches",
  state: { cursor: null, count: 0 },  // Initial state
  commitInterval: 10,                  // Checkpoint every 10 iterations
  historyEvery: 10,                    // Trim history every 10 iterations
  historyKeep: 10,                     // Keep last 10 iterations of history
  run: async (ctx, state) => {
    const batch = await ctx.step("fetch", () => fetchBatch(state.cursor));

    if (!batch.items.length) {
      return Loop.break(state.count);  // Exit with final value
    }

    await ctx.step("process", () => processBatch(batch.items));

    return Loop.continue({             // Continue with new state
      cursor: batch.nextCursor,
      count: state.count + batch.items.length,
    });
  },
});
```

**Simple loops** (no state needed):

```typescript
const result = await ctx.loop("my-loop", async (ctx) => {
  // ... do work
  if (done) return Loop.break(finalValue);
  return Loop.continue(undefined);
});
```

### Sleep

Pause workflow execution for a duration or until a specific time.

```typescript
// Sleep for duration
await ctx.sleep("wait-5-min", 5 * 60 * 1000);

// Sleep until timestamp
await ctx.sleepUntil("wait-midnight", midnightTimestamp);
```

Short sleeps (< `driver.workerPollInterval`) wait in memory. Longer sleeps yield to the scheduler and set an alarm for wake-up.

### Queue Messages

Wait for external events delivered via `handle.message()`.

```typescript
// Wait for a single message
const data = await ctx.queue.next<PaymentResult>("payment", {
  names: ["payment-completed"],
});

// Wait for N messages
const items = await ctx.queue.nextBatch<Item>("batch", {
  names: ["item-added"],
  count: 10,
});

// Wait with timeout (returns [] on timeout)
const [result] = await ctx.queue.nextBatch<Response>("api-response", {
  names: ["response-received"],
  timeout: 30000,
});

// Wait for up to N messages with timeout
const timedBatch = await ctx.queue.nextBatch<Item>("batch-timeout", {
  names: ["item-added"],
  count: 10,
  timeout: 60000,
});
```

**Message delivery:** Messages are loaded once at workflow start. If a message is sent during execution, the workflow yields and picks it up on the next run.

### Join (Parallel - Wait All)

Execute multiple branches in parallel and wait for all to complete.

```typescript
const results = await ctx.join("fetch-all", {
  user: {
    run: async (ctx) => {
      return await ctx.step("get-user", () => fetchUser(userId));
    }
  },
  posts: {
    run: async (ctx) => {
      return await ctx.step("get-posts", () => fetchPosts(userId));
    }
  },
  notifications: {
    run: async (ctx) => {
      return await ctx.step("get-notifs", () => fetchNotifications(userId));
    }
  },
});

// results.user, results.posts, results.notifications are all available
// Type-safe: each branch output type is preserved
```

If any branch fails, all errors are collected into a `JoinError`:

```typescript
import { JoinError } from "@rivetkit/workflow-engine";

try {
  await ctx.join("risky", { /* branches */ });
} catch (error) {
  if (error instanceof JoinError) {
    console.log("Failed branches:", Object.keys(error.errors));
  }
}
```

### Race (Parallel - First Wins)

Execute multiple branches and return when the first completes.

```typescript
const { winner, value } = await ctx.race("timeout-race", [
  {
    name: "work",
    run: async (ctx) => {
      return await ctx.step("do-work", () => doExpensiveWork());
    }
  },
  {
    name: "timeout",
    run: async (ctx) => {
      await ctx.sleep("wait", 30000);
      return null;  // Timeout value
    }
  },
]);

if (winner === "work") {
  console.log("Work completed:", value);
} else {
  console.log("Timed out");
}
```

- Other branches are cancelled via `AbortSignal` when a winner is determined
- If all branches fail, throws `RaceError` with all errors

### Eviction and Cancellation

**Eviction** - Graceful shutdown (workflow can be resumed elsewhere):

```typescript
handle.evict();  // Request shutdown

// In workflow, check eviction status:
if (ctx.isEvicted()) {
  // Clean up and return
}

// Or use the abort signal directly:
await fetch(url, { signal: ctx.abortSignal });
```

**Cancellation** - Permanent stop:

```typescript
await handle.cancel();  // Sets state to "cancelled", clears alarms
```

### Recovery

Reset exhausted retries after a workflow fails. This clears step retry metadata,
removes the workflow error, and schedules the workflow to run again.

```typescript
await handle.recover();  // Resets retry attempts for failed steps
```

### Workflow Migrations (Removed)

When removing steps from workflow code, use `ctx.removed()` to maintain history compatibility:

```typescript
async function myWorkflow(ctx: WorkflowContextInterface) {
  // This step was removed in v2
  await ctx.removed("old-validation", "step");

  // New code continues here
  await ctx.step("new-logic", async () => { /* ... */ });
}
```

This creates a placeholder entry that satisfies history validation without executing anything.

## Configuration Constants

Default values are exported and can be referenced when overriding:

```typescript
import {
  DEFAULT_MAX_RETRIES,        // 3
  DEFAULT_RETRY_BACKOFF_BASE, // 100ms
  DEFAULT_RETRY_BACKOFF_MAX,  // 30000ms
  DEFAULT_LOOP_COMMIT_INTERVAL, // 20 iterations
  DEFAULT_STEP_TIMEOUT,       // 30000ms
} from "@rivetkit/workflow-engine";
```

## Error Types

```typescript
import {
  // User-facing errors
  CriticalError,        // Throw to skip retries
  RollbackError,        // Throw to force rollback
  RollbackCheckpointError, // Rollback used without checkpoint
  StepExhaustedError,   // Step failed after all retries
  JoinError,            // One or more join branches failed
  RaceError,            // All race branches failed
  HistoryDivergedError, // Workflow code changed incompatibly

  // Internal yield errors (caught by runtime)
  SleepError,           // Workflow sleeping
  MessageWaitError,     // Waiting for messages
  EvictedError,         // Workflow evicted

  // User errors
  EntryInProgressError, // Forgot to await a step
  CancelledError,       // Branch cancelled (race)
} from "@rivetkit/workflow-engine";
```

## Workflow States

```typescript
type WorkflowState =
  | "pending"    // Not yet started
  | "running"    // Currently executing
  | "sleeping"      // Waiting for deadline or message
  | "completed"     // Finished successfully
  | "failed"        // Unrecoverable error
  | "rolling_back"  // Running rollback handlers
  | "cancelled";    // Permanently stopped
```

## Best Practices

1. **Unique step names** - Each step/loop/sleep/queue.next within a scope must have a unique name

2. **Deterministic code** - Workflow code outside of steps must be deterministic. Don't use `Math.random()`, `Date.now()`, or read external state outside steps.

3. **Use steps for side effects** - All I/O and non-deterministic operations should be inside steps

4. **Use CriticalError or RollbackError for permanent failures** - When an error is unrecoverable, throw one to avoid wasting retries

5. **Check eviction in long operations** - Use `ctx.isEvicted()` or `ctx.abortSignal` to handle graceful shutdown

6. **Pass AbortSignal to cancellable operations**:
   ```typescript
   await ctx.step("fetch", async () => {
     return fetch(url, { signal: ctx.abortSignal });
   });
   ```

7. **Ephemeral steps for batching** - Use `ephemeral: true` for steps where you want to batch writes:
   ```typescript
   // These batch their writes
   await ctx.step({ name: "a", ephemeral: true, run: async () => { ... }});
   await ctx.step({ name: "b", ephemeral: true, run: async () => { ... }});
   // This flushes all pending writes
   await ctx.step({ name: "c", run: async () => { ... }});
   ```

8. do not try catch in side of a workflow 

9. do not use native loops

## Further Reading

See [architecture.md](./architecture.md) for detailed implementation information including:

- Storage schema and key encoding
- Location system and NameIndex optimization
- Driver requirements
- Error handling internals
- Loop state management and history forgetting
