import { faChevronLeft, faChevronRight, Icon } from "@rivet-gg/icons";
import { deployOptions, templates } from "@rivetkit/example-registry";
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
import { Suspense, useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { match, P } from "ts-pattern";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	ButtonCard,
	CodeFrame,
	CodeGroup,
	CodePreview,
	ExternalLinkCard,
	FormField,
	H1,
	Label,
	Ping,
	Skeleton,
} from "@/components";
import {
	useDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";
import { TemplatesList } from "@/components/templates-list";
import { defineStepper } from "@/components/ui/stepper";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
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

export function GettingStarted({
	displayOnboarding,
	displayBackendOnboarding,
	template,
	noTemplate,
}: {
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

	if (
		!template &&
		!noTemplate &&
		displayOnboarding &&
		displayBackendOnboarding
	) {
		return (
			<Content className="flex flex-col">
				<TemplatesList
					showBackHome={false}
					getTemplateLink={(template) => ({
						to: ".",
						search: { template },
					})}
					startFromScratchLink={{
						to: ".",
						search: { noTemplate: true },
					}}
				/>
			</Content>
		);
	}

	return (
		<Content className="flex flex-col items-center justify-safe-center">
			<motion.div
				className="max-w-[32rem] mx-auto w-full"
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
							}}
						>
							Back to Templates
						</Link>
					</Button>
				) : null}
			</motion.div>
			<motion.div
				className="relative"
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3 }}
			>
				<H1 className="mt-8 text-center">Get started with Rivet</H1>
				<p className="text-center text-muted-foreground max-w-2xl mx-auto">
					Follow these steps to set up your project quickly and
					easily.
				</p>
				<div className="mt-8 w-[32rem]">
					<StepperForm
						{...stepper}
						formId="onboarding"
						initialStep={
							displayBackendOnboarding ? undefined : "frontend"
						}
						defaultValues={{
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
							provider: ProviderSetup,
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
									<BackendSetup template={template} />
								</Suspense>
							),
							frontend: () => <FrontendSetup />,
						}}
						onSubmit={() => {}}
						onPartialSubmit={async ({ stepper, values }) => {
							if (stepper.current.id === "backend") {
								const status =
									await queryClient.ensureQueryData(
										dataProvider.runnerHealthCheckQueryOptions(
											{
												runnerUrl:
													ConnectServerlessForm.endpointSchema.parse(
														values.endpoint,
													),
												headers: Object.fromEntries(
													values.headers,
												),
											},
										),
									);

								const config = await buildServerlessConfig(
									dataProvider,
									{
										...values,
										endpoint: status.url || values.endpoint,
									},
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
				</div>
			</motion.div>
		</Content>
	);
}

function StepperFooter() {
	const s = stepper.useStepper();
	const router = useRouter();
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

			<Button
				variant="link"
				className="text-muted-foreground"
				size="xs"
				onClick={() => {
					router.invalidate();
					return router.navigate({
						to: ".",
						search: {
							skipOnboarding: true,
						},
					});
				}}
				endIcon={<Icon icon={faChevronRight} />}
			>
				Skip Setup
			</Button>
		</div>
	);
}

function ProviderSetup() {
	const navigate = useNavigate();
	const showAll = useSearch({ strict: false, select: (s) => s?.showAll });

	const { control } = useFormContext();
	const s = stepper.useStepper();

	return (
		<div>
			<FormField
				control={control}
				name="provider"
				render={({ field }) => (
					<>
						{deployOptions
							.filter((option) => !option.specializedPlatform)
							.slice(0, showAll ? deployOptions.length : 3)
							.map((option) => (
								<ButtonCard
									key={option.name}
									icon={option.icon}
									title={option.displayName}
									description={option.description}
									className="text-left mb-4 w-full min-w-0"
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
		<div className="-my-6 flex justify-center">
			<div className="h-4 border-l w-px" />
		</div>
	);
}

function BackendSetup({ template }: { template?: string }) {
	const provider = useWatch({ name: "provider" });
	const templateDetails = templates.find((t) => t.name === template);
	const options = deployOptions.find((p) => p.name === provider);
	return (
		<div className="flex flex-col gap-6">
			{match({ template, provider })
				.with({ provider: "vercel", template: P.string }, () => (
					<DeployToVercelCard template={templateDetails?.providers.vercel.name || template || "chat-room"} />
				))
				// .with("railway", () => (
				// 	<RailwayQuickSetupInfo template={template} />
				// ))
				.with(
					{ provider: P.string, template: P.string },
					({ template }) => <TemplateSetup template={template} />,
				)
				.otherwise(() => (
					<div className="space-y-2 border rounded-md p-4">
						<p>
							Follow the{" "}
							<a
								href="https://www.rivet.dev/docs/actors/quickstart/"
								className="underline"
								target="_blank"
								rel="noopener noreferrer"
							>
								Quickstart guide
							</a>{" "}
							to set up your project.
						</p>
					</div>
				))}
			<Connector />

			{(provider === "vercel" && !template) || provider !== "vercel" ? (
				<>
					<ExternalLinkCard
						icon={options?.icon}
						title={`View ${options?.displayName} Guide`}
						href={`https://www.rivet.dev${options?.href || "/docs/getting-started"}`}
					/>
					<Connector />
				</>
			) : null}
			<div className="space-y-2 border rounded-md p-4">
				<p className="mb-4">
					Set these environment variables in your deployment.
				</p>
				<Label>Environment Variables</Label>
				<EnvVariables
					endpoint={useEndpoint()}
					runnerName={useWatch({ name: "runnerName" }) as string}
				/>
			</div>
			<Connector />
			<div className="space-y-2 border rounded-md p-4">
				<p className="mb-4">
					Deploy your code and paste your deployment's endpoint.
				</p>
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
	const npx = (
		<CodeFrame
			language="bash"
			title="npm"
			footer="Clone example"
			code={() => code({ cmd: "npx", lib: "giget@latest", template })}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={code({ cmd: "npx", lib: "giget@latest", template })}
			/>
		</CodeFrame>
	);

	const yarn = (
		<CodeFrame
			language="bash"
			title="yarn"
			footer="Clone example"
			code={() =>
				code({ cmd: "yarn dlx", lib: "giget@latest", template })
			}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={code({ cmd: "yarn dlx", lib: "giget@latest", template })}
			/>
		</CodeFrame>
	);

	const pnpm = (
		<CodeFrame
			language="bash"
			title="pnpm"
			footer="Clone example"
			code={() => code({ cmd: "pnpx", lib: "giget@latest", template })}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={code({ cmd: "pnpx", lib: "giget@latest", template })}
			/>
		</CodeFrame>
	);

	const bun = (
		<CodeFrame
			language="bash"
			title="bun"
			footer="Clone example"
			code={() => code({ cmd: "bunx", lib: "giget@latest", template })}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={code({ cmd: "bunx", lib: "giget@latest", template })}
			/>
		</CodeFrame>
	);

	const deno = (
		<CodeFrame
			language="bash"
			title="deno"
			footer="Clone example"
			code={() =>
				code({ cmd: "deno run -A", lib: "npm:giget@latest", template })
			}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={code({
					cmd: "deno run -A",
					lib: "npm:giget@latest",
					template,
				})}
			/>
		</CodeFrame>
	);

	const git = (
		<CodeFrame
			language="bash"
			title="git"
			footer="Clone example"
			code={() => manualCode({ template })}
			className="m-0"
		>
			<CodePreview
				language="bash"
				className="text-left"
				code={manualCode({ template })}
			/>
		</CodeFrame>
	);

	return (
		<CodeGroup className="my-0">
			{[npx, yarn, pnpm, bun, deno, git]}
		</CodeGroup>
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
