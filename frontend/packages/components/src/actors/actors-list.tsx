import {
	Button,
	Checkbox,
	CommandGroup,
	CommandItem,
	cn,
	createFiltersPicker,
	createFiltersSchema,
	DocsSheet,
	FilterCreator,
	type FilterDefinitions,
	FilterOp,
	type OnFiltersChange,
	type OptionsProviderProps,
	ScrollArea,
	ShimmerLine,
	SmallText,
} from "@rivet-gg/components";
import {
	faActors,
	faCalendarCircleMinus,
	faCalendarCirclePlus,
	faCalendarMinus,
	faCalendarPlus,
	faCode,
	faGlobe,
	faReact,
	faSignalBars,
	faTag,
	faTs,
	Icon,
} from "@rivet-gg/icons";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useAtomValue, useSetAtom } from "jotai";
import { useCallback, useMemo } from "react";
import {
	actorFiltersAtom,
	actorFiltersCountAtom,
	actorRegionsAtom,
	actorsAtomsAtom,
	actorsPaginationAtom,
	actorsQueryAtom,
	actorTagsAtom,
	filteredActorsCountAtom,
} from "./actor-context";
import { ActorStatus } from "./actor-status";
import type { ActorStatus as ActorStatusType } from "./actor-status-indicator";
import { ActorTag } from "./actor-tags";
import { ActorsListRow } from "./actors-list-row";
import { useActorsView } from "./actors-view-context-provider";
import { CreateActorButton } from "./create-actor-button";
import { GoToActorButton } from "./go-to-actor-button";

export function ActorsList() {
	return (
		<>
			<ScrollArea className="w-full @container/main">
				<div className="grid grid-cols-[2rem_1rem_1fr_1fr_1fr_1fr] @lg/main:grid-cols-[2rem_min-content_min-content_minmax(1rem,2fr)_minmax(min-content,1fr)_minmax(min-content,1fr)] items-center justify-center gap-x-4 w-full min-w-[450px] @container/table">
					<div className="grid grid-cols-subgrid col-span-full sticky top-0 z-[1] bg-card">
						<div className="col-span-full border-b justify-between flex px-2 py-2 gap-1 relative h-[45px]">
							<Filters />
							<div className="flex gap-1">
								<GoToActorButton />
								<CreateActorButton />
							</div>
							<LoadingIndicator />
						</div>
						<div className="grid grid-cols-subgrid col-span-full font-semibold text-sm px-1 pr-4 h-[45px] items-center  border-b">
							<div />
							<div>
								<span className="hidden  @[500px]/table:inline">
									Region
								</span>
								<span className="@[500px]/table:hidden">
									<Icon icon={faGlobe} />
								</span>
							</div>
							<div>ID</div>
							<div>Tags</div>
							<div>
								<span className="hidden @[500px]/table:inline">
									Created
								</span>
								<span className="@[500px]/table:hidden">
									<Icon icon={faCalendarPlus} />
								</span>
							</div>
							<div>
								<span className="hidden @[500px]/table:inline">
									Destroyed
								</span>
								<span className="@[500px]/table:hidden">
									<Icon icon={faCalendarMinus} />
								</span>
							</div>
						</div>
					</div>
					<List />
					<Pagination />
				</div>
			</ScrollArea>
		</>
	);
}

function LoadingIndicator() {
	const state = useAtomValue(actorsQueryAtom);
	if (state.isLoading) {
		return <ShimmerLine className="bottom-0" />;
	}
	return null;
}

function List() {
	const actors = useAtomValue(actorsAtomsAtom);
	return (
		<>
			{actors.map((actor) => (
				<ActorsListRow key={`${actor}`} actor={actor} />
			))}
		</>
	);
}

function Pagination() {
	const { hasNextPage, isFetchingNextPage, fetchNextPage } =
		useAtomValue(actorsPaginationAtom);

	if (hasNextPage) {
		return (
			<div className="col-span-full flex w-full justify-center py-4">
				<Button
					variant="outline"
					mx="4"
					isLoading={isFetchingNextPage}
					onClick={() => fetchNextPage()}
				>
					Load more
				</Button>
			</div>
		);
	}

	return <EmptyState />;
}

