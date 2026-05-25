# examples/kitchen-sink/CLAUDE.md

## Testing Against Production (Rivet Cloud)

### Rivet Cloud Managed-Pool Deploys

- The `Rivet Deploy (kitchen-sink)` GitHub Action (`.github/workflows/rivet-deploy-kitchen-sink.yml`) builds `examples/kitchen-sink/Dockerfile` from the repo root and ships it to a Rivet Cloud managed pool via `rivet-dev/deploy-action@v1.1.0`.
- The workflow only triggers on changes under `examples/kitchen-sink/**`, the bundled `rivetkit-typescript/packages/**`, `engine/sdks/typescript/**`, and `shared/typescript/**` (everything the Dockerfile actually copies).
- The Dockerfile bakes `ENV PORT=8080` and `ENV KITCHEN_SINK_SERVERLESS_URL=cloud` so the container listens on 8080 and `server.ts` enters serverless mode (serves `/api/rivet/*` via the registry handler instead of calling `registry.start()`). Do NOT use `RIVET_RUN_ENGINE=1` — that flag starts an embedded engine, which conflicts with the engine endpoint that Rivet Cloud injects (`ZodError: cannot specify both startEngine and endpoint`). The `managed-pool-config.environment` field on the deploy-action is not currently honored by the cloud API, so do not rely on it; configure runtime env via `ENV` in the Dockerfile.
- The CI workflow prebuilds `rivetkit-napi.linux-x64-gnu.node` via `docker/build/linux-x64-gnu.Dockerfile` and the runtime Dockerfile copies the whole built `/app` workspace tree (not `pnpm deploy`). `pnpm deploy --legacy` leaves workspace packages as symlinks back to `/app/rivetkit-typescript/...` which then dangle in the runtime stage, so the kitchen-sink image keeps the workspace source + symlinked `node_modules` + chunked `.pnpm` store instead. Image size is large but registry-friendly because the `.pnpm` store is split into 8 chunked `COPY` layers (the registry 503s on a single large layer).
- Token is the `KITCHEN_SINK_RIVET_CLOUD_TOKEN` repo secret (a `cloud_api_*` token scoped to cloud-api.rivet.dev for this kitchen-sink project only). Do not confuse with the engine `pk_*` token used for actor/gateway calls.

### Cloud Run Deploys

There are two Cloud Run services maintained for the kitchen-sink in `dev-projects-491221` / `us-east4`:

- `kitchen-sink-staging` → engine namespace `kitchen-sink-gv34-staging-52gh` on `api.staging.rivet.dev`.
- `rivet-kitchen-sink` → engine namespace under `kitchen-sink-29a8-cloud-run-*` on `api.rivet.dev`.

#### Deploying the current workspace (with local rivetkit changes)

