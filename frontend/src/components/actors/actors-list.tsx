import {
	faActors,
	faMagnifyingGlass,
	faQuestionSquare,
	faSidebarFlip,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useQuery,
	useQueryClient,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Suspense, useCallback, useRef, useState, type RefObject } from "react";
import { useLocalStorage } from "usehooks-ts";
import { RECORDS_PER_PAGE } from "@/app/data-providers/default-data-provider";
import {
	Button,
	FiltersDisplay,
	ls,
	type OnFiltersChange,
	ScrollArea,
	ShimmerLine,
	SmallText,
	WithTooltip,
} from "@/components";
import { VisibilitySensor } from "../visibility-sensor";
import { useActorsFilters, useFiltersValue } from "./actor-filters-context";
import { useActorsLayout } from "./actors-layout-context";
import { ActorsListRow, ActorsListRowSkeleton } from "./actors-list-row";
import { useActorsView } from "./actors-view-context-provider";
import { CreateActorButton } from "./create-actor-button";
import { useDataProvider } from "./data-provider";
import { NoProvidersAlert } from "./no-providers-alert";

export function ActorsList() {
	const viewportRef = useRef<HTMLDivElement>(null);

	return (
		<ScrollArea
			className="h-full w-full @container/main"
			viewportRef={viewportRef}
		>
			<TopBar />
			<Page1Poller />
			<Suspense fallback={<ListSkeleton />}>
				<List viewportRef={viewportRef} />
				<Pagination />
			</Suspense>
		</ScrollArea>
	);
}

function ActorNameLabel() {
	const { n } = useSearch({ from: "/_context" });
	const buildId = n?.[0];
	const { data: builds = [] } = useInfiniteQuery(
		useDataProvider().buildsQueryOptions(),
	);

	if (!buildId) return null;

	const build = builds.find((b) => b.id === buildId);
	const meta = build?.name?.metadata as
		| Record<string, unknown>
		| undefined;
	const displayName =
		typeof meta?.name === "string" ? meta.name : (buildId ?? "");

	return (
		<span className="text-sm font-medium text-foreground truncate min-w-0">
			{displayName}
		</span>
	);
}

function Page1Poller() {
	const { n } = useSearch({ from: "/_context" });
	const filters = useFiltersValue({ onlyStatic: true });
	useQuery(useDataProvider().actorsListPage1PollQueryOptions({ n, filters }));
	return null;
}

function TopBar() {
	const { isDetailsColCollapsed, detailsRef } = useActorsLayout();
	const { n } = useSearch({ from: "/_context" });
	const filters = useFiltersValue({ onlyStatic: true });
	const { data: actors = [], isLoading } = useInfiniteQuery(
		useDataProvider().actorsListQueryOptions({ n, filters }),
	);
	const filtersCount = Object.values(filters).length;

	// When there are no instances and the user hasn't applied any filter,
	// suppress both the "+" button and the search/display row so the empty
	// state below is the only call-to-action. Filters keep the row visible
	// because the user still needs a way to clear them.
	const showInstanceTools =
		isLoading || actors.length > 0 || filtersCount > 0;

	return (
		<div className="col-span-full border-b sticky top-0 bg-card z-[1]">
			<div className="flex items-center px-3 gap-2 h-[45px]">
				<ActorNameLabel />
				{showInstanceTools ? (
					<div className="ml-auto flex items-center gap-1 shrink-0">
						<CreateActorButton iconOnly label="Create Instance" />
					</div>
				) : null}
				{isDetailsColCollapsed ? (
					<WithTooltip
						trigger={
							<Button
								onClick={() => detailsRef.current?.expand()}
								variant="outline"
								size="icon-sm"
								className={showInstanceTools ? "" : "ml-auto"}
							>
								<Icon icon={faSidebarFlip} />
							</Button>
						}
						content="Expand details column"
					/>
				) : null}
			</div>
			{showInstanceTools ? (
				<div className="flex items-center pl-1.5 pr-2.5 gap-2 h-10 border-t">
					<InstanceSearchInput />
					<Display />
				</div>
			) : null}
			<LoadingIndicator />
		</div>
	);
}

