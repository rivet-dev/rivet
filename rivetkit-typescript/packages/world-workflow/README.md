# @rivetkit/world-workflow

Vercel Workflow SDK [World](https://useworkflow.dev/docs/deploying/building-a-world)
implementation backed by [Rivet Actors](https://rivet.dev/docs/actors).

This package replaces traditional Postgres + queue infrastructure with three
Rivet actors:

- `workflowRun` — one per workflow run. Owns the append-only event log,
  materialized run/step/hook state, and per-run streams.
- `coordinator` — singleton. Indexes runs for cross-run queries and enforces
  global hook token uniqueness.
- `queueRunner` — one per `__wkf_workflow_*` / `__wkf_step_*` queue. Wraps a
  Rivet durable queue with retry and idempotency handling.

## Status

Early scaffold. The package compiles as a standalone workspace member and
provides working implementations for the Storage, Queue, and most of the
Streamer surface. Known gaps:

- `readFromStream` live streaming is not yet wired to the `streamAppended`
  actor event. Use `getStreamChunks` to drain chunks in a loop.
- `hooks.get(hookId)` does not have a global index; callers must use
  `hooks.getByToken`.
- `events.listByCorrelationId` returns an empty result pending a
  coordinator-level correlation index.

The goal is to pass the 84 E2E tests in the Workflow SDK test suite; each gap
above is a concrete follow-up.

## Usage

```ts
import { createRivetWorld, registry } from "@rivetkit/world-workflow";

// Start the registry (e.g. as part of your server entry point)
registry.start();

// Build a World the Workflow SDK can target
export const world = createRivetWorld({
	endpoint: process.env.RIVET_ENDPOINT,
});
```

Then set `WORKFLOW_TARGET_WORLD=@rivetkit/world-workflow` to activate it.

## Development

```bash
pnpm -F @rivetkit/world-workflow build
pnpm -F @rivetkit/world-workflow check-types
pnpm -F @rivetkit/world-workflow test
```
