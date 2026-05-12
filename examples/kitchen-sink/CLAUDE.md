# examples/kitchen-sink/CLAUDE.md

## Testing Against Production (Rivet Cloud)

### Cloud Run Deploys

- Deploy local kitchen-sink builds with `scripts/docker/build-push-kitchen-sink-local.sh`; it builds RivetKit packages with Turbo on the host, packs workspace packages into tarballs, installs a portable app `node_modules`, copies the built NAPI `.node`, and Docker copies only the prepared app.
- Use `PUSH=0` for local image smoke tests and default `PUSH=1` for staging deploy images.
- After building and pushing, update Cloud Run service `kitchen-sink-staging` in project `dev-projects-491221`, region `us-east4`, to the pushed image tag.
- Verify staging with `curl -fsS "$(gcloud run services describe kitchen-sink-staging --region us-east4 --project dev-projects-491221 --format='value(status.url)')/api/rivet/metadata"`.
- Only use the old temp-copy preview-package flow when explicitly validating already-published npm preview packages instead of local workspace code.

Example flow:

```bash
# 1. Build and push a host-built local workspace image.
./scripts/docker/build-push-kitchen-sink-local.sh

# 2. Point Cloud Run at that image.
gcloud run services update kitchen-sink-staging \
  --region us-east4 \
  --project dev-projects-491221 \
  --image "us-east4-docker.pkg.dev/dev-projects-491221/cloud-run-source-deploy/rivet-dev-rivet/rivet-kitchen-sink:$(git rev-parse HEAD)"
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
  -d '{"name":"<actorName>","key":"<key>","runner_name_selector":"k8s","crash_policy":"sleep"}'
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

## Cloud Namespaces

- Cloud project `kitchen-sink-gv34` lives in cloud-staging-473708; both its namespaces route through `api.staging.rivet.dev`.
- Staging namespace: `kitchen-sink-gv34-staging-52gh` — used by Cloud Run service `kitchen-sink-staging` (project `dev-projects-491221`, region `us-east4`).
- Production-tier namespace: `kitchen-sink-gv34-production-d4ob` — defined in cloud-staging but not currently bound to a deployed Cloud Run service.
- Cloud project `kitchen-sink-29a8` lives in cloud-prod-474518; all its namespaces (`production-3591`, `test-N-*`) route through `api.rivet.dev`.
- Cloud project `long-running-62k7` lives in cloud-prod-474518; namespace `production-tik7` is used by Cloud Run service `long-running-test-rivetkit` (project `dev-projects-491221`, region `us-west1`).
- A cloud project's namespaces always live in exactly one cloud DB; the cloud DB's environment determines the engine API host.

## Scripts

### `scripts/sqlite-cold-start-bench.ts` — SQLite cold-read harness

- Keep cold wake/open measured with a tiny SQLite action separately from cold full-read throughput, and keep the main read path free of CPU-heavy diagnostic probes like payload `LIKE` scans.
- The default SQLite cold-start benchmark runs un-compacted and compacted scenarios separately; keep both on inline transaction sizes unless chunked DELTA reads are being explicitly tested.
- Use `cold_start_reverse_probe` for reverse VFS scan measurements; large payload overflow rows create scattered reverse page access.

### `scripts/sqlite-realworld-bench.ts` — SQLite real-world harness

- Measure only server-reported SQLite time for the cold-wake main phase; write comparable JSON results under `.agent/benchmarks/sqlite-realworld/`.

### `scripts/soak.ts` — Cloud Run soak harness

- Drives sustained workload against the live `kitchen-sink-staging` Cloud Run service to verify correctness, validate autoscale, and detect memory leaks in unstable rivetkit code.
- Hardcoded to staging: Cloud Run service `kitchen-sink-staging` (project `dev-projects-491221`, region `us-east4`) and engine namespace `kitchen-sink-gv34-staging-52gh` at `api.staging.rivet.dev`. Never repoint at production from this script.
- Three modes: `--mode=churn` (rapid actor lifecycle for leak detection), `--mode=steady` (stepped actor population for per-actor memory regression), `--mode=scale` (sustained WebSocket concurrency to validate autoscale).
- Forces a fresh Cloud Run revision per run by bumping a `SOAK_RUN_ID` env var via `gcloud run services update`; the new `revision_name` becomes the filter label for all metric/log queries. This is the only supported way to reset memory baseline since Cloud Run has no instance-restart API.
- Pulls CPU, memory, instance_count, and request metrics straight from Cloud Monitoring filtered by `revision_name`; do not add an in-process `/metrics` endpoint to the kitchen-sink server.
- Cloud Run autoscales on `containerConcurrency` and CPU only — memory does NOT trigger scaling. Scale-mode tests must drive concurrent in-flight requests above the cap (long-lived WebSockets are the cheapest way) rather than expecting memory pressure to add instances.
- The script must not mutate Cloud Run service config (`maxScale`, `containerConcurrency`, memory, CPU). Set sane defaults once at the service level (`maxScale=10` recommended so scale mode has headroom). If `churn` or `steady` runs see `instance_count > 1` in the post-hoc metrics, the report flags the run inconclusive rather than averaging across instances.
- Writes JSONL events to `/tmp/soak-<RUN_ID>.jsonl` and prints only high-level progress + the file path to stdout. Append-only; do not interleave verbose logs with progress output.
- Pulls error logs via Cloud Logging filtered by the run's `revision_name` and `severity>=ERROR` and joins them into the same JSONL. Ensure the Cloud Run service has `RIVET_LOG_LEVEL=DEBUG` (already set) and `RUST_LOG=rivetkit_core=debug,info` so rivetkit-core errors surface.
- Companion `scripts/soak-report.ts` re-runs analysis against an existing JSONL so a workload can be re-evaluated without replaying it.
