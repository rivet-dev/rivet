import {
	faAws,
	faSquare as faGoogleCloud,
	faSquare as faHetzner,
	faPlus,
	faQuestionCircle,
	faRailway,
	faServer,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	notFound,
	Link as RouterLink,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { RunnerConfigsTable } from "@/app/runner-config-table";
import { RunnersTable } from "@/app/runners-table";
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	H1,
	H3,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/connect",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
});

export function RouteComponent() {
	const { data: runnerNamesCount, isLoading } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnerNamesQueryOptions(),
		select: (data) => data.pages[0].names?.length,
		refetchInterval: 5000,
	});

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className=" mt-2 flex justify-between items-center px-6 py-4">
				<H1>Connect</H1>
				<div>
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
				Connect your RivetKit application to Rivet Cloud. Use your cloud
				of choice to run Rivet Actors.
			</p>

			<hr className="mb-4" />
			{isLoading ? (
				<div className="p-4 px-6 max-w-5xl ">
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
			) : runnerNamesCount === 0 ? (
				<div className="p-4 px-6 max-w-5xl">
					<H3>Add Provider</H3>
					<div className="grid grid-cols-3 gap-2 my-4">
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
							startIcon={<Icon icon={faHetzner} />}
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
			) : (
				<>
					<Providers />
					<Runners />
				</>
			)}
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
		<div className="p-4 px-6 max-w-5xl">
			<H3>Providers</H3>

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
		<div className="pb-4 px-6 max-w-5xl ">
			<div className="flex gap-2 items-center mb-4 mt-6">
				<H3 className="">Runners</H3>

				<ProviderDropdown>
					<Button
						className="min-w-32"
						variant="outline"
						startIcon={<Icon icon={faPlus} />}
					>
						Add Runner
					</Button>
				</ProviderDropdown>
			</div>
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
					indicator={<Icon icon={faHetzner} />}
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
