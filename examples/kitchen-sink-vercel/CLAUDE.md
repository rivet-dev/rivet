# examples/kitchen-sink/CLAUDE.md

## Testing Against Production (Rivet Cloud)

### Cloud Run Deploys

- Deploy the kitchen-sink to Cloud Run from an isolated temp build context that pins the published `rivetkit` preview version, so root workspace `resolutions` do not silently swap in local packages.
- Copy `examples/kitchen-sink` to a temp directory and edit that temp copy instead of building from the monorepo root.
- Pin the temp copy to the exact published preview packages you want to test, such as `rivetkit@0.0.0-pr.4667.33279e9` and `@rivetkit/react@0.0.0-pr.4667.33279e9`.
- Build and push the image from that temp context, then update the target Cloud Run service to that image.
- Do not build the repo workspace directly when validating a published preview package, because the root `package.json` `resolutions` will route the app back to local workspace packages.

Example flow:

```bash
# 1. Copy the kitchen-sink out of the workspace.
cp -R examples/kitchen-sink /tmp/kitchen-sink-cloud-run

# 2. Edit /tmp/kitchen-sink-cloud-run/package.json to pin the published preview packages.

# 3. Build and push the image from the temp context.
docker build -t us-east4-docker.pkg.dev/<project>/<repo>/<image>:<tag> /tmp/kitchen-sink-cloud-run
docker push us-east4-docker.pkg.dev/<project>/<repo>/<image>:<tag>

# 4. Point Cloud Run at that image.
gcloud run services update <service> \
  --region us-east4 \
  --project <project> \
  --image us-east4-docker.pkg.dev/<project>/<repo>/<image>:<tag>
```

The kitchen-sink is deployed on Railway via Rivet Cloud. To test actors and inspect their SQLite databases, use the Rivet gateway API.

See [Debugging Docs](https://rivet.dev/docs/actors/debugging) for full inspector documentation.

### Setup

Set your namespace and token as environment variables:

```bash
export RIVET_NS="<namespace>"
export RIVET_TOKEN="<token>"
export GW="https://api.rivet.dev/gateway"
```

### Create or Get an Actor

```bash
curl -s -X PUT "https://api.rivet.dev/actors?namespace=${RIVET_NS}" \
  -H "Authorization: Bearer ${RIVET_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"name":"<actorName>","key":"<key>","runner_name_selector":"default","crash_policy":"sleep"}'
```

This returns `{"actor":{"actor_id":"<ACTOR_ID>", ...}, "created": true/false}`.

### Call an Action

```bash
curl -s -X POST "${GW}/<ACTOR_ID>@${RIVET_TOKEN}/action/<actionName>" \
  -H 'Content-Type: application/json' \
  -H 'x-rivet-encoding: json' \
  -d '{"args":[...]}'
```

### Inspector Endpoints

All inspector endpoints require the actor ID in the gateway URL path:

```bash
# Database schema (tables, columns, record counts)
curl -s "${GW}/<ACTOR_ID>@${RIVET_TOKEN}/inspector/database/schema" \
  -H "Authorization: Bearer ${RIVET_TOKEN}"

# Database rows for a specific table
curl -s "${GW}/<ACTOR_ID>@${RIVET_TOKEN}/inspector/database/rows?table=<table>&limit=100" \
  -H "Authorization: Bearer ${RIVET_TOKEN}"

# Actor metrics (KV ops, SQL statements, action counts)
curl -s "${GW}/<ACTOR_ID>@${RIVET_TOKEN}/inspector/metrics" \
  -H "Authorization: Bearer ${RIVET_TOKEN}"

# Full summary (state, connections, RPCs, queue, workflow)
curl -s "${GW}/<ACTOR_ID>@${RIVET_TOKEN}/inspector/summary" \
  -H "Authorization: Bearer ${RIVET_TOKEN}"
```

### SQLite Actor Types

The kitchen-sink has three SQLite actor types to test:

| Actor | DB Type | Actions |
|-------|---------|---------|
| `sqliteRawActor` | Raw `db()` from `rivetkit/db` | `addTodo`, `getTodos`, `toggleTodo`, `deleteTodo` |
| `sqliteDrizzleActor` | Drizzle `db()` from `rivetkit/db/drizzle` | `addTodo`, `getTodos`, `toggleTodo`, `deleteTodo` |
| `parallelismTest` | Raw `db()` + state | `incrementState`, `getStateCount`, `incrementSqlite`, `getSqliteCount` |
