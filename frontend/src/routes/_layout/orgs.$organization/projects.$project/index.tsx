import type { Rivet } from "@rivet-gg/cloud";
import {
	faAdd,
	faArrowRight,
	faChevronRight,
	faRefresh,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute, Link, notFound } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { Button, H1, Skeleton, Text, WithTooltip } from "@/components";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { projectsQueryOptions } from "@/queries/manager-cloud";

export const Route = createFileRoute(
	"/_layout/orgs/$organization/projects/$project/",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => {
			throw notFound();
		}),
});

function RouteComponent() {
	const {
		data: projects,
		isRefetching,
		isSuccess,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
		isLoading,
		isError,
		refetch,
	} = useInfiniteQuery(projectsQueryOptions());

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="max-w-5xl mx-auto my-2 flex justify-between items-center px-6 py-4">
				<H1>Projects</H1>
				<div className="flex items-center gap-2">
					<WithTooltip
						content="Refresh"
						trigger={
							<Button
								size="icon"
								isLoading={isRefetching}
								variant="outline"
								onClick={() => refetch()}
							>
								<Icon icon={faRefresh} />
							</Button>
						}
					/>
				</div>
			</div>

			<hr className="mb-4" />

			<div className="p-4">
				<div className="max-w-5xl mx-auto p-2">
					<div className="grid grid-cols-3 gap-4">
						{isSuccess
							? projects.map((project) => (
									<Row key={project.id} {...project} />
								))
							: null}

						<Link
							to="."
							search={(old) => ({
								...old,
								modal: "create-project",
							})}
						>
							<div className="p-4 flex border rounded-lg flex-col min-h-20 justify-end hover:bg-secondary transition-colors cursor-pointer">
								<Text className="font-medium text-lg flex items-center">
									<span className="flex-1">
										Create New Project
									</span>
									<Icon icon={faChevronRight} />
								</Text>
							</div>
						</Link>

						{hasNextPage ? (
							<VisibilitySensor onChange={fetchNextPage} />
						) : null}

						{isLoading || isFetchingNextPage ? (
							<>
								<TileSkeleton />
								<TileSkeleton />
								<TileSkeleton />
								<TileSkeleton />
								<TileSkeleton />
								<TileSkeleton />
							</>
						) : null}
					</div>
				</div>
			</div>
		</div>
	);
}

function TileSkeleton() {
	return (
		<div className="p-4 flex border rounded-lg flex-col">
			<Skeleton className="h-6 w-32 mb-2" />
			<Skeleton className="h-4 w-48 mb-4" />
			<Skeleton className="h-8 w-full" />
		</div>
	);
}

function Row(project: Rivet.Project) {
	return (
		<div className="p-4 flex border rounded-lg flex-col">
			<Text className="font-medium text-lg">{project.name}</Text>
			{/* <Text className="text-sm text-muted-foreground mb-4">
				{project.}
			</Text> */}
			<Button>View Project</Button>
		</div>
	);
}
