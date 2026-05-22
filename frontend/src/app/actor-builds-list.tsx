import { useInfiniteQuery, usePrefetchInfiniteQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { Fragment } from "react";
import { Button, cn, Skeleton } from "@/components";
import { ActorIcon } from "@/components/lazy-icon";
import { useEngineCompatDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { features } from "@/lib/features";
import { RECORDS_PER_PAGE } from "./data-providers/default-data-provider";

const ICON_CLASS =
	"opacity-80 group-hover:opacity-100 group-data-active:opacity-100";

export function ActorBuildsList() {
	usePrefetchInfiniteQuery({...useEngineCompatDataProvider().buildsQueryOptions(), pages: Infinity});
	const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(useEngineCompatDataProvider().buildsQueryOptions());

	const navigate = useNavigate();

	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{data?.length === 0 ? (
					<p className="text-xs text-muted-foreground ms-1 px-1">
						Connect RivetKit to see instances.
					</p>
				) : null}
				{data?.toSorted((a, b) => a.id.localeCompare(b.id)).map((build) => {
					const actorMeta = build.name.metadata as
						| Record<string, unknown>
						| undefined;
					const iconValue =
						typeof actorMeta?.icon === "string"
							? actorMeta.icon
							: null;
					const displayName =
						typeof actorMeta?.name === "string"
							? actorMeta.name
							: build.id;

					return (
						<Button
							key={build.id}
							className={cn(
								"text-muted-foreground justify-start font-medium px-1",
								"data-active:text-foreground data-active:bg-foreground/[0.06]",
							)}
							startIcon={
								<ActorIcon
									iconValue={iconValue}
									className={ICON_CLASS}
									emojiClassName={cn(ICON_CLASS, "text-sm")}
								/>
							}
							variant={"ghost"}
							size="sm"
							onClick={() => {
								// eslint-disable-next-line @typescript-eslint/no-explicit-any
								return (navigate as any)({
									to: features.platform
										? "/orgs/$organization/projects/$project/ns/$namespace"
										: "/ns/$namespace",
									search: (old: Record<string, unknown>) => ({
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
									{displayName}
								</span>
							</Link>
						</Button>
					);
				})}
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
			{hasNextPage && !isFetchingNextPage ? (
				<VisibilitySensor onChange={fetchNextPage} />
			) : null}
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
