# Workflow Engine Architecture

This document describes the architecture of the workflow engine, a durable execution system for TypeScript.

## Overview

The workflow engine enables writing long-running, fault-tolerant workflows as regular async functions. Workflows can be interrupted at any point (process crash, eviction, sleep) and resume from where they left off. This is achieved through:

1. **History tracking** - Every operation is recorded in persistent storage
2. **Replay** - On restart, operations replay from history instead of re-executing
3. **Deterministic execution** - Same inputs produce same execution path

## Isolation Model

**Critical architectural assumption**: Each workflow instance operates on an isolated KV namespace.

```
┌─────────────────────────────────────────────────────────────┐
│                      Host System                             │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │   Workflow A    │  │   Workflow B    │                   │
│  │  ┌───────────┐  │  │  ┌───────────┐  │                   │
│  │  │  Engine   │  │  │  │  Engine   │  │                   │
│  │  └─────┬─────┘  │  │  └─────┬─────┘  │                   │
│  │        │        │  │        │        │                   │
│  │  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │                   │
│  │  │  Driver   │  │  │  │  Driver   │  │                   │
│  │  │(isolated) │  │  │  │(isolated) │  │                   │
│  │  └─────┬─────┘  │  │  └─────┬─────┘  │                   │
│  │        │        │  │        │        │                   │
│  │  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │                   │
│  │  │   KV A    │  │  │  │   KV B    │  │                   │
│  │  └───────────┘  │  │  └───────────┘  │                   │
│  └─────────────────┘  └─────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

Key guarantees:

1. **KV Isolation** - Each workflow's `EngineDriver` operates on a completely separate KV namespace. The workflow engine does not include workflow IDs in KV keys because isolation is provided by the driver implementation.

2. **Single Writer** - A workflow instance is the **only** reader and writer of its KV namespace during execution. There is no concurrent access from other workflow instances.

3. **Message Delivery** - Messages are written to the workflow's isolated KV by external systems (via `WorkflowHandle.message()`), then read by the workflow on its next execution. Since each workflow has its own KV, messages are inherently workflow-scoped.

4. **External Mutation** - The only external mutations to a workflow's KV are:
   - Message delivery (appending to message queue)
   - Eviction markers (not yet implemented)

   These are coordinated through the host system's scheduling to avoid conflicts.

This isolation model means:
- The `EngineDriver` interface has no workflow ID parameters for KV operations
- Keys like `messages/0`, `history/...` are relative to each workflow's namespace
- The host system (e.g., Cloudflare Durable Objects, dedicated actor processes) provides the isolation boundary
- Alarms use workflow IDs because they may be managed by a shared scheduler

## Driver Requirements

The `EngineDriver` implementation must satisfy these requirements:

1. **Sorted list results** - `list()` MUST return entries sorted by key in lexicographic byte order. The workflow engine relies on this for:
   - Message FIFO ordering (messages consumed in order received)
   - Name registry reconstruction (names at correct indices)
   - Deterministic replay behavior

2. **Atomic batch writes** - `batch()` SHOULD be atomic (all-or-nothing). If atomicity is not possible, partial writes may cause inconsistent state on crash.

3. **Prefix isolation** - `list(prefix)` and `deletePrefix(prefix)` must only affect keys that start with the exact prefix bytes.

4. **No concurrent modification** - The driver may assume no other writer modifies the KV during workflow execution (see Isolation Model).

## Core Concepts

### Workflow

A workflow is an async function that receives a `WorkflowContext` and optional input:

```typescript
async function myWorkflow(ctx: WorkflowContext, input: MyInput): Promise<MyOutput> {
  const result = await ctx.step("fetch-data", async () => {
    return await fetchData(input.id);
  });
  return result;
}
```

### Entries

Every operation in a workflow creates an **entry** in the history. Entry types:

| Type | Purpose |
|------|---------|
| `step` | Execute arbitrary async code |
| `loop` | Iterate with durable state |
| `sleep` | Wait for a duration or timestamp |
| `message` | Wait for external events |
| `join` | Execute branches in parallel, wait for all |
| `race` | Execute branches in parallel, first wins |
| `removed` | Placeholder for migrated-away entries |

### Location System

Each entry is identified by a **location** - a path through the workflow's execution tree:

```
step("a")           -> [0]           -> "a"
step("b")           -> [1]           -> "b"
loop("outer") {
  step("inner")     -> [2, ~0, 3]    -> "outer/~0/inner"
}
join("parallel") {
  branch "x" {
    step("work")    -> [4, 5, 6]     -> "parallel/x/work"
  }
}
```

#### NameIndex Optimization

Locations use numeric indices into a **name registry** rather than storing strings directly:

```typescript
type NameIndex = number;
type PathSegment = NameIndex | LoopIterationMarker;
type Location = PathSegment[];

