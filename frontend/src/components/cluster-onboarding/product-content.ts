import type { ProductSection } from "@/components/products/product-picker";
import { SETUP_PROMPT_URL } from "./model";

/** Mock content for the per-product pages in the cluster sidebar. */
export interface ProductPageContent {
	id: string;
	label: string;
	markFileName: string;
	section: ProductSection;
	description: string;
	guidesUrl: string;
	apiDocsUrl: string;
	/** Quickstart steps, mirroring the product's quickstart on rivet.dev. */
	steps: QuickstartStep[];
	guides: DocLink[];
	apiReference: DocLink[];
}

/** Which API the quickstart code targets. Toggled at the top of the quickstart. */
export type QuickstartLang = "typescript" | "http";

export const QUICKSTART_LANGS: { id: QuickstartLang; label: string }[] = [
	{ id: "typescript", label: "TypeScript" },
	{ id: "http", label: "HTTP" },
];

/** Cluster values substituted into every snippet, the same ones the overview shows. */
export interface QuickstartContext {
	/** External endpoint, e.g. https://api.test-cluster-3.rivet.run */
	endpoint: string;
	/** Admin token for the cluster. */
	token: string;
	/** Namespace selected in the sidebar. */
	namespace: string;
}

export interface CodeBlockSpec {
	/** Tab label on the code block, e.g. "Terminal" or a file name. */
	kind: "terminal" | "file";
	fileName?: string;
	code: (ctx: QuickstartContext) => string;
}

export interface QuickstartStep {
	title: string;
	description?: string;
	/**
	 * Code per API. `all` shows on every tab; a language key overrides it. A
	 * step with blocks but nothing for the active tab is skipped on that tab.
	 */
	blocks?: Partial<Record<QuickstartLang | "all", CodeBlockSpec[]>>;
	/** Short note rendered after the code. */
	after?: string;
}

export interface DocLink {
	label: string;
	url: string;
}

const AGENTOS_DOCS = "https://rivet.dev/agentos/docs";
const DYNAMIC_APPS_DOCS = "https://rivet.dev/dynamic-apps/docs";
const STREAMS_DOCS = "https://rivet.dev/integrations/durable-streams/";
const ACTORS_DOCS = "https://rivet.dev/actors/docs";
const WORKFLOWS_DOCS = "https://rivet.dev/workflows/docs";

const terminal = (code: CodeBlockSpec["code"]): CodeBlockSpec => ({
	kind: "terminal",
	code,
});
const file = (
	fileName: string,
	code: CodeBlockSpec["code"],
): CodeBlockSpec => ({
	kind: "file",
	fileName,
	code,
});

/** The three variables every snippet needs, as a shell prefix. */
const envPrefix = (ctx: QuickstartContext) =>
	`RIVET_ENDPOINT=${ctx.endpoint} \\
RIVET_NAMESPACE=${ctx.namespace} \\
RIVET_TOKEN=${ctx.token} \\
`;

