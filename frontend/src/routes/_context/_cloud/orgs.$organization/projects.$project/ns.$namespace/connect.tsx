/** biome-ignore-all lint/correctness/useHookAtTopLevel: guarded by build-time constants */
import {
	faAws,
	faChevronRight,
	faGoogleCloud,
	faHetznerH,
	faNextjs,
	faNodeJs,
	faPlus,
	faQuestionCircle,
	faRailway,
	faReact,
	faServer,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useSuspenseInfiniteQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import {
	createFileRoute,
	Link,
	notFound,
	Link as RouterLink,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { hasProvider } from "@/app/data-providers/engine-data-provider";
import { HelpDropdown } from "@/app/help-dropdown";
import { RunnerConfigsTable } from "@/app/runner-config-table";
import { RunnersTable } from "@/app/runners-table";
import {
	Button,
	CodeFrame,
	CodeGroup,
	CodePreview,
	DocsSheet,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	getConfig,
	H1,
	H2,
	H3,
	Skeleton,
} from "@/components";
import {
	useCloudNamespaceDataProvider,
	useEngineCompatDataProvider,
	useEngineNamespaceDataProvider,
} from "@/components/actors";
import { cloudEnv, engineEnv } from "@/lib/env";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/connect",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
	pendingComponent: DataLoadingPlaceholder,
});

export function RouteComponent() {
	const { data: runnerConfigsCount, isLoading } = useSuspenseInfiniteQuery({
		...useEngineCompatDataProvider().runnerConfigsQueryOptions(),
		select: (data) => Object.values(data.pages[0].runnerConfigs).length,
		refetchInterval: 5000,
	});

	const _navigate = Route.useNavigate();

	const hasConfigs =
		runnerConfigsCount !== undefined && runnerConfigsCount > 0;

	if (isLoading) {
		return (
			<div className="bg-card h-full border my-2 mr-2 rounded-lg overflow-auto">
				<div className=" max-w-5xl mx-auto">
					<div className="mt-2 flex justify-between items-center px-6 py-4 ">
						<H1>Connect</H1>
						<div className="flex gap-4">
							<Link to="." search={{ modal: "tokens" }}>
								<Button variant="outline">Tokens</Button>
							</Link>
							<HelpDropdown>
								<Button
									variant="outline"
									startIcon={<Icon icon={faQuestionCircle} />}
								>
									Need help?
								</Button>
							</HelpDropdown>
						</div>
					</div>
					<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
						Connect your RivetKit application to Rivet Cloud. Use
						your cloud of choice to run Rivet Actors.
					</p>
				</div>

				<hr className="mb-4" />
				<div className="p-4 px-6 max-w-5xl mx-auto ">
					<Skeleton className="h-8 w-48 mb-4" />
					<div className="grid grid-cols-3 gap-2 my-4">
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
					</div>
				</div>
			</div>
		);
	}

	if (!hasConfigs) {
		return (
			<div className="h-full border my-2 mr-2 px-4 py-4 rounded-lg flex flex-col items-center justify-safe-center overflow-auto @container">
				<div className="grid grid-cols-1 @7xl:grid-cols-2 gap-8 justify-safe-center">
					<div className="max-w-3xl border rounded-lg w-full bg-card">
						<div className="mt-2 flex justify-between items-center px-6 py-4 sticky top-0">
							<H2>Create New Project</H2>
						</div>
						<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
							Start a new RivetKit project with Rivet Cloud. Use
							one of our templates to get started quickly.
						</p>

						<hr className="mb-4" />
						<div className="p-4 px-6">
							<H3>1-Click Deploy From Template</H3>
							<div className="grid grid-cols-2 @4xl:grid-cols-3 gap-2 my-4">
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faVercel} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-q-vercel" }}
									>
										Vercel
									</RouterLink>
								</Button>
								<OneClickDeployRailwayButton />
							</div>
						</div>
						<div className="px-6 mb-8">
							<H3>Quickstart Guides</H3>
							<div className="grid grid-cols-2 @4xl:grid-cols-3 gap-2 my-4">
								<DocsSheet
									path="/docs/actors/quickstart/backend"
									title={"JavaScript Quickstart"}
								>
									<Button
										size="lg"
										variant="outline"
										className="min-w-48 h-auto min-h-28 text-xl"
										startIcon={<Icon icon={faNodeJs} />}
									>
										Node.js & Bun
									</Button>
								</DocsSheet>
								<DocsSheet
									path="/docs/actors/quickstart/react"
									title={"React Quickstart"}
								>
									<Button
										size="lg"
										variant="outline"
										className="min-w-48 h-auto min-h-28 text-xl"
										startIcon={<Icon icon={faReact} />}
									>
										React
									</Button>
								</DocsSheet>
								<DocsSheet
									path="/docs/actors/quickstart/next-js"
									title={"Next.js Quickstart"}
								>
									<Button
										size="lg"
										variant="outline"
										className="min-w-48 h-auto min-h-28 text-xl"
										startIcon={<Icon icon={faNextjs} />}
									>
										Next.js
									</Button>
								</DocsSheet>
							</div>
						</div>
					</div>
					<div className="max-w-3xl w-full border rounded-lg bg-card">
						<div className="mt-2 flex justify-between items-center px-6 py-4">
							<H2>Connect Existing Project</H2>
							<div className="flex gap-4">
								<Link to="." search={{ modal: "tokens" }}>
									<Button variant="outline">Tokens</Button>
								</Link>
								<HelpDropdown>
									<Button
										variant="outline"
										startIcon={
											<Icon icon={faQuestionCircle} />
										}
									>
										Need help?
									</Button>
								</HelpDropdown>
							</div>
						</div>
						<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
							Connect your RivetKit application to Rivet Cloud.
							Use your cloud of choice to run Rivet Actors.
						</p>

						<hr className="mb-4" />
						<div className="p-4 px-6 max-w-5xl">
							<H3>Add Provider</H3>
							<div className="grid grid-cols-2 @4xl:grid-cols-3 gap-2 my-4">
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faVercel} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-vercel" }}
									>
										Vercel
									</RouterLink>
								</Button>
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faRailway} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-railway" }}
									>
										Railway
									</RouterLink>
								</Button>
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faAws} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-aws" }}
									>
										AWS ECS
									</RouterLink>
								</Button>

								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faGoogleCloud} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-gcp" }}
									>
										Google Cloud Run
									</RouterLink>
								</Button>
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faHetznerH} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-hetzner" }}
									>
										Hetzner
									</RouterLink>
								</Button>
								<Button
									size="lg"
									variant="outline"
									className="min-w-48 h-auto min-h-28 text-xl"
									startIcon={<Icon icon={faServer} />}
									asChild
								>
									<RouterLink
										to="."
										search={{ modal: "connect-custom" }}
									>
										Custom
									</RouterLink>
								</Button>
							</div>
						</div>
					</div>
				</div>
			</div>
		);
	}

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg overflow-auto @container">
			<div className=" ">
				<div className="mb-4 pt-2 max-w-5xl mx-auto">
					<div className="flex justify-between items-center px-10 py-4 ">
						<H1>Connect</H1>
						<div className="flex gap-4">
							<Link to="." search={{ modal: "tokens" }}>
								<Button variant="outline">Tokens</Button>
							</Link>
							<HelpDropdown>
								<Button
									variant="outline"
									startIcon={<Icon icon={faQuestionCircle} />}
								>
									Need help?
								</Button>
							</HelpDropdown>
						</div>
					</div>
					<p className="max-w-5xl mb-6 px-10 text-muted-foreground">
						Connect your RivetKit application to Rivet Cloud. Use
						your cloud of choice to run Rivet Actors.
					</p>
				</div>

				<hr className="mb-6" />

				<div className="px-4">
					<ConnectYourFrontend />
					<Providers />
					<Runners />
				</div>
			</div>
		</div>
	);
}