function InstanceSearchInput() {
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const dataProvider = useDataProvider();
	const { n } = useSearch({ from: "/_context" });
	const filters = useFiltersValue({ onlyStatic: true });
	const { data: actors = [] } = useInfiniteQuery(
		dataProvider.actorsListQueryOptions({ n, filters }),
	);

	const [value, setValue] = useState("");
	const [isPending, setIsPending] = useState(false);

	const placeholder = `Search ${actors.length} instance${actors.length === 1 ? "" : "s"}…`;

	const handleSubmit = async () => {
		const trimmed = value.trim();
		if (!trimmed) return;
		setIsPending(true);
		try {
			await queryClient.fetchQuery(
				dataProvider.actorQueryOptions(trimmed),
			);
			void navigate({
				to: ".",
				search: (prev) => ({ ...prev, actorId: trimmed }),
			});
		} catch {
			void navigate({
				to: ".",
				search: (prev) => ({ ...prev, actorKey: trimmed }),
			});
		} finally {
			setValue("");
			setIsPending(false);
		}
	};

	return (
		<div className="relative flex-1 min-w-0">
			<Icon
				icon={faMagnifyingGlass}
				className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground size-3.5 pointer-events-none"
			/>
			<input
				type="text"
				value={value}
				placeholder={placeholder}
				disabled={isPending}
				onChange={(e) => setValue(e.target.value)}
				onKeyDown={(e) => {
					if (e.key === "Enter") void handleSubmit();
					if (e.key === "Escape") setValue("");
				}}
				className="w-full h-7 rounded-md bg-foreground/[0.04] text-xs pl-8 pr-2 placeholder:text-muted-foreground focus:outline-none focus:bg-foreground/[0.06] transition-colors"
			/>
		</div>
	);
}

function LoadingIndicator() {
	const n = useSearch({
		from: "/_context",
		select: (state) => state.n,
	});

	const filters = useFiltersValue({ onlyStatic: true });
	const { isLoading } = useInfiniteQuery(
		useDataProvider().actorsListQueryOptions({ n, filters }),
	);
	if (isLoading) {
		return <ShimmerLine className="bottom-0" />;
	}
	return null;
}

function List({
	viewportRef,
}: { viewportRef: RefObject<HTMLDivElement | null> }) {
	const filters = useFiltersValue({ onlyStatic: true });
	const { actorId, actorKey, n } = useSearch({
		from: "/_context",
	});
	const { data: actors = [] } = useInfiniteQuery(
		useDataProvider().actorsListQueryOptions({ n, filters }),
	);

	const rowVirtualizer = useVirtualizer({
		count: actors.length,
		getScrollElement: () => viewportRef.current,
		estimateSize: () => 36,
		overscan: 5,
	});

	return (
		<div
			className="relative w-full"
			style={{ height: rowVirtualizer.getTotalSize() }}
		>
			{rowVirtualizer.getVirtualItems().map((virtualItem) => {
				const actor = actors[virtualItem.index];
				return (
					<div
						key={actor.actorId}
						ref={rowVirtualizer.measureElement}
						data-index={virtualItem.index}
						className="absolute inset-x-0"
						style={{
							transform: `translateY(${virtualItem.start}px)`,
						}}
					>
						<ActorsListRow
							actorKey={actor.key}
							actorId={actor.actorId}
							isCurrent={
								actorId === actor.actorId ||
								actorKey === actor.key
							}
						/>
					</div>
				);
			})}
		</div>
	);
}

