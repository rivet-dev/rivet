import { faActorsBorderless, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { Fragment } from "react";
import { match } from "ts-pattern";
import { Button, cn, Skeleton } from "@/components";
import { useDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { RECORDS_PER_PAGE } from "./data-providers/default-data-provider";

export function ActorBuildsList() {
	const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(useDataProvider().buildsQueryOptions());

	const navigate = useNavigate();

	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{data?.length === 0 ? (
					<p className="text-xs text-muted-foreground ms-1 px-1">
						Connect RivetKit to see instances.
					</p>
				) : null}
				{data?.map((build) => (
					<Button
						key={build.id}
						className={cn(
							"text-muted-foreground justify-start font-medium px-1",
							"data-active:text-foreground data-active:bg-accent",
						)}
						startIcon={
							<Icon
								icon={faActorsBorderless}
								className="opacity-80 group-hover:opacity-100 group-data-active:opacity-100 "
							/>
						}
						variant={"ghost"}
						size="sm"
						onClick={() => {
							navigate({
								to: match(__APP_TYPE__)
									.with("engine", () => "/ns/$namespace")
									.with(
										"cloud",
										() =>
											"/orgs/$organization/projects/$project/ns/$namespace",
									)
									.otherwise(() => "/"),

								search: (old) => ({
									...old,
									actorId: undefined,
									n: [build.id],
								}),
							});
						}}
						asChild
					>
						<Link
							to="."
							search={(old) => ({
								...old,
								actorId: undefined,
								n: [build.id],
							})}
						>
							<span className="text-ellipsis overflow-hidden whitespace-nowrap">
								{build.id}
							</span>
						</Link>
					</Button>
				))}
				{isFetchingNextPage || isLoading
					? Array(RECORDS_PER_PAGE)
							.fill(null)
							.map((_, i) => (
								// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
								<Fragment key={i}>
									<Skeleton className="w-full h-6 my-1" />
								</Fragment>
							))
					: null}
			</div>
			{hasNextPage ? <VisibilitySensor onChange={fetchNextPage} /> : null}
		</div>
	);
}

export function ActorBuildsListSkeleton() {
	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{Array(RECORDS_PER_PAGE)
					.fill(null)
					.map((_, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
						<Fragment key={i}>
							<Skeleton className="w-full h-6 my-1" />
						</Fragment>
					))}
			</div>
		</div>
	);
}