export const AGENTOS_CONTENT: ProductPageContent = {
	id: "actor:agentos",
	label: "agentOS",
	markFileName: "agentos-mark.svg",
	section: "products",
	description:
		"Durable VMs for coding agents. Create a VM by key on this cluster, then control its processes, filesystem, network, and language runtimes from the client or plain HTTP.",
	guidesUrl: `${AGENTOS_DOCS}/quickstart-hosted/`,
	apiDocsUrl: `${AGENTOS_DOCS}/hosted-api/`,
	steps: [
		{
			title: "Install the client",
			description:
				"The client is optional. Every action is also a plain HTTP call.",
			blocks: {
				typescript: [terminal(() => "npm install @rivet-dev/agentos")],
			},
		},
		{
			title: "Create a VM and use it",
			description:
				"The VM is created on first use and the same key returns the same VM. Run a command, write a file, then start a server and expose it through an expiring preview URL.",
			blocks: {
				typescript: [
					file(
						"vm.ts",
						(
							ctx,
						) => `import { createAgentOsClient } from "@rivet-dev/agentos";

const client = createAgentOsClient({
	endpoint: "${ctx.endpoint}",
	namespace: "${ctx.namespace}",
	token: "${ctx.token}",
});

// Creates the VM on first use and returns the same VM for the same key.
const vm = client.agentOS.getOrCreate(["my-vm"]);

const result = await vm.v1.process.run({
	command: "sh",
	args: ["-c", "echo hello && ls /home/agentos"],
	options: { captureStdio: true },
});
console.log(result.stdout);

await vm.v1.filesystem.writeFile({
	path: "/home/agentos/hello.txt",
	content: "Hello, world!",
});

// Start a server and expose it through an expiring preview URL.
await vm.v1.filesystem.writeFile({
	path: "/home/agentos/server.js",
	content: \`require("node:http").createServer((req, res) => res.end("hello")).listen(3000);\`,
});
await vm.v1.process.spawn({ command: "node", args: ["/home/agentos/server.js"] });
const preview = await vm.v1.network.preview.create({ port: 3000, ttlMs: 60 * 60 * 1_000 });`,
					),
				],
				http: [
					terminal(
						(
							ctx,
						) => `curl -X PUT "${ctx.endpoint}/actors?namespace=${ctx.namespace}" \\
  -H "Authorization: Bearer ${ctx.token}" \\
  --json '{
    "name": "agentOS",
    "key": "my-vm",
    "runner_name_selector": "services",
    "crash_policy": "sleep"
  }'
# → { "actor": { "actor_id": "…" }, "created": true }

curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.process.run" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{
    "command": "sh",
    "args": ["-c", "echo hello && ls /home/agentos"],
    "options": { "captureStdio": true }
  }]}'

curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.filesystem.writeFile" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{
    "path": "/home/agentos/server.js",
    "content": "require(\\"node:http\\").createServer((req, res) => res.end(\\"hello\\")).listen(3000);"
  }]}'

curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.process.spawn" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{"command": "node", "args": ["/home/agentos/server.js"]}]}'

curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.network.preview.create" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{"port": 3000, "ttlMs": 3600000}]}'
# → { "output": { "path": "/preview/…/", "token": "…", … } }`,
					),
				],
			},
			after: "Open the preview at /gateway/$VM_ID@<token>/request followed by the returned path. A running server keeps the VM awake.",
		},
		{
			title: "Configure the VM",
			description:
				"Configuration is stored with the VM and versioned. Replace it with v1.config.set, then restart so filesystem and software changes take effect.",
			blocks: {
				typescript: [
					file(
						"vm.ts",
						() => `await vm.v1.config.set({
	config: {
		// Keep files across sleep and restarts, and mount durable storage at /data.
		filesystem: {
			root: { type: "durable" },
			mounts: [{ path: "/data", backend: { type: "durable" } }],
		},

		// Environment variables for guest processes.
		environment: { NODE_ENV: "production" },

		// Deny network access except to one host.
		permissions: {
			network: {
				default: "deny",
				rules: [
					{
						mode: "allow",
						operations: ["*"],
						patterns: ["dns://api.example.com", "tcp://api.example.com:*"],
					},
				],
			},
		},
	},
});

// Filesystem and software changes apply after a restart.
await vm.v1.vm.restart({});`,
					),
				],
				http: [
					terminal(
						(
							ctx,
						) => `curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.config.set" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{"config": {
    "filesystem": {
      "root": { "type": "durable" },
      "mounts": [{ "path": "/data", "backend": { "type": "durable" } }]
    },
    "environment": { "NODE_ENV": "production" },
    "permissions": {
      "network": {
        "default": "deny",
        "rules": [{
          "mode": "allow",
          "operations": ["*"],
          "patterns": ["dns://api.example.com", "tcp://api.example.com:*"]
        }]
      }
    }
  }}]}'

curl "${ctx.endpoint}/gateway/$VM_ID/action/v1.vm.restart" \\
  -H "x-rivet-token: ${ctx.token}" \\
  --json '{"args": [{}]}'`,
					),
				],
			},
		},
	],
	guides: [
		{
			label: "Quickstart (Hosted)",
			url: `${AGENTOS_DOCS}/quickstart-hosted/`,
		},
		{ label: "Quickstart (Library)", url: `${AGENTOS_DOCS}/quickstart/` },
		{ label: "Processes", url: `${AGENTOS_DOCS}/processes/` },
		{ label: "Filesystem", url: `${AGENTOS_DOCS}/filesystem/` },
		{ label: "Networking", url: `${AGENTOS_DOCS}/networking/` },
		{ label: "Persistence & sleep", url: `${AGENTOS_DOCS}/persistence/` },
		{ label: "Permissions", url: `${AGENTOS_DOCS}/permissions/` },
		{ label: "Software", url: `${AGENTOS_DOCS}/software/` },
		{ label: "Crons & schedules", url: `${AGENTOS_DOCS}/cron/` },
		{ label: "Security model", url: `${AGENTOS_DOCS}/security-model/` },
	],
	apiReference: [
		{ label: "v1.process.run", url: `${AGENTOS_DOCS}/hosted-api/#process` },
		{
			label: "v1.filesystem.writeFile",
			url: `${AGENTOS_DOCS}/hosted-api/#filesystem`,
		},
		{
			label: "v1.network.preview.create",
			url: `${AGENTOS_DOCS}/hosted-api/#network`,
		},
		{ label: "v1.config.set", url: `${AGENTOS_DOCS}/hosted-api/#config` },
		{ label: "v1.vm.restart", url: `${AGENTOS_DOCS}/hosted-api/#vm` },
	],
};

