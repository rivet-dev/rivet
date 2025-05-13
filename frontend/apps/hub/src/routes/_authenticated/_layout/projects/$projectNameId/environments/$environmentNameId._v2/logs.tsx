import * as Layout from "@/domains/project/layouts/servers-layout";
import {
	type FunctionInvoke,
	logsAggregatedQueryOptions,
	projectActorsQueryOptions,
	routesQueryOptions,
} from "@/domains/project/queries";
import {
	cn,
	VirtualScrollArea,
	WithTooltip,
	FilterCreator,
	Button,
	toRecord,
	ToggleGroup,
	ToggleGroupItem,
	ShimmerLine,
	type FilterDefinitions,
	type OnFiltersChange,
	FilterValueSchema,
	FilterOp,
} from "@rivet-gg/components";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useQuery,
} from "@tanstack/react-query";
import {
	createFileRoute,
	Link,
	type ErrorComponentProps,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { format } from "date-fns";
import { forwardRef, useCallback, useMemo, useRef, useState } from "react";
import { z } from "zod";
import type { Virtualizer } from "@tanstack/react-virtual";
import {
	faAngleDown,
	faAngleUp,
	faFontCase,
	faKey,
	faRegex,
	faSignal,
	faSwap,
	Icon,
} from "@rivet-gg/icons";
import {
	ActorObjectInspector,
	ActorRegion,
	ConsoleMessageVariantIcon,
	getConsoleMessageVariant,
	useActorsView,
} from "@rivet-gg/components/actors";
import { ErrorComponent } from "@/components/error-component";
import { useDebounceCallback } from "usehooks-ts";

const searchSchema = z
	.object({
		level: FilterValueSchema,
		routeId: FilterValueSchema,
		actorId: FilterValueSchema,

		search: z.string().optional(),
		flags: z.array(z.enum(["case-sensitive", "regex"])).optional(),
	})
	.strip();

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2/logs",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "v2",
	},
	component: ProjectFunctionsRoute,
	pendingComponent: Layout.Content.Skeleton,
	errorComponent(props: ErrorComponentProps) {
		return (
			<div className="p-4">
				<div className="max-w-5xl mx-auto">
					<ErrorComponent {...props} />
				</div>
			</div>
		);
	},
});

