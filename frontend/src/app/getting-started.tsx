import { Accordion, AccordionContent } from "@radix-ui/react-accordion";
import {
	faArrowRight,
	faBroadcastTower,
	faCopy,
	faDatabase,
	faDiagramProject,
	faLayerGroup,
	faMagnifyingGlass,
	faPlug,
	Icon,
} from "@rivet-gg/icons";
import { deployOptions, type Provider } from "@rivetkit/shared-data";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseInfiniteQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { Link, useNavigate, useRouter } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { type ReactNode, Suspense, useEffect, useMemo, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { toast } from "sonner";
import { match } from "ts-pattern";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	AccordionItem,
	AccordionTrigger,
	CodeFrame,
	type CodeFrameLikeElement,
	CodeGroup,
	CodeGroupSyncProvider,
	CodePreview,
	Combobox,
	FormField,
	Ping,
	Skeleton,
	useInterval,
} from "@/components";
import {
	useCloudNamespaceDataProvider,
	useDataProvider,
} from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { deriveProviderFromMetadata } from "@/lib/data";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { cloudEnv } from "@/lib/env";
import { usePublishableToken } from "@/queries/accessors";
import { queryClient } from "@/queries/global";
import { cn } from "../components/lib/utils";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { DeploymentCheck } from "./deployment-check";
import { useEndpoint } from "./dialogs/connect-manual-serverfull-frame";
import {
	buildServerlessConfig,
	ConfigurationAccordion,
} from "./dialogs/connect-manual-serverless-frame";
import { EnvVariables, useRivetDsn } from "./env-variables";
import { StepperForm } from "./forms/stepper-form";
import { Content } from "./layout";

const stepper = defineStepper(
	{
		id: "install",
		title: "Install RivetKit",
		schema: z.object({}),
		group: "local",
	},
	{
		id: "run",
		title: "Run locally",
		schema: z.object({}),
		group: "local",
	},
	{
		id: "explore",
		title: "Explore Rivet Actors",
		schema: z.object({}),
		group: "local",
	},
	{
		id: "provider",
		title: "Ready to deploy?",
		schema: z.object({ provider: z.string() }),
		group: "deploy",
	},
	{
		id: "backend",
		title: "Connect your Backend",
		assist: true,
		group: "deploy",
		schema: ConnectServerlessForm.deploymentSchema
			.pick({
				success: true,
			})
			.or(
				z.object({
					...ConnectServerlessForm.configurationSchema.shape,
					...ConnectServerlessForm.deploymentSchema.shape,
				}),
			),
	},
	{
		id: "frontend",
		title: "Create your first Actor",
		assist: true,
		group: "deploy",
		schema: z.object({}),
		showNext: false,
		showPrevious: false,
	},
);