export const DYNAMIC_APPS_CONTENT: ProductPageContent = {
	id: "actor:dynamic-apps",
	label: "Dynamic Apps",
	markFileName: "dynamic-apps-mark.svg",
	section: "products",
	description:
		"Deploy AI-generated apps for your users. Your backend hosts the router; each app runs on this cluster through agentOS with its own namespace, SQLite, workflows, and multiplayer state.",
	guidesUrl: `${DYNAMIC_APPS_DOCS}/quickstart/`,
	apiDocsUrl: `${DYNAMIC_APPS_DOCS}/deploy/`,
	steps: [
		{
			title: "Install",
			description: "Use Node.js 22 or newer.",
			blocks: {
				all: [
					terminal(
						() => `npm add @rivet-dev/dynamic-apps @hono/node-server hono
npm add --save-dev tsx
npm pkg set type=module`,
					),
				],
			},
		},
		{
			title: "Set up the router",
			description:
				"Mount the private Rivet callback separately from application traffic. The callback lets this cluster reach your server to manage deployments; ordinary app requests are served by appsRouter through agentOS.",
			blocks: {
				typescript: [
					file(
						"src/server.ts",
						() => `import { serve } from "@hono/node-server";
import { appsRouter } from "@rivet-dev/dynamic-apps";
import { Hono } from "hono";

const server = new Hono();

// This is how the Rivet control plane communicates with your backend.
server.all("/api/rivet/*", (c) => appsRouter.fetch(c.req.raw));

// Mount every deployed application at /apps/:appId.
server.route("/apps", appsRouter);

// Serve the host router over HTTP.
serve({ fetch: server.fetch, port: 3000 });`,
					),
				],
				http: [
					file(
						"src/server.ts",
						() => `import { serve } from "@hono/node-server";
import { appsRouter, deployApp } from "@rivet-dev/dynamic-apps";
import { Hono } from "hono";

const server = new Hono();

// This is how the Rivet control plane communicates with your backend.
server.all("/api/rivet/*", (c) => appsRouter.fetch(c.req.raw));

// Mount every deployed application at /apps/:appId.
server.route("/apps", appsRouter);

// Accept app files over HTTP and publish them as a release.
server.post("/deploy", async (c) => {
	const { appId, files } = await c.req.json();
	await deployApp({ appId, files });
	return c.json({ url: \`/apps/\${appId}/\` });
});

// Serve the host router over HTTP.
serve({ fetch: server.fetch, port: 3000 });`,
					),
				],
			},
		},
		{
			title: "Run the host",
			description:
				"The host connects to this cluster with the endpoint, namespace, and token below. Each deployed app gets its own namespace under it.",
			blocks: {
				all: [
					terminal((ctx) => `${envPrefix(ctx)}npx tsx src/server.ts`),
				],
			},
		},
		{
			title: "Generate and deploy",
			description:
				"An LLM writes the app as a set of files. Pass the skills from @rivet-dev/dynamic-apps as the system prompt so the model knows the supported project layout, then hand the files to deployApp().",
			blocks: {
				typescript: [
					file(
						"src/deploy.ts",
						() => `import { anthropic } from "@ai-sdk/anthropic";
import {
	deployApp,
	rivetActorsSkill,
	webServerSkill,
} from "@rivet-dev/dynamic-apps";
import { generateObject } from "ai";
import { z } from "zod";

const { object } = await generateObject({
	model: anthropic("claude-sonnet-5"),
	// Skills teach the model the supported project layout.
	system: [webServerSkill, rivetActorsSkill].join("\\n\\n"),
	schema: z.object({ files: z.record(z.string(), z.string()) }),
	prompt: "Build a team board.",
});

await deployApp({ appId: "team-board", files: object.files });`,
					),
					terminal((ctx) => `${envPrefix(ctx)}npx tsx src/deploy.ts`),
				],
				http: [
					terminal(
						() => `curl -X POST http://localhost:3000/deploy \\
  --json '{
    "appId": "team-board",
    "files": {
      "package.json": "{\\"private\\":true,\\"type\\":\\"module\\",\\"main\\":\\"index.js\\"}",
      "index.js": "export default { fetch() { return new Response(\\"hello\\"); } };"
    }
  }'
# → { "url": "/apps/team-board/" }`,
					),
				],
			},
		},
		{
			title: "Visit the app",
			description:
				"Open http://localhost:3000/apps/team-board/. The bare /apps/team-board path redirects to its trailing-slash form.",
			blocks: {
				all: [
					terminal(
						() => "curl http://localhost:3000/apps/team-board/",
					),
				],
			},
		},
		{
			title: "Deploy the host",
			description:
				"The host is a normal Node.js server. Run it anywhere that can reach this cluster with the same three variables; the apps themselves already run here.",
			blocks: {
				all: [
					terminal((ctx) => `${envPrefix(ctx)}node dist/server.js`),
				],
			},
		},
	],
	guides: [
		{ label: "Quickstart", url: `${DYNAMIC_APPS_DOCS}/quickstart/` },
		{ label: "Architecture", url: `${DYNAMIC_APPS_DOCS}/architecture/` },
		{ label: "Generate", url: `${DYNAMIC_APPS_DOCS}/generate/` },
		{ label: "Deploy", url: `${DYNAMIC_APPS_DOCS}/deploy/` },
		{ label: "Connect to Rivet", url: `${DYNAMIC_APPS_DOCS}/connect/` },
		{ label: "Routing", url: `${DYNAMIC_APPS_DOCS}/routing/` },
		{
			label: "Authentication",
			url: `${DYNAMIC_APPS_DOCS}/authentication/`,
		},
		{ label: "Backends", url: `${DYNAMIC_APPS_DOCS}/backends/` },
		{
			label: "Static websites",
			url: `${DYNAMIC_APPS_DOCS}/static-websites/`,
		},
		{ label: "SQLite", url: `${DYNAMIC_APPS_DOCS}/sqlite/` },
		{ label: "Workflows", url: `${DYNAMIC_APPS_DOCS}/workflows/` },
		{ label: "Multiplayer", url: `${DYNAMIC_APPS_DOCS}/multiplayer/` },
	],
	apiReference: [
		{ label: "appsRouter", url: `${DYNAMIC_APPS_DOCS}/routing/` },
		{ label: "deployApp", url: `${DYNAMIC_APPS_DOCS}/deploy/` },
		{ label: "webServerSkill", url: `${DYNAMIC_APPS_DOCS}/generate/` },
		{ label: "rivetActorsSkill", url: `${DYNAMIC_APPS_DOCS}/generate/` },
	],
};

