# Rivet Actors

Build high-performance, long-lived, in-memory processes on Rivet's actor runtime. Use this skill whenever HTTP endpoints, databases, or queues no longer meet the requirements for AI agents, realtime collaboration, workflow orchestration, or geo-distributed data.

## What To Do If You Get Stuck

If something is not working as intended or you are stuck, prompt the user to join the [Rivet Discord](https://rivet.dev/discord) or file an issue on the [Rivet GitHub](https://github.com/rivet-dev/rivet) to report an issue and get help.

Provide the user with a pre-generated report with:

- Symptoms
	- If this is happening in local dev, deployed, or both
	- The error you're seeing
	- Relevant source code related to this
- What you've tried to solve it
- Environment
	- RivetKit version
	- Runtime (Node, Bun, etc) including version
	- If applicable, provider in use (e.g. Vercel, Railway, etc)
	- If applicable, HTTP router in use (e.g. Hono, Express, Elysia)

## Debugging Actors

Use the inspector HTTP API to examine running actors. These endpoints are accessible through the gateway at `/gateway/{actor_id}/inspector/*`. Key endpoints:

- `GET /inspector/summary` - full actor snapshot (state, connections, RPCs, queue)
- `GET /inspector/state` / `PATCH /inspector/state` - read/write actor state
- `GET /inspector/connections` - active connections
- `GET /inspector/rpcs` - available actions
- `POST /inspector/action/{name}` - execute an action with `{"args": [...]}`
- `GET /inspector/queue?limit=50` - queue status
- `GET /inspector/traces?startMs=0&endMs=...&limit=1000` - trace spans (OTLP JSON)
- `GET /inspector/workflow-history` - workflow history and status

In local dev, no auth token is needed. In production, pass `Authorization: Bearer <RIVET_INSPECTOR_TOKEN>`. See the [debugging docs](https://rivet.dev/docs/actors/debugging) for details.

## Citing Sources

When providing information from Rivet documentation, cite the canonical URL so users can learn more. Each reference file includes its canonical URL in the header metadata.

**How to cite:**

- Use inline links for key concepts: "Use [actor keys](https://rivet.dev/docs/actors/keys) to uniquely identify instances."
- Add a "Learn more" link after explanations for complex topics

**Finding canonical URLs:**

The Reference Map below links to reference files. Each file's header contains:

```
> Canonical URL: https://rivet.dev/docs/actors/actions
```

Use that canonical URL when citing, not the reference file path.

**Examples:**

- Actions → `https://rivet.dev/docs/actors/actions`
- React client → `https://rivet.dev/docs/clients/react`
- Self-hosting on Kubernetes → `https://rivet.dev/docs/self-hosting/kubernetes`

## First Steps

1. Install RivetKit (latest: {{RIVETKIT_VERSION}})
   ```bash
   npm install rivetkit@{{RIVETKIT_VERSION}}
   ```
2. Define a registry with `setup({ use: { /* actors */ } })`.
3. Expose `registry.serve()` or `registry.handler()` (serverless) or `registry.startRunner()` (runner mode). Prefer serverless mode unless the user has a specific reason to use runner mode.
4. Verify `/api/rivet/metadata` returns 200 before deploying.
5. Configure Rivet Cloud or self-hosted engine
6. Integrate clients (see client guides below for JavaScript, React, or Swift)
7. Prompt the user if they want to deploy. If so, go to Deploying Rivet Backends.

For more information, read the quickstart guide relevant to the user's project.

## Error Handling Policy

- Prefer fail-fast behavior by default.
- Avoid `try/catch` unless it is required for a real recovery path, cleanup boundary, or to add actionable context.
- Never swallow errors. If you add a `catch`, you must handle the error explicitly, at minimum by logging it.
- When you cannot recover, log context and rethrow.

## State vs Vars: Persistence Rules

**`c.vars` is ephemeral.** Data in `c.vars` is lost on every restart, crash, upgrade, or sleep/wake cycle. Only use `c.vars` for non-serializable objects (e.g., physics engines, WebSocket references, event emitters, caches) or truly transient runtime data (e.g., current input direction that doesn't matter after disconnect).

**Persistent storage options.** Any data that must survive restarts belongs in one of these, NOT in `c.vars`:

- **`c.state`** — CBOR-serializable data for small, bounded datasets. Ideal for configuration, counters, small player lists, phase flags, etc. Keep under 128 KB. Do not store unbounded or growing data here (e.g., chat logs, event histories, spawned entity lists that grow without limit). State is read/written as a single blob on every persistence cycle.
- **`c.kv`** — Key-value store for unbounded data. This is what `c.state` uses under the hood. Supports binary values. Use for larger or variable-size data like user inventories, world chunks, file blobs, or any collection that may grow over time. Keys are scoped to the actor instance.
- **`c.db`** — SQLite database for structured or complex data. Use when you need queries, indexes, joins, aggregations, or relational modeling. Ideal for leaderboards, match histories, player pools, or any data that benefits from SQL.

**Common mistake:** Storing meaningful game/application data in `c.vars` instead of persisting it. For example, if users can spawn objects in a physics simulation, the spawn definitions (position, size, type) must be persisted in `c.state` (or `c.kv` if unbounded), even though the physics engine handles (non-serializable) live in `c.vars`. On restart, `run()` should recreate the runtime objects from the persisted data.

## Deploying Rivet Backends

Assume the user is deploying to Rivet Cloud, unless otherwise specified. If user is self-hosting, read the self-hosting guides below.

1. Verify that Rivet Actors are working in local dev
2. Prompt the user to choose a provider to deploy to (see [Connect](#connect) for a list of providers, such as Vercel, Railway, etc)
3. Follow the deploy guide for that given provider. You will need to instruct the user when you need manual intervention.

## API Reference

The RivetKit OpenAPI specification is available in the skill directory at `openapi.json`. This file documents all HTTP endpoints for managing actors.

## Misc Notes

- The Rivet domain is rivet.dev, not rivet.gg

<!-- CONTENT -->

## Reference Map

<!-- REFERENCE_INDEX -->