export function GettingStarted({
	displayBackendOnboarding,
	provider,
}: {
	provider?: Provider;
	displayOnboarding?: boolean;
	displayBackendOnboarding?: boolean;
}) {
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: datacenters } = useSuspenseInfiniteQuery(
		dataProvider.datacentersQueryOptions(),
	);

	const { mutateAsync: mutateAsyncManagedPool } = useMutation({
		...dataProvider.upsertCurrentNamespaceManagedPoolMutationOptions(),
	});

	const { mutateAsync } = useMutation({
		...dataProvider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.runnerConfigsQueryOptions(),
			);
		},
	});

	const navigate = useNavigate();

	return (
		<Content className="flex flex-col items-center justify-safe-center">
			<motion.div
				className="relative min-w-0 overflow-hidden w-full"
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3 }}
			>
				<div className="-full flex items-safe-center justify-center [&_[data-component='stepper']>form]:mx-auto [&_[data-component='stepper']]:overflow-x-hidden [&_[data-component='stepper']]:w-full [&:has([data-wide='true'])_[data-component='stepper']>form]:max-w-[64rem] has-[[data-wide='true']]:w-auto [&_[data-component='stepper']>form]:max-w-[32rem] px-4 h-full overflow-auto pt-8">
					<CodeGroupSyncProvider>
						<StepperForm
							{...stepper}
							singlePage
							formId="onboarding"
							className="mb-8 mt-12"
							initialStep={
								provider
									? "backend"
									: displayBackendOnboarding
										? undefined
										: "frontend"
							}
							defaultValues={{
								provider: provider || "rivet",
								runnerName: "default",
								slotsPerRunner: 1,
								maxRunners: 10000,
								minRunners: 1,
								runnerMargin: 0,
								headers: [],

								requestLifespan: 900,
								datacenters: Object.fromEntries(
									datacenters.map((dc) => [dc.name, true]),
								),
							}}
							content={{
								install: () => (
									<StepContent>
										<InstallStep />
									</StepContent>
								),
								run: () => (
									<StepContent>
										<RunLocallyStep />
									</StepContent>
								),
								explore: () => (
									<StepContent wide>
										<ExploreRivet />
									</StepContent>
								),
								provider: () => (
									<StepContent>
										<ProviderSetup />
									</StepContent>
								),
								backend: () => (
									<StepContent>
										<Suspense
											fallback={
												<div className="space-y-6">
													<Skeleton className="w-full h-[180px]" />
													<Skeleton className="w-full h-[250px]" />
													<Skeleton className="w-full h-[200px]" />
												</div>
											}
										>
											<BackendSetup />
										</Suspense>
									</StepContent>
								),
								frontend: () => (
									<StepContent>
										<FrontendSetup />
									</StepContent>
								),
							}}
							onSubmit={() => {}}
							onPartialSubmit={async ({ stepper, values }) => {
								if (
									stepper.current.id === "provider" &&
									values.provider === "rivet"
								) {
									mutateAsyncManagedPool({
										displayName: "default",
										pool: "default",
										minCount: 0,
										maxCount: 1000,
									});
									return;
								}
								if (
									stepper.current.id === "backend" &&
									"endpoint" in values &&
									values.endpoint &&
									values.provider !== "rivet" &&
									values.success
								) {
									const config = await buildServerlessConfig(
										dataProvider,
										values,
										{ provider: values.provider },
									);

									await mutateAsync({
										name: values.runnerName,
										config,
									});

									await navigate({
										to: ".",
										search: (s) => ({
											...s,
											backendOnboardingSuccess: true,
										}),
									});
								}
							}}
						>
							<StepperFooter />
						</StepperForm>
					</CodeGroupSyncProvider>
				</div>
			</motion.div>
		</Content>
	);
}

function StepContent({
	children,
	wide,
}: {
	children: ReactNode;
	wide?: boolean;
}) {
	return (
		<motion.div
			className="mx-auto"
			data-component="step-content"
			data-wide={wide ? "true" : "false"}
			animate={{ maxWidth: wide ? "56rem" : "32rem", width: "100%" }}
			transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
			style={{ maxWidth: wide ? "56rem" : "32rem" }}
		>
			{children}
		</motion.div>
	);
}

function StepperFooter() {
	const s = stepper.useStepper();
	return (
		<div className="flex flex-col items-center gap-4 mt-6">
			{s.current.group === "local" && s.current.id !== "explore" ? (
				<Button
					type="button"
					variant="link"
					size="xs"
					className="text-muted-foreground"
					onClick={() => s.goTo("provider")}
					endIcon={<Icon icon={faArrowRight} className="ms-1" />}
				>
					Already have a project working locally? Skip to deploy
				</Button>
			) : null}
			{s.isLast ? (
				<Button
					asChild
					variant="link"
					size="xs"
					className="text-muted-foreground"
				>
					<Link to="." search={{ modal: "create-actor" }}>
						Manually Create Actor
					</Link>
				</Button>
			) : null}
		</div>
	);
}

function ProviderSetup() {
	const { setValue, control } = useFormContext();

	const filteredOptions = deployOptions.filter(
		(option) => !option.specializedPlatform,
	);

	return (
		<div>
			<p className="text-sm text-muted-foreground mb-4">
				Deploy your application to Rivet Cloud, our serverless hosting
				solution. We manage the actor orchestration, state, and scaling
				for you.
			</p>
			<FormField
				control={control}
				name="provider"
				render={({ field }) => {
					const rivetCloud = filteredOptions.find(
						(o) => o.name === "rivet",
					);
					const rest = filteredOptions.filter(
						(o) => o.name !== "rivet",
					);
					return (
						<div className="flex flex-col gap-2">
							{rivetCloud ? (
								<ProviderCard
									option={rivetCloud}
									isSelected={
										field.value === rivetCloud.name
									}
									onSelect={() =>
										setValue("provider", rivetCloud.name)
									}
								/>
							) : null}
							<div className="grid grid-cols-2 gap-2">
								{rest.map((option) => (
									<ProviderCard
										key={option.name}
										option={option}
										isSelected={
											field.value === option.name
										}
										onSelect={() =>
											setValue("provider", option.name)
										}
									/>
								))}
							</div>
						</div>
					);
				}}
			/>
		</div>
	);
}