// Name registry: ["a", "b", "outer", "inner", "parallel", "x", "work"]
// Location [4, 5, 6] resolves to "parallel/x/work"
```

This optimization reduces storage size when the same names appear many times.

## Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        runWorkflow()                         │
│  Entry point that orchestrates workflow execution            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    WorkflowContextImpl                       │
│  Implements WorkflowContext interface                        │
│  - step(), loop(), sleep(), queue.next(), join(), race()     │
│  - Manages current location                                  │
│  - Creates branch contexts for parallel execution            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         Storage                              │
│  In-memory representation of workflow state                  │
│  - nameRegistry: string[]                                    │
│  - history: Map<string, Entry>                               │
│  - entryMetadata: Map<string, EntryMetadata>                 │
│  - messages: Message[]                                         │
│  - state: WorkflowState                                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       EngineDriver                           │
│  Interface for persistent storage (per-workflow isolated)    │
│  - get/set/delete/list for KV operations                     │
│  - batch for atomic writes                                   │
│  - setAlarm/clearAlarm for scheduled wake-ups                │
│  See "Isolation Model" above for KV scoping                  │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

### First Execution

```
1. runWorkflow() called
2. loadStorage() loads empty state from driver
3. Workflow function executes
4. Each ctx.step() call:
   a. Check history for existing entry (none found)
   b. Create new entry
   c. Execute the step callback
   d. Save output to entry
   e. flush() writes to driver
5. Workflow completes, final flush()
```

### Replay Execution

```
1. runWorkflow() called
2. loadStorage() loads previous state from driver
3. Workflow function executes
4. Each ctx.step() call:
   a. Check history for existing entry (found!)
   b. Entry has output -> return immediately (no callback execution)
5. Workflow continues from where it left off
```

### Sleep/Message Yielding

```
1. ctx.sleep() or ctx.queue.next() called
2. Check if deadline passed or message available from messageDriver.receiveMessages()
3. If not ready:
   a. Throw SleepError or MessageWaitError
   b. runWorkflow() catches error
   c. flush() saves current state
   d. setAlarm() schedules wake-up (for sleep)
   e. Return { state: "sleeping", ... }
4. External scheduler calls runWorkflow() again when ready
5. loadStorage() loads workflow state/history
6. Replay proceeds, sleep/message now succeeds
```

### Message Delivery Model

**Important**: The workflow's KV is only mutated by the workflow engine itself during execution. Messages are handled exclusively by the message driver:

```
1. External system sends message to workflow
2. WorkflowHandle.message() forwards it to EngineDriver.messageDriver.addMessage()
3. External system or runtime triggers workflow wake-up
4. runWorkflow() called
5. Workflow calls messageDriver.receiveMessages() from ctx.queue.next()
6. If message found: consume and continue
7. If not found: yield with MessageWaitError
```

The workflow engine never stores queue messages in workflow KV.

## Storage Schema

Data is stored using binary key encoding with fdb-tuple for proper byte ordering:

```
Key Format (binary tuples):                  Value Format:
[1, index]                                -> BARE (name string)
[2, ...locationSegments]                  -> BARE+versioning (Entry)
[3, 1]                                    -> text (WorkflowState)
[3, 2]                                    -> CBOR (workflow output)
[3, 3]                                    -> CBOR (WorkflowError)
[3, 4]                                    -> text (version)
[3, 5]                                    -> CBOR (workflow input)
[4, entryId]                              -> BARE+versioning (EntryMetadata)

Key prefixes:
1 = NAMES         - Name registry
2 = HISTORY       - History entries
3 = WORKFLOW      - Workflow metadata
4 = ENTRY_METADATA - Entry metadata

Location segments in keys:
- NameIndex (number) -> encoded directly
- LoopIterationMarker -> [loopIdx, iteration] nested tuple
```

The fdb-tuple encoding ensures:
- Proper lexicographic byte ordering for `list()` operations
- Compact representation for numeric indices
- Nested tuples for complex segments (loop iterations)

### Entry Structure

```typescript
interface Entry {
  id: string;           // UUID
  location: Location;   // Path in execution tree
  kind: EntryKind;      // Type-specific data
  dirty: boolean;       // Needs flushing?
}

