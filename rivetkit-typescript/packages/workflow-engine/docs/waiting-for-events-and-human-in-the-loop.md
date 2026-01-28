# Waiting for Events & Human-in-the-Loop

Workflows can pause until external events arrive. This enables human approvals, webhook-driven workflows, and long-lived processes that respond to external signals.

## Message Delivery Model

- Messages are persisted via `handle.message()`.
- Messages are loaded at workflow start.
- If a message arrives while the workflow is running, the workflow yields and picks it up on the next run.

In live mode (`runWorkflow(..., { mode: "live" })`), incoming messages can also wake a workflow waiting on `ctx.listen()`.

## Listening for Messages

```ts
const approval = await ctx.listen<string>("wait-approval", "approval-granted");
```

Use the `listen*` variants to wait for multiple messages or apply timeouts:

```ts
const items = await ctx.listenN("batch", "item-added", 10);
const result = await ctx.listenWithTimeout("approval", "approval-granted", 60000);
```

## Deadlines and Timeouts

Use `listenUntil` or `listenWithTimeout` to model approval windows:

```ts
const approval = await ctx.listenUntil(
  "approval-window",
  "approval-granted",
  Date.now() + 24 * 60 * 60 * 1000,
);
```

If the deadline passes, the method returns `null` instead of throwing.

## Human-in-the-Loop Example

```ts
const approval = await ctx.listenWithTimeout(
  "manual-approval",
  "approval-granted",
  30 * 60 * 1000,
);

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

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:207` for listen helpers.
- `rivetkit-typescript/packages/workflow-engine/architecture.md:218` for message delivery details.