export const DURABLE_STREAMS_CONTENT: ProductPageContent = {
	id: "actor:durable-streams",
	label: "Durable Streams",
	markFileName: "durable-streams-mark.svg",
	section: "services",
	description:
		"An open standard for real-time streams with durable, replayable history. This cluster runs the protocol on Rivet Actors, so every stream is persisted here.",
	guidesUrl: STREAMS_DOCS,
	apiDocsUrl: "https://github.com/durable-streams/durable-streams",
	steps: [
		{
			title: "Install the client",
			description:
				"The official client speaks the protocol to any Durable Streams server.",
			blocks: {
				typescript: [
					terminal(() => "npm install @durable-streams/client"),
				],
			},
		},
		{
			title: "Create a stream and append",
			description:
				"Streams live at /durable-streams/v1/stream/<path> on this cluster's endpoint. The first write creates the stream.",
			blocks: {
				typescript: [
					file(
						"stream.ts",
						(
							ctx,
						) => `import { DurableStream } from "@durable-streams/client";

const stream = await DurableStream.create({
	url: "${ctx.endpoint}/durable-streams/v1/stream/demo",
	contentType: "application/json",
	headers: { Authorization: "Bearer ${ctx.token}" },
});

await stream.append(JSON.stringify({ message: "hello" }));`,
					),
				],
				http: [
					terminal(
						(ctx) => `curl -i -X PUT \\
  -H "Authorization: Bearer ${ctx.token}" \\
  -H 'content-type: application/json' \\
  --data '[{"message":"hello"}]' \\
  ${ctx.endpoint}/durable-streams/v1/stream/demo

curl -i -X POST \\
  -H "Authorization: Bearer ${ctx.token}" \\
  -H 'content-type: application/json' \\
  --data '{"message":"world"}' \\
  ${ctx.endpoint}/durable-streams/v1/stream/demo`,
					),
				],
			},
		},
		{
			title: "Read the stream",
			description:
				"offset=-1 replays the whole history, then keeps delivering new records as they are appended.",
			blocks: {
				typescript: [
					file(
						"stream.ts",
						() => `const res = await stream.stream<{ message: string }>({ offset: -1 });

res.subscribeJson(async (batch) => {
	for (const item of batch.items) console.log(item.message);
});`,
					),
				],
				http: [
					terminal(
						(
							ctx,
						) => `curl -H "Authorization: Bearer ${ctx.token}" \\
  '${ctx.endpoint}/durable-streams/v1/stream/demo?offset=-1'`,
					),
				],
			},
		},
		{
			title: "Develop locally",
			description:
				"Start the local Rivet control plane and the Durable Streams worker, then point the same code at http://127.0.0.1:8642 with no token.",
			blocks: {
				all: [
					terminal(
						() => `npx @rivet-dev/services dev
# http://127.0.0.1:8642/durable-streams/v1/stream/<path>`,
					),
				],
			},
		},
	],
	guides: [
		{ label: "Quickstart", url: STREAMS_DOCS },
		{
			label: "Protocol specification",
			url: "https://github.com/durable-streams/durable-streams",
		},
		{
			label: "Integration source",
			url: "https://github.com/rivet-dev/rivet-durable-streams",
		},
		{
			label: "Self-hosting the worker",
			url: "https://rivet.dev/docs/deploy/self-host/control-plane/",
		},
	],
	apiReference: [
		{ label: "PUT /v1/stream/:path", url: `${STREAMS_DOCS}#http` },
		{ label: "POST /v1/stream/:path", url: `${STREAMS_DOCS}#http` },
		{ label: "GET /v1/stream/:path?offset", url: `${STREAMS_DOCS}#http` },
		{
			label: "DurableStream.create",
			url: `${STREAMS_DOCS}#typescript-client`,
		},
	],
};

