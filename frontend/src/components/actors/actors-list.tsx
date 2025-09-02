import {
	// @ts-expect-error
	faActors,
	faMagnifyingGlass,
	faNodeJs,
	faQuestionSquare,
	faReact,
	faSidebar,
	faSidebarFlip,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { Navigate, useNavigate, useSearch } from "@tanstack/react-router";
import { Suspense, useCallback } from "react";
import {
	Button,
	DocsSheet,
	FilterCreator,
	FiltersDisplay,
	type OnFiltersChange,
	type PickFiltersOptions,
	ScrollArea,
	ShimmerLine,
	SmallText,
	WithTooltip,
} from "@/components";
import { VisibilitySensor } from "../visibility-sensor";
import { useActorsFilters } from "./actor-filters-context";
import { useActorsLayout } from "./actors-layout-context";
import { ActorsListRow, ActorsListRowSkeleton } from "./actors-list-row";
import { useActorsView } from "./actors-view-context-provider";
import { CreateActorButton } from "./create-actor-button";
import { ACTORS_PER_PAGE, useManager } from "./manager-context";
import { useRootLayout } from "./root-layout-context";

export function ActorsList() {
	return (
		<ScrollArea className="w-full @container/main">
			<TopBar />
			<div className="grid grid-cols-[2rem_4fr_1fr] @lg/main:grid-cols-[2rem_4fr_1fr] items-center justify-center gap-x-4 w-full @container/table">
				<Suspense fallback={<ListSkeleton />}>
					<ActorIdPrefiller />
					<List />
					<Pagination />
				</Suspense>
			</div>
		</ScrollArea>
	);
}

function TopBar() {
	const { isSidebarCollapsed, sidebarRef } = useRootLayout();
	const { isDetailsColCollapsed, detailsRef } = useActorsLayout();

	return (
		<div className="col-span-full border-b flex px-2 py-2 gap-1 relative @lg/h-[45px]">
			{isSidebarCollapsed ? (
				<WithTooltip
					trigger={
						<Button
							onClick={() => sidebarRef.current?.expand()}
							variant="outline"
							size="icon-sm"
						>
							<Icon icon={faSidebar} />
						</Button>
					}
					content="Expand Actor Names column"
				/>
			) : null}
			<div className="justify-between flex flex-1 flex-wrap gap-2 w-full">
				{__APP_TYPE__ === "hub" ? <Filters /> : <div />}
				<div className="flex gap-1">
					<CreateActorButton />
					<Display />
				</div>
			</div>
			{isDetailsColCollapsed ? (
				<WithTooltip
					trigger={
						<Button
							onClick={() => detailsRef.current?.expand()}
							variant="outline"
							size="icon-sm"
						>
							<Icon icon={faSidebarFlip} />
						</Button>
					}
					content="Expand details column"
				/>
			) : null}
			<LoadingIndicator />
		</div>
	);
}

function LoadingIndicator() {
	const n = useSearch({
		from: "/_layout",
		select: (state) => state.n,
	});
	const filters = useFiltersValue({ includeEphemeral: false });
	const { isLoading } = useInfiniteQuery(
		useManager().actorsListQueryOptions({ n, filters }),
	);
	if (isLoading) {
		return <ShimmerLine className="bottom-0" />;
	}
	return null;
}

function List() {
	const filters = useFiltersValue({ includeEphemeral: false });
	const { actorId, n } = useSearch({
		from: "/_layout",
	});
	const { data: actorIds = [] } = useInfiniteQuery(
		useManager().actorsListQueryOptions({ n, filters }),
	);

	return (
		<>
			{actorIds.map((id) => (
				<ActorsListRow
					key={id}
					actorId={id}
					isCurrent={actorId === id}
				/>
			))}
		</>
	);
}

function ActorIdPrefiller() {
	const { n, actorId } = useSearch({
		from: "/_layout",
		select: (state) => ({
			n: state.n,
			actorId: state.actorId,
		}),
	});
	const filters = useFiltersValue({ includeEphemeral: false });
	const { data } = useSuspenseInfiniteQuery(
		useManager().actorsListQueryOptions({
			n,
			filters,
		}),
	);

	if (!actorId && data?.[0]) {
		return (
			<Navigate
				to="."
				search={(search) => ({ ...search, actorId: data?.[0] })}
				replace
			/>
		);
	}

	return null;
}

function Pagination() {
	const n = useSearch({
		from: "/_layout",
		select: (state) => state.n,
	});
	const filters = useFiltersValue({ includeEphemeral: false });
	const { hasNextPage, isFetchingNextPage, fetchNextPage, data } =
		useSuspenseInfiniteQuery(
			useManager().actorsListPaginationQueryOptions({
				n,
				filters,
			}),
		);

	if (isFetchingNextPage) {
		return <ListSkeleton />;
	}

	if (hasNextPage) {
		return <VisibilitySensor onChange={fetchNextPage} />;
	}

	return <EmptyState count={data || 0} />;
}

export function ListSkeleton() {
	return (
		<div className="grid grid-cols-subgrid col-span-full">
			{Array(ACTORS_PER_PAGE)
				.fill(null)
				.map((_, i) => (
					<ActorsListRowSkeleton key={i} />
				))}
		</div>
	);
}

function useFiltersValue(opts: PickFiltersOptions = {}) {
	const { pick } = useActorsFilters();
	return useSearch({
		from: "/_layout",
		select: (state) => pick(state, opts),
	});
}

function EmptyState({ count }: { count: number }) {
	const navigate = useNavigate();
	const names = useSearch({
		from: "/_layout",
		select: (state) => state.n,
	});
	const { copy, links } = useActorsView();
	const { remove, pick } = useActorsFilters();

	const { data: availableNamesCount = 0 } = useInfiniteQuery(
		useManager().buildsCountQueryOptions(),
	);

	const filtersCount = useSearch({
		from: "/_layout",
		select: (state) =>
			Object.values(pick(state, { includeEphemeral: false })).length,
	});

	const clearFilters = () => {
		navigate({
			to: ".",
			search: (prev) => ({
				...remove(prev),
			}),
		});
	};

	return (
		<div className=" col-span-full my-4 flex flex-col items-center gap-2 justify-center">
			{(!names || names?.length === 0) && availableNamesCount > 0 ? (
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2 my-12">
					<Icon icon={faQuestionSquare} className="text-4xl" />
					<p className="max-w-[400px]">
						No Actor Name selected.
						<br />
						<span className="text-sm text-muted-foreground">
							Select an Actor Name from the list on the left.
						</span>
					</p>
				</div>
			) : count === 0 ? (
				filtersCount === 0 ? (
					<div className="gap-2 flex flex-col items-center justify-center">
						<Icon icon={faActors} className="text-4xl mt-8" />
						<SmallText className="text-center my-0">
							{copy.noActorsFound}
						</SmallText>
						<div className="mt-4 flex flex-col gap-2 items-center justify-center">
							<CreateActorButton variant="secondary" />{" "}
							<SmallText className="mt-4 mb-1">
								Use one of the quick start guides to get
								started.
							</SmallText>
							<div className="flex gap-2">
								<DocsSheet
									path={links.gettingStarted.node}
									title="Node.js & Bun Quickstart"
								>
									<Button
										className="flex-1"
										variant="outline"
										startIcon={<Icon icon={faNodeJs} />}
									>
										Node.js & Bun
									</Button>
								</DocsSheet>
								<DocsSheet
									path={links.gettingStarted.react}
									title="React Quickstart"
								>
									<Button
										className="flex-1"
										variant="outline"
										startIcon={<Icon icon={faReact} />}
									>
										React
									</Button>
								</DocsSheet>
							</div>
						</div>
					</div>
				) : (
					<>
						<SmallText className="text-foreground text-center mt-8 mb-2">
							{copy.noActorsMatchFilter}
						</SmallText>
						<Button variant="outline" mx="4" onClick={clearFilters}>
							Clear filter
						</Button>
					</>
				)
			) : (
				<SmallText className="text-muted-foreground text-center text-xs">
					{copy.noMoreActors}
				</SmallText>
			)}
		</div>
	);
}

function useFiltersChangeCallback(): OnFiltersChange {
	const navigate = useNavigate();
	const { pick, remove } = useActorsFilters();

	return useCallback(
		(fnOrValue) => {
			if (typeof fnOrValue === "function") {
				navigate({
					to: ".",
					search: (old) => {
						const filters = pick(old);
						const prev = remove(old);

						return {
							...prev,
							...Object.fromEntries(
								Object.entries(fnOrValue(filters)).filter(
									([, filter]) => filter.value.length > 0,
								),
							),
						};
					},
				});
			} else {
				navigate({
					to: ".",
					search: (value) => ({
						...remove(value),
						...Object.fromEntries(
							Object.entries(fnOrValue).filter(
								([, filter]) => filter.value.length > 0,
							),
						),
					}),
				});
			}
		},
		[navigate, pick],
	);
}

function Filters() {
	const { definitions } = useActorsFilters();
	const filters = useFiltersValue();
	const onFiltersChange = useFiltersChangeCallback();

	return (
		<FilterCreator
			text="Go to Actor"
			value={filters}
			onChange={onFiltersChange}
			definitions={definitions}
			icon={<Icon icon={faMagnifyingGlass} />}
		/>
	);
}

function Display() {
	const { definitions } = useActorsFilters();
	const filters = useFiltersValue();
	const onFiltersChange = useFiltersChangeCallback();

	return (
		<FiltersDisplay
			value={filters}
			definitions={definitions}
			onChange={onFiltersChange}
		/>
	);
}
