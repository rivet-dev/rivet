import {
	faAws,
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
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import {
	createFileRoute,
	notFound,
	Link as RouterLink,
	useNavigate,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { PublishableTokenCodeGroup } from "@/app/publishable-token-code-group";
import { RunnerConfigsTable } from "@/app/runner-config-table";
import { RunnersTable } from "@/app/runners-table";
import { SidebarToggle } from "@/app/sidebar-toggle";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	cn,
	DocsSheet,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	H1,
	H2,
	H3,
	Ping,
	Skeleton,
	WithTooltip,
} from "@/components";
import { ActorRegion, useEngineCompatDataProvider } from "@/components/actors";
import { useRootLayout } from "@/components/actors/root-layout-context";
import { useEndpoint, usePublishableToken } from "@/queries/accessors";

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
	const engineCompatDataProvider = useEngineCompatDataProvider();

	const runnerNamesQueryOptions =
		engineCompatDataProvider.runnerNamesQueryOptions();
	const runnerConfigsQueryOptions =
		engineCompatDataProvider.runnerConfigsQueryOptions();

	const { data: runnerNamesCount, isLoading: isRunnerNamesLoading } =
		useSuspenseInfiniteQuery({
			...runnerNamesQueryOptions,
			queryKey: [...runnerNamesQueryOptions.queryKey, "count"],
			select: (data) => data.pages[0].names.length,
			refetchInterval: 5000,
		});

	const { data: runnerConfigsCount, isLoading: isRunnerConfigsLoading } =
		useSuspenseInfiniteQuery({
			...runnerConfigsQueryOptions,
			queryKey: [...runnerConfigsQueryOptions.queryKey, "count"],
			select: (data) =>
				Object.entries(data.pages[0].runnerConfigs).length,
			refetchInterval: 5000,
		});

	const isLoading = isRunnerNamesLoading || isRunnerConfigsLoading;
	const hasRunnerNames =
		runnerNamesCount !== undefined && runnerNamesCount > 0;
	const hasRunnerConfigs =
		runnerConfigsCount !== undefined && runnerConfigsCount > 0;

	const { isSidebarCollapsed } = useRootLayout();

	if (isLoading) {
		return (
			<div
				className={cn(
					"h-full overflow-auto",
					!isSidebarCollapsed &&
						"border my-2 bg-card rounded-lg mr-2",
				)}
			>
				<div className=" max-w-5xl mx-auto">
					<div className="mt-2 flex justify-between items-center px-6 py-4 ">
						<SidebarToggle className="absolute left-4" />
						<H1>Overview</H1>
						<HelpDropdown>
							<Button
								variant="outline"
								startIcon={<Icon icon={faQuestionCircle} />}
							>
								Need help?
							</Button>
						</HelpDropdown>
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

	if (!hasRunnerNames && !hasRunnerConfigs) {
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
							<HelpDropdown>
								<Button
									variant="outline"
									startIcon={<Icon icon={faQuestionCircle} />}
								>
									Need help?
								</Button>
							</HelpDropdown>
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
		<div
			className={cn(
				" h-full overflow-auto @container transition-colors",
				!isSidebarCollapsed && "border my-2 bg-card rounded-lg mr-2",
			)}
		>
			<div className=" ">
				<div className="mb-4 pt-2 max-w-5xl mx-auto">
					<div className="flex justify-between items-center px-6 py-4 ">
						<SidebarToggle className="absolute left-4" />
						<H1>Overview</H1>
						<HelpDropdown>
							<Button
								variant="outline"
								startIcon={<Icon icon={faQuestionCircle} />}
							>
								Need help?
							</Button>
						</HelpDropdown>
					</div>
					<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
						Connect your RivetKit application to Rivet Cloud. Use
						your cloud of choice to run Rivet Actors.
					</p>
				</div>

				<hr className="mb-6" />

				<div className="px-4">
					<ConnectYourFrontend />
					<Providers />
					<Runners />
					<DatacentersStatus />
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
			<div className="flex gap-2 items-center justify-between mb-2">
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

function ConnectYourFrontend() {
	const token = usePublishableToken();
	const endpoint = useEndpoint();
	const dataProvider = useEngineCompatDataProvider();
	const namespace = dataProvider.engineNamespace;

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Connect Your Frontend</H3>
			</div>
			<p className="mb-8">
				This token is safe to publish on your frontend.
			</p>
			<div>
				<PublishableTokenCodeGroup
					token={token}
					endpoint={endpoint}
					namespace={namespace}
				/>
			</div>
		</div>
	);
}

function ProviderDropdown({ children }: { children: React.ReactNode }) {
	const navigate = useNavigate();
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
			<DropdownMenuContent className="w-[--radix-popper-anchor-width]">
				<DropdownMenuItem
					className="relative"
					indicator={<Icon icon={faVercel} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-vercel" },
						})
					}
				>
					Vercel
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faRailway} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-railway" },
						})
					}
				>
					Railway
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faAws} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-aws" },
						})
					}
				>
					AWS ECS
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faGoogleCloud} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-gcp" },
						})
					}
				>
					Google Cloud Run
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faHetznerH} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-hetzner" },
						})
					}
				>
					Hetzner
				</DropdownMenuItem>
				<DropdownMenuItem
					indicator={<Icon icon={faServer} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-custom" },
						})
					}
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

function DatacentersStatus() {
	const dataProvider = useEngineCompatDataProvider();

	usePrefetchInfiniteQuery({
		...dataProvider.regionsQueryOptions(),
		maxPages: Infinity,
	});

	const { data } = useInfiniteQuery(dataProvider.regionsQueryOptions());

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 @6xl:px-0">
			<Accordion type="single" collapsible>
				<AccordionItem value="advanced">
					<AccordionTrigger className="text-muted-foreground hover:text-foreground">
						Advanced
					</AccordionTrigger>
					<AccordionContent className="@6xl:border @6xl:rounded-lg bg-muted/10 p-4">
						<WithTooltip
							trigger={
								<p className="inline-block">
									{data?.length} datacenters are currently
									active and available.
								</p>
							}
							content={
								<div className="max-w-sm">
									<ul className="list-outside list-disc">
										{data?.map((region) => (
											<li
												key={region.id}
												className="flex items-center"
											>
												<div className="inline-flex gap-2 items-center h-full">
													<ActorRegion
														showLabel
														regionId={region.id}
													/>{" "}
													({region.name})
													<Ping
														variant="success"
														className="relative left-0 right-0 top-0"
													/>
												</div>
											</li>
										))}
									</ul>
								</div>
							}
						/>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	);
}
