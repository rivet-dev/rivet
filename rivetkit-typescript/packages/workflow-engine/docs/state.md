# State

Workflow state is persisted through the engine driver so workflows can resume deterministically after restarts. This document covers the user-facing state model and how to access it.

## Workflow State Machine

Workflows move through these states:

- `pending`: not started yet
- `running`: currently executing
- `sleeping`: waiting for a deadline or message
- `rolling_back`: executing rollback handlers
- `failed`: failed after rollback
- `completed`: finished successfully
- `cancelled`: permanently stopped

## Stored Workflow Data

The engine persists:

- Workflow input (captured on first run)
- Workflow output (when completed)
- Workflow error metadata (when failed)
- History entries for steps, loops, sleeps, joins, races, messages, and rollback checkpoints

## Reading State

Use the workflow handle to query current state or output:

```ts
const state = await handle.getState();
const output = await handle.getOutput();
```

## Durable Workflow Data

There is no mutable shared state inside a workflow. To make data durable:

- Return values from `ctx.step()` and pass them forward.
- Use `ctx.loop()` state for iterative workflows.
- Avoid storing critical data in module-level variables, which are not persisted.

## Related

- `rivetkit-typescript/packages/workflow-engine/QUICKSTART.md:406` for workflow state enum.
