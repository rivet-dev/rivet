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
import { queryClient } from "@/queries/global";
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
				<div className="-full flex items-safe-center justify-center [&_[data-component='stepper']]:w-auto px-4 h-full overflow-auto pt-8">
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
								maxRunners: 100_000,
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
													<Skeleton className="w-96 h-[180px]" />
													<Skeleton className="w-96 h-[250px]" />
													<Skeleton className="w-96 h-[200px]" />
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
										maxCount: 100_000,
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

	return (
		<div>
			<p className="text-sm text-muted-foreground mb-4">
				Deploy your application to Rivet Cloud, our serverless hosting
				solution. We manage the actor orchestration, state, and scaling
				for you.
			</p>
			<div className="flex items-center justify-start gap-4">
				<FormField
					control={control}
					name="provider"
					render={({ field }) => (
						<Combobox
							className="w-full"
							onValueChange={(value) =>
								setValue("provider", value)
							}
							value={field.value}
							options={deployOptions
								.filter((option) => !option.specializedPlatform)
								.map((option) => ({
									value: option.name,
									label: (
										<div className="flex items-center">
											<Icon
												icon={option.icon}
												className="me-2 !w-4 h-auto"
											/>
											{option.displayName}
										</div>
									),
								}))}
						/>
					)}
				/>
			</div>
		</div>
	);
}

function InstallStep() {
	return (
		<div className="flex flex-col gap-4">
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

const agentPrompt = `Read through the existing project to understand the codebase. I want to add Rivet Actors to this project. Ask me what I'd like to build with actors, then set it up using RivetKit and run it locally. Use the RivetKit skill for guidance.`;

function RunLocallyStep() {
	return (
		<div className="flex flex-col gap-5">
			<div>
				<p className="font-medium mb-1.5">
					Follow the quickstart guide
				</p>
				<p className="text-sm text-muted-foreground mb-2">
					Set up a Rivet actor project manually step-by-step.
				</p>
				<Button variant="outline" asChild>
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
			<div className="relative flex items-center gap-3">
				<div className="flex-1 border-t border-dashed" />
				<span className="text-xs text-muted-foreground">or</span>
				<div className="flex-1 border-t border-dashed" />
			</div>
			<div>
				<p className="font-medium mb-1.5">Use your coding agent</p>

				<p className="text-sm text-muted-foreground mb-2">
					Install the Rivet skill so your coding agent knows how to
					work with RivetKit.
				</p>
				<PackageManagerCode
					npx={`npx skills add ${skillsPath}`}
					yarn={`yarn dlx skills add ${skillsPath}`}
					bun={`bunx skills add ${skillsPath}`}
					deno={`deno run -A npm:skills add ${skillsPath}`}
					pnpm={`pnpx skills add ${skillsPath}`}
					git={`git clone https://github.com/${skillsPath}.git .skills`}
				/>

				<p className="text-sm text-muted-foreground mb-2 mt-4">
					Copy this prompt into your coding agent:
				</p>
				<div className="relative group rounded-md bg-muted/50 p-3 pr-10 text-sm font-mono leading-relaxed">
					{agentPrompt}
					<button
						type="button"
						className="absolute top-2 right-2 p-1.5 rounded-md text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-foreground hover:bg-muted transition-all"
						onClick={() => {
							navigator.clipboard.writeText(agentPrompt);
							toast.success("Copied to clipboard");
						}}
					>
						<Icon icon={faCopy} className="w-3.5 h-3.5" />
					</button>
				</div>
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

const rivetAgentInstructionsCode = `Load the Rivet skill and then:
1. Create a Dockerfile for the RivetKit deployment
2. Add the secret RIVET_CLOUD_TOKEN to your GitHub repository
3. Create .github/workflows/rivet-deploy.yml using rivet-dev/deploy-action@v1
4. Push and verify the GitHub Action succeeded
5. Once deployed, confirm actors appear in the Rivet dashboard`;

function useAgentInstructionsCode(provider?: Provider) {
	const providerDetails = deployOptions.find((p) => p.name === provider);
	const endpoint = useEndpoint();
	const runnerName = useWatch({ name: "runnerName" }) as string;
	const publishableToken = useRivetDsn({ kind: "publishable", endpoint });
	const secretToken = useRivetDsn({ kind: "secret", endpoint });

	if (provider === "rivet") {
		return rivetAgentInstructionsCode;
	}

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
	const code = useAgentInstructionsCode(provider);

	return (
		<Button
			type="button"
			variant="outline"
			size="sm"
			startIcon={<Icon icon={faCopy} />}
			onClick={() => {
				navigator.clipboard.writeText(code);
				toast.success("Copied to clipboard");
			}}
		>
			Using a Coding Agent? Copy Agent prompt
		</Button>
	);
}

const githubActionYaml = `name: Rivet Deploy

on:
  pull_request:
    types: [opened, synchronize, reopened, closed]
  push:
    branches: [main]

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
					<CopyAgentInstructionsButton provider="rivet" />
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
					<p className="font-medium mb-2">
						Verify GitHub Action succeeded
					</p>
					<p className="text-sm text-muted-foreground">
						Push your changes and confirm the{" "}
						<strong>Rivet Deploy</strong> workflow completes
						successfully in your repository's Actions tab.
					</p>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={5} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">Waiting for deploy</p>
					<p className="text-sm text-muted-foreground mb-3">
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
					</p>
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
						<div className="mt-4">
							<CopyAgentInstructionsButton provider={provider} />
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
								<p className="inline-block">
									The actor file is in the correct location
									and has the correct name.
								</p>
							</li>
							<li>
								<p className="inline-block">
									The actor is being exported properly.
								</p>
							</li>
							<li>
								<p className="inline-block">
									Check the terminal output for any errors
									during the build or runtime.
								</p>
							</li>
							<li>
								<p className="inline-block">
									Make sure your coding agent has completed
									the setup steps correctly.
								</p>
							</li>
							<li>
								<p className="inline-block mb-1">
									You're using correct environment variables:
								</p>
								<EnvVariables endpoint={endpoint} />
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
