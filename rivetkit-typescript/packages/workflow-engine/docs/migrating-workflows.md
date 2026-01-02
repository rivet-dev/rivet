# Migrating Workflows

Workflow history is durable, which means the workflow engine replays past entries on restart. When you change workflow code, you have to keep history compatibility in mind so replays still match. This document covers user-facing migration rules and the helpers available.

## Compatibility Rules

- Step, loop, join, race, sleep, and message names are part of the workflow history. Renaming or removing them without a migration will cause a `HistoryDivergedError` on replay.
- Code inside `ctx.step()` callbacks can change freely because the step result is replayed instead of re-executed.
- The order of entries matters. Moving a step before or after another entry can break replay unless you migrate the old location.
- Adding new entries is safe as long as they are after existing ones or gated behind new logic that only runs for new workflows.

## Removing or Renaming Entries

Use `ctx.removed()` to preserve compatibility when you remove or rename an entry. This writes a placeholder entry into history so the replay tree stays aligned.

```ts
import { WorkflowContextInterface } from "@rivetkit/workflow-engine";

async function checkoutWorkflow(ctx: WorkflowContextInterface, orderId: string) {
  // Step removed in v2, keep name/location compatible
  await ctx.removed("validate-cart", "step");

  // New step name replaces the old one
  await ctx.step("validate-order", async () => validateOrder(orderId));
}
```

`ctx.removed()` accepts the original name and entry type (`"step"`, `"loop"`, `"join"`, `"race"`, `"sleep"`, or `"message"`).

## Strategy for Safe Migrations

- Prefer additive changes. Add new steps while leaving old ones in place until you can safely migrate.
- If you need to move logic, keep the old step name and add a new step name for the new logic.
- For renames, remove the old entry with `ctx.removed()` and introduce the new entry at the new location.
- Avoid branching on `Math.random()` or `Date.now()` outside of steps, because non-determinism will diverge history.

## Versioning Inputs and Outputs

Workflow input is persisted on first run. If you evolve the input schema, consider versioning the input or handling both shapes:

```ts
type CheckoutInput =
  | { version: 1; orderId: string }
  | { version: 2; orderId: string; coupons?: string[] };

async function checkoutWorkflow(ctx: WorkflowContextInterface, input: CheckoutInput) {
  const orderId = input.orderId;
  const coupons = input.version === 2 ? input.coupons ?? [] : [];
  await ctx.step("charge", () => charge(orderId, coupons));
}
```

## When to Start Fresh

If the workflow history can be discarded, create a new workflow ID or a new workflow type. This lets you avoid complex migrations at the cost of losing the old history for those instances.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:54` for an overview of `ctx.removed()`.
- `rivetkit-typescript/packages/workflow-engine/architecture.md:88` for history and location details.