function ProviderCard({
	option,
	isSelected,
	onSelect,
}: {
	option: (typeof deployOptions)[number];
	isSelected: boolean;
	onSelect: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onSelect}
			className={cn(
				"flex items-center gap-3 rounded-lg border px-4 py-3 text-left transition-colors cursor-pointer",
				isSelected
					? "border-primary bg-primary/5"
					: "border-border hover:border-muted-foreground/50",
			)}
		>
			<Icon
				icon={option.icon}
				className="!w-5 h-auto shrink-0 text-muted-foreground"
			/>
			<div className="min-w-0">
				<div className="flex items-center gap-2">
					<p className="text-sm font-medium truncate">
						{option.displayName}
					</p>
					{option.badge ? (
						<Badge
							variant="secondary"
							className="shrink-0 text-[10px] px-1.5 py-0"
						>
							{option.badge}
						</Badge>
					) : null}
				</div>
				<p className="text-xs text-muted-foreground truncate">
					{option.description}
				</p>
			</div>
		</button>
	);
}

function InstallStep() {
	return (
		<div className="flex flex-col gap-4">
			<AgentPromptBanner code={agentPrompt} />
			<p className="text-sm text-muted-foreground">
				Add RivetKit to your project to get started with Rivet Actors.
			</p>
			<PackageManagerCode
				npx="npm install rivetkit"
				yarn="yarn add rivetkit"
				pnpm="pnpm add rivetkit"
				bun="bun add rivetkit"
				deno="deno add npm:rivetkit"
			/>
		</div>
	);
}

const agentPrompt = `# RivetKit Local Dev Setup

Read through the existing project to understand the codebase. I want to add Rivet Actors to this project.

Before making any changes, read the Rivet Actors documentation at https://rivet.dev/docs/actors to understand how RivetKit works, including the actor API, state management, events, actions, connections, and client setup.

Add a note to the project's CLAUDE.md, AGENTS.md, or similar AI agent instructions file linking to https://rivet.dev/llms.txt as a reference for working with RivetKit in future conversations. Create the file if one doesn't exist.

## Walk me through the following steps

### Step 1: Ask what to build

Ask me what I'd like to build with actors before writing any code. Suggest ideas based on the project, such as:

- AI agents
- Coding agent & sandbox orchestration
- Realtime collaboration (shared docs, cursors, chat)
- Workflow automation (background jobs, queues, scheduling)
- Per-user or per-tenant data backends
- Multiplayer games
- WebSocket servers and custom protocols
- Local-first sync (offline-first apps that sync state when reconnected)
- Rate limiters & session management (per-user stateful middleware)

### Step 2: Install RivetKit

Install RivetKit: \`npm install rivetkit\`

If the project needs a frontend (recommended for realtime features), also install the React client: \`npm install @rivetkit/react\`

### Step 3: Define actors and registry

Create an actors file with a registry:

\`\`\`ts
import { actor, setup } from "rivetkit";

const myActor = actor({
  state: { /* initial state */ },
  actions: {
    myAction: (c, arg: string) => { /* ... */ },
  },
});

export const registry = setup({
  use: { myActor },
});
\`\`\`

### Step 4: Expose the server

If the project already has a server (Hono, Express, etc), integrate with \`registry.handler()\`:

\`\`\`ts
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
\`\`\`

Otherwise, use \`registry.serve()\` for a standalone server.

### Step 5: Connect a client

\`createClient\` from \`rivetkit/client\` works on both frontend and backend. It automatically detects the environment:
- **Browser**: defaults to \`window.location.origin + "/api/rivet"\`
- **Server**: defaults to \`http://127.0.0.1:6420\`

\`\`\`ts
import { createClient } from "rivetkit/client";
import type { registry } from "./actors";

const client = createClient<typeof registry>();
const handle = client.myActor.getOrCreate(["my-key"]);

// Stateless call
await handle.myAction("hello");

// Stateful connection (for realtime events)
const conn = handle.connect();
conn.on("eventName", (data) => console.log(data));
\`\`\`

For React frontends, use \`@rivetkit/react\` for hooks-based integration (see https://rivet.dev/docs/clients/react):

\`\`\`tsx
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./actors";

const { useActor } = createRivetKit<typeof registry>();

function MyComponent() {
  const actor = useActor({ name: "myActor", key: ["my-key"] });
  actor.useEvent("eventName", (data) => console.log(data));
  return <button onClick={() => actor.connection?.myAction("hello")}>Go</button>;
}
\`\`\`

### Step 6: Verify setup

- Verify \`/api/rivet/metadata\` returns 200.
- Test actors programmatically using the inspector HTTP API to verify they work (see https://rivet.dev/docs/actors/debugging). Key endpoints are available at \`/api/rivet/actor/{actor_id}/inspector/*\`:
  - \`GET /inspector/summary\` - full actor snapshot
  - \`POST /inspector/action/{name}\` - execute an action with \`{"args": [...]}\`
  - \`GET /inspector/state\` - read actor state
- If a frontend was implemented, use browser MCP to verify it works end-to-end.

For detailed setup instructions, see the quickstart guides:

- Node.js & Bun: https://rivet.dev/docs/actors/quickstart/backend
- React: https://rivet.dev/docs/actors/quickstart/react
- Next.js: https://rivet.dev/docs/actors/quickstart/next-js
- Cloudflare Workers: https://rivet.dev/docs/actors/quickstart/cloudflare-workers

## If You Get Stuck

Check the troubleshooting guide at https://rivet.dev/docs/actors/troubleshooting. If that doesn't help, prompt the user to join the Rivet Discord (https://rivet.dev/discord) or file an issue on GitHub (https://github.com/rivet-dev/rivet). Generate a report with: symptoms (error, local vs deployed), what you've tried, and environment (RivetKit version, runtime, provider, HTTP router).`;