export const DEPLOY_ACTOR_CONTENT: ProductPageContent = {
	id: "actor:deploy",
	label: "Deploy an Actor",
	markFileName: "actors-mark.svg",
	section: "products",
	description:
		"Actors are the primitive for realtime, stateful workloads: durable state, actions, events, scheduling, and SQLite, scaled to zero when idle. Write one, run it locally, then deploy it to this cluster.",
	guidesUrl: `${ACTORS_DOCS}/quickstart/backend`,
	apiDocsUrl: "https://rivet.dev/docs/api/",
	steps: [
		{
			title: "Install Rivet",
			blocks: { all: [terminal(() => "npm install rivetkit")] },
		},
		{
			title: "Create actors and start server",
			description:
				"Create a file with your actors, set up the registry, and start the server.",
			blocks: {
				all: [
					file(
						"index.ts",
						() => `import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});

registry.start();`,
					),
				],
			},
		},
		{
			title: "Run server",
			description:
				"Your server is now running on http://localhost:6420. Open it in your browser, or point your AI agent at it, to inspect your actors live.",
			blocks: { all: [terminal(() => "npx tsx --watch index.ts")] },
		},
		{
			title: "Connect to the actor",
			description:
				"This code can run in your frontend or your backend. The endpoint carries the namespace and token.",
			blocks: {
				typescript: [
					file(
						"client.ts",
						(
							ctx,
						) => `import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(
	"${ctx.endpoint.replace("://", `://${ctx.namespace}:${ctx.token}@`)}",
);

// Get or create a counter actor for the key "my-counter"
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions
const count = await counter.increment(3);
console.log("New count:", count);

// Listen to realtime events
const connection = counter.connect();
connection.on("newCount", (newCount: number) => {
	console.log("Count changed:", newCount);
});

// Increment through connection
await connection.increment(1);`,
					),
				],
				http: [
					terminal(
						(
							ctx,
						) => `curl "${ctx.endpoint}/gateway/counter/action/increment?rvt-namespace=${ctx.namespace}&rvt-method=getOrCreate&rvt-key=my-counter&rvt-pool=default" \\
  -H "x-rivet-token: ${ctx.token}" \\
  -H "Content-Type: application/json" \\
  -d '{"args": [3]}'
# → 3`,
					),
				],
			},
		},
		{
			title: "Deploy",
			description:
				"Builds your project and pushes it to this cluster's actor nodes. The CLI reads the token from the environment.",
			blocks: {
				all: [terminal((ctx) => `${envPrefix(ctx)}npx rivet deploy`)],
			},
		},
	],
	guides: [
		{ label: "Quickstart", url: `${ACTORS_DOCS}/quickstart/backend` },
		{ label: "State", url: `${ACTORS_DOCS}/state` },
		{ label: "Actions", url: `${ACTORS_DOCS}/actions` },
		{ label: "Events", url: `${ACTORS_DOCS}/events` },
		{ label: "Lifecycle", url: `${ACTORS_DOCS}/lifecycle` },
		{ label: "Scheduling", url: `${ACTORS_DOCS}/schedule` },
		{ label: "Queues", url: `${ACTORS_DOCS}/queues` },
		{ label: "SQLite", url: `${ACTORS_DOCS}/sqlite` },
	],
	apiReference: [
		{ label: "Gateway", url: "https://rivet.dev/docs/api/gateway" },
		{ label: "Actors", url: "https://rivet.dev/docs/api/actors" },
		{ label: "Tokens", url: "https://rivet.dev/docs/api/tokens" },
		{
			label: "Control plane",
			url: "https://rivet.dev/docs/api/control-plane",
		},
	],
};

