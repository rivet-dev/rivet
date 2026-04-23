import { faPlus, faQuestionCircle, Icon } from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useQuery,
} from "@tanstack/react-query";
import { createFileRoute, notFound, useNavigate } from "@tanstack/react-router";
import { HelpDropdown } from "@/app/help-dropdown";
import { features } from "@/lib/features";
import { Content } from "@/app/layout";
import { ProviderDropdown } from "@/app/provider-dropdown";
import { RunnerConfigsTable } from "@/app/runner-config-table";
import { RunnersTable } from "@/app/runners-table";
import { SidebarToggle } from "@/app/sidebar-toggle";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	Code,
	H1,
	H2,
	H3,
	H4,
	Ping,
	Skeleton,
} from "@/components";
import {
	ActorRegion,
	useCloudNamespaceDataProvider,
	useDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";
import { NoProvidersAlert } from "@/components/actors/no-providers-alert";
import { CloudApiTokens, PublishableToken, SecretToken } from "./tokens";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/ns/$namespace/settings",
)({
	component:RouteComponent,
		
	pendingComponent: DataLoadingPlaceholder,
	beforeLoad: async () => {
		if(!features.multitenancy) {
			throw notFound();
		}
	},
	loader: async ({ context }) => {
		const dataProvider = context.dataProvider;
		await Promise.all([
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnerConfigsQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnersQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.datacentersQueryOptions(),
			),
			context.queryClient.prefetchQuery(
				dataProvider.currentProjectQueryOptions(),
			),
			context.queryClient.prefetchQuery(
				dataProvider.currentNamespaceQueryOptions(),
			),
		]);
	},
	loaderDeps() {
		return [];
	},
});

export function RouteComponent() {
	return (
		<Content>
			<div className=" ">
				<div className="mb-4 pt-2 max-w-5xl mx-auto">
					<div className="flex justify-between items-center px-6 @6xl:px-0 py-4 ">
						<SidebarToggle className="absolute left-4" />
						<H1>Settings</H1>
						<HelpDropdown>
							<Button
								variant="outline"
								startIcon={<Icon icon={faQuestionCircle} />}
							>
								Need help?
							</Button>
						</HelpDropdown>
					</div>
					<p className="max-w-5xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
						Connect your RivetKit application to Rivet Cloud. Use
						your cloud of choice to run Rivet Actors.
					</p>
				</div>

				<hr className="mb-6" />

				<div className="px-4">
					<NoRunnersAlert />
					<Providers />
					<Runners />
					<Advanced />
				</div>
			</div>
		</Content>
	);
}

function NoRunnersAlert() {
	const dataProvider = useDataProvider();
	const { data: runnerNamesCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerNamesQueryOptions(),
		select: (data) => data.pages.flatMap((page) => page.names).length,
	});

	const { data: runnerConfigsCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		select: (data) =>
			data.pages.flatMap((page) => Object.keys(page.runnerConfigs))
				.length,
	});

	const runnersCount = runnerConfigsCount + runnerNamesCount;

	if (runnersCount > 0) {
		return null;
	}

	return (
		<div className="max-w-5xl mx-auto mb-6 px-6 @6xl:px-0 flex flex-col items-start">
			<NoProvidersAlert variant="connect" />
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
		<div className="p-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg">
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
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg ">
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

function Advanced() {
	return (
		<Accordion type="single" collapsible>
			<AccordionItem value="advanced">
				<AccordionTrigger className="max-w-5xl mx-auto px-6 py-6 @6xl:px-0">
					Advanced
				</AccordionTrigger>
				<AccordionContent>
					<SecretToken />
					<PublishableToken />
					{features.auth ? <CloudApiTokens /> : null}
					<DatacenterStatus />
					{features.namespaceManagement ? <DangerZone /> : null}
				</AccordionContent>
			</AccordionItem>
		</Accordion>
	);
}

function DatacenterStatus() {
	const dataProvider = useEngineCompatDataProvider();

	usePrefetchInfiniteQuery({
		...dataProvider.datacentersQueryOptions(),
		maxPages: Infinity,
	});
	const { data } = useInfiniteQuery(dataProvider.datacentersQueryOptions());

	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg ">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Datacenters Status</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				These are the datacenters where Rivet Engine is currently
				running and available to run your Rivet Actors.
			</p>

			<ul className="flex flex-col gap-1">
				{data?.map((region) => (
					<li key={region.name}>
						<div className="inline-flex gap-2 items-center h-full">
							<Ping
								variant="success"
								className="relative left-0 right-0 top-0"
							/>
							<ActorRegion showLabel regionId={region.name} />{" "}
							<Code className="text-xs">{region.name}</Code>
						</div>
					</li>
				))}
			</ul>
		</div>
	);
}

function DangerZone() {
	const dataProvider = useCloudNamespaceDataProvider();
	const navigate = useNavigate();

	const { data: project } = useQuery(
		dataProvider.currentProjectQueryOptions(),
	);

	const { data: namespace } = useQuery(
		dataProvider.currentNamespaceQueryOptions(),
	);

	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-t @6xl:border @6xl:rounded-lg">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Danger Zone</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Perform actions that could affect the stability of your Rivet
				Actors and Runners.
			</p>

			<div className="border border-destructive rounded-md p-4 bg-destructive/10 mb-4">
				<H4 className="mb-2 text-destructive-foreground">
					Archive namespace '{namespace?.displayName}'
				</H4>
				<p className=" mb-4">
					Archiving this namespace will permanently remove all
					associated Rivet Actors, Runners, and configurations. This
					action cannot be undone.
				</p>
				<Button
					variant="destructive"
					onClick={() =>
						navigate({
							to: ".",
							search: {
								modal: "delete-namespace",
								displayName: namespace?.displayName,
							},
						})
					}
				>
					Archive namespace
				</Button>
			</div>

			<div className="border border-destructive rounded-md p-4 bg-destructive/10 mb-4">
				<H4 className="mb-2 text-destructive-foreground">
					Archive project '{project?.displayName}'
				</H4>
				<p className=" mb-4">
					Archiving this project will permanently remove all
					associated Rivet Actors, Runners, and configurations. This
					action cannot be undone.
				</p>
				<Button
					variant="destructive"
					onClick={() =>
						navigate({
							to: ".",
							search: {
								modal: "delete-project",
								displayName: project?.displayName,
							},
						})
					}
				>
					Archive project
				</Button>
			</div>
		</div>
	);
}
