# examples/sandbox/CLAUDE.md

## Testing Against Production (Rivet Cloud)

The sandbox is deployed on Railway via Rivet Cloud. To test actors and inspect their SQLite databases, use the Rivet gateway API.

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

The sandbox has three SQLite actor types to test:

| Actor | DB Type | Actions |
|-------|---------|---------|
| `sqliteRawActor` | Raw `db()` from `rivetkit/db` | `addTodo`, `getTodos`, `toggleTodo`, `deleteTodo` |
| `sqliteDrizzleActor` | Drizzle `db()` from `rivetkit/db/drizzle` | `addTodo`, `getTodos`, `toggleTodo`, `deleteTodo` |
| `parallelismTest` | Raw `db()` + state | `incrementState`, `getStateCount`, `incrementSqlite`, `getSqliteCount` |