function EmptyState() {
	const count = useAtomValue(filteredActorsCountAtom);
	const filtersCount = useAtomValue(actorFiltersCountAtom);
	const setFilters = useSetAtom(actorFiltersAtom);
	const { copy } = useActorsView();

	return (
		<div className=" col-span-full my-4 flex flex-col items-center gap-2 justify-center">
			{count === 0 ? (
				filtersCount === 0 ? (
					<div className="gap-2 flex flex-col items-center justify-center">
						<Icon icon={faActors} className="text-4xl mb-2 mt-8" />
						<SmallText className="text-center">
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
									path="https://rivetkit.org/actors/quickstart-backend"
									title="Node.js & Bun Quick Start"
								>
									<Button
										variant="outline"
										size="sm"
										startIcon={<Icon icon={faTs} />}
									>
										TypeScript
									</Button>
								</DocsSheet>
								<DocsSheet
									path="https://rivetkit.org/actors/quickstart-frontend"
									title="React Quick Start"
								>
									<Button
										variant="outline"
										size="sm"
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
						<Button
							variant="outline"
							mx="4"
							onClick={() =>
								setFilters({
									tags: undefined,
									region: undefined,
									createdAt: undefined,
									destroyedAt: undefined,
									status: undefined,
									devMode: undefined,
								})
							}
						>
							Clear filters
						</Button>
					</>
				)
			) : (
				<SmallText className="text-foreground text-center">
					{copy.noMoreActors}
				</SmallText>
			)}
		</div>
	);
}

const FILTER_DEFINITIONS = {
	tags: {
		type: "select",
		label: "Tags",
		icon: faTag,
		options: TagsOptions,
		operators: {
			[FilterOp.EQUAL]: "is one of",
			[FilterOp.NOT_EQUAL]: "is not one of",
		},
	},
	createdAt: {
		type: "date",
		label: "Created",
		icon: faCalendarCirclePlus,
	},
	destroyedAt: {
		type: "date",
		label: "Destroyed",
		icon: faCalendarCircleMinus,
	},
	status: {
		type: "select",
		label: "Status",
		icon: faSignalBars,
		options: StatusOptions,
		display: ({ value }) => {
			if (value.length > 1) {
				return <span>{value.length} statuses</span>;
			}
			return (
				<ActorStatus
					className="border-0 p-0"
					status={value[0] as ActorStatusType}
				/>
			);
		},
	},
	region: {
		type: "select",
		label: "Region",
		icon: faGlobe,
		options: RegionOptions,
		display: ({ value }) => {
			if (value.length > 1) {
				return <span>{value.length} regions</span>;
			}
			const region = useAtomValue(actorRegionsAtom).find(
				(region) => region.id === value[0],
			);
			return <span>{region?.name}</span>;
		},
		operators: {
			[FilterOp.EQUAL]: "is one of",
			[FilterOp.NOT_EQUAL]: "is not one of",
		},
	},
	devMode: {
		type: "boolean",
		label: "Show hidden actors",
		icon: faCode,
	},
} satisfies FilterDefinitions;

export const ActorsListFiltersSchema = createFiltersSchema(FILTER_DEFINITIONS);

export const pickActorListFilters = createFiltersPicker(FILTER_DEFINITIONS);

