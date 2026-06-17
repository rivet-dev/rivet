import {
	faArrowRight,
	faCheck,
	faChevronDown,
	faCopy,
	Icon,
} from "@rivet-gg/icons";
import { deployOptions, type Provider } from "@rivetkit/shared-data";
import {
	useMutation,
	useQuery,
	useSuspenseInfiniteQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { type ReactNode, Suspense } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { toast } from "sonner";
import { match } from "ts-pattern";
import z from "zod";
import * as ConnectServerfullForm from "@/app/forms/connect-manual-serverfull-form";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	CodeFrame,
	CodeGroup,
	CodeGroupSyncProvider,
	CodePreview,
	Skeleton,
} from "@/components";
import {
	useCloudNamespaceDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { deriveProviderFromMetadata } from "@/lib/data";
import { cloudEnv, engineEnv } from "@/lib/env";
import { features } from "@/lib/features";
import { usePublishableToken } from "@/queries/accessors";
import { queryClient } from "@/queries/global";
import { cn } from "../components/lib/utils";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../components/ui/dropdown-menu";
import { TEST_IDS } from "../utils/test-ids";
import { DeploymentCheck } from "./deployment-check";
import { useEndpoint } from "./dialogs/connect-manual-serverfull-frame";
import {
	buildServerlessConfig,
	Configuration,
	ConfigurationAccordion,
} from "./dialogs/connect-manual-serverless-frame";
import { EnvVariables, useRivetDsn } from "./env-variables";
import { StepperForm } from "./forms/stepper-form";
import { Content } from "./layout";
import { RunnerConfigToggleGroup } from "./runner-config-toggle-group";
import {
	getAgentInstructionsPrompt,
	getComputeAddendum,
} from "@/content/agent-prompts";

function platformTitle(provider: unknown): string {
	return (
		deployOptions.find((o) => o.name === provider)?.displayName ??
		"Rivet Compute"
	);
}

const stepper = defineStepper(
	{
		id: "local",
		title: "Run locally",
		description: "Get your first Rivet Actor running on your machine.",
		next: "Continue to deploy",
		schema: z.object({}),
		group: "local",
	},
	{
		id: "deploy",
		title: "Deploy",
		titleFor: (values: Record<string, unknown>) =>
			`Deploy to ${platformTitle(values.provider)}`,
		next: "Done",
		previous: "Back",
		assist: true,
		group: "deploy",
		schema: (values: Record<string, unknown>) => {
			const provider = (values.provider as string) || "rivet";
			if (provider === "rivet") {
				return z.object({ success: z.literal(true) });
			}
			if ((values.mode as string) === "serverfull") {
				return z.object({
					mode: z.literal("serverfull"),
					runnerName: z.string().min(1, "Runner name is required"),
					datacenter: z.string().min(1, "Please select a region"),
					customName: z
						.string()
						.trim()
						.max(32, "Name is too long")
						.optional(),
					customIcon: z.string().optional(),
				});
			}
			return z.object({
				mode: z
					.union([z.literal("serverless"), z.literal("serverfull")])
					.optional(),
				...ConnectServerlessForm.configurationSchema.shape,
				...ConnectServerlessForm.deploymentSchema.shape,
			});
		},
	},
);

export function GettingStarted({
	displayFrontendOnboarding,
	provider,
}: {
	provider?: Provider;
	displayFrontendOnboarding?: boolean;
}) {
	const dataProvider = useEngineCompatDataProvider();
	useSuspenseInfiniteQuery(dataProvider.datacentersQueryOptions());

	const { mutateAsync: mutateAsyncManagedPool } = useMutation(
		"upsertCurrentNamespaceManagedPoolMutationOptions" in dataProvider
			? dataProvider.upsertCurrentNamespaceManagedPoolMutationOptions()
			: {
					mutationFn: async () => {
						throw new Error(
							"Managed pools are only available in cloud",
						);
					},
				},
	);

	const { data: initialRunnerConfig } = useSuspenseQuery({
		...dataProvider.runnerConfigQueryOptions({
			name: "default",
			safe: true,
		}),
		select: (data) => {
			const config = Object.values(data?.datacenters || {}).find(
				(dc) => dc.serverless,
			);
			const serverlessConfig = config?.serverless;

			if (!serverlessConfig) {
				return null;
			}

			return {
				...serverlessConfig,
				runnerName: "default",
				endpoint: serverlessConfig.url,
				headers: Array.from(
					Object.entries(serverlessConfig.headers || {}),
				),
				provider: deriveProviderFromMetadata(config?.metadata || {}),
				datacenters: Object.fromEntries(
					Object.keys(data.datacenters).map((name) => [name, true]),
				),
			};
		},
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

	// Rivet Compute is cloud-only (its deploy path consumes cloud-namespace data
	// providers and managed-pool APIs). On flavors without compute, default to
	// the first non-rivet platform so the deploy step renders the engine-backed
	// connect flow instead of crashing on a missing cloud route.
	const defaultProvider = features.compute
		? provider || "rivet"
		: provider && provider !== "rivet"
			? provider
			: (deployOptions.find((o) => o.name !== "rivet")?.name ?? "custom");

	const defaultValues = {
		runnerName: "default",
		headers: [],
		requestLifespan: 900,
		drainGracePeriod: 0,
		provider: defaultProvider,
		datacenters: {},
		datacenter: "",
		mode: "serverless" as "serverless" | "serverfull",
		...(initialRunnerConfig || {}),
	};

	// Saves a non-Rivet provider's runner config on finish. The Rivet path
	// deploys via the CLI and writes no config here.
	const saveProviderConfig = async (values: Record<string, unknown>) => {
		const v = values as unknown as {
			runnerName: string;
			provider: string;
			mode?: "serverless" | "serverfull";
			datacenter?: string;
			customName?: string;
			customIcon?: string;
		};
		if (v.mode === "serverfull") {
			const existingConfig = await queryClient.fetchQuery(
				dataProvider.runnerConfigQueryOptions({
					name: v.runnerName,
					safe: true,
				}),
			);
			const existing = existingConfig?.datacenters || {};
			const isCustom =
				v.provider === "custom" || v.provider === "custom-platform";
			const customName = isCustom
				? v.customName?.trim() || undefined
				: undefined;
			const customIcon = isCustom ? v.customIcon || undefined : undefined;
			await mutateAsync({
				name: v.runnerName,
				config: {
					...existing,
					[v.datacenter as string]: {
						normal: {},
						metadata: {
							provider: v.provider,
							...(customName ? { customName } : {}),
							...(customIcon ? { customIcon } : {}),
						},
					},
				},
			});
		} else {
			const config = await buildServerlessConfig(
				dataProvider,
				values as unknown as Parameters<
					typeof buildServerlessConfig
				>[1],
				{ provider: v.provider },
			);
			await mutateAsync({ name: v.runnerName, config });
		}
	};

	return (
		<Content className="flex-1 min-h-0 !h-auto !overflow-hidden flex flex-col items-center justify-safe-center">
			<motion.div
				className="relative min-w-0 w-full flex-1 min-h-0 flex flex-col"
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3 }}
				data-testid={TEST_IDS.Onboarding.GettingStartedWizard}
			>
				<SkipOnboardingHeaderLink />
				<div className="flex-1 min-h-0 overflow-auto flex items-safe-center justify-center px-4 py-8 [&_[data-component='stepper']]:w-full [&_[data-component='stepper']>form]:w-full">
					<div className="relative w-full max-w-[36rem] rounded-xl border bg-card p-6 sm:p-8 shadow-sm">
						<CodeGroupSyncProvider>
							<StepperForm
								{...stepper}
								singlePage
								formId="onboarding"
								className="mt-2"
								header={<OnboardingHeader />}
								initialStep={
									displayFrontendOnboarding || provider
										? "deploy"
										: undefined
								}
								defaultValues={defaultValues}
								content={{
									local: () => (
										<StepContent>
											<RunLocallyStep />
										</StepContent>
									),
									deploy: () => (
										<StepContent>
											<Suspense
												fallback={
													<div className="space-y-6">
														<Skeleton className="w-full h-[180px]" />
														<Skeleton className="w-full h-[200px]" />
													</div>
												}
											>
												<DeployScreen />
											</Suspense>
										</StepContent>
									),
								}}
								onSubmit={async ({ values, form }) => {
									// Finish: the deploy step is last. Save a
									// non-Rivet provider's config, then go to the
									// dashboard.
									const accumulated = values as Record<
										string,
										unknown
									>;
									const live = form.getValues() as Record<
										string,
										unknown
									>;
									const provider = (accumulated.provider ??
										live.provider) as string | undefined;
									if (provider && provider !== "rivet") {
										await saveProviderConfig({
											...accumulated,
											provider,
										});
									}
									await navigate({
										to: ".",
										search: (s) => ({
											...s,
											skipOnboarding: true,
										}),
									});
								}}
								onPartialSubmit={async ({
									stepper,
									values,
									form,
								}) => {
									// Provider may be lost from accumulated values
									// after form reset, so read it from the live form.
									const provider = ((
										values as Record<string, unknown>
									).provider ??
										(
											form.getValues() as Record<
												string,
												unknown
											>
										).provider) as string | undefined;
									// On entering the deploy step, default to Rivet
									// Compute and provision its managed pool.
									if (stepper.current.id === "local") {
										if (
											features.compute &&
											(provider ?? "rivet") === "rivet"
										) {
											try {
												await mutateAsyncManagedPool({
													displayName: "default",
													pool: "default",
													image: undefined,
													maxConcurrentActors: 50_000,
													environment: {},
													command: undefined,
													args: [],
												});
											} catch (error) {
												console.error(
													"Failed to create default managed pool during onboarding",
													error,
												);
												toast.error(
													"Couldn't create the default Rivet Compute pool. You can retry from the deploy step.",
												);
											}
										}

										await Promise.all([
											...(features.auth &&
											"publishableTokenQueryOptions" in
												dataProvider
												? [
														queryClient.prefetchQuery(
															dataProvider.publishableTokenQueryOptions(),
														),
														queryClient.prefetchInfiniteQuery(
															dataProvider.datacentersQueryOptions(),
														),
													]
												: []),
											dataProvider.engineAdminTokenQueryOptions(),
										]);
									}
								}}
							>
								<StepperFooter />
							</StepperForm>
						</CodeGroupSyncProvider>
					</div>
				</div>
			</motion.div>
		</Content>
	);
}

function StepContent({ children }: { children: ReactNode }) {
	return (
		<div className="w-full" data-component="step-content">
			{children}
		</div>
	);
}

function StepperFooter() {
	return null;
}

// Header rendered above each step. A full-width segmented progress bar anchors
// the top of the card, with the step label and (on the deploy step) the platform
// switcher on the row below it so nothing competes with the left-aligned title.
function OnboardingHeader() {
	const s = stepper.useStepper();
	return (
		<OnboardingProgress
			action={s.current.id === "deploy" ? <SwitchPlatform /> : null}
		/>
	);
}

// Platform switcher pinned top-right on the deploy screen. Defaults to Rivet
// Compute; selecting another option updates the `provider` form field, which
// re-tunes the deploy screen.
function SwitchPlatform() {
	const { setValue } = useFormContext();
	const provider = (useWatch({ name: "provider" }) as string) || "rivet";
	const options = deployOptions.filter(
		(o) => features.compute || o.name !== "rivet",
	);
	const otherOptions = options.filter((o) => o.name !== provider);
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					type="button"
					variant="outline"
					endIcon={<Icon icon={faChevronDown} className="ms-2" />}
				>
					Switch platform
					{otherOptions.length > 0 ? (
						<span className="ms-2 flex items-center -space-x-1.5">
							{otherOptions.slice(0, 4).map((option) => (
								<span
									key={option.name}
									className="flex size-5 items-center justify-center rounded-full border bg-background"
								>
									<Icon
										icon={option.icon}
										className="!size-3 text-muted-foreground"
									/>
								</span>
							))}
						</span>
					) : null}
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent
				side="bottom"
				align="end"
				sideOffset={6}
				className="w-[22rem] max-h-[60vh] overflow-auto"
			>
				{options.map((option) => (
					<DropdownMenuItem
						key={option.name}
						className="items-start gap-3 py-2"
						onClick={() =>
							setValue("provider", option.name, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Icon
							icon={option.icon}
							className="!size-4 mt-0.5 shrink-0 text-muted-foreground"
						/>
						<div className="min-w-0 flex-1">
							<div className="flex items-center gap-2">
								<span className="text-sm font-medium">
									{option.displayName}
								</span>
								{option.badge ? (
									<Badge
										variant="outline"
										className="text-[10px] leading-none py-0.5 px-1.5 font-medium"
									>
										{option.badge}
									</Badge>
								) : null}
								{provider === option.name ? (
									<Icon
										icon={faCheck}
										className="size-3 text-primary"
									/>
								) : null}
							</div>
							<p className="text-xs text-muted-foreground">
								{option.description}
							</p>
						</div>
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

// Screen 2: deploy. Rivet Compute (default) uses the CLI + a live deploy check;
// other platforms reuse the existing connect setup.
function DeployScreen() {
	const provider = (useWatch({ name: "provider" }) as string) || "rivet";
	// The rivet deploy path is cloud-only; never take it without compute.
	if (provider === "rivet" && features.compute) {
		return <RivetDeploy />;
	}
	return <BackendSetup />;
}

function RivetDeploy() {
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: cloudToken } = useSuspenseQuery(
		dataProvider.createApiTokenQueryOptions({ name: "Onboarding" }),
	);
	const deployCommand = `npx @rivetkit/cli deploy --token ${cloudToken ?? "<RIVET_CLOUD_TOKEN>"}`;
	return (
		<div className="flex flex-col gap-6">
			<CopyAgentInstructionsButton provider="rivet" />
			<OrDivider label="or deploy manually" />
			<div>
				<p className="text-sm text-muted-foreground mb-3">
					Run this from your project root. The CLI builds and pushes
					your image and provisions Rivet Compute. The token is saved
					to{" "}
					<code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">
						~/.rivet/credentials
					</code>{" "}
					so later deploys can omit it.
				</p>
				<CommandBox command={deployCommand} />
			</div>
			<div className="border rounded-lg py-8">
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
						validatePool={(data) => !!data?.config?.image}
					/>
				</div>
			</div>
		</div>
	);
}

function SkipOnboardingHeaderLink() {
	if (features.platform) return null;
	return (
		<div className="absolute top-2 right-2 z-10">
			<Button
				asChild
				variant="ghost"
				size="sm"
				className="text-muted-foreground hover:text-foreground"
				endIcon={<Icon icon={faArrowRight} className="ms-1" />}
			>
				<Link to="." search={(s) => ({ ...s, skipOnboarding: true })}>
					Skip onboarding
				</Link>
			</Button>
		</div>
	);
}

function OnboardingProgress({ action }: { action?: ReactNode }) {
	const s = stepper.useStepper();
	const steps = s.all;
	const currentIndex = steps.findIndex((step) => step.id === s.current.id);
	const total = steps.length;
	const groupLabel = s.current.group === "local" ? "Local setup" : "Deploy";
	return (
		<div className="mb-6 flex flex-col gap-2">
			<div
				role="progressbar"
				aria-valuemin={1}
				aria-valuemax={total}
				aria-valuenow={currentIndex + 1}
				aria-valuetext={`Step ${currentIndex + 1} of ${total}, ${groupLabel}`}
				className="flex gap-1.5"
			>
				{steps.map((step, i) => (
					<div
						key={step.id}
						className={cn(
							"h-1 flex-1 rounded-full transition-colors",
							i <= currentIndex ? "bg-primary" : "bg-muted",
						)}
					/>
				))}
			</div>
			<div className="flex min-h-8 items-center justify-between gap-4">
				<div className="text-xs text-muted-foreground tabular-nums">
					Step {currentIndex + 1} of {total} · {groupLabel}
				</div>
				{action}
			</div>
		</div>
	);
}

function OrDivider({ label }: { label: string }) {
	return (
		<div className="flex items-center gap-3">
			<div className="h-px flex-1 bg-border" />
			<span className="text-xs text-muted-foreground">{label}</span>
			<div className="h-px flex-1 bg-border" />
		</div>
	);
}

function CommandBox({ command }: { command: string }) {
	return (
		<CodeFrame
			language="bash"
			code={() => command}
			hideFooter
			className="group my-0"
		>
			<CodePreview code={command} language="bash" className="text-left" />
		</CodeFrame>
	);
}

function RunLocallyStep() {
	const code = useAgentInstructionsCode();
	return (
		<div className="flex flex-col gap-6">
			<AgentPromptBanner
				code={code}
				title="Use your coding agent"
				description="Copy a prompt that scaffolds and runs your first Actor for you."
			/>
			<OrDivider label="or do it yourself" />
			<div className="w-full flex items-center justify-between gap-4 rounded-lg px-4 py-4 border border-border">
				<div className="min-w-0">
					<p className="font-medium mb-1">
						Follow the quickstart guide
					</p>
					<p className="text-sm text-muted-foreground">
						Build a Rivet Actor project by hand, step by step.
					</p>
				</div>
				<Button variant="outline" asChild className="shrink-0">
					<a
						href="https://rivet.dev/docs/actors/quickstart/"
						target="_blank"
						rel="noopener noreferrer"
					>
						Quickstart guide
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

function useAgentInstructionsCode({
	provider,
	runnerName = "default",
	endpoint,
}: {
	provider?: Provider;
	runnerName?: string;
	endpoint?: string;
} = {}) {
	const providerDetails = provider
		? deployOptions.find((p) => p.name === provider)
		: undefined;
	const providerStr =
		providerDetails?.displayName ?? provider ?? "your chosen provider";
	const publishableToken = useRivetDsn({ kind: "publishable", endpoint });
	const secretToken = useRivetDsn({ kind: "secret", endpoint });

	return getAgentInstructionsPrompt({
		providerStr,
		publishableToken,
		secretToken,
		runnerName,
	});
}

function useComputeInstructionsCode() {
	const agentInstructions = useAgentInstructionsCode({ provider: "rivet" });
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: cloudToken } = useSuspenseQuery(
		dataProvider.createApiTokenQueryOptions({ name: "Onboarding" }),
	);
	const publishableRawToken = usePublishableToken();
	const namespace = dataProvider.engineNamespace;

	const computeAddendum = getComputeAddendum({
		cloudToken,
		publishableToken: publishableRawToken ?? "<PUBLISHABLE_TOKEN>",
		namespace,
		project: dataProvider.project,
		organization: dataProvider.organization,
		cloudNamespace: dataProvider.cloudNamespace,
		apiUrl: cloudEnv().VITE_APP_API_URL,
		cloudApiUrl: cloudEnv().VITE_APP_CLOUD_API_URL,
	});

	return `${agentInstructions}\n\n---\n\n${computeAddendum}`;
}

function CopyAgentInstructionsButton({ provider }: { provider?: Provider }) {
	// The compute prompt reads cloud-namespace data; only available with compute.
	if (provider === "rivet" && features.compute) {
		return <ComputeCopyAgentInstructionsButton />;
	}
	return <GenericCopyAgentInstructionsButton provider={provider} />;
}

function AgentPromptBanner({
	code,
	containsSecret = false,
	title = "Use your coding agent",
	description = "Have your coding agent complete these steps to deploy to Rivet Compute.",
}: {
	code: string;
	containsSecret?: boolean;
	title?: string;
	description?: string;
}) {
	return (
		<button
			type="button"
			onClick={() => {
				navigator.clipboard.writeText(code);
				toast.success(
					containsSecret
						? "Copied to clipboard — includes a secret deploy token, paste only into your agent"
						: "Copied to clipboard",
				);
			}}
			className="relative w-full flex items-center justify-between gap-4 rounded-lg px-4 py-4 border border-primary group cursor-pointer text-left"
		>
			<Badge className="absolute -top-2.5 left-4 z-10 bg-background">
				Recommended
			</Badge>
			<div className="min-w-0">
				<p className="font-medium mb-1">{title}</p>
				<p className="text-sm text-muted-foreground">{description}</p>
				{containsSecret ? (
					<p className="mt-1 text-xs text-muted-foreground">
						Includes a secret deploy token. Paste only into your
						coding agent.
					</p>
				) : null}
			</div>
			<Button asChild variant="outline" className="shrink-0">
				<div>
					<Icon icon={faCopy} className="me-2 text-primary" />
					Copy prompt
				</div>
			</Button>
		</button>
	);
}

function ComputeCopyAgentInstructionsButton() {
	const code = useComputeInstructionsCode();
	return <AgentPromptBanner code={code} containsSecret />;
}

function GenericCopyAgentInstructionsButton({
	provider,
}: {
	provider?: Provider;
}) {
	const endpoint = useEndpoint();
	const runnerName = useWatch({ name: "runnerName" }) as string;
	const code = useAgentInstructionsCode({ provider, runnerName, endpoint });
	return (
		<AgentPromptBanner
			code={code}
			containsSecret
			description={`Have your coding agent complete these steps to deploy to ${platformTitle(provider)}.`}
		/>
	);
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
      - uses: rivet-dev/deploy-action@v1.1.2
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
			<OrDivider label="or set it up manually" />
			<div className="flex gap-3">
				<StepNumber n={1} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">Create a Dockerfile</p>
					<p className="text-sm text-muted-foreground">
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
					<p className="font-medium mb-2">Deploy to Rivet Compute</p>
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
								validatePool={(data) => !!data?.config?.image}
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
	const mode = useWatch({ name: "mode" }) as
		| "serverless"
		| "serverfull"
		| undefined;
	const { setValue } = useFormContext();

	if (provider === "rivet" && features.compute) {
		return <BackendSetupRivet />;
	}

	return (
		<div className="flex flex-col gap-6">
			<CopyAgentInstructionsButton provider={provider} />
			<OrDivider label="or set it up manually" />
			<div>
				<RunnerConfigToggleGroup
					mode={mode ?? "serverless"}
					onChange={(value) =>
						setValue("mode", value, {
							shouldDirty: true,
							shouldTouch: true,
							shouldValidate: true,
						})
					}
				/>
				<p className="text-xs text-muted-foreground text-center -mt-2">
					{(mode ?? "serverless") === "serverfull"
						? "Runner: a long-lived process you keep running that connects to Rivet."
						: "Serverless: Rivet invokes your deployment on demand and scales to zero."}
				</p>
			</div>
			{mode === "serverfull" ? (
				<BackendSetupServerfull provider={provider} />
			) : (
				<BackendSetupServerless provider={provider} />
			)}
		</div>
	);
}

function BackendSetupServerless({ provider }: { provider: Provider }) {
	const endpoint = useWatch({ name: "endpoint" });
	const isCustom = provider === "custom" || provider === "custom-platform";
	return (
		<>
			<div className="flex gap-3">
				<StepNumber n={1} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">
						Set environment variables
					</p>
					<p className="text-sm text-muted-foreground mb-3">
						Configure the following environment variables in your
						deployment.
					</p>
					<div className="space-y-2">
						<EnvVariables endpoint={endpoint ?? ""} />
					</div>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={2} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-4">
						Paste your deployment endpoint
					</p>
					<div className="space-y-2">
						<Configuration
							runnerName={false}
							datacenters
							headers={false}
							requestLifespan={false}
							drainGracePeriod={false}
						/>
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
								.otherwise(() => "https://your-deployment.com")}
						/>
						<ConfigurationAccordion
							datacenters={false}
							prefixFields={
								isCustom ? (
									<ConnectServerlessForm.CustomBranding />
								) : null
							}
						/>
						<ConnectServerlessForm.ConnectionCheck
							provider={provider}
						/>
					</div>
				</div>
			</div>
		</>
	);
}

function BackendSetupServerfull({ provider }: { provider: Provider }) {
	const isCustom = provider === "custom" || provider === "custom-platform";
	const endpoint = useServerfullEndpoint();
	const runnerName = useWatch({ name: "runnerName" });

	return (
		<>
			<div className="flex gap-3">
				<StepNumber n={1} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-4">Configure your runner</p>
					<div className="space-y-3">
						<ConnectServerfullForm.RunnerName />
						{isCustom ? (
							<ConnectServerlessForm.CustomBranding />
						) : null}
						<ConnectServerfullForm.Datacenter />
					</div>
				</div>
			</div>
			<div className="flex gap-3">
				<StepNumber n={2} />
				<div className="flex-1 min-w-0">
					<p className="font-medium mb-2">
						Set environment variables
					</p>
					<p className="text-sm text-muted-foreground mb-3">
						Set the following environment variables on the machine
						running your runner.
					</p>
					<div className="space-y-2">
						<EnvVariables
							endpoint={endpoint}
							runnerName={runnerName}
							showPublicEndpoint={false}
						/>
					</div>
				</div>
			</div>
		</>
	);
}

function useServerfullEndpoint() {
	const datacenter = useWatch({ name: "datacenter" });
	const dataProvider = useEngineCompatDataProvider();
	const { data } = useQuery(
		dataProvider.datacenterQueryOptions(datacenter || "auto"),
	);
	return data?.url || engineEnv().VITE_APP_API_URL;
}
