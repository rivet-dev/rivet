import {
	faArrowRight,
	faBookOpen,
	faChevronLeft,
	faClaude,
	faCopy,
	faCursor,
	faVscode,
	Icon,
} from "@rivet-gg/icons";
import {
	deployOptions,
	type Provider,
	templates,
} from "@rivetkit/example-registry";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import {
	Link,
	useNavigate,
	useRouter,
	useSearch,
} from "@tanstack/react-router";
import { motion } from "framer-motion";
import { type ReactNode, Suspense, useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { match, P } from "ts-pattern";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	Badge,
	ButtonCard,
	Code,
	CodeFrame,
	type CodeFrameLikeElement,
	CodeGroup,
	CodeGroupSyncProvider,
	CodePreview,
	CopyTrigger,
	ExternalLinkCard,
	FormField,
	H1,
	Label,
	Ping,
	Skeleton,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/components";
import {
	useDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";
import { PathSelection } from "@/components/onboarding/path-selection";
import { TemplatesList } from "@/components/templates-list";
import { defineStepper } from "@/components/ui/stepper";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
import { TEST_IDS } from "@/utils/test-ids";
import { Button } from "../components/ui/button";
import { useEndpoint } from "./dialogs/connect-manual-serverfull-frame";
import {
	buildServerlessConfig,
	ConfigurationAccordion,
} from "./dialogs/connect-manual-serverless-frame";
import { DeployToVercelCard } from "./dialogs/connect-quick-vercel-frame";
import { EnvVariables } from "./env-variables";
import { StepperForm } from "./forms/stepper-form";
import { Content } from "./layout";

const stepper = defineStepper(
	{
		id: "provider",
		title: "Choose Provider",
		schema: z.object({ provider: z.string() }),
		showNext: false,
	},
	{
		id: "backend",
		title: "Connect your Backend",
		assist: true,
		schema: z.object({
			...ConnectServerlessForm.configurationSchema.shape,
			...ConnectServerlessForm.deploymentSchema.shape,
		}),
	},
	{
		id: "frontend",
		title: "Create your first Actor",
		assist: true,
		schema: z.object({}),
		showNext: false,
		showPrevious: false,
	},
);

type Flow = "template" | "agent" | "manual";

export function GettingStarted({
	displayOnboarding,
	displayBackendOnboarding,
	flow,
	template,
	provider,
	noTemplate,
}: {
	flow?: Flow;
	provider?: Provider;
	displayOnboarding?: boolean;
	displayBackendOnboarding?: boolean;
	template?: string;
	noTemplate?: boolean;
}) {
	const dataProvider = useEngineCompatDataProvider();
	const { data: datacenters } = useSuspenseInfiniteQuery(
		dataProvider.datacentersQueryOptions(),
	);

	const { mutateAsync } = useMutation({
		...dataProvider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.runnerConfigsQueryOptions(),
			);
		},
	});

	const navigate = useNavigate();

	if (!flow) {
		return <PathSelection />;
	}

	if (
		flow === "template" &&
		displayOnboarding &&
		displayBackendOnboarding &&
		!template
	) {
		return (
			<Content className="flex flex-col">
				<TemplatesList
					back={
						flow === "template" ? (
							<Link
								// @ts-expect-error
								search={({ flow: _, ...old }: any) => ({
									...old,
								})}
							>
								Back
							</Link>
						) : undefined
					}
					getTemplateLink={(template) => ({
						to: ".",
						search: { template, flow },
					})}
					startFromScratchLink={{
						to: ".",
						search: { noTemplate: true, flow: "manual" },
					}}
				/>
			</Content>
		);
	}

	return (
		<Content className="flex flex-col items-center justify-safe-center">
			<motion.div
				className="max-w-[32rem] mx-auto w-full mt-14"
				initial={{ opacity: 0, y: -20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3, delay: 0.5 }}
			>
				{displayOnboarding && displayBackendOnboarding ? (
					<Button
						className=" text-muted-foreground px-0.5 py-1 h-auto -mx-0.5"
						startIcon={<Icon icon={faChevronLeft} />}
						size="xs"
						variant="link"
						asChild
					>
						<Link
							to="."
							search={{
								template: undefined,
								noTemplate: undefined,
								flow: flow === "template" ? flow : undefined,
							}}
						>
							{flow === "template" ? "Back to Templates" : "Back"}
						</Link>
					</Button>
				) : null}
			</motion.div>
			<motion.div
				className="relative mb-8"
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3 }}
			>
				<H1 className="mt-8 text-center">Get started with Rivet</H1>
				<p className="text-center text-muted-foreground max-w-2xl mx-auto mt-2">
					Follow these steps to set up your project quickly and
					easily.
				</p>
				<div className="mt-8 w-[32rem]">
					<CodeGroupSyncProvider>
						<StepperForm
							{...stepper}
							formId="onboarding"
							initialStep={
								provider
									? "backend"
									: displayBackendOnboarding
										? undefined
										: "frontend"
							}
							defaultValues={{
								provider,
								runnerName: "default",
								slotsPerRunner: 1,
								maxRunners: 10000,
								minRunners: 1,
								runnerMargin: 0,
								headers: [],
								success: false,
								requestLifespan: 900,
								datacenters: Object.fromEntries(
									datacenters.map((dc) => [dc.name, true]),
								),
							}}
							content={{
								provider: () => (
									<ProviderSetup template={template} />
								),
								backend: () => (
									<Suspense
										fallback={
											<div className="space-y-6">
												<Skeleton className="w-full h-[180px]" />
												<Skeleton className="w-full h-[250px]" />
												<Skeleton className="w-full h-[200px]" />
											</div>
										}
									>
										<BackendSetup
											template={template}
											flow={flow}
										/>
									</Suspense>
								),
								frontend: () => <FrontendSetup />,
							}}
							onSubmit={() => {}}
							onPartialSubmit={async ({ stepper, values }) => {
								if (stepper.current.id === "backend") {
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

function StepperFooter() {
	const s = stepper.useStepper();
	return (
		<div className="flex items-center justify-center gap-4">
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

function ProviderSetup({ template }: { template?: string }) {
	const navigate = useNavigate();
	const showAll = useSearch({ strict: false, select: (s) => s?.showAll });

	const templateDetails = templates.find((t) => t.name === template);

	const { control } = useFormContext();
	const s = stepper.useStepper();

	return (
		<div data-testid={TEST_IDS.Onboarding.IntegrationProviderSelection}>
			<FormField
				control={control}
				name="provider"
				render={({ field }) => (
					<>
						{deployOptions
							.filter((option) => !option.specializedPlatform)
							.filter((option) => {
								if (!templateDetails) return true;
								if (option.name === "vercel") {
									return !!templateDetails.providers?.vercel;
								}
								return true;
							})
							.slice(0, showAll ? deployOptions.length : 3)
							.map((option) => (
								<ButtonCard
									key={option.name}
									icon={option.icon}
									title={option.displayName}
									description={option.description}
									className="text-left mb-4 w-full min-w-0"
									data-testid={TEST_IDS.Onboarding.IntegrationProviderOption(
										option.name,
									)}
									onClick={() => {
										field.onChange(option.name);
										s.next();
									}}
								/>
							))}
					</>
				)}
			/>

			<div className="text-center mt-2 mb-4">
				{showAll ? (
					<Button
						variant="ghost"
						size="sm"
						type="button"
						onClick={() => {
							return navigate({
								to: ".",
								search: (s) => ({
									...s,
									showAll: undefined,
								}),
								replace: true,
							});
						}}
					>
						Show fewer options
					</Button>
				) : (
					<Button
						variant="ghost"
						size="sm"
						type="button"
						onClick={() => {
							return navigate({
								to: ".",
								search: (s) => ({ ...s, showAll: true }),
								replace: true,
							});
						}}
					>
						Show {deployOptions.length - 3} more options
					</Button>
				)}
			</div>
		</div>
	);
}

function Connector() {
	return (
		<div className="-my-10 flex justify-center">
			<div className="h-6 border-l w-px" />
		</div>
	);
}

function BackendSetup({ template, flow }: { template?: string; flow?: Flow }) {
	const provider = useWatch({ name: "provider" });
	const templateDetails = templates.find((t) => t.name === template);
	const providerDetails = deployOptions.find((p) => p.name === provider);
	return (
		<div className="flex flex-col gap-10">
			{flow !== "agent" ? (
				<>
					<SkillsSetup />
					<Connector />
				</>
			) : null}
			{match({ template, provider, flow })
				// .with(
				// 	{ provider: P.any, template: undefined, flow: "agent" },
				// 	() => <McpSetup />,
				// )
				.with({ provider: "vercel", template: P.string }, () => (
					<DeployToVercelCard
						template={
							templateDetails?.providers?.vercel?.name ||
							template ||
							"chat-room"
						}
					/>
				))
				// .with("railway", () => (
				// 	<RailwayQuickSetupInfo template={template} />
				// ))
				.with(
					{ provider: P.string, template: P.string },
					({ template }) => <TemplateSetup template={template} />,
				)
				.otherwise(() => (
					<ExternalLinkCard
						icon={faBookOpen}
						title={"Follow the Quickstart Guide"}
						href={"https://www.rivet.dev/docs/actors/quickstart/"}
					/>
				))}
			<Connector />

			{/* {((provider === "vercel" && !template) || provider !== "vercel") &&
			flow !== "agent" ? (
				<>
					<ExternalLinkCard
						icon={providerDetails?.icon}
						title={`View ${providerDetails?.displayName} Guide`}
						href={`https://www.rivet.dev${providerDetails?.href || "/docs/getting-started"}`}
					/>
					<Connector />
				</>
			) : null} */}
			{/* {flow === "agent" ? (
				<>
					<div className="border rounded-md p-4">
						<p className="mb-4">
							Ask your Coding Agent to set up Rivet for you:
						</p>
						<p className="pl-4 border-l-2 border-primary-500 text-muted-foreground mb-4">
							Integrate Rivet into{" "}
							{providerDetails?.displayName || provider}, deploy,
							then tell me the url to connect to Rivet.
						</p>
						<div className="flex items-end">
							<CopyTrigger
								value={`Integrate Rivet into ${providerDetails?.displayName || provider}, deploy, then tell me the url to connect to Rivet.`}
								className="ml-auto"
							>
								<Button
									endIcon={<Icon icon={faCopy} />}
									variant="outline"
								>
									Copy instructions
								</Button>
							</CopyTrigger>
						</div>
					</div>
					<Connector />
				</>
			) : null} */}
			<div className="space-y-2 border rounded-md p-4">
				{/* {flow === "agent" ? (
					<p className="mb-4">
						Tell your Coding Agent to use following environment
						variables in your deployment.
					</p>
				) : ( */}
				<p className="mb-4">
					Set these environment variables in your deployment.
				</p>
				{/* )} */}
				<Label>Environment Variables</Label>
				<EnvVariables
					endpoint={useEndpoint()}
					runnerName={useWatch({ name: "runnerName" }) as string}
				/>
			</div>
			<Connector />
			<div className="space-y-2 border rounded-md p-4">
				{/* {flow === "agent" ? (
					<p className="mb-4">
						Paste the endpoint that your Coding Agent provides after
						deployment.
					</p>
				) : ( */}
				<p className="mb-4">
					Deploy your code and paste your deployment's endpoint.
				</p>
				{/* )} */}
				<div>
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
					<ConfigurationAccordion />
				</div>
				<ConnectServerlessForm.ConnectionCheck provider={provider} />
			</div>
		</div>
	);
}

const code = ({
	cmd = "npx",
	lib = "giget@latest",
	template = "chat-room",
}: {
	cmd?: string;
	lib?: string;
	template?: string;
}) =>
	`${cmd} ${lib} gh:rivet-dev/rivet/examples/${template} ${template} --install`;

const manualCode = ({
	template = "chat-room",
}: {
	template?: string;
}) => `git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/${template}
pnpm install
pnpm run dev`;

function TemplateSetup({ template = "chat-room" }: { template?: string }) {
	return (
		<PackageManagerCode
			npx={code({ cmd: "npx", template })}
			yarn={code({ cmd: "yarn dlx", template })}
			bun={code({ cmd: "bunx", template })}
			deno={code({
				cmd: "deno run -A",
				lib: "npm:giget@latest",
				template,
			})}
			pnpm={code({ cmd: "pnpx", template })}
			git={manualCode({ template })}
			header={<p className="pt-2 pb-4 px-4 border-b">Clone example</p>}
		/>
	);
}

const skillsPath = "rivet-dev/skills";
function SkillsSetup() {
	return (
		<PackageManagerCode
			npx={`npx skills add ${skillsPath}`}
			yarn={`yarn dlx skills add ${skillsPath}`}
			bun={`bunx skills add ${skillsPath}`}
			deno={`deno run -A npm:skills add ${skillsPath}`}
			pnpm={`pnpx skills add ${skillsPath}`}
			git={`git clone https://github.com/${skillsPath}.git .skills`}
			header={
				<p className="pt-2 pb-4 px-4 border-b flex items-center gap-2">
					Install RivetKit skills <Badge>Recommended</Badge>
				</p>
			}
		/>
	);
}

const MCP_URL = "https://mcp.rivet.dev/mcp";
const MCP_NAME = "rivet";

const claudeCode = `claude mcp add --transport http ${MCP_NAME} ${MCP_URL}`;
const cursorCode = JSON.stringify(
	{
		mcpServers: {
			[MCP_NAME]: {
				url: MCP_URL,
			},
		},
	},
	null,
	2,
);

const installCursorUrl = `cursor://anysphere.cursor-deeplink/mcp/install?name=${MCP_NAME}&config=${encodeURIComponent(JSON.stringify({ url: MCP_URL }))}`;

const installVsCodeUrl = `https://vscode.dev/redirect/mcp/install?name=${encodeURIComponent(MCP_NAME)}&config=${encodeURIComponent(JSON.stringify({ url: MCP_URL }))}`;

const vscodeCode = `code --add-mcp '${JSON.stringify({ name: MCP_NAME, url: MCP_URL })}'`;

// instruct user to set up MCP (Model Context Protocol) if agent flow,
function McpSetup() {
	return (
		<div className="border rounded-md pt-2 space-y-4">
			<Tabs defaultValue="claude">
				<TabsList>
					<TabsTrigger value="claude">
						<Icon icon={faClaude} className="mr-1" /> Claude Code
					</TabsTrigger>
					<TabsTrigger value="cursor">
						<Icon icon={faCursor} className="mr-1" /> Cursor
					</TabsTrigger>
					<TabsTrigger value="vscode">
						<Icon icon={faVscode} className="mr-1" /> VSCode
					</TabsTrigger>
				</TabsList>
				<TabsContent value="claude" className="px-4 pb-4">
					<p>Use the Claude Code CLI to add Rivet MCP:</p>
					<CodeFrame
						language="bash"
						code={() => claudeCode}
						className="m-0 mt-4"
					>
						<CodePreview
							language="bash"
							className="text-left"
							code={claudeCode}
						/>
					</CodeFrame>
				</TabsContent>
				<TabsContent value="cursor" className="px-4 pb-4">
					<Button className="mb-2" variant="outline" asChild>
						<a href={installCursorUrl}>Install in Cursor</a>
					</Button>
					<p>
						Or, go to <Code>Cursor Settings</Code>{" "}
						<Icon icon={faArrowRight} /> <Code>MCP</Code>{" "}
						<Icon icon={faArrowRight} /> <Code>New MCP Server</Code>
						, and use configuration below:
					</p>
					<CodeFrame
						language="json"
						code={() => cursorCode}
						className="m-0 mt-4"
					>
						<CodePreview
							language="json"
							className="text-left"
							code={cursorCode}
						/>
					</CodeFrame>
				</TabsContent>
				<TabsContent value="vscode" className="px-4 pb-4">
					<Button className="mb-2" variant="outline" asChild>
						<a href={installVsCodeUrl}>Install in VSCode</a>
					</Button>
					<p>Or, you can install manually using VSCode CLI:</p>
					<CodeFrame
						language="bash"
						code={() => vscodeCode}
						className="m-0 mt-4"
					>
						<CodePreview
							language="bash"
							className="text-left"
							code={vscodeCode}
						/>
					</CodeFrame>
				</TabsContent>
			</Tabs>
		</div>
	);
}

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

	return (
		<div className="space-y-2">
			<div className="border rounded-md py-10">
				<div className="flex gap-2 justify-center items-center">
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