function RunLocallyStep() {
	return (
		<div className="flex flex-col gap-5">
			<AgentPromptBanner code={agentPrompt} />
			<div className="w-full flex items-center justify-between rounded-lg px-4 py-3 border border-border">
				<div>
					<p className="font-medium mb-1.5">
						Follow the quickstart guide
					</p>
					<p className="text-sm text-muted-foreground">
						Set up a Rivet actor project manually step-by-step.
					</p>
				</div>
				<Button variant="outline" asChild className="shrink-0 ml-4">
					<a
						href="https://rivet.dev/docs/actors/quickstart/"
						target="_blank"
						rel="noopener noreferrer"
					>
						Quickstart Guide
						<Icon icon={faArrowRight} className="ms-2" />
					</a>
				</Button>
			</div>
		</div>
	);
}

function StepNumber({ n }: { n: number }) {
	return (
		<div className="flex-shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary text-xs font-medium flex items-center justify-center mt-0.5">
			{n}
		</div>
	);
}

const exploreFeatures = [
	{
		id: "inspector",
		icon: faMagnifyingGlass,
		label: "Inspector",
		title: "RivetKit Inspector",
		description:
			"A built-in visual debugger that runs locally. View active actors, monitor connections, and trace every interaction in real-time.",
		docsUrl: "https://rivet.dev/docs/actors/debugging",
	},
	{
		id: "state",
		icon: faLayerGroup,
		label: "In-memory state",
		title: "In-memory State",
		description:
			"Each actor has its own isolated state co-located with compute for instant reads and writes. Persist with SQLite or BYO database.",
		docsUrl: "https://rivet.dev/docs/actors/state",
	},
	{
		id: "storage",
		icon: faDatabase,
		label: "Storage",
		title: "Built-in Storage",
		description:
			"Actors have built-in KV storage and SQLite. Browse stored data and watch writes happen live in the inspector.",
		docsUrl: "https://rivet.dev/docs/actors/storage",
	},
	{
		id: "workflows",
		icon: faDiagramProject,
		label: "Workflows",
		title: "Durable Workflows",
		description:
			"Orchestrate multi-step processes that survive crashes and restarts. Automatic retries and step-through history visualization.",
		docsUrl: "https://rivet.dev/docs/actors/workflows",
	},
	{
		id: "events",
		icon: faBroadcastTower,
		label: "Events",
		title: "Event Streams",
		description:
			"Actors can broadcast events to connected clients. Real-time bidirectional streaming built in.",
		docsUrl: "https://rivet.dev/docs/actors/events",
	},
	{
		id: "rpcs",
		icon: faPlug,
		label: "RPCs",
		title: "Remote Procedure Calls",
		description:
			"Call actor methods directly from your client with full type safety. The inspector shows every RPC call, its arguments, and response.",
		docsUrl: "https://rivet.dev/docs/actors/rpc",
	},
];