export const WORKFLOWS_CONTENT: ProductPageContent = {
	id: "actor:workflows",
	label: "Workflows",
	markFileName: "workflows-mark.svg",
	section: "products",
	description:
		"Durable TypeScript workflows built on Actors. Each step records its result, so a run that crashes or is redeployed resumes at the first step that never completed.",
	guidesUrl: `${WORKFLOWS_DOCS}/quickstart/`,
	apiDocsUrl: `${WORKFLOWS_DOCS}/reference/`,
	steps: [
		{
			title: "Install Workflows",
			blocks: {
				all: [
					terminal(() => "npm install rivetkit @rivet-dev/workflows"),
				],
			},
		},
		{
			title: "Write a workflow",
			description:
				"A workflow is an async function. Each `ctx.step` runs once and records its result; on replay, recorded steps return instantly.",
			blocks: {
				all: [
					file(
						"index.ts",
						() => `import { setup, workflow } from "@rivet-dev/workflows";

export const invoice = workflow({
	state: { total: 0, status: "idle" as "idle" | "complete" },
	run: async (ctx) => {
		const subtotal = await ctx.step("load-subtotal", () => loadSubtotal());
		const tax = await ctx.step("calculate-tax", () => calculateTax(subtotal));
		await ctx.step("save-invoice", async (step) => {
			step.state.total = subtotal + tax;
			step.state.status = "complete";
		});
	},
	actions: {
		getState: (c) => c.state,
	},
});

async function loadSubtotal(): Promise<number> {
	const res = await fetch("https://api.example.com/carts/main");
	return ((await res.json()) as { subtotal: number }).subtotal;
}

async function calculateTax(subtotal: number): Promise<number> {
	const res = await fetch("https://api.example.com/tax/quote", {
		method: "POST",
		body: JSON.stringify({ subtotal }),
	});
	return ((await res.json()) as { tax: number }).tax;
}

export const registry = setup({ use: { invoice } });

registry.start();`,
					),
				],
			},
		},
		{
			title: "Run server",
			description:
				"Your server is now running on http://localhost:6420. Open it to watch each step record as the run progresses.",
			blocks: { all: [terminal(() => "npx tsx --watch index.ts")] },
		},
		{
			title: "Start a run",
			description:
				"Each key is its own run with its own state and history. Fetching the state starts the run on first use.",
			blocks: {
				typescript: [
					file(
						"client.ts",
						(
							ctx,
						) => `import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(
	"${ctx.endpoint.replace("://", `://${ctx.namespace}:${ctx.token}@`)}",
);

const run = client.invoice.getOrCreate(["inv-1042"]);
const state = await run.getState();
console.log(state.status, state.total);`,
					),
				],
				http: [
					terminal(
						(
							ctx,
						) => `curl "${ctx.endpoint}/gateway/invoice/action/getState?rvt-namespace=${ctx.namespace}&rvt-method=getOrCreate&rvt-key=inv-1042&rvt-pool=default" \\
  -H "x-rivet-token: ${ctx.token}" \\
  -H "Content-Type: application/json" \\
  -d '{"args": []}'
# → {"total":0,"status":"idle"}`,
					),
				],
			},
		},
		{
			title: "Deploy",
			description:
				"Builds your project and pushes it to this cluster's actor nodes. Runs in progress keep their version; new runs take the latest.",
			blocks: {
				all: [terminal((ctx) => `${envPrefix(ctx)}npx rivet deploy`)],
			},
		},
	],
	guides: [
		{ label: "Quickstart", url: `${WORKFLOWS_DOCS}/quickstart/` },
		{ label: "Steps", url: `${WORKFLOWS_DOCS}/steps/` },
		{ label: "Queues & Messaging", url: `${WORKFLOWS_DOCS}/queues/` },
		{
			label: "Timers & Concurrency",
			url: `${WORKFLOWS_DOCS}/timers-and-concurrency/`,
		},
		{
			label: "Failure & Recovery",
			url: `${WORKFLOWS_DOCS}/failure-and-recovery/`,
		},
		{ label: "Versioning", url: `${WORKFLOWS_DOCS}/versioning/` },
		{ label: "Patterns", url: `${WORKFLOWS_DOCS}/patterns/` },
	],
	apiReference: [
		{ label: "Reference", url: `${WORKFLOWS_DOCS}/reference/` },
		{ label: "Gateway", url: "https://rivet.dev/docs/api/gateway" },
		{ label: "Actors", url: "https://rivet.dev/docs/api/actors" },
		{ label: "Tokens", url: "https://rivet.dev/docs/api/tokens" },
	],
};

