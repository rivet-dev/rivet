export function getComputeAddendum({
	cloudToken,
	publishableToken,
	namespace,
	apiUrl,
	cloudApiUrl,
}: {
	cloudToken: string;
	publishableToken: string;
	namespace: string;
	apiUrl: string;
	cloudApiUrl: string;
}) {
	return `# Compute Deployment Steps
## Step 1: Load the RivetKit docs

Read https://rivet.dev/llms.txt to understand how RivetKit works (actors, state, events, actions, connections, clients).

Add a note to the project's \`CLAUDE.md\` / \`AGENTS.md\` (create one if missing) linking to https://rivet.dev/llms.txt as the reference for future RivetKit work.

## Step 2: Keep registry.start() (serverless is automatic)

Do **not** rewrite the project into a hand-mounted HTTP handler. Keep the existing \`registry.start()\` call as-is.

When the app runs on Rivet Compute, Compute automatically runs it in serverless mode (it sets \`RIVETKIT_RUNTIME_MODE=serverless\` for you). In that mode \`registry.start()\` binds an HTTP listener instead of opening a long-lived connection to the engine, so no manual Hono handler is needed. The client API is still served under \`/api/rivet\`, so a frontend should target that mount path:

\`\`\`ts
const client = createClient(location.origin + "/api/rivet");
\`\`\`

**Serving a frontend:** \`registry.start()\` serves static files automatically. Put the frontend build output in a \`public/\` directory and it is served with zero extra wiring. If the build outputs somewhere else (e.g. \`dist/\`), set \`RIVETKIT_PUBLIC_DIR\` to that directory.

See https://rivet.dev/docs/general/runtime-modes for local vs. serverless modes and https://rivet.dev/docs/connect/rivet-compute for the full Compute integration guide.

## Step 3: Create Dockerfile

\`npx @rivetkit/cli deploy\` builds your project from a \`Dockerfile\`. If the project does not already have one, create it. Use this as a starting point and adjust the package manager (npm/pnpm/yarn), file paths, and entrypoint to match the project. Make sure the frontend build lands in \`public/\` (or set \`RIVETKIT_PUBLIC_DIR\`), and that the entrypoint calls \`registry.start()\`:

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

Do **not** set \`RIVETKIT_RUNTIME_MODE\` in the Dockerfile. Compute injects it at deploy time.

If the project does not already have a \`.dockerignore\`, create one:

\`\`\`
node_modules/
dist/
.env
.git/
\`\`\`

If Docker is installed, build and run the image to verify it works before proceeding. Pass \`-e RIVETKIT_RUNTIME_MODE=serverless\` to simulate how Compute runs it (otherwise the container defaults to engine/envoy mode and the check is not representative):

\`\`\`bash
docker build -t rivet-test . && docker run --rm -p 3000:3000 -e RIVETKIT_RUNTIME_MODE=serverless rivet-test
\`\`\`

Verify the container starts and is connectable (e.g. \`curl http://localhost:3000/api/rivet/health\` should return 200). If Docker is not installed, skip this and proceed.

## Step 4: Deploy with the Rivet CLI

Deploy the project with a single command. \`@rivetkit/cli\` builds the \`Dockerfile\`, pushes the image to Rivet's registry, and creates/updates the \`default\` managed pool. The project, organization, and namespace are auto-detected from the token, so you do not need to pass them:

\`\`\`bash
npx @rivetkit/cli deploy --token "${cloudToken}" --env PORT=3000
\`\`\`

Notes:
- The image is built for \`linux/amd64\`. \`--env PORT=3000\` tells Rivet Compute which port to route to. \`registry.start()\` binds the port from \`RIVET_PORT\` (default 3000), so the two line up by default. To use a different port, set both \`--env PORT=<port>\` and \`--env RIVET_PORT=<port>\` to the same value and update the \`EXPOSE\` line to match. Setting \`PORT\` alone does not change the port the app listens on.
- \`--token\` is the \`cloud_api_*\` Cloud API token. The command also caches it to \`~/.rivet/credentials\`, so later \`deploy\` calls can omit \`--token\`.
- Pass \`--yes\` to skip interactive prompts in non-interactive environments.

When the command finishes successfully, proceed to Step 5 to verify the deployment is live.

## Step 5: Verify Deployment

**Token types used in this step:**
- \`cloud_api_*\` is the \`--token\` passed to \`@rivetkit/cli deploy\`, cached in \`~/.rivet/credentials\`. It is a management token scoped to the Cloud API (cloud-api.rivet.dev). The CLI uses it for logs.
- \`pk_*\` is the publishable token below, a public key scoped to the Rivet Engine API (api.rivet.dev). Use this for creating actors and calling gateway endpoints.

These are different tokens with different scopes. Do not mix them up.

\`@rivetkit/cli deploy\` waits for the managed pool to become ready before it exits, so a successful deploy means the deployment is already live. You do not need to poll deployment status separately.

If the deploy fails or you need to debug, read the deployment logs with the CLI (it resolves the token from \`~/.rivet/credentials\`):

\`\`\`bash
npx @rivetkit/cli logs
\`\`\`

Verify actors work end-to-end:

1. Create an actor. Actors require a key field (string, not array):
   \`\`\`bash
   curl -X POST "${apiUrl}/actors?namespace=${namespace}" \\
     -H "Authorization: Bearer ${publishableToken}" \\
     -H "Content-Type: application/json" \\
     -d '{"name": "<ACTOR_NAME>", "key": "<KEY>", "runner_name_selector": "default", "crash_policy": "restart"}'
   \`\`\`
   Replace \`<ACTOR_NAME>\` with a valid actor name from the registry and \`<KEY>\` with an appropriate key string (e.g. "general"). Note the \`actor_id\` from the response.

2. Wait ~10 seconds for the actor to start, then hit its health endpoint through the gateway using the public token:
   \`\`\`bash
   curl "${apiUrl}/gateway/<ACTOR_ID>/api/rivet/health" \\
     -H "x-rivet-token: ${publishableToken}"
   \`\`\`
   This should return ok with a 200 status.

3. If the health check returns actor_runner_failed, check the logs to diagnose:
   \`\`\`bash
   npx @rivetkit/cli logs
   \`\`\`

4. Common issues:
   - "actor should have a key": The key field was missing from the create request.
   - Token 401: Make sure you're using the correct API URLs (${apiUrl}, ${cloudApiUrl}).
   - "Failed to start container: Please ensure your container starts successfully on the specified port (3000 if unspecified). Make sure your image was built for linux/amd64.": Ensure the container listens on \`RIVET_PORT\` (3000 by default) and that the \`--env PORT\` value passed to \`@rivetkit/cli deploy\` matches it.

## Troubleshooting

- Deployment and logs are done with \`npx @rivetkit/cli deploy\` and \`npx @rivetkit/cli logs\`. Actor creation and health checks are done via HTTP APIs (curl) as shown in Step 5.
- Architecture: \`@rivetkit/cli deploy\` builds your Docker image and pushes it to Rivet. Rivet runs the container serverlessly. When you create an actor, Rivet communicates with the \`/api/rivet/*\` endpoint inside the container to manage its lifecycle.
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