const CAROUSEL_INTERVAL = 5000;

const GIF_SRC = `/onboarding-demo.gif?t=${Date.now()}`;

function ExploreRivet() {
	const [activeIndex, setActiveIndex] = useState(0);
	const feature = exploreFeatures[activeIndex];

	const { reset } = useInterval(() => {
		setActiveIndex((prev) => (prev + 1) % exploreFeatures.length);
	}, CAROUSEL_INTERVAL);

	return (
		<div className="flex flex-col gap-6">
			<div className="rounded-lg border bg-muted/30 aspect-video flex items-center justify-center overflow-hidden">
				<img
					src={GIF_SRC}
					alt="Rivet Actors demo"
					className="w-full h-full object-cover"
				/>
			</div>
			<div className="flex gap-0">
				{exploreFeatures.map((f, i) => (
					<button
						key={f.id}
						type="button"
						onClick={() => {
							setActiveIndex(i);
							reset();
						}}
						className={`flex-1 text-left px-3 pt-3 pb-2 transition-colors relative ${
							i === activeIndex
								? "text-foreground"
								: "text-muted-foreground hover:text-foreground"
						}`}
					>
						<div className="absolute top-0 left-0 right-0 h-0.5 bg-muted overflow-hidden rounded-full">
							<div
								className="h-full bg-primary rounded-full"
								style={{
									width: i === activeIndex ? "100%" : "0%",
									transition:
										i === activeIndex
											? `width ${CAROUSEL_INTERVAL}ms linear`
											: "none",
								}}
							/>
						</div>
						<div className="flex items-center gap-1.5 mb-1">
							<Icon
								icon={f.icon}
								className="w-3 h-3 flex-shrink-0"
							/>
							<span className="text-xs font-semibold truncate">
								{f.label}
							</span>
						</div>
					</button>
				))}
			</div>
			<div className="min-h-[4.5rem]">
				<h3 className="text-base font-semibold mb-1">
					{feature.title}
				</h3>
				<p className="text-sm text-muted-foreground leading-relaxed">
					{feature.description}{" "}
					<a
						href={feature.docsUrl}
						target="_blank"
						rel="noreferrer"
						className="inline-flex items-center gap-1 text-primary hover:underline"
					>
						Learn more
						<Icon icon={faArrowRight} className="w-3 h-3" />
					</a>
				</p>
			</div>
		</div>
	);
}

function buildRivetAgentInstructionsCode({
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
	return `## Step 1: Load Rivet Skill

Load the Rivet skill. If the skill is not installed, tell the user to run this and restart the agent:

\`\`\`
npx skills add rivet-dev/skills -s rivetkit
\`\`\`

## Step 2: Create Dockerfile

If the project does not already have a \`Dockerfile\`, create one. Use this as a starting point and adjust the package manager (npm/pnpm/yarn), file paths, and entrypoint to match the project:

\`\`\`dockerfile
FROM node:24-alpine

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build --if-present

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

## Step 3: Add GitHub Secret

Add the secret RIVET_CLOUD_TOKEN to your GitHub repository:

\`\`\`
gh secret set RIVET_CLOUD_TOKEN --body "${cloudToken}"
\`\`\`

## Step 4: Create GitHub Action

Create .github/workflows/rivet-deploy.yml using rivet-dev/deploy-action@v1:

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
      - uses: rivet-dev/deploy-action@v1
        with:
          rivet-token: \${{ secrets.RIVET_CLOUD_TOKEN }}
\`\`\`

## Step 5: Push and Verify

Push the changes and monitor the GitHub Action until it completes:

1. Push to the repository.
2. Use \`gh run list\` and \`gh run watch\` to poll the action status. Do not move on until the run finishes.
3. If the action succeeds, proceed to Step 6.
4. If the action fails, inspect the logs with \`gh run view --log-failed\`, fix the issue, push again, and repeat from step 2.

## Step 6: Verify Deployment

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

## Troubleshooting

- There is no Rivet CLI. Do not attempt to use or install one. All deployment is done via the GitHub Action and all interaction is done via HTTP APIs (curl).
- Architecture: The GitHub Action builds your Docker image and pushes it to Rivet. Rivet runs the container serverlessly. When you create an actor, Rivet communicates with the \`/api/rivet/*\` endpoint inside the container to manage its lifecycle.
- For more troubleshooting help, see: https://rivet.dev/docs/actors/troubleshooting/`;
}

function useRivetAgentInstructionsCode() {
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: cloudToken } = useSuspenseQuery(
		dataProvider.createApiTokenQueryOptions({ name: "Onboarding" }),
	);
	const publishableRawToken = usePublishableToken();
	const namespace = dataProvider.engineNamespace;

	return buildRivetAgentInstructionsCode({
		cloudToken,
		publishableToken: publishableRawToken,
		namespace,
		project: dataProvider.project,
		organization: dataProvider.organization,
		cloudNamespace: dataProvider.cloudNamespace,
		apiUrl: cloudEnv().VITE_APP_API_URL,
		cloudApiUrl: cloudEnv().VITE_APP_CLOUD_API_URL,
	});
}

