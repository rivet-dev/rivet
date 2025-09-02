import {
	// @ts-expect-error
	faActorsBorderless,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { Fragment } from "react";
import { Button, cn, Skeleton } from "@/components";
import { ACTORS_PER_PAGE, useManager } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";

export function ActorBuildsList() {
	const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(useManager().buildsQueryOptions());

	const navigate = useNavigate({ from: "/" });

	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{data?.length === 0 ? (
					<p className="text-xs text-muted-foreground ms-2">
						No instances found.
					</p>
				) : null}
				{data?.map((build) => (
					<Button
						key={build.name}
						className={cn(
							"text-muted-foreground justify-start",
							"data-active:text-foreground data-active:bg-accent",
						)}
						startIcon={
							<Icon
								icon={faActorsBorderless}
								className="opacity-80"
							/>
						}
						variant={"ghost"}
						size="sm"
						onClick={() => {
							navigate({
								to:
									__APP_TYPE__ === "engine"
										? "/ns/$namespace"
										: "/",
								search: (old) => ({
									...old,
									n: [build.name],
								}),
							});
						}}
						asChild
					>
						<Link
							to="."
							search={(old) => ({ ...old, n: [build.name] })}
						>
							<span className="text-ellipsis overflow-hidden whitespace-nowrap">
								{build.name}
							</span>
						</Link>
					</Button>
				))}
				{isFetchingNextPage || isLoading
					? Array(ACTORS_PER_PAGE)
							.fill(null)
							.map((_, i) => (
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