interface StepEntry {
  type: "step";
  data: {
    output?: unknown;   // Successful result
    error?: string;     // Error message if failed
  };
}
```

### Metadata Structure

```typescript
interface EntryMetadata {
  status: "pending" | "running" | "completed" | "failed" | "exhausted";
  attempts: number;
  lastAttemptAt: number;
  createdAt: number;
  completedAt?: number;
  dirty: boolean;
}
```

## Error Handling

### Retryable Errors

Regular errors thrown from step callbacks trigger retry logic:

1. Error caught, saved to entry
2. `StepFailedError` thrown
3. On next run, metadata.attempts checked
4. If attempts < maxRetries, apply backoff and retry
5. If attempts >= maxRetries, throw `StepExhaustedError`

### Critical Errors

`CriticalError` and `RollbackError` bypass retry logic:

```typescript
await ctx.step("validate", async () => {
  if (!isValid(input)) {
    throw new CriticalError("Invalid input");
  }
});

await ctx.step("halt", async () => {
  throw new RollbackError("Stop and roll back");
});
```

### Yield Errors

These message the workflow should pause:

| Error | Meaning |
|-------|---------|
| `SleepError` | Waiting for a deadline |
| `MessageWaitError` | Waiting for messages |
| `EvictedError` | Workflow being moved to another worker |

## Parallel Execution

### Join (All)

```typescript
const results = await ctx.join("fetch-all", {
  user: { run: async (ctx) => await ctx.step("user", fetchUser) },
  posts: { run: async (ctx) => await ctx.step("posts", fetchPosts) },
});
// results.user and results.posts available
```

- All branches execute concurrently
- Waits for ALL branches to complete
- If any branch fails, collects all errors into `JoinError`
- Branch state tracked: pending -> running -> completed/failed

### Race (First)

```typescript
const { winner, value } = await ctx.race("timeout", [
  { name: "work", run: async (ctx) => await doWork(ctx) },
  { name: "timeout", run: async (ctx) => { await ctx.sleep("wait", 5000); return null; } },
]);
```

- All branches execute concurrently
- Returns when FIRST branch completes
- Other branches are cancelled via AbortMessage
- Winner tracked in history for replay

## Loop State Management

Loops maintain durable state across iterations:

```typescript
await ctx.loop({
  name: "process-items",
  state: { cursor: null, processed: 0 },
  commitInterval: 10,
  historyEvery: 10,
  historyKeep: 10,
  run: async (ctx, state) => {
    const batch = await ctx.step("fetch", () => fetchBatch(state.cursor));
    if (!batch.items.length) {
      return Loop.break(state.processed);
    }
    await ctx.step("process", () => processBatch(batch.items));
    return Loop.continue({
      cursor: batch.nextCursor,
      processed: state.processed + batch.items.length,
    });
  },
});
```

### Commit Interval

- State is persisted every `commitInterval` iterations
- On crash, replay resumes from last committed state

### History Retention

Loop history is trimmed every `historyEvery` iterations, keeping the most recent `historyKeep` iterations. Rollback only replays the last retained iteration.

After trimming (historyEvery=10, historyKeep=10):

```
Before trim at iteration 20:
  process-items/~0/fetch, ~0/process
  process-items/~1/fetch, ~1/process
  ...
  process-items/~19/fetch, ~19/process

After trim:
  process-items/~10/fetch, ~10/process  (kept)
  ...
  process-items/~19/fetch, ~19/process  (kept)
  // Iterations 0-9 deleted
```

## Dirty Tracking

To minimize writes, entries track a `dirty` flag:

1. New entries created with `dirty: true`
2. Modified entries set `dirty = true`
3. `flush()` only writes entries where `dirty === true`
4. After write, `dirty = false`

This means replay operations that don't modify state don't trigger writes.

## Design Decisions

### Why Path-Based Locations?

Alternative: Coordinate-based (index into flat array)

Path-based advantages:
- Human-readable keys for debugging
- Natural hierarchy for nested structures
- Prefix-based queries for loop cleanup
- Stable across code changes (names vs positions)

### Why NameIndex?

Locations could store strings directly, but:
- Same names repeat frequently (e.g., "step-1" in every loop iteration)
- Numeric indices compress better
- Registry loaded once, indices resolved in memory

### Why Dirty Tracking?

Could flush everything on every operation, but:
- Replay would write identical data
- Batch operations would have redundant writes
- Dirty tracking makes replay essentially read-only

### Why Sequential Test Execution?

Tests share a module-level `driver` variable via `beforeEach`. While each test gets a fresh driver, Vitest's parallel execution caused race conditions. Sequential execution ensures isolation.

## Future Considerations

1. **Version checking** - Detect workflow code changes
2. **Compaction** - Merge history entries to reduce size
3. **Sharding** - Distribute workflow state across multiple keys
4. **Observability** - Structured logging, metrics, tracing
5. **Workflow composition** - Child workflows, messages between workflows
