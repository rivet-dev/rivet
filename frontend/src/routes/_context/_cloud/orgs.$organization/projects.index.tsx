import { faChevronRight, faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";
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
	errorComponent: RouteError,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						<Card>
							<CardHeader>
								<CardTitle className="flex items-center">
									Projects
								</CardTitle>
								<CardDescription>
									Manage your organization&apos;s projects.
								</CardDescription>
							</CardHeader>
							<CardContent>
								<div className="flex flex-col">
									<ProjectList />
								</div>
							</CardContent>
						</Card>
					</div>
				</div>
			</div>
		</RouteLayout>
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
