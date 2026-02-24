# Waiting for Events & Human-in-the-Loop

Workflows can pause until external events arrive. This enables human approvals, webhook-driven workflows, and long-lived processes that respond to external signals.

## Message Delivery Model

- Messages are persisted via `handle.message()`.
- Messages are loaded at workflow start.
- If a message arrives while the workflow is running, the workflow yields and picks it up on the next run.

In live mode (`runWorkflow(..., { mode: "live" })`), incoming messages can also wake a workflow waiting on `ctx.queue.next()` / `ctx.queue.nextBatch()`.

## Waiting for Queue Messages

```ts
const approval = await ctx.queue.next<string>("wait-approval", {
  names: ["approval-granted"],
});
```

Use `nextBatch` with `count` / `timeout` to wait for batches or apply deadlines:

```ts
const items = await ctx.queue.nextBatch("batch", {
  names: ["item-added"],
  count: 10,
});
const [result] = await ctx.queue.nextBatch("approval", {
  names: ["approval-granted"],
  timeout: 60000,
});
```

## Deadlines and Timeouts

Use `timeout` to model approval windows:

```ts
const [approval] = await ctx.queue.nextBatch("approval-window", {
  names: ["approval-granted"],
  timeout: 24 * 60 * 60 * 1000,
});
```

If the deadline passes, `ctx.queue.nextBatch(...)` returns `[]`.

## Human-in-the-Loop Example

```ts
const [approval] = await ctx.queue.nextBatch("manual-approval", {
  names: ["approval-granted"],
  timeout: 30 * 60 * 1000,
});

if (!approval) {
  await ctx.step("notify-timeout", () => sendTimeoutNotice());
  return "timed-out";
}

await ctx.step("proceed", () => runApprovedWork());
```

## Best Practices

- Keep message names stable and unique per scope.
- Store any state you need for follow-up steps inside step outputs.
- Use `handle.wake()` or send another message if you need to resume a yielded workflow.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:207` for queue waits.
- `rivetkit-typescript/packages/workflow-engine/architecture.md:218` for message delivery details.