function Filters() {
	const navigate = useNavigate();
	const filters = useSearch({ strict: false });

	const onFiltersChange: OnFiltersChange = useCallback(
		(fnOrValue) => {
			if (typeof fnOrValue === "function") {
				navigate({
					search: ({ actorId, tab, ...filters }) => ({
						actorId,
						tab,
						...Object.fromEntries(
							Object.entries(fnOrValue(filters)).filter(
								([, filter]) => filter.value.length > 0,
							),
						),
					}),
				});
			} else {
				navigate({
					search: (value) => ({
						actorId: value.actorId,
						tab: value.tab,
						...Object.fromEntries(
							Object.entries(fnOrValue).filter(
								([, filter]) => filter.value.length > 0,
							),
						),
					}),
				});
			}
		},
		[navigate],
	);

	const { copy } = useActorsView();

	const filtersDefs = useMemo(() => {
		return {
			...FILTER_DEFINITIONS,
			devMode: {
				...FILTER_DEFINITIONS.devMode,
				hidden: true,
				label: copy.showHiddenActors,
			},
		};
	}, [copy.showHiddenActors]);

	return (
		<FilterCreator
			value={filters}
			onChange={onFiltersChange}
			definitions={filtersDefs}
		/>
	);
}

function TagsOptions({ onSelect, value: filterValue }: OptionsProviderProps) {
	const tags = useAtomValue(actorTagsAtom);

	const values = filterValue.map((filter) => filter.split("="));

	return (
		<CommandGroup>
			{tags.map(({ key, value }) => {
				const isSelected = values.some(
					([filterKey, filterValue]) =>
						filterKey === key && filterValue === value,
				);
				return (
					<CommandItem
						key={`${key}-${value}`}
						className="group flex gap-2 items-center"
						value={`${key}=${value}`}
						onSelect={() => {
							if (isSelected) {
								onSelect(
									values
										.filter(
											([filterKey, filterValue]) =>
												filterKey !== key ||
												filterValue !== value,
										)
										.map((pair) => pair.join("=")),
									{ closeAfter: true },
								);
								return;
							}
							onSelect([...filterValue, `${key}=${value}`], {
								closeAfter: true,
							});
						}}
					>
						<Checkbox
							checked={isSelected}
							className={cn({
								"opacity-0 group-data-[selected=true]:opacity-100":
									!isSelected,
							})}
						/>
						<ActorTag className="text-foreground">
							<span className="break-all">
								{key}={value}
							</span>
						</ActorTag>
					</CommandItem>
				);
			})}
		</CommandGroup>
	);
}

function StatusOptions({ onSelect, value: filterValue }: OptionsProviderProps) {
	return (
		<CommandGroup>
			{["running", "starting", "crashed", "stopped"].map((key) => {
				const isSelected = filterValue.some((val) => val === key);
				return (
					<CommandItem
						key={key}
						className="group flex gap-2 items-center"
						value={key}
						onSelect={() => {
							if (isSelected) {
								onSelect(
									filterValue.filter(
										(filterKey) => filterKey !== key,
									),
									{ closeAfter: true },
								);
								return;
							}

							onSelect([...filterValue, key], {
								closeAfter: true,
							});
						}}
					>
						<Checkbox
							checked={isSelected}
							className={cn({
								"opacity-0 group-data-[selected=true]:opacity-100":
									!isSelected,
							})}
						/>
						<ActorStatus status={key as ActorStatusType} />
					</CommandItem>
				);
			})}
		</CommandGroup>
	);
}

function RegionOptions({ onSelect, value: filterValue }: OptionsProviderProps) {
	const regions = useAtomValue(actorRegionsAtom);
	return (
		<CommandGroup>
			{regions.map(({ id, name }) => {
				const isSelected = filterValue.some((val) => val === id);
				return (
					<CommandItem
						key={id}
						className="group flex gap-2 items-center"
						value={id}
						onSelect={() => {
							if (isSelected) {
								onSelect(
									filterValue.filter(
										(filterKey) => filterKey !== id,
									),
									{ closeAfter: true },
								);
								return;
							}

							onSelect([...filterValue, id], {
								closeAfter: true,
							});
						}}
					>
						<Checkbox
							checked={isSelected}
							className={cn({
								"opacity-0 group-data-[selected=true]:opacity-100":
									!isSelected,
							})}
						/>
						<SmallText>{name}</SmallText>
					</CommandItem>
				);
			})}
		</CommandGroup>
	);
}
