import { faQuestionCircle, faRailway, faVercel, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	notFound,
	Link as RouterLink,
	useNavigate,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { RunnersTable } from "@/app/runners-table";
import {
	Button,
	DocsSheet,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	H1,
	H3,
	Link,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { docsLinks } from "@/content/data";

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
	});

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="max-w-5xl  mt-2 flex justify-between items-center px-6 py-4">
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
				of choice to run Rivet Actors.{" "}
				<DocsSheet path={docsLinks.runners} title="Runners">
					<Link className="cursor-pointer">
						Learn more about runners.
					</Link>
				</DocsSheet>
			</p>

			<hr className="mb-4" />
			{isLoading ? (
				<div className="p-4 px-6 max-w-5xl ">
					<Skeleton className="h-8 w-48 mb-4" />
					<div className="flex flex-wrap gap-2 my-4">
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
					</div>
				</div>
			) : runnerNamesCount !== 0 ? (
				<div className="p-4 px-6 max-w-5xl">
					<H3>Select Provider</H3>
					<div className="flex flex-wrap gap-2 my-4">
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
	const { data } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnerConfigsQueryOptions(),
		refetchInterval: 5000,
	});

	const navigate = Route.useNavigate();

	return (
		<div className="p-4 px-6 max-w-5xl">
			<H3>Providers</H3>
			<div className="flex flex-wrap gap-2 mt-4">
				{data?.map(([name]) => (
					<Button
						key={name}
						size="lg"
						variant="outline"
						className="min-w-32"
						asChild
					>
						<RouterLink
							to="."
							search={{
								modal: "edit-runner-config",
								config: name,
							}}
						>
							{name}
						</RouterLink>
					</Button>
				))}
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button
							size="lg"
							className="min-w-32"
							variant="outline"
						></Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent>
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
					</DropdownMenuContent>
				</DropdownMenu>
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
			<H3 className="mb-4 mt-6">Runners</H3>
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