function useOtherAgentInstructionsCode(provider?: Provider) {
	const providerDetails = deployOptions.find((p) => p.name === provider);
	const endpoint = useEndpoint();
	const runnerName = useWatch({ name: "runnerName" }) as string;
	const publishableToken = useRivetDsn({ kind: "publishable", endpoint });
	const secretToken = useRivetDsn({ kind: "secret", endpoint });

	const providerStr =
		providerDetails?.displayName ?? provider ?? "your chosen provider";
	return `Load the Rivet skill and then:
1. Integrate Rivet in to the project
2. Verify it works on the local machine
3. Deploy to ${providerStr} and configure
   the following environment variables:

  RIVET_PUBLIC_ENDPOINT=${publishableToken}
  RIVET_ENDPOINT=${secretToken}${
		runnerName !== "default"
			? `
  RIVET_RUNNER_NAME=${runnerName}`
			: ""
  }

4. Tell the user the URL to past in
   to the Rivet dashboard`;
}

function CopyAgentInstructionsButton({ provider }: { provider?: Provider }) {
	if (provider === "rivet") {
		return <RivetCopyAgentInstructionsButton />;
	}
	return <OtherCopyAgentInstructionsButton provider={provider} />;
}

function AgentPromptBanner({ code }: { code: string }) {
	return (
		<button
			type="button"
			onClick={() => {
				navigator.clipboard.writeText(code);
				toast.success("Copied to clipboard");
			}}
			className="relative w-full flex items-center justify-between rounded-lg px-4 py-3 border border-primary overflow-hidden group cursor-pointer"
		>
			<div className="flex items-center gap-2 text-left">
				<Badge className="shrink-0">Recommended</Badge>
				<span className="text-sm font-medium text-white">
					Using a Coding Agent? Use this pre-built prompt to get started
					faster.
				</span>
			</div>
			<Button
				asChild
				variant="ghost"
				size="sm"
				className="relative z-10 flex items-center gap-1.5 text-xs font-semibold shrink-0 ml-4"
			>
				<div>
					<Icon icon={faCopy} className="w-3.5 h-3.5 text-primary" />
					Copy prompt
				</div>
			</Button>
		</button>
	);
}

function RivetCopyAgentInstructionsButton() {
	const code = useRivetAgentInstructionsCode();
	return <AgentPromptBanner code={code} />;
}

function OtherCopyAgentInstructionsButton({
	provider,
}: {
	provider?: Provider;
}) {
	const code = useOtherAgentInstructionsCode(provider);
	return <AgentPromptBanner code={code} />;
}

const githubActionYaml = `name: Rivet Deploy

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
      - uses: rivet-dev/deploy-action@v1
        with:
          rivet-token: \${{ secrets.RIVET_CLOUD_TOKEN }}`;