export const PRODUCT_PAGES: Record<string, ProductPageContent> =
	Object.fromEntries(
		[
			AGENTOS_CONTENT,
			DYNAMIC_APPS_CONTENT,
			DURABLE_STREAMS_CONTENT,
			WORKFLOWS_CONTENT,
			DEPLOY_ACTOR_CONTENT,
		].map((p) => [p.id, p]),
	);

/** The steps visible on one tab, with their code resolved against the cluster. */
export function resolveQuickstart(
	product: ProductPageContent,
	lang: QuickstartLang,
	ctx: QuickstartContext,
): ResolvedStep[] {
	const steps: ResolvedStep[] = [];
	for (const step of product.steps) {
		const specs = step.blocks?.[lang] ?? step.blocks?.all;
		if (step.blocks && !specs) continue;
		steps.push({
			title: step.title,
			description: step.description,
			after: step.after,
			blocks: (specs ?? []).map((spec) => ({
				kind: spec.kind,
				fileName: spec.fileName,
				code: spec.code(ctx),
			})),
		});
	}
	return steps;
}

export interface ResolvedStep {
	title: string;
	description?: string;
	after?: string;
	blocks: { kind: CodeBlockSpec["kind"]; fileName?: string; code: string }[];
}

/** The prompt the "Copy prompt for agent" button copies: the shared Rivet setup prompt plus the product. */
export function productAgentPrompt(product: ProductPageContent): string {
	return `Fetch and execute the appropriate instructions to set me up for Rivet from ${SETUP_PROMPT_URL}, then help me get started with ${product.label} (${product.guidesUrl}) and deploy it to my cluster.`;
}
