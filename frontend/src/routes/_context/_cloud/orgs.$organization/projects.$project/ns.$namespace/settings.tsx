import {
	faAws,
	faGoogleCloud,
	faHetznerH,
	faPlus,
	faQuestionCircle,
	faNetlify,
	faRailway,
	faServer,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
} from "@tanstack/react-query";
import { createFileRoute, notFound, useNavigate } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
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
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	H1,
	H2,
	H3,
	H4,
	Ping,
	Skeleton,
} from "@/components";
import { ActorRegion, useEngineCompatDataProvider } from "@/components/actors";
import { CloudApiTokens, PublishableToken, SecretToken } from "./tokens";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/settings",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
	pendingComponent: DataLoadingPlaceholder,
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
					<Providers />
					<Runners />
					<Advanced />
				</div>
			</div>
		</Content>
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
					indicator={<Icon icon={faNetlify} />}
					onSelect={() =>
						navigate({
							to: ".",
							search: { modal: "connect-netlify" },
						})
					}
				>
					Netlify
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
					{__APP_TYPE__ === "cloud" ? <CloudApiTokens /> : null}
					<DatacenterStatus />
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
