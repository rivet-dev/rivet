# Advanced

This document covers advanced behavior that impacts long-term workflow operation.

## History Retention

Workflow history grows as entries are added. The engine provides a few built-in mechanisms to limit growth:

- Loop iterations are compacted using `historyEvery` and `historyKeep`. Older iterations are deleted after each retention window, so rollback only replays the last retained iteration.
- `ctx.race()` removes history for losing branches after a winner is chosen.
- `ctx.removed()` lets you keep history compatible while removing old entries.

If you need to delete an entire workflow’s history, remove its driver namespace or use a new workflow ID.

## OpenTelemetry Observability

The workflow engine does not ship with built-in OpenTelemetry tracing. To add observability:

- Wrap step callbacks with your tracing spans.
- Use step, join, and race names as span identifiers.
- Instrument your driver implementation to emit metrics for storage and scheduling operations.

This approach keeps tracing deterministic while still giving you visibility into workflow execution.

## Related

- `rivetkit-typescript/packages/workflow-engine/architecture.md:90` for history entry details.