function BackendSetupRivet() {
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: cloudToken } = useSuspenseQuery(
		dataProvider.createApiTokenQueryOptions({ name: "Onboarding" }),
	);

	const ghSecretCmd = cloudToken
		? `gh secret set RIVET_CLOUD_TOKEN --body "${cloudToken}"`
		: "gh secret set RIVET_CLOUD_TOKEN";

	return (
		<div className="flex flex-col gap-6">
			<CopyAgentInstructionsButton provider="rivet" />
			<div className="flex gap-3">
				<StepNumber n={1} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">
						Create a Dockerfile for your RivetKit deployment
					</p>
					<p className="text-sm text-muted-foreground mb-3">
						Add a{" "}
						<code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">
							Dockerfile
						</code>{" "}
						to the root of your project that builds and runs your
						RivetKit server.
					</p>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={2} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">Add GitHub secret</p>
					<p className="text-sm text-muted-foreground mb-3">
						Add your Rivet token as a repository secret named{" "}
						<code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">
							RIVET_CLOUD_TOKEN
						</code>
						.
					</p>
					<CodeGroup className="my-0">
						{[
							<CodeFrame
								key="gh-secret"
								language="bash"
								title="bash"
								code={() => ghSecretCmd}
								className="m-0"
							>
								<CodePreview
									language="bash"
									className="text-left"
									code={ghSecretCmd}
								/>
							</CodeFrame>,
						]}
					</CodeGroup>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={3} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">Add GitHub Action</p>
					<p className="text-sm text-muted-foreground mb-3">
						Create{" "}
						<code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">
							.github/workflows/rivet-deploy.yml
						</code>{" "}
						to automatically deploy on every push and pull request.
					</p>
					<CodeGroup className="my-0">
						{[
							<CodeFrame
								key="gh-action"
								language="yaml"
								title=".github/workflows/rivet-deploy.yml"
								code={() => githubActionYaml}
								className="m-0"
							>
								<CodePreview
									language="yaml"
									className="text-left"
									code={githubActionYaml}
								/>
							</CodeFrame>,
						]}
					</CodeGroup>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={4} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">Deploy to Rivet Cloud</p>
					<p className="text-sm text-muted-foreground mb-2">
						Push your changes to trigger the{" "}
						<strong>Rivet Deploy</strong> workflow. The status check
						below will update automatically once your backend is
						deployed.
					</p>
					<div className="border rounded-md py-8">
						<div className="flex gap-2 justify-center items-center flex-col py-2 px-8">
							<DeploymentCheck
								validateConfig={(data) =>
									!!data?.find(([, value]) =>
										Object.values(value.datacenters).some(
											(dc) =>
												dc.serverless &&
												deriveProviderFromMetadata(
													dc.metadata,
												) === "rivet",
										),
									)
								}
								validatePool={(data) => !!data?.config.image}
							/>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}

function BackendSetup() {
	const provider = useWatch({ name: "provider" });

	if (provider !== "rivet") {
		return (
			<div className="flex flex-col gap-6">
				<CopyAgentInstructionsButton provider={provider} />

				<div className="flex gap-3">
					<StepNumber n={1} />
					<div className="flex-1 min-w-0">
						<p className="font-medium mb-2">
							Paste your deployment endpoint
						</p>
						<p className="text-sm text-muted-foreground mb-3">
							Your coding agent will provide a URL after
							deployment.
						</p>
						<div className="space-y-2">
							<ConnectServerlessForm.Endpoint
								placeholder={match(provider)
									.with(
										"vercel",
										() =>
											"https://your-vercel-deployment.vercel.app",
									)
									.with(
										"railway",
										() => "https://your-app.up.railway.app",
									)
									.otherwise(
										() => "https://your-deployment.com",
									)}
							/>
							<ConfigurationAccordion />
							<ConnectServerlessForm.ConnectionCheck
								provider={provider}
							/>
						</div>
					</div>
				</div>
			</div>
		);
	}

	return <BackendSetupRivet />;
}

const skillsPath = "rivet-dev/skills";