function Pagination() {
	const n = useSearch({
		from: "/_context",
		select: (state) => state.n,
	});
	const filters = useFiltersValue({ onlyStatic: true });
	const { hasNextPage, isFetchingNextPage, fetchNextPage, data } =
		useSuspenseInfiniteQuery(
			useDataProvider().actorsListPaginationQueryOptions({
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
		<div className="w-full">
			{Array(RECORDS_PER_PAGE)
				.fill(null)
				.map((_, i) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
					<ActorsListRowSkeleton key={i} />
				))}
		</div>
	);
}

const useRunnerConfigs = () => {
	const dataProvider = useDataProvider();
	const { data: runnerNamesCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerNamesQueryOptions(),
		select: (data) => data.pages.flatMap((page) => page.names).length,
	});

	const { data: runnerConfigsCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		select: (data) =>
			data.pages.flatMap((page) => Object.keys(page.runnerConfigs))
				.length,
	});

	return runnerConfigsCount + runnerNamesCount;
};

function EmptyState({ count }: { count: number }) {
	const navigate = useNavigate();
	const names = useSearch({
		from: "/_context",
		select: (state) => state.n,
	});
	const { copy } = useActorsView();
	const { remove, pick } = useActorsFilters();

	const dataProvider = useDataProvider();

	const { data: availableNamesCount = 0 } = useInfiniteQuery(
		dataProvider.buildsCountQueryOptions(),
	);

	const runnerConfigsCount = useRunnerConfigs();

	const filtersCount = useSearch({
		from: "/_context",
		select: (state) =>
			Object.values(pick(state, { onlyStatic: true })).length,
	});

	const clearFilters = () => {
		return navigate({
			to: ".",
			search: (prev) => ({
				...remove(prev || {}),
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
					runnerConfigsCount === 0 ? (
						<div className="px-4">
							<NoProvidersAlert />
						</div>
					) : (
						<div className="mt-12 mx-auto max-w-sm flex flex-col items-center gap-4 text-center">
							<div className="flex h-12 w-12 items-center justify-center rounded-full bg-foreground/[0.04] border border-foreground/10">
								<Icon
									icon={faActors}
									className="text-lg text-muted-foreground/80"
								/>
							</div>
							<div className="flex flex-col gap-1">
								<p className="text-sm font-medium text-foreground">
									{names && names.length > 0
										? "No instances yet"
										: "No actors yet"}
								</p>
								<SmallText className="text-muted-foreground my-0 leading-relaxed">
									{names && names.length > 0
										? "Instances of this actor will appear here as your code calls them."
										: copy.noActorsFound}
								</SmallText>
							</div>
							<CreateActorButton
								label={
									names && names.length > 0
										? "Create Instance"
										: undefined
								}
							/>
						</div>
					)
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
	const [value, setLs] = useLocalStorage(
		ls.actorsEphemeralFilters.key,
		() => ({ wakeOnSelect: { value: ["1"] } }),
		{
			deserializer: (value) => JSON.parse(value),
			serializer: (value) => JSON.stringify(value),
		},
	);

	return useCallback(
		(fnOrValue) => {
			if (typeof fnOrValue === "function") {
				navigate({
					to: ".",
					search: (old) => {
						const filters = pick(old || {}, { onlyStatic: true });
						const prev = remove(old || {});

						return {
							...prev,
							...Object.fromEntries(
								Object.entries(
									pick(fnOrValue(filters), {
										onlyStatic: true,
									}),
								).filter(
									([, filter]) => filter.value.length > 0,
								),
							),
						};
					},
				});

				setLs(pick(fnOrValue(value), { onlyEphemeral: true }));
			} else {
				navigate({
					to: ".",
					search: (value) => ({
						...remove(value || {}),
						...Object.fromEntries(
							Object.entries(
								pick(fnOrValue, { onlyStatic: true }),
							).filter(([, filter]) => filter.value.length > 0),
						),
					}),
				});
				setLs(pick(fnOrValue || {}, { onlyEphemeral: true }));
			}
		},
		[navigate, pick, remove, setLs, value],
	);
}

function Display() {
	const { definitions } = useActorsFilters();
	const filters = useFiltersValue();
	const onFiltersChange = useFiltersChangeCallback();

	return (
		<>
			<FiltersDisplay
				value={filters}
				definitions={definitions}
				onChange={onFiltersChange}
			/>
		</>
	);
}
