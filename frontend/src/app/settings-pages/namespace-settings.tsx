import { faPlus, Icon } from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useQuery,
} from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, Suspense } from "react";
import { ProviderDropdown } from "@/app/provider-dropdown";
import { RunnerConfigsTable } from "@/app/runner-config-table";
import { RunnersTable } from "@/app/runners-table";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	Code,
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
import { features } from "@/lib/features";
import {
	CloudApiTokens,
	PublishableToken,
	SecretToken,
} from "@/routes/_context/orgs.$organization/projects.$project/ns.$namespace/tokens";

export function NamespaceSettingsContent() {
	return (
		<div className="space-y-4">
			<NoRunnersAlert />
			<Providers />
			<Runners />
			<Advanced />
		</div>
	);
}

/** Compact section card matching the drawer's tighter typography. */
function SectionCard({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: string;
	action?: ReactNode;
	children: ReactNode;
}) {
	return (
		<div className="p-5 rounded-xl border border-foreground/10 bg-card">
			<div className="flex gap-3 items-start justify-between mb-3">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold text-foreground">
						{title}
					</h3>
					{description ? (
						<p className="text-xs text-muted-foreground mt-0.5">
							{description}
						</p>
					) : null}
				</div>
				{action ? <div className="shrink-0">{action}</div> : null}
			</div>
			{children}
		</div>
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

	if (runnerConfigsCount + runnerNamesCount > 0) {
		return null;
	}

	return (
		<div className="flex flex-col items-start">
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
		<SectionCard
			title="Providers"
			description="Clouds connected to Rivet for running Rivet Actors."
			action={
				<ProviderDropdown>
					<Button
						variant="outline"
						size="sm"
						startIcon={<Icon icon={faPlus} className="size-3" />}
					>
						Add Provider
					</Button>
				</ProviderDropdown>
			}
		>
			<div className="border border-foreground/10 rounded-md overflow-hidden">
				<RunnerConfigsTable
					isLoading={isLoading}
					isError={isError}
					configs={configs || []}
					fetchNextPage={fetchNextPage}
					hasNextPage={hasNextPage}
				/>
			</div>
		</SectionCard>
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
	const {
		isLoading: isLoadingEnvoys,
		isError: isErrorEnvoys,
		data: envoys,
		hasNextPage: hasNextEnvoysPage,
		fetchNextPage: fetchNextEnvoysPage,
	} = useInfiniteQuery({
		...useEngineCompatDataProvider().currentNamespaceEnvoyListQueryOptions(),
		refetchInterval: 5000,
	});

	const allRunners = [...(envoys || []), ...(runners || [])];

	return (
		<SectionCard
			title="Runners"
			description="Processes connected to Rivet Cloud and ready to start running Rivet Actors."
		>
			<div className="border border-foreground/10 rounded-md overflow-hidden">
				<RunnersTable
					isLoading={isLoading || isLoadingEnvoys}
					isError={isError || isErrorEnvoys}
					runners={allRunners || []}
					fetchNextPage={() => {
						if (hasNextEnvoysPage) fetchNextEnvoysPage();
						if (hasNextPage) fetchNextPage();
					}}
					hasNextPage={hasNextPage || hasNextEnvoysPage}
				/>
			</div>
		</SectionCard>
	);
}

function Advanced() {
	return (
		<Accordion type="single" collapsible>
			<AccordionItem
				value="advanced"
				className="rounded-xl border border-foreground/10 bg-card overflow-hidden"
			>
				<AccordionTrigger className="px-5 py-4 text-sm font-semibold hover:no-underline">
					Advanced
				</AccordionTrigger>
				<AccordionContent className="px-5 pb-5 pt-0">
					<Suspense
						fallback={
							<div className="space-y-3">
								<Skeleton className="w-full h-16 rounded-md" />
								<Skeleton className="w-full h-16 rounded-md" />
								<Skeleton className="w-full h-16 rounded-md" />
							</div>
						}
					>
						<div className="space-y-4">
							<SecretToken />
							<PublishableToken />
							{features.auth ? <CloudApiTokens /> : null}
							<DatacenterStatus />
							{features.dangerZone ? <DangerZone /> : null}
						</div>
					</Suspense>
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
		<SectionCard
			title="Datacenters status"
			description="Where the Rivet Engine is currently running and ready to run your Rivet Actors."
		>
			<ul className="flex flex-col gap-1">
				{data?.map((region) => (
					<li key={region.name}>
						<div className="inline-flex gap-2 items-center h-full text-sm">
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
		</SectionCard>
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
		<SectionCard
			title="Danger zone"
			description="Actions that affect the stability of your Rivet Actors and Runners."
		>
			<div className="border border-destructive rounded-md p-4 bg-destructive/10 mb-3">
				<h4 className="text-sm font-semibold mb-1 text-destructive-foreground">
					Archive namespace '{namespace?.displayName}'
				</h4>
				<p className="text-xs text-muted-foreground mb-3">
					Permanently removes all associated Rivet Actors, Runners,
					and configurations. Cannot be undone.
				</p>
				<Button
					variant="destructive"
					size="sm"
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

			<div className="border border-destructive rounded-md p-4 bg-destructive/10">
				<h4 className="text-sm font-semibold mb-1 text-destructive-foreground">
					Archive project '{project?.displayName}'
				</h4>
				<p className="text-xs text-muted-foreground mb-3">
					Permanently removes all associated Rivet Actors, Runners,
					and configurations. Cannot be undone.
				</p>
				<Button
					variant="destructive"
					size="sm"
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
		</SectionCard>
	);
}