function Providers() {
	const {
		isLoading,
		isError,
		data: configs,
		hasNextPage,
		fetchNextPage,
	} = useInfiniteQuery({
		...useEngineCompatDataProvider().runnerConfigsQueryOptions(),
		refetchInterval: 5000,
	});

	return (
		<div className="p-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2">
				<H3>Providers</H3>

				<ProviderDropdown>
					<Button
						className="min-w-32"
						variant="outline"
						startIcon={<Icon icon={faPlus} />}
					>
						Add Provider
					</Button>
				</ProviderDropdown>
			</div>
			<p className="mb-6 text-muted-foreground">
				Clouds connected to Rivet for running Rivet Actors.
			</p>

			<div className="max-w-5xl mx-auto">
				<div className="border rounded-md">
					<RunnerConfigsTable
						isLoading={isLoading}
						isError={isError}
						configs={configs || []}
						fetchNextPage={fetchNextPage}
						hasNextPage={hasNextPage}
					/>
				</div>
			</div>
		</div>
	);
}

function Runners() {
	const {
		isLoading,
		isError,
		data: runners,
		hasNextPage,
		fetchNextPage,
	} = useInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		refetchInterval: 5000,
	});

	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Runners</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Processes connected to Rivet Cloud and ready to start running
				Rivet Actors.
			</p>
			<div className="max-w-5xl mx-auto">
				<div className="border rounded-md">
					<RunnersTable
						isLoading={isLoading}
						isError={isError}
						runners={runners || []}
						fetchNextPage={fetchNextPage}
						hasNextPage={hasNextPage}
					/>
				</div>
			</div>
		</div>
	);
}

