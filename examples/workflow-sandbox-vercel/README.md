> **Note:** This is the Vercel-optimized version of the [workflow-sandbox](../workflow-sandbox) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fworkflow-sandbox-vercel&project-name=workflow-sandbox-vercel)

# Workflow Sandbox

Interactive sandbox for testing all RivetKit workflow patterns.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/workflow-sandbox
npm install
npm run dev
```


## Features

This example demonstrates all workflow features through a tabbed interface:

- **Steps** - Multi-step order processing with automatic retries
- **Sleep** - Durable countdown timers that survive restarts
- **Loops** - Batch processing with persistent cursor state
- **Listen** - Approval queue with timeout-based decisions
- **Join** - Parallel data fetching (wait-all pattern)
- **Race** - Work vs timeout (first-wins pattern)
- **Rollback** - Compensating transactions with rollback handlers

## Implementation

Each workflow pattern is implemented as a separate actor with its own state and workflow loop. The key patterns demonstrated:

### Steps
```typescript
await loopCtx.step("validate", async () => { /* ... */ });
await loopCtx.step("charge", async () => { /* ... */ });
await loopCtx.step("fulfill", async () => { /* ... */ });
```

### Sleep
```typescript
await loopCtx.sleep("countdown", durationMs);
```

### Loops
```typescript
await ctx.loop({
  name: "batch-loop",
  state: { cursor: 0 },
  run: async (loopCtx, state) => {
    // Process batch
    return Loop.continue({ cursor: state.cursor + 1 });
  },
});
```

### Listen
```typescript
const decision = await loopCtx.listenWithTimeout<Decision>(
  "wait-decision",
  "decision-queue",
  30000 // 30 second timeout
);
```

### Join
```typescript
const results = await loopCtx.join("fetch-all", {
  users: { run: async (ctx) => fetchUsers() },
  orders: { run: async (ctx) => fetchOrders() },
});
```

### Race
```typescript
const { winner, value } = await loopCtx.race("work-vs-timeout", [
  { name: "work", run: async (ctx) => doWork() },
  { name: "timeout", run: async (ctx) => ctx.sleep("wait", timeout) },
]);
```

### Rollback
```typescript
await loopCtx.rollbackCheckpoint("payment-checkpoint");

await loopCtx.step({
  name: "charge-card",
  run: async () => chargeCard(),
  rollback: async () => refundCard(),
});
```

See the implementation in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/workflow-sandbox/src/actors.ts).

## Resources

Read more about [workflows](/docs/workflows).

## License

MIT