Use [`scripts/deploy-cloud-run.sh`](file:///home/nathan/r8/examples/kitchen-sink/scripts/deploy-cloud-run.sh). It builds [`Dockerfile`](file:///home/nathan/r8/examples/kitchen-sink/Dockerfile) from the monorepo root, tags with `manual-<sha>`, pushes to the `cloud-run-source-deploy` Artifact Registry repo, and updates `rivet-kitchen-sink` (prod). Pass `--also-staging` to also update `kitchen-sink-staging`. The script curls `/api/rivet/health` after each update to verify the new revision came up.

```bash
# Build the napi binary once if it's missing.
cd rivetkit-typescript/packages/rivetkit-napi && pnpm build:release && cd -

# Deploy.
examples/kitchen-sink/scripts/deploy-cloud-run.sh
```

Things that must be true for the deploy to actually start serving on Rivet Cloud:

- Container listens on `$PORT` (default 8080). The [Dockerfile](file:///home/nathan/r8/examples/kitchen-sink/Dockerfile) bakes `ENV PORT=8080`.
- `server.ts` must enter serverless mode (`registry.handler(...)`, not `registry.start()`). The Dockerfile sets `ENV KITCHEN_SINK_SERVERLESS_URL=cloud` to force that. Do NOT use `RIVET_RUN_ENGINE=1` — it also turns on `startEngine`, which collides with the engine endpoint Rivet Cloud injects (`ZodError: cannot specify both startEngine and endpoint`).
- `serverlessPoolConfig()` in [src/index.ts](file:///home/nathan/r8/examples/kitchen-sink/src/index.ts) returns `undefined` whenever `_RIVET_COMPUTE=1` (Rivet Cloud managed compute) **or** `SANDBOX_MODE=serverless` (Cloud Run via Rivet's deploy pipeline) is set. The platform configures the runner pool itself; the in-process `configurePool` would try to hit `GET /datacenters`, which the per-namespace `sk_` token cannot do (`no permission to list datacenters in namespace any with target any`).
- Rivet-injected envs on Cloud Run: `RIVET_PUBLIC_ENDPOINT` (pk_) + `RIVET_ENDPOINT` (sk_) + `SANDBOX_MODE=serverless` + (on the managed-pool path) `RIVET_RUNNER_VERSION` + `_RIVET_COMPUTE=1`. Do not duplicate them in the image.

#### Deploying a published rivetkit preview build instead

When validating a published rivetkit preview (instead of the local workspace), build from an isolated temp context so the root `package.json` `resolutions` do not silently swap the published package back to the workspace copy:

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

### Railway Deploys

The kitchen-sink also runs on Railway as a long-lived ("serverful") fleet for load tests. Railway pulls a pre-built Docker image — it does NOT build from the repo. Railway's RAILPACK auto-build cannot reproduce our Dockerfile because it depends on the pre-built `rivetkit-napi.linux-x64-gnu.node` being copied into `examples/kitchen-sink/` (gitignored).

Service info:
- Project: `kitchen-sink` (`cec5b904-ff03-4621-9bb7-f82f9678be68`)
- Environment: `production` (`615d08c3-5eca-4a4c-baaf-352807974d9d`)
- Service: `rivet` (`592ea18b-a70f-459d-bda5-2cc61f7f6b4e`)
- Source: Docker Hub image `nathanflurry/rivet-kitchen-sink:<tag>` (private repo, Railway has cached pull creds)

Deploy steps:

```bash
# 1. Build + push a fresh tag (reuses the same Dockerfile Cloud Run uses; the
#    pre-built rivetkit-napi.linux-x64-gnu.node must already exist at
#    examples/kitchen-sink/rivetkit-napi.linux-x64-gnu.node).
SHA=$(git rev-parse --short HEAD)
TAG="railway-$(date +%Y%m%d-%H%M%S)-$SHA"
docker build -f examples/kitchen-sink/Dockerfile -t nathanflurry/rivet-kitchen-sink:$TAG .
docker push nathanflurry/rivet-kitchen-sink:$TAG

# 2. Point Railway at the new tag and trigger a deploy (GraphQL).
TOKEN=$(python3 -c "import json; print(json.load(open('/home/nathan/.railway/config.json'))['user']['accessToken'])")
curl -sS -X POST https://backboard.railway.com/graphql/v2 \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"query\":\"mutation(\$sid:String!,\$eid:String!,\$input:ServiceInstanceUpdateInput!){serviceInstanceUpdate(serviceId:\$sid,environmentId:\$eid,input:\$input)}\",\"variables\":{\"sid\":\"592ea18b-a70f-459d-bda5-2cc61f7f6b4e\",\"eid\":\"615d08c3-5eca-4a4c-baaf-352807974d9d\",\"input\":{\"source\":{\"image\":\"docker.io/nathanflurry/rivet-kitchen-sink:$TAG\"}}}}"
curl -sS -X POST https://backboard.railway.com/graphql/v2 \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"query":"mutation($sid:String!,$eid:String!){serviceInstanceDeployV2(serviceId:$sid,environmentId:$eid)}","variables":{"sid":"592ea18b-a70f-459d-bda5-2cc61f7f6b4e","eid":"615d08c3-5eca-4a4c-baaf-352807974d9d"}}'

# 3. Watch the deploy.
railway status
railway logs --deployment --lines 200 <deployment-id>
```

Gotchas:
- Do NOT use `railway up`. It uploads the monorepo and runs RAILPACK / Nixpacks, which cannot reproduce the kitchen-sink build (missing NAPI binary, vite `/frontend/main.tsx` resolution under Railpack root, etc.). The image-source flow above is the only supported path.
- Do NOT set a custom `startCommand` on the Railway service. The image's CMD (`node --enable-source-maps dist-server/server.mjs`) is correct; any override that runs `pnpm start` will fail with `sh: 1: tsx: not found` because the production image does not install `tsx`.
- `dockerfilePath` on the service instance is irrelevant when `source.image` is set, but Railway sometimes shows a stale value (e.g. `examples/kitchen-sink/Dockerfile` or even `/examples/sandbox/Dockerfile`). Ignore it — the image-source path bypasses building.
- Image registry is private. Railway authenticates via cached creds from a prior push. If a fresh region/replica fails to pull, set Private Image Credentials on the service: Settings → Source → Private Image Credentials, or via GraphQL `serviceInstanceUpdate({ registryCredentials: { username, password } })`. Alternative: make the Docker Hub repo public.
- Verify the image runs locally before pushing to Railway:
  ```bash
  docker run --rm -p 8080:8080 -e PORT=8080 \
    -e RIVET_ENDPOINT="<endpoint>" -e RIVET_POOL=rw \
    nathanflurry/rivet-kitchen-sink:$TAG
  curl http://localhost:8080/api/rivet/health
  ```

To test actors and inspect their SQLite databases, use the Rivet gateway API.

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
