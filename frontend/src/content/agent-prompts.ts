export function getComputeAddendum({
	cloudToken,
	publishableToken,
	namespace,
	project,
	organization,
	cloudNamespace,
	apiUrl,
	cloudApiUrl,
}: {
	cloudToken: string;
	publishableToken: string;
	namespace: string;
	project: string;
	organization: string;
	cloudNamespace: string;
	apiUrl: string;
	cloudApiUrl: string;
}) {
	return `# Compute Deployment Steps
## Step 1: Load the RivetKit docs

Read https://rivet.dev/llms.txt to understand how RivetKit works (actors, state, events, actions, connections, clients).

Add a note to the project's \`CLAUDE.md\` / \`AGENTS.md\` (create one if missing) linking to https://rivet.dev/llms.txt as the reference for future RivetKit work.

## Step 2: Switch to serverless mode

Rivet Compute is a serverless platform, so the project must mount RivetKit as an HTTP handler. Calls like \`registry.start()\` or \`registry.listen()\` run **serverfully** and will not work on Compute.

If the project currently calls \`registry.start()\` or \`registry.listen()\`, replace it with an HTTP server that mounts the registry handler. With Hono on Node:

\`\`\`ts
import { Hono } from "hono";
import { serve } from "@hono/node-server";
import { registry } from "./actors";

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
serve({ fetch: app.fetch, port: Number(process.env.PORT ?? 3000) });
\`\`\`

If a frontend calls \`createClient(location.origin)\`, update it to target the mount path:

\`\`\`ts
const client = createClient(location.origin + "/api/rivet");
\`\`\`

See https://rivet.dev/docs/general/runtime-modes for local vs. serverless modes and https://rivet.dev/docs/connect/rivet-compute for the full Compute integration guide.

## Step 3: Create Dockerfile

If the project does not already have a \`Dockerfile\`, create one. Use this as a starting point and adjust the package manager (npm/pnpm/yarn), file paths, and entrypoint to match the project:

\`\`\`dockerfile
FROM node:24-alpine

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build --if-present

EXPOSE 3000

CMD ["node", "dist/index.js"]
\`\`\`

If the project does not already have a \`.dockerignore\`, create one:

\`\`\`
node_modules/
dist/
.env
.git/
\`\`\`

If Docker is installed, build and run the image to verify it works before proceeding:

\`\`\`bash
docker build -t rivet-test . && docker run --rm -p 3000:3000 rivet-test
\`\`\`

Verify the container starts and is connectable (e.g. \`curl http://localhost:3000/health\`). If Docker is not installed, skip this and proceed.

## Step 4: Add GitHub Secret

Add the secret RIVET_CLOUD_TOKEN to your GitHub repository:

\`\`\`
gh secret set RIVET_CLOUD_TOKEN --body "${cloudToken}"
\`\`\`

## Step 5: Create GitHub Action

Create .github/workflows/rivet-deploy.yml using rivet-dev/deploy-action@v1.1.2:

\`\`\`
name: Rivet Deploy

on:
  pull_request:
    types: [opened, synchronize, reopened, closed]
  push:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: rivet-deploy-\${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

jobs:
  rivet-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
      - uses: rivet-dev/deploy-action@v1.1.2
        with:
          rivet-token: \${{ secrets.RIVET_CLOUD_TOKEN }}
          compute-pool-config: '{"environment":{"PORT":"3000"}}'
\`\`\`

The container must be reachable on port 3000. If using another port, set \`environment.PORT\` in \`compute-pool-config\` to that port.

## Step 6: Push and Verify

Push the changes and monitor the GitHub Action until it completes:

1. Push to the repository.
2. Use \`gh run list\` and \`gh run watch\` to poll the action status. Do not move on until the run finishes.
3. If the action succeeds, proceed to Step 7.
4. If the action fails, inspect the logs with \`gh run view --log-failed\`, fix the issue, push again, and repeat from step 2.

## Step 7: Verify Deployment

**Token types used in this step:**
- \`cloud_api_*\` (the \`RIVET_CLOUD_TOKEN\` secret) — a management token scoped to the Cloud API (cloud-api.rivet.dev). Use this for admin operations like checking deployment status and fetching logs.
- \`pk_*\` (the publishable token below) — a public key scoped to the Rivet Engine API (api.rivet.dev). Use this for creating actors and calling gateway endpoints.

These are different tokens with different scopes. Do not mix them up.

Once deployed, verify the deployment works:

1. Poll the deployment status every 5 seconds until status is "ready". Stop and investigate if status is "error".
   \`\`\`bash
   curl -s "${cloudApiUrl}/projects/${project}/namespaces/${cloudNamespace}/managed-pools/default?org=${organization}" \\
     -H "Authorization: Bearer ${cloudToken}"
   \`\`\`

   If status is "Failed to start container", poll logs history to diagnose:
   \`\`\`bash
   curl -s "${cloudApiUrl}/projects/${project}/namespaces/${cloudNamespace}/managed-pools/default/logs/history?org=${organization}" \\
     -H "Authorization: Bearer ${cloudToken}"
   \`\`\`

2. Create an actor. Actors require a key field (string, not array):
   \`\`\`bash
   curl -X POST "${apiUrl}/actors?namespace=${namespace}" \\
     -H "Authorization: Bearer ${publishableToken}" \\
     -H "Content-Type: application/json" \\
     -d '{"name": "<ACTOR_NAME>", "key": "<KEY>", "runner_name_selector": "default", "crash_policy": "restart"}'
   \`\`\`
   Replace \`<ACTOR_NAME>\` with a valid actor name from the registry and \`<KEY>\` with an appropriate key string (e.g. "general"). Note the \`actor_id\` from the response.

3. Wait ~10 seconds for the actor to start, then hit its health endpoint through the gateway using the public token:
   \`\`\`bash
   curl "${apiUrl}/gateway/<ACTOR_ID>/health" \\
     -H "x-rivet-token: ${publishableToken}"
   \`\`\`
   This should return ok with a 200 status.

4. If the health check returns actor_runner_failed, check the runner logs via SSE to diagnose:
   \`\`\`bash
   curl --max-time 15 "${cloudApiUrl}/projects/${project}/namespaces/${cloudNamespace}/managed-pools/default/logs?org=${organization}" \\
     -H "Authorization: Bearer ${cloudToken}"
   \`\`\`

5. Common issues:
   - "actor should have a key": The key field was missing from the create request.
   - Token 401: Make sure you're using the correct API URLs (${apiUrl}, ${cloudApiUrl}).
   - "Failed to start container: Please ensure your container starts successfully on the specified port (8080 if unspecified). Make sure your image was built for linux/amd64.": Ensure the container is connectable on PORT 3000 or whichever PORT is specified in your \`compute-pool-config\` in the GitHub Action.

## Troubleshooting

- There is no Rivet CLI. Do not attempt to use or install one. All deployment is done via the GitHub Action and all interaction is done via HTTP APIs (curl).
- Architecture: The GitHub Action builds your Docker image and pushes it to Rivet. Rivet runs the container serverlessly. When you create an actor, Rivet communicates with the \`/api/rivet/*\` endpoint inside the container to manage its lifecycle.
- For more troubleshooting help, see: https://rivet.dev/docs/actors/troubleshooting/`;
}