function FrontendSetup() {
	const dataProvider = useDataProvider();

	const { data: builds } = useInfiniteQuery({
		...dataProvider.buildsQueryOptions(),
		maxPages: 1,
	});

	const { data: actors } = useQuery({
		...dataProvider.actorsCountQueryOptions(),
		enabled: (builds?.length || 0) > 0,
		maxPages: 1,
		refetchInterval: 2500,
	});

	const hasActors = (actors || 0) > 0;

	const navigate = useNavigate();
	const router = useRouter();

	const { data: config } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		select: (data) =>
			Object.values(data.pages[0].runnerConfigs || {})
				.flatMap((r) => Object.values(r.datacenters))
				.filter((dc) => dc.serverless)?.[0].serverless,
	});

	const deploymentUrl = useMemo(() => {
		if (!config?.url) return null;
		try {
			const url = new URL(config.url);
			url.pathname = "/";
			return url.toString();
		} catch {
			return null;
		}
	}, [config?.url]);

	useEffect(() => {
		const success = async () => {
			successfulBackendSetupEffect();
			router.invalidate();
			return navigate({
				to: ".",
				search: (s) => ({
					...s,
					onboardingSuccess: true,
				}),
			});
		};
		if (hasActors) {
			success().catch((error) => {
				console.error(error);
			});
		}
	}, [hasActors, navigate, router]);

	const endpoint = useEndpoint();

	return (
		<div className="space-y-2">
			<div className="border rounded-md py-10">
				<div className="flex gap-2 justify-center items-center py-2 px-8">
					<div className="relative mr-4">
						<Ping variant="pending" className="relative" />
					</div>
					<p>Waiting for an Actor to be created...</p>
				</div>

				<div className="flex items-center justify-center mt-6 gap-4">
					{deploymentUrl ? (
						<Button variant="outline">
							<a
								href={deploymentUrl}
								target="_blank"
								rel="noopener noreferrer"
							>
								Visit Deployment
							</a>
						</Button>
					) : (
						<Button variant="outline" asChild>
							<Link to="." search={{ modal: "create-actor" }}>
								Create Actor
							</Link>
						</Button>
					)}
				</div>
			</div>
			<Accordion type="single" collapsible className="mt-10">
				<AccordionItem value="troubleshooting">
					<AccordionTrigger className="w-full flex items-center justify-between px-4 py-2">
						Troubleshooting
					</AccordionTrigger>
					<AccordionContent className="px-4 py-2  rounded-md border">
						<p className="mt-2">
							If your actor isn't showing up, check the following:
						</p>
						<ul className="list-disc list-inside mt-2">
							<li>
								<span>
									The actor file is in the correct location
									and has the correct name.
								</span>
							</li>
							<li>
								<span>
									The actor is being exported properly.
								</span>
							</li>
							<li>
								<span>
									Check the terminal output for any errors
									during the build or runtime.
								</span>
							</li>
							<li>
								<span>
									Make sure your coding agent has completed
									the setup steps correctly.
								</span>
							</li>
							<li>
								<span className="inline-block mb-1">
									You're using correct environment variables:
								</span>
								<Suspense
									fallback={
										<Skeleton className="w-full h-20" />
									}
								>
									<EnvVariables endpoint={endpoint} />
								</Suspense>
							</li>
						</ul>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	);
}

function PackageManagerCode(props: {
	npx?: string;
	yarn?: string;
	pnpm?: string;
	bun?: string;
	deno?: string;
	git?: string;
	footer?: ReactNode;
	header?: ReactNode;
}) {
	const npx = props.npx ? (
		<CodeFrame
			language="bash"
			title="npm"
			footer={props.footer}
			code={() => props.npx || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.npx}
			/>
		</CodeFrame>
	) : null;

	const yarn = props.yarn ? (
		<CodeFrame
			language="bash"
			title="yarn"
			footer={props.footer}
			code={() => props.yarn || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.yarn}
			/>
		</CodeFrame>
	) : null;

	const pnpm = props.pnpm ? (
		<CodeFrame
			language="bash"
			title="pnpm"
			footer={props.footer}
			code={() => props.pnpm || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.pnpm}
			/>
		</CodeFrame>
	) : null;

	const bun = props.bun ? (
		<CodeFrame
			language="bash"
			title="bun"
			footer={props.footer}
			code={() => props.bun || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.bun}
			/>
		</CodeFrame>
	) : null;

	const deno = props.deno ? (
		<CodeFrame
			language="bash"
			title="deno"
			footer={props.footer}
			code={() => props.deno || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.deno}
			/>
		</CodeFrame>
	) : null;

	const git = props.git ? (
		<CodeFrame
			language="bash"
			title="git"
			footer={props.footer}
			code={() => props.git || ""}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={props.git}
			/>
		</CodeFrame>
	) : null;

	return (
		<CodeGroup
			className="my-0"
			syncId="package-manager"
			header={props.header}
		>
			{[npx, yarn, pnpm, bun, deno, git].filter(
				(el): el is CodeFrameLikeElement => Boolean(el),
			)}
		</CodeGroup>
	);
}