function usePublishableToken() {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			return useSuspenseQuery(
				useCloudNamespaceDataProvider().publishableTokenQueryOptions(),
			).data;
		})
		.with("engine", () => {
			return useSuspenseQuery(
				useEngineNamespaceDataProvider().engineAdminTokenQueryOptions(),
			).data;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
}

const useEndpoint = () => {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			return cloudEnv().VITE_APP_API_URL;
		})
		.with("engine", () => {
			return getConfig().apiUrl;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
};

function ConnectYourFrontend() {
	const token = usePublishableToken();
	const endpoint = useEndpoint();
	const dataProvider = useEngineCompatDataProvider();
	const namespace = dataProvider.engineNamespace;

	const { data: configs } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		refetchInterval: 5000,
	});

	// Check if Vercel is connected
	const hasVercel = hasProvider(configs, ["vercel", "next-js"]);

	const nextJsTab = (
		<CodeFrame
			language="typescript"
			title="Next.js"
			icon={faNextjs}
			footer={
				<DocsSheet
					path={"/docs/actors/quickstart/next-js"}
					title={"Next.js Quickstart"}
				>
					<span className="cursor-pointer hover:underline">
						See Next.js Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</DocsSheet>
			}
		>
			<CodePreview
				code={nextJsCode({ token, endpoint, namespace })}
				language="typescript"
			/>
		</CodeFrame>
	);

	const reactTab = (
		<CodeFrame
			language="typescript"
			title="React"
			icon={faReact}
			footer={
				<DocsSheet
					path={"/docs/actors/quickstart/react"}
					title={"React Quickstart"}
				>
					<span className="cursor-pointer hover:underline">
						See React Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</DocsSheet>
			}
		>
			<CodePreview
				code={reactCode({ token, endpoint, namespace })}
				language="typescript"
			/>
		</CodeFrame>
	);

	const javascriptTab = (
		<CodeFrame
			language="typescript"
			title="JavaScript"
			icon={faNodeJs}
			footer={
				<DocsSheet
					path={"/docs/actors/quickstart/backend"}
					title={"JavaScript Quickstart"}
				>
					<span className="cursor-pointer hover:underline">
						See JavaScript Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</DocsSheet>
			}
		>
			<CodePreview
				code={javascriptCode({
					token,
					endpoint,
					namespace,
				})}
				language="typescript"
			/>
		</CodeFrame>
	);

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Connect Your Frontend</H3>
			</div>
			<p className="mb-8">
				This token is safe to publish on your frontend.
			</p>
			<div>
				<CodeGroup>
					{hasVercel
						? [nextJsTab, reactTab, javascriptTab]
						: [javascriptTab, reactTab, nextJsTab]}
				</CodeGroup>
			</div>
		</div>
	);
}

const javascriptCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

const client = createClient<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	token: "${token}",
});`;

const reactCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	token: "${token}",
});`;

const nextJsCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `"use client";
import { createRivetKit } from "@rivetkit/next-js/client";
import type { registry } from "@/rivet/registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: "${engineEnv().VITE_APP_API_URL}",
	namespace: "${namespace}",
	token: "${token}",
});
`;

function ProviderDropdown({ children }: { children: React.ReactNode }) {
	const navigate = Route.useNavigate();
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
			<DropdownMenuContent className="w-[--radix-popper-anchor-width]">
				<DropdownMenuItem
					className="relative"
					indicator={<Icon icon={faVercel} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-vercel" },
						});
					}}
				>
					Vercel
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faRailway} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-railway" },
						});
					}}
				>
					Railway
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faAws} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-aws" },
						});
					}}
				>
					AWS ECS
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faGoogleCloud} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-gcp" },
						});
					}}
				>
					Google Cloud Run
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faHetznerH} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-hetzner" },
						});
					}}
				>
					Hetzner
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faServer} />}
					onSelect={() => {
						navigate({
							to: ".",
							search: { modal: "connect-custom" },
						});
					}}
				>
					Custom
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function DataLoadingPlaceholder() {
	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="mt-2 flex justify-between items-center px-6 py-4 max-w-5xl mx-auto">
				<H2 className="mb-2">
					<Skeleton className="w-48 h-8" />
				</H2>
			</div>
			<p className="max-w-5xl mb-6 px-6 text-muted-foreground mx-auto">
				<Skeleton className="w-full h-4" />
			</p>
			<hr className="mb-4" />
			<div className="p-4 px-6 max-w-5xl mx-auto ">
				<Skeleton className="h-8 w-48 mb-2" />
				<Skeleton className="h-6 w-72 mb-6" />
				<div className="flex flex-wrap gap-2 my-4">
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
				</div>
			</div>
			<div className="p-4 px-6 max-w-5xl mx-auto">
				<Skeleton className="h-8 w-48 mb-2" />
				<Skeleton className="h-6 w-72 mb-6" />
				<div className="flex flex-wrap gap-2 my-4">
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
				</div>
			</div>
		</div>
	);
}

function OneClickDeployRailwayButton() {
	return (
		<Button
			size="lg"
			variant="outline"
			className="min-w-48 h-auto min-h-28 text-xl"
			startIcon={<Icon icon={faRailway} />}
			asChild
		>
			<RouterLink to="." search={{ modal: "connect-q-railway" }}>
				Railway
			</RouterLink>
		</Button>
	);
}