export function getAgentInstructionsPrompt({
	providerStr,
	publishableToken,
	secretToken,
	runnerName,
}: {
	providerStr: string;
	publishableToken: string;
	secretToken: string;
	runnerName: string;
}) {
	const poolLine =
		runnerName !== "default" ? `\n  RIVET_POOL=${runnerName}` : "";
	return `# RivetKit Setup & Deploy

Read https://rivet.dev/llms.txt to understand how RivetKit works (actors, state, events, actions, connections, clients).

Add a note to the project's \`CLAUDE.md\` / \`AGENTS.md\` (create one if missing) linking to https://rivet.dev/llms.txt as the reference for future RivetKit work.

## Step 1: New project or existing?

Ask the user:

> Do you want to **build a new project with RivetKit**, or **integrate RivetKit into this existing project**?

Branch based on their answer.

---

## Path A: New project

### 1. Suggest something to build

Recommend something realtime, fast to spin up, and with a simple frontend. Good starting points:

- **Chat app** (rooms + messages, great default)
- Collaborative todo / list
- Live cursors / presence demo
- Simple multiplayer game (tic-tac-toe, drawing board)

Ask what they want to build. If they're unsure, default to a chat app.

### 2. Build it

Scaffold a minimal project with RivetKit:

- \`npm install rivetkit\` (or pnpm/yarn/whatever is being used)
- Add a frontend (plain HTML/JS or React via \`@rivetkit/react\` — keep it small).
- Define actors + registry (see https://rivet.dev/docs/actors).
- Serve via \`registry.listen({ port: 3001, publicDir: "<frontend-output-dir>" })\` so one command serves both API and frontend.
- Add a local dev script (e.g. \`npm run dev\`) that builds the frontend and starts the server.

Reference quickstarts:
- Node.js / Bun: https://rivet.dev/docs/actors/quickstart/backend
- React: https://rivet.dev/docs/actors/quickstart/react

### 3. Verify

- \`GET /api/rivet/metadata\` returns 200.
- Drive actors via the inspector HTTP API: https://rivet.dev/docs/actors/debugging
- If there's a frontend, open it in a browser and confirm the realtime behavior works end-to-end.

### 4. Wrap up

Give the user:

- A brief overview of the project (what was built, where the actors live, how state/events flow).
- The command to run locally.
- Next steps to deploy + Rivet link.

**Deploy steps:**

1. Review the docs for ${providerStr} (the following docs are available, pick the one that relates to ${providerStr}
    https://rivet.dev/docs/connect/rivet-compute/
    https://rivet.dev/docs/connect/aws-ecs/
    https://rivet.dev/docs/connect/aws-lambda/
    https://rivet.dev/docs/connect/cloudflare/
    https://rivet.dev/docs/connect/custom/
    https://rivet.dev/docs/connect/freestyle/
    https://rivet.dev/docs/connect/gcp-cloud-run/
    https://rivet.dev/docs/connect/hetzner/
    https://rivet.dev/docs/connect/kubernetes/
    https://rivet.dev/docs/connect/railway/
    https://rivet.dev/docs/connect/supabase/
    https://rivet.dev/docs/connect/vercel/
    https://rivet.dev/docs/connect/vm-and-bare-metal/
)
2. Configure and deploy using the following environment variables:
  RIVET_PUBLIC_ENDPOINT=${publishableToken}
  RIVET_ENDPOINT=${secretToken}${poolLine}
3. Finally, if applicable (not necessary for Rivet Compute), tell the user the URL to paste in to the Rivet dashboard

---

## Path B: Integrate into existing project

### 1. Understand the project + propose

Read through the project to understand what it does. Then ask:

> What do you want to build, or how do you want to integrate RivetKit?

Give recommendations grounded in what you see. Examples:
- Chat app with video → move chat into a RivetKit actor for realtime + state.
- Todo app → per-list actor for collaborative editing.
- Dashboard → per-user actor for live metrics / presence.

### 2. Plan

Describe the plan before writing code:
- Which features become actors.
- What state each actor holds.
- Which actions + events the frontend will use.

Get confirmation before implementing.

### 3. Integrate (actors only)

- \`npm install rivetkit\`
- Add actors + registry (see https://rivet.dev/docs/actors).
- Mount on the existing server: \`app.all("/api/rivet/*", (c) => registry.handler(c.req.raw))\` (or the equivalent for the project's framework).
- Do **not** touch the frontend yet unless the user asks.

Verify with \`/api/rivet/metadata\` and the inspector API (https://rivet.dev/docs/actors/debugging).

### 4. Wrap up

Give the user:
- A short overview of what was added.
- The command to run locally.

Then ask:

1. **Want to integrate this into the frontend?** Point at https://rivet.dev/docs/clients/react (or the relevant client doc) and wire it up if yes.
2. **Want to deploy?** If yes, **Deploy steps:**

    a. Review the docs for ${providerStr} (the following docs are available, pick the one that relates to ${providerStr}
        https://rivet.dev/docs/connect/rivet-compute/
        https://rivet.dev/docs/connect/aws-ecs/
        https://rivet.dev/docs/connect/aws-lambda/
        https://rivet.dev/docs/connect/cloudflare/
        https://rivet.dev/docs/connect/custom/
        https://rivet.dev/docs/connect/freestyle/
        https://rivet.dev/docs/connect/gcp-cloud-run/
        https://rivet.dev/docs/connect/hetzner/
        https://rivet.dev/docs/connect/kubernetes/
        https://rivet.dev/docs/connect/railway/
        https://rivet.dev/docs/connect/supabase/
        https://rivet.dev/docs/connect/vercel/
        https://rivet.dev/docs/connect/vm-and-bare-metal/
    )
    b. Configure and deploy using the following environment variables:
        RIVET_PUBLIC_ENDPOINT=${publishableToken}
        RIVET_ENDPOINT=${secretToken}${poolLine}
    c. Finally, if applicable (not necessary for Rivet Compute), tell the user the URL to paste in to the Rivet dashboard

Link docs:
- Actors: https://rivet.dev/docs/actors
- Clients: https://rivet.dev/docs/clients
- Troubleshooting: https://rivet.dev/docs/actors/troubleshooting

---

## If you get stuck

Check https://rivet.dev/docs/actors/troubleshooting. If that doesn't help, point the user at:
- Discord: https://rivet.dev/discord
- GitHub issues: https://github.com/rivet-dev/rivet

Include in the report: symptoms, what was tried, RivetKit version, runtime, HTTP router.`;
}