function ProjectFunctionsRoute() {
	const { environmentNameId, projectNameId } = Route.useParams();

	const navigate = Route.useNavigate();
	const { flags, search, level, routeId, actorId } = Route.useSearch();

	const filters = { level, routeId, actorId };

	usePrefetchInfiniteQuery({
		...projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			includeDestroyed: true,
			tags: {},
		}),
		pages: 10,
	});

	const { data: actors } = useInfiniteQuery({
		...projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			includeDestroyed: true,
			tags: {},
		}),
	});

	const { data: rows, isLoading: isLoadingLogs } = useQuery(
		logsAggregatedQueryOptions({
			projectNameId,
			environmentNameId,
			search: search
				? {
						text: decodeURIComponent(search),
						caseSensitive:
							flags?.includes("case-sensitive") ?? false,
						enableRegex: flags?.includes("regex") ?? false,
					}
				: undefined,
		}),
	);

	const { data: routes } = useQuery(
		routesQueryOptions({
			projectNameId,
			environmentNameId,
		}),
	);

	const searchInputRef = useRef<HTMLInputElement>(null);

	const onFiltersChange: OnFiltersChange = useCallback(
		(fnOrValue) => {
			if (typeof fnOrValue === "function") {
				navigate({
					search: ({ search, flags, ...filters }) => ({
						search,
						flags,
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
						search: value.search,
						flags: value.flags,
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

	const viewportRef = useRef<HTMLDivElement>(null);
	const virtualizerRef = useRef<Virtualizer<HTMLDivElement, Element>>(null);

	const definitions = useMemo(
		() =>
			({
				level: {
					label: "Level",
					icon: faSignal,
					type: "select",
					options: [
						{ label: "Info", value: "info" },
						{ label: "Warning", value: "warning" },
						{ label: "Error", value: "error" },
					],
				},
				routeId: {
					label: "Route",
					type: "select",
					icon: faSwap,
					options:
						routes?.map((route) => ({
							label: `${route.hostname}${route.path}${route.routeSubpaths ? "/*" : ""}`,
							value: route.id,
						})) ?? [],
				},
				actorId: {
					label: "Instance",
					type: "select",
					icon: faKey,
					options:
						actors?.map((actor) => {
							const name = toRecord(actor.tags).name as string;
							return {
								label: (
									<div className="flex items-center gap-1">
										<ActorRegion
											regionId={actor.region}
											className="justify-start"
										/>

										{name ? (
											<span>
												{name}{" "}
												<span>
													({actor.id.split("-")[0]})
												</span>
											</span>
										) : (
											actor.id.split("-")[0]
										)}
									</div>
								),
								value: actor.id,
							};
						}) ?? [],
				},
			}) satisfies FilterDefinitions,
		[actors, routes],
	);

	// filter all rows by filters
	const filteredRows =
		rows?.filter((row) => {
			const satisfiesFilters = Object.entries(filters).every(
				([defId, filter]) => {
					if (!filter) return true;
					const { operator, value } = filter;
					const def = definitions[defId as keyof typeof definitions];
					if (!def || value.length === 0) return true;

					if (defId === "level") {
						if (operator === FilterOp.EQUAL) {
							return value.includes(row.level as string);
						}
						if (operator === FilterOp.NOT_EQUAL) {
							return value.length === 1
								? row.level !== value[0]
								: !value.includes(row.level as string);
						}
					}

					if (defId === "routeId") {
						const route = routes?.filter(
							(route) => value.includes(route.id) && !!route,
						);
						const actor = actors?.find(
							(actor) => actor.id === row.actorId,
						);

						if (!route || !actor) return true;

						if (operator === FilterOp.EQUAL) {
							return route.some((r) => {
								return Object.entries(
									r.target.actors?.selectorTags || {},
								).some(([key, value]) => {
									return toRecord(actor.tags)[key] === value;
								});
							});
						}
						if (operator === FilterOp.NOT_EQUAL) {
							return route.every((r) => {
								return Object.entries(
									r.target.actors?.selectorTags || {},
								).every(([key, value]) => {
									return toRecord(actor.tags)[key] !== value;
								});
							});
						}
					}

					if (defId === "actorId") {
						const actor = actors?.find(
							(actor) => actor.id === row.actorId,
						);
						if (!actor) return true;

						if (operator === FilterOp.EQUAL) {
							return value.includes(actor.id);
						}
						if (operator === FilterOp.NOT_EQUAL) {
							return value.length === 1
								? actor.id !== value[0]
								: !value.includes(actor.id);
						}
					}
				},
			);

			return (
				satisfiesFilters &&
				row.line.toLowerCase().includes(search || "")
			);
		}) ?? [];

	const setSearch = useDebounceCallback((search) => {
		navigate({
			search: (value) => ({
				...value,
				search,
			}),
		});
	}, 500);

	const [expanded, setExpanded] = useState(() => [] as string[]);

	return (
		<div className="flex flex-col max-w-full max-h-full w-full h-full bg-card relative">
			<div className="flex px-2 w-full border-b sticky top-0 min-h-[42px]">
				<input
					ref={searchInputRef}
					type="text"
					placeholder="Search..."
					className="min-w-24 h-full rounded-md px-2 text-xs bg-card outline-none placeholder:text-muted-foreground text-foreground"
					defaultValue={search}
					onChange={(e) => setSearch(e.target.value)}
				/>
				<ToggleGroup
					type="multiple"
					variant="outline"
					size="xs"
					className="mr-2 gap-0"
					value={flags}
					onValueChange={(flags) => {
						navigate({
							search: (value) => ({
								...value,
								flags,
							}),
						});
					}}
				>
					<ToggleGroupItem
						value="case-sensitive"
						className="text-xs border border-r-0 rounded-se-none rounded-ee-none"
					>
						<Icon icon={faFontCase} />
					</ToggleGroupItem>
					<ToggleGroupItem
						value="regex"
						className=" text-xs border rounded-es-none rounded-ss-none"
					>
						<Icon icon={faRegex} />
					</ToggleGroupItem>
				</ToggleGroup>
				<div className="h-full border-l mr-2" />
				<FilterCreator
					value={filters}
					onChange={onFiltersChange}
					definitions={definitions}
				/>
			</div>
			<div className="flex flex-1 min-h-0 max-h-full">
				<VirtualScrollArea
					className="w-full h-full"
					virtualizerRef={virtualizerRef}
					viewportRef={viewportRef}
					count={filteredRows?.length || 0}
					getRowData={(index) => ({
						...filteredRows[index],
						isExpanded: expanded.includes(filteredRows[index].id),
						expand: () =>
							setExpanded((prev) => {
								if (prev.includes(filteredRows[index].id)) {
									return prev.filter(
										(id) => id !== filteredRows[index].id,
									);
								}
								return [...prev, filteredRows[index].id];
							}),
					})}
					estimateSize={() => 28}
					row={FunctionRow}
				>
					{isLoadingLogs ? (
						<ShimmerLine className="sticky top-0" />
					) : null}
					{!isLoadingLogs && filteredRows?.length === 0 ? (
						<div className="flex items-center justify-center w-full h-full gap-2 flex-col py-6">
							<p className="text-muted-foreground text-sm">
								No logs found.
							</p>

							{Object.values(filters).length > 0 ||
							(search?.length || 0) > 0 ? (
								<Button
									variant="outline"
									className="text-sm ml-2"
									onClick={() => {
										if (searchInputRef.current) {
											searchInputRef.current.value = "";
										}
										navigate({
											search: (value) => ({
												...value,
												filters: [],
												search: "",
											}),
										});
									}}
								>
									Clear filters
								</Button>
							) : null}
						</div>
					) : null}
				</VirtualScrollArea>
			</div>
		</div>
	);
}

const FunctionRow = forwardRef<
	HTMLButtonElement,
	FunctionInvoke & {
		isExpanded: boolean;
		expand: () => void;
		className?: string;
	}
>(
	(
		{
			id,
			timestamp,
			message,
			line,
			properties,
			level,
			actorId,
			actorName,
			regionId,
			isFormatted,
			isExpanded,
			actorTags,
			expand,
			...props
		},
		ref,
	) => {
		const { copy } = useActorsView();
		return (
			<button
				ref={ref}
				{...props}
				onClick={() => (isFormatted ? expand() : null)}
				type="button"
				data-open={isExpanded}
				className={cn(
					"w-full flex-1 min-h-0 border-b flex-col text-left flex pl-3 pr-5 py-1",
					isFormatted
						? "cursor-pointer outline -outline-offset-2 outline-1 outline-transparent hover:bg-muted hover:text-muted-foreground focus-within:bg-muted data-[open=true]:bg-accent data-[open=true]:text-accent-foreground transition-colors"
						: "cursor-default",
					getConsoleMessageVariant(level),
					props.className,
				)}
			>
				<div className="flex items-center justify-center gap-2 text-xs font-mono-console whitespace-pre-wrap">
					{isFormatted ? (
						<Icon
							icon={isExpanded ? faAngleUp : faAngleDown}
							className="text-foreground/30 w-[11px] h-auto"
						/>
					) : (
						<ConsoleMessageVariantIcon
							variant={level}
							className="text-xs w-[11px] h-auto  opacity-60"
						/>
					)}
					<div className="min-h-4 text-foreground/30 flex-shrink-0">
						{timestamp
							? format(timestamp, "LLL dd HH:mm:ss").toUpperCase()
							: null}
					</div>
					<WithTooltip
						trigger={
							<ActorBadge
								actorName={actorName}
								actorId={actorId}
								actorTags={actorTags}
								regionId={regionId}
							/>
						}
						content={copy.goToActor}
					/>
					<div className="pl-2 min-h-4 flex-1 break-words min-w-0">
						{line}
					</div>
				</div>
				{isExpanded && isFormatted ? (
					// biome-ignore lint/a11y/useKeyWithClickEvents: we prevent default click
					<div onClick={(e) => e.stopPropagation()}>
						<ActorObjectInspector
							data={properties}
							expandPaths={["$"]}
						/>
					</div>
				) : null}
			</button>
		);
	},
);

const ActorBadge = forwardRef<
	HTMLButtonElement,
	{
		actorId: string;
		actorName: string;
		regionId: string;
		actorTags: Record<string, unknown>;
	}
>(({ actorId, actorName, regionId, actorTags, ...props }, ref) => {
	return (
		<Button ref={ref} variant="ghost" size="xs" asChild {...props}>
			<Link
				to={
					actorTags.framework === "actor-core"
						? "/projects/$projectNameId/environments/$environmentNameId/actors"
						: actorTags.type === "function"
							? "/projects/$projectNameId/environments/$environmentNameId/functions"
							: "/projects/$projectNameId/environments/$environmentNameId/containers"
				}
				params={Route.useParams()}
				search={{ actorId }}
			>
				<ActorRegion regionId={regionId} className="justify-start" />
				<span>{actorName}</span>
			</Link>
		</Button>
	);
});
