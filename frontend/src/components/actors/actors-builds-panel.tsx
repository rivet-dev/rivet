import {
	// @ts-expect-error
	faActorsBorderless,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Fragment } from "react/jsx-runtime";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { Skeleton } from "../ui/skeleton";
import { VisibilitySensor } from "../visibility-sensor";
import { useFilters } from "./actor-filters-context";
import { ACTORS_PER_PAGE, useManager } from "./manager-context";

export function ActorsBuildsPanel() {
	const {
		data,
		isSuccess,
		isLoading,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
	} = useInfiniteQuery(useManager().buildsQueryOptions());

	const navigate = useNavigate({ from: "/ns/$namespace" });
	const search = useSearch({ from: "/_layout/ns/$namespace" });

	const isSearchingById = useFilters(
		(filters) => filters.id?.value?.length > 0,
	);

	return (
		<ScrollArea className="w-full h-full">
			<div className="flex flex-col gap-[1px] p-3 mt-1">
				{isSearchingById ? (
					<p className="text-xs text-muted-foreground mb-4">
						When searching by ID, the list of names is not
						available.
					</p>
				) : null}
				{data?.map((build) => (
					<Button
						key={build.name}
						className="justify-start"
						disabled={isSearchingById}
						variant={
							!isSearchingById && search.n?.includes(build.name)
								? "secondary"
								: "ghost"
						}
						size="sm"
						startIcon={<Icon icon={faActorsBorderless} />}
						onClick={() => {
							navigate({
								to: ".",
								search: {
									n: [build.name],
								},
							});
						}}
					>
						<span className="text-ellipsis overflow-hidden whitespace-nowrap">
							{build.name}
						</span>
					</Button>
				))}
				{isSuccess && data.length === 0 && (
					<div className="text-sm text-muted-foreground">
						No Actor Names found.
					</div>
				)}
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
		</ScrollArea>
	);
}
