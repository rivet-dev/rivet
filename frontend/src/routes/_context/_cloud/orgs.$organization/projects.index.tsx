import {
	faChevronLeft,
	faChevronRight,
	faHome,
	faPlus,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Skeleton,
} from "@/components";
import { VisibilitySensor } from "@/components/visibility-sensor";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex flex-col gap-6 px-4 w-full mx-auto h-screen min-h-0 max-h-screen items-center justify-safe-center overflow-auto py-8">
			<div className="flex flex-col items-center gap-6 min-h-fit">
				<div>
					<Breadcrumbs />
					<Card className="w-full sm:w-96 mb-4">
						<CardHeader>
							<CardTitle>Projects</CardTitle>
							<CardDescription>
								Select a project to continue.
							</CardDescription>
						</CardHeader>
						<CardContent>
							<ProjectList />
						</CardContent>
					</Card>
				</div>
			</div>
		</div>
	);
}

function ProjectList() {
	const { data, isLoading, hasNextPage, fetchNextPage } = useInfiniteQuery(
		Route.useRouteContext().dataProvider.currentOrgProjectsQueryOptions(),
	);

	return (
		<div className="flex flex-col border rounded-md w-full">
			{isLoading
				? Array(5)
						.fill(null)
						.map((_, i) => <ListItemSkeleton key={i} />)
				: null}

			{data?.map((project) => (
				<Link
					key={project.id}
					className="p-2 border-b last:border-0 w-full flex text-left items-center hover:bg-accent rounded-md transition-colors"
					to="/orgs/$organization/projects/$project"
					from={Route.to}
					params={{ project: project.name }}
				>
					<span className="flex-1 truncate">{project.name}</span>
					<Icon icon={faChevronRight} className="ml-auto" />
				</Link>
			))}
			{hasNextPage ? <VisibilitySensor onChange={fetchNextPage} /> : null}
			<Link from={Route.to} to="." search={{ modal: "create-project" }}>
				<div className="p-2 w-full flex items-center justify-center text-sm hover:bg-accent rounded-md transition-colors cursor-pointer">
					<Icon icon={faPlus} className="mr-1" /> Create Project
				</div>
			</Link>
		</div>
	);
}

function ListItemSkeleton() {
	return (
		<div className="p-2 border-b last:border-0 w-full flex text-left items-center rounded-md transition-colors h-10">
			<Skeleton className="size-4 mr-2 rounded-full" />
			<Skeleton className="flex-1 h-4 rounded" />
		</div>
	);
}

function Breadcrumbs() {
	const { data } = useQuery(
		Route.useRouteContext().dataProvider.organizationQueryOptions({
			org: Route.useParams().organization,
		}),
	);
	return (
		<div className="text-xs text-muted-foreground mb-2 flex gap-1 items-center">
			<Link from={Route.to} to="/orgs" className="hover:underline">
				<Icon icon={faHome} />
			</Link>
			<Icon icon={faChevronRight} />
			<span className="text-foreground font-medium">
				{data?.name || <Skeleton className="h-4 w-16" />}
			</span>
		</div>
	);
}
