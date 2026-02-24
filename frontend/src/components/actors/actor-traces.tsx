import { faChevronDown, faSpinnerThird, Icon } from "@rivet-gg/icons";
import type {
	OtlpAnyValue,
	OtlpExportTraceServiceRequestJson,
	OtlpKeyValue,
	OtlpSpan,
	OtlpSpanEvent,
} from "@rivetkit/traces";
import { readRangeWireToOtlp } from "@rivetkit/traces/otlp";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { type ReactElement, useMemo, useRef, useState } from "react";
import type { DateRange } from "../datepicker";
import { RangeDatePicker } from "../datepicker";
import { cn } from "../lib/utils";
import { Button } from "../ui/button";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import { useActorInspector } from "./actor-inspector-context";
import { ActorObjectInspector } from "./console/actor-inspector";
import type { ActorId } from "./queries";
import { SpanSidebar } from "./traces/span-sidebar";
import { TimelineView } from "./traces/traces-timeline";
import type { SpanNode, TraceItem } from "./traces/types";

const PRESET_OPTIONS = [
	{ label: "5 min", ms: 5 * 60 * 1000 },
	{ label: "15 min", ms: 15 * 60 * 1000 },
	{ label: "30 min", ms: 30 * 60 * 1000 },
	{ label: "1 hour", ms: 60 * 60 * 1000 },
	{ label: "3 hours", ms: 3 * 60 * 60 * 1000 },
	{ label: "6 hours", ms: 6 * 60 * 60 * 1000 },
	{ label: "12 hours", ms: 12 * 60 * 60 * 1000 },
	{ label: "24 hours", ms: 24 * 60 * 60 * 1000 },
	{ label: "2 days", ms: 2 * 24 * 60 * 60 * 1000 },
	{ label: "7 days", ms: 7 * 24 * 60 * 60 * 1000 },
	{ label: "14 days", ms: 14 * 24 * 60 * 60 * 1000 },
];

const DEFAULT_PRESET_MS = 30 * 60 * 1000;
const GAP_THRESHOLD_MS = 500;
const DEFAULT_LIMIT = 1000;

type ViewType = "list" | "timeline";

export function ActorTraces({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();
	const [viewType, setViewType] = useState<ViewType>("list");
	const [isLive, setIsLive] = useState(true);
	const [presetMs, setPresetMs] = useState(DEFAULT_PRESET_MS);
	const [customRange, setCustomRange] = useState<DateRange | undefined>(
		() => {
			const now = Date.now();
			return {
				from: new Date(now - DEFAULT_PRESET_MS),
				to: new Date(now),
			};
		},
	);

	const query = useQuery({
		queryKey: [
			"actor",
			actorId,
			"traces",
			isLive,
			presetMs,
			customRange?.from?.getTime(),
			customRange?.to?.getTime(),
			DEFAULT_LIMIT,
		],
		queryFn: async () => {
			const now = Date.now();
			const rangeStartMs = isLive
				? now - presetMs
				: (customRange?.from?.getTime() ?? now - presetMs);
			const rangeEndMs = isLive
				? now
				: (customRange?.to?.getTime() ?? now);
			const startMs = Math.min(rangeStartMs, rangeEndMs);
			const endMs = Math.max(rangeStartMs, rangeEndMs);
			return inspector.api.getTraces({
				startMs,
				endMs,
				limit: DEFAULT_LIMIT,
			});
		},
		enabled:
			inspector.isInspectorAvailable &&
			inspector.features.traces.supported,
		refetchInterval: isLive ? 1000 : false,
		staleTime: 0,
	});

	const queryResult = useMemo(() => {
		if (!query.data) {
			return null;
		}
		return readRangeWireToOtlp(query.data);
	}, [query.data]);

	const traceTree = useMemo(() => {
		if (!queryResult) {
			return [];
		}
		const spans = extractSpans(queryResult.otlp);
		return buildSpanTree(spans);
	}, [queryResult]);

	const nowMs = Date.now();
	const nowNs = BigInt(nowMs) * 1_000_000n;
	const liveRange = {
		from: new Date(nowMs - presetMs),
		to: new Date(nowMs),
	};
	const displayRange = isLive ? liveRange : customRange;

	const onPresetChange = (value: string) => {
		const ms = Number(value);
		if (Number.isNaN(ms)) {
			return;
		}
		setPresetMs(ms);
		if (!isLive) {
			const now = Date.now();
			setCustomRange({
				from: new Date(now - ms),
				to: new Date(now),
			});
		}
	};

	if (query.isLoading) {
		return (
			<div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Loading traces...
			</div>
		);
	}

	if (query.isError) {
		return (
			<div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
				Traces are currently unavailable.
			</div>
		);
	}

	return (
		<div className="flex flex-col h-full min-h-0">
			<div className="border-b px-3 py-2 flex flex-wrap gap-2 items-center">
				<Button
					variant={isLive ? "default" : "outline"}
					size="sm"
					onClick={() =>
						setIsLive((value) => {
							if (value) {
								const now = Date.now();
								setCustomRange({
									from: new Date(now - presetMs),
									to: new Date(now),
								});
							}
							return !value;
						})
					}
				>
					Live
				</Button>
				<Select value={`${presetMs}`} onValueChange={onPresetChange}>
					<SelectTrigger className="h-8 text-xs w-[150px]">
						<SelectValue placeholder="Time range" />
					</SelectTrigger>
					<SelectContent>
						{PRESET_OPTIONS.map((option) => (
							<SelectItem
								key={option.label}
								value={`${option.ms}`}
							>
								{option.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				<div
					className={cn(
						"flex items-center",
						isLive && "pointer-events-none opacity-60",
					)}
				>
					<RangeDatePicker
						date={displayRange}
						onDateChange={(range) => {
							setCustomRange(range);
							setIsLive(false);
						}}
					/>
				</div>
				<div className="text-xs text-muted-foreground">
					{displayRange?.from && displayRange?.to
						? `${format(displayRange.from, "PPpp")} → ${format(
								displayRange.to,
								"PPpp",
							)}`
						: "Select a time range"}
				</div>
				<div className="flex-1" />
				{/* <ToggleGroup
					type="single"
					value={viewType}
					onValueChange={(value) =>
						value && setViewType(value as ViewType)
					}
					size="sm"
				>
					<ToggleGroupItem value="list" aria-label="List view">
						<Icon icon={faList} className="size-4" />
					</ToggleGroupItem>
					<ToggleGroupItem
						value="timeline"
						aria-label="Timeline view"
					>
						<Icon icon={faTimeline} className="size-4" />
					</ToggleGroupItem>
				</ToggleGroup> */}
			</div>

			{viewType === "list" ? (
				<div className="flex-1 min-h-0 overflow-y-auto px-3 py-3 space-y-2">
					{traceTree.length === 0 ? (
						<div className="flex items-center justify-center text-sm text-muted-foreground">
							No traces found for this time range.
						</div>
					) : (
						renderItemsWithGaps(
							traceTree.map((node) => ({
								type: "span" as const,
								node,
								timeNs: node.startNs,
							})),
							0,
							nowNs,
						)
					)}
					{queryResult?.clamped ? (
						<div className="text-xs text-muted-foreground">
							Results truncated at {DEFAULT_LIMIT} spans.
						</div>
					) : null}
				</div>
			) : (
				<div className="flex min-h-0 flex-1">
					<SpanSidebar
						spans={traceTree}
						selectedSpanId={null}
						selectedEventIndex={null}
						onSelectSpan={() => {}}
						onSelectEvent={() => {}}
					/>
					<div className="flex-1 flex flex-col min-w-0">
						<TimelineView
							spans={traceTree}
							selectedSpanId={null}
							selectedEventIndex={null}
							onSelectSpan={() => {}}
							onSelectEvent={() => {}}
						/>
					</div>
				</div>
			)}
		</div>
	);
}

function renderItemsWithGaps(
	items: TraceItem[],
	depth: number,
	nowNs: bigint,
): ReactElement[] {
	const sorted = [...items].sort((a, b) =>
		a.timeNs < b.timeNs ? -1 : a.timeNs > b.timeNs ? 1 : 0,
	);
	const nodes: ReactElement[] = [];
	for (let i = 0; i < sorted.length; i++) {
		const item = sorted[i];
		if (i > 0) {
			const prev = sorted[i - 1];
			const gapMs = nsToMs(item.timeNs - prev.timeNs);
			if (gapMs > GAP_THRESHOLD_MS) {
				nodes.push(
					<GapMarker
						key={`gap-${depth}-${i}-${gapMs}`}
						ms={gapMs}
						depth={depth}
					/>,
				);
			}
		}
		if (item.type === "span") {
			nodes.push(
				<TraceSpanItem
					key={item.node.span.spanId}
					node={item.node}
					depth={depth}
					nowNs={nowNs}
				/>,
			);
		} else {
			nodes.push(
				<TraceEventRow
					key={`${item.event.name}-${item.event.timeUnixNano}`}
					event={item.event}
					depth={depth}
				/>,
			);
		}
	}
	return nodes;
}

function TraceSpanItem({
	node,
	depth,
	nowNs,
}: {
	node: SpanNode;
	depth: number;
	nowNs: bigint;
}) {
	const [isOpen, setIsOpen] = useState(false);
	const startMs = nsToMs(node.startNs);
	const endNs = node.endNs ?? nowNs;
	const durationMs = Math.max(0, nsToMs(endNs - node.startNs));
	const subeventCount = node.children.length + node.events.length;
	const subeventLabel = subeventCount === 1 ? "subevent" : "subevents";
	const isActive = node.endNs == null;
	const items = useMemo(() => {
		const result: TraceItem[] = [];
		for (const child of node.children) {
			result.push({ type: "span", node: child, timeNs: child.startNs });
		}
		for (const event of node.events) {
			result.push({
				type: "event",
				event,
				timeNs: BigInt(event.timeUnixNano),
			});
		}
		return result;
	}, [node]);

	const details = buildSpanDetails(node.span);

	return (
		<div
			className={cn(
				"border rounded-md bg-background",
				depth > 0 && "ml-4",
			)}
		>
			<button
				type="button"
				onClick={() => setIsOpen((value) => !value)}
				className="w-full text-left px-4 py-3 flex items-center justify-between gap-4"
			>
				<div className="flex items-center gap-2 min-w-0">
					<Icon
						icon={faChevronDown}
						className={cn(
							"h-4 w-4 text-muted-foreground transition-transform",
							!isOpen && "-rotate-90",
						)}
					/>
					<div className="flex items-center gap-2 min-w-0">
						<span className="text-sm font-medium truncate">
							{node.span.name || "Unnamed span"}
						</span>
						{isActive ? (
							<Icon
								icon={faSpinnerThird}
								className="h-3 w-3 animate-spin text-muted-foreground"
							/>
						) : null}
					</div>
				</div>
				<div className="flex items-center gap-3 text-xs text-muted-foreground whitespace-nowrap">
					<span title={format(new Date(startMs), "PPpp")}>
						{format(new Date(startMs), "p")}
					</span>
					<span>{formatDuration(durationMs)}</span>
					<span>
						{subeventCount} {subeventLabel}
					</span>
				</div>
			</button>
			{isOpen ? (
				<div className="border-t px-4 py-3 space-y-3">
					{details ? (
						<div className="rounded-md border bg-muted/20 p-3">
							<div className="text-xs font-medium mb-2">
								Span details
							</div>
							<ActorObjectInspector data={details} />
						</div>
					) : null}
					{items.length === 0 ? (
						<div className="text-xs text-muted-foreground">
							No subevents.
						</div>
					) : (
						<div className="space-y-2">
							{renderItemsWithGaps(items, depth + 1, nowNs)}
						</div>
					)}
				</div>
			) : null}
		</div>
	);
}

function TraceEventRow({
	event,
	depth,
}: {
	event: OtlpSpanEvent;
	depth: number;
}) {
	const eventMs = nsToMs(BigInt(event.timeUnixNano));
	const attributes = otlpAttributesToObject(event.attributes);
	return (
		<div className={cn("rounded-md border px-4 py-3", depth > 0 && "ml-4")}>
			<div className="flex items-center justify-between gap-3">
				<div className="text-sm font-medium truncate">
					{event.name || "Event"}
				</div>
				<div className="text-xs text-muted-foreground whitespace-nowrap">
					{format(new Date(eventMs), "p")}
				</div>
			</div>
			{attributes ? (
				<div className="mt-2">
					<ActorObjectInspector data={attributes} />
				</div>
			) : null}
		</div>
	);
}

function GapMarker({ ms, depth }: { ms: number; depth: number }) {
	return (
		<div
			className={cn(
				"text-xs text-muted-foreground flex items-center gap-2 px-2",
				depth > 0 && "ml-4",
			)}
		>
			<div className="h-px flex-1 bg-border" />
			<span>{formatGap(ms)}</span>
			<div className="h-px flex-1 bg-border" />
		</div>
	);
}

function extractSpans(otlp: OtlpExportTraceServiceRequestJson): OtlpSpan[] {
	const spans: OtlpSpan[] = [];
	for (const resource of otlp.resourceSpans ?? []) {
		for (const scope of resource.scopeSpans ?? []) {
			spans.push(...(scope.spans ?? []));
		}
	}
	return spans;
}

function buildSpanTree(spans: OtlpSpan[]): SpanNode[] {
	const byId = new Map<string, SpanNode>();
	for (const span of spans) {
		byId.set(span.spanId, {
			span,
			timeUnixNano: BigInt(span.startTimeUnixNano),
			spanId: span.spanId,
			endTimeUnixNano: span.endTimeUnixNano,
			startTimeUnixNano: span.startTimeUnixNano,
			name: span.name,
			startNs: BigInt(span.startTimeUnixNano),
			endNs: span.endTimeUnixNano ? BigInt(span.endTimeUnixNano) : null,
			children: [],
			events: span.events ?? [],
		});
	}

	const roots: SpanNode[] = [];
	for (const node of byId.values()) {
		const parentId = node.span.parentSpanId;
		if (parentId && byId.has(parentId)) {
			byId.get(parentId)?.children.push(node);
		} else {
			roots.push(node);
		}
	}

	for (const node of byId.values()) {
		node.children.sort((a, b) =>
			a.startNs < b.startNs ? -1 : a.startNs > b.startNs ? 1 : 0,
		);
		node.events.sort((a, b) =>
			BigInt(a.timeUnixNano) < BigInt(b.timeUnixNano)
				? -1
				: BigInt(a.timeUnixNano) > BigInt(b.timeUnixNano)
					? 1
					: 0,
		);
	}

	roots.sort((a, b) => (a.startNs < b.startNs ? -1 : 1));
	return roots;
}

function otlpAttributesToObject(
	attributes?: OtlpKeyValue[],
): Record<string, unknown> | null {
	if (!attributes || attributes.length === 0) {
		return null;
	}
	const out: Record<string, unknown> = {};
	for (const entry of attributes) {
		if (!entry.key) {
			continue;
		}
		out[entry.key] = otlpAnyValueToJs(entry.value);
	}
	return out;
}

function otlpAnyValueToJs(value?: OtlpAnyValue): unknown {
	if (!value) {
		return null;
	}
	if (value.stringValue !== undefined) {
		return value.stringValue;
	}
	if (value.boolValue !== undefined) {
		return value.boolValue;
	}
	if (value.intValue !== undefined) {
		return value.intValue;
	}
	if (value.doubleValue !== undefined) {
		return value.doubleValue;
	}
	if (value.bytesValue !== undefined) {
		return value.bytesValue;
	}
	if (value.arrayValue?.values) {
		return value.arrayValue.values.map((item) => otlpAnyValueToJs(item));
	}
	if (value.kvlistValue?.values) {
		const obj: Record<string, unknown> = {};
		for (const entry of value.kvlistValue.values) {
			obj[entry.key] = otlpAnyValueToJs(entry.value);
		}
		return obj;
	}
	return null;
}

function nsToMs(ns: bigint): number {
	return Number(ns / 1_000_000n);
}

function formatDuration(ms: number): string {
	if (ms < 1000) {
		return `${Math.round(ms)}ms`;
	}
	const seconds = ms / 1000;
	if (seconds < 60) {
		return `${seconds < 10 ? seconds.toFixed(1) : Math.round(seconds)}s`;
	}
	const minutes = Math.floor(seconds / 60);
	const remSeconds = Math.round(seconds % 60);
	if (minutes < 60) {
		return `${minutes}m ${remSeconds}s`;
	}
	const hours = Math.floor(minutes / 60);
	const remMinutes = minutes % 60;
	if (hours < 24) {
		return `${hours}h ${remMinutes}m`;
	}
	const days = Math.floor(hours / 24);
	const remHours = hours % 24;
	return `${days}d ${remHours}h`;
}

function formatGap(ms: number): string {
	const seconds = ms / 1000;
	if (seconds < 60) {
		const value =
			seconds < 10 ? seconds.toFixed(1) : Math.round(seconds).toString();
		return `${value} ${Number(value) === 1 ? "second" : "seconds"}`;
	}
	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) {
		return `${minutes} ${minutes === 1 ? "minute" : "minutes"}`;
	}
	const hours = Math.floor(minutes / 60);
	if (hours < 24) {
		return `${hours} ${hours === 1 ? "hour" : "hours"}`;
	}
	const days = Math.floor(hours / 24);
	return `${days} ${days === 1 ? "day" : "days"}`;
}

interface FlatSpan {
	node: SpanNode;
	depth: number;
	row: number;
}

function flattenSpanTree(nodes: SpanNode[]): FlatSpan[] {
	const result: FlatSpan[] = [];
	let row = 0;

	function traverse(node: SpanNode, depth: number) {
		result.push({ node, depth, row: row++ });
		for (const child of node.children) {
			traverse(child, depth + 1);
		}
	}

	for (const node of nodes) {
		traverse(node, 0);
	}
	return result;
}

function getAllSpanNodes(nodes: SpanNode[]): SpanNode[] {
	const result: SpanNode[] = [];

	function traverse(node: SpanNode) {
		result.push(node);
		for (const child of node.children) {
			traverse(child);
		}
	}

	for (const node of nodes) {
		traverse(node);
	}
	return result;
}

interface TimelineGap {
	afterRow: number;
	gapMs: number;
	startNs: bigint;
	endNs: bigint;
}

const GAP_VISUAL_WIDTH_PERCENT = 2;

function TracesTimelineView({
	spans,
	nowNs,
	clamped,
	limit,
}: {
	spans: SpanNode[];
	nowNs: bigint;
	clamped?: boolean;
	limit: number;
}) {
	const [selectedSpanId, setSelectedSpanId] = useState<string | null>(null);
	const sidebarRef = useRef<HTMLDivElement>(null);
	const timelineRef = useRef<HTMLDivElement>(null);

	const flatSpans = useMemo(() => flattenSpanTree(spans), [spans]);
	const allSpans = useMemo(() => getAllSpanNodes(spans), [spans]);

	const { minTime, maxTime } = useMemo(() => {
		if (allSpans.length === 0) {
			return { minTime: 0n, maxTime: 0n };
		}

		const starts = allSpans.map((s) => s.startNs);
		const ends = allSpans.map((s) => s.endNs ?? nowNs);

		const min = starts.reduce((a, b) => (a < b ? a : b), starts[0]);
		const max = ends.reduce((a, b) => (a > b ? a : b), ends[0]);

		return { minTime: min, maxTime: max };
	}, [allSpans, nowNs]);

	const horizontalGaps = useMemo(() => {
		const result: TimelineGap[] = [];
		const sorted = [...allSpans].sort((a, b) =>
			a.startNs < b.startNs ? -1 : a.startNs > b.startNs ? 1 : 0,
		);

		let lastEndNs = minTime;
		for (const span of sorted) {
			const gapMs = nsToMs(span.startNs - lastEndNs);
			if (gapMs > GAP_THRESHOLD_MS) {
				result.push({
					afterRow: -1,
					gapMs,
					startNs: lastEndNs,
					endNs: span.startNs,
				});
			}
			const spanEnd = span.endNs ?? nowNs;
			if (spanEnd > lastEndNs) {
				lastEndNs = spanEnd;
			}
		}
		return result;
	}, [allSpans, minTime, nowNs]);

	const compressedDuration = useMemo(() => {
		const totalGap = horizontalGaps.reduce((sum, g) => sum + g.gapMs, 0);
		const fullDuration = nsToMs(maxTime - minTime);
		const gapVisualTime =
			(horizontalGaps.length * GAP_VISUAL_WIDTH_PERCENT * fullDuration) /
			100;
		const compressed = fullDuration - totalGap + gapVisualTime;
		return Math.max(compressed, 1);
	}, [horizontalGaps, minTime, maxTime]);

	const verticalGaps = useMemo(() => {
		const result: TimelineGap[] = [];
		const sorted = [...flatSpans].sort((a, b) =>
			a.node.startNs < b.node.startNs
				? -1
				: a.node.startNs > b.node.startNs
					? 1
					: 0,
		);

		for (let i = 1; i < sorted.length; i++) {
			const prev = sorted[i - 1];
			const curr = sorted[i];
			const gapMs = nsToMs(curr.node.startNs - prev.node.startNs);
			if (gapMs > GAP_THRESHOLD_MS) {
				result.push({
					afterRow: prev.row,
					gapMs,
					startNs: prev.node.startNs,
					endNs: curr.node.startNs,
				});
			}
		}
		return result;
	}, [flatSpans]);

	const rowHeight = 36;
	const rowGap = 4;
	const gapMarkerHeight = 24;

	const getRowTop = (row: number) => {
		let top = row * (rowHeight + rowGap);
		for (const gap of verticalGaps) {
			if (gap.afterRow < row) {
				top += gapMarkerHeight;
			}
		}
		return top;
	};

	const totalHeight = useMemo(() => {
		const baseHeight = flatSpans.length * (rowHeight + rowGap);
		const gapsHeight = verticalGaps.length * gapMarkerHeight;
		return baseHeight + gapsHeight;
	}, [flatSpans.length, verticalGaps.length]);

	const getCompressedPosition = (timeNs: bigint): number => {
		if (compressedDuration === 0) return 0;

		let position = nsToMs(timeNs - minTime);
		let gapsBefore = 0;

		for (const gap of horizontalGaps) {
			if (timeNs > gap.endNs) {
				position -= gap.gapMs;
				gapsBefore++;
			} else if (timeNs > gap.startNs) {
				const gapProgress = nsToMs(timeNs - gap.startNs) / gap.gapMs;
				const fullDuration = nsToMs(maxTime - minTime);
				position -= gap.gapMs;
				position +=
					(gapProgress * GAP_VISUAL_WIDTH_PERCENT * fullDuration) /
					100;
				gapsBefore++;
			}
		}

		const fullDuration = nsToMs(maxTime - minTime);
		position +=
			(gapsBefore * GAP_VISUAL_WIDTH_PERCENT * fullDuration) / 100;

		return (position / compressedDuration) * 100;
	};

	const getSpanStyle = (node: SpanNode) => {
		if (compressedDuration === 0) return { left: "0%", width: "100%" };

		const startPercent = getCompressedPosition(node.startNs);
		const endTime = node.endNs ?? nowNs;
		const endPercent = getCompressedPosition(endTime);
		const widthPercent = endPercent - startPercent;

		return {
			left: `${startPercent}%`,
			width: `${Math.max(widthPercent, 0.5)}%`,
		};
	};

	const selectedNode = useMemo(() => {
		if (!selectedSpanId) return null;
		return allSpans.find((s) => s.span.spanId === selectedSpanId) ?? null;
	}, [allSpans, selectedSpanId]);

	const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
		const scrollTop = e.currentTarget.scrollTop;
		if (sidebarRef.current && e.currentTarget === timelineRef.current) {
			sidebarRef.current.scrollTop = scrollTop;
		} else if (
			timelineRef.current &&
			e.currentTarget === sidebarRef.current
		) {
			timelineRef.current.scrollTop = scrollTop;
		}
	};

	if (spans.length === 0) {
		return (
			<div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
				No traces found for this time range.
			</div>
		);
	}

	return (
		<div className="flex-1 min-h-0 flex flex-col">
			<div className="flex-1 flex min-h-0">
				<div
					ref={sidebarRef}
					className="w-64 border-r overflow-y-auto shrink-0"
					onScroll={handleScroll}
				>
					<div className="sticky top-0 z-10 h-8 border-b bg-background/95 backdrop-blur px-3 flex items-center">
						<span className="text-xs font-medium text-muted-foreground">
							Spans
						</span>
					</div>
					<div
						className="relative"
						style={{ height: `${totalHeight}px` }}
					>
						{flatSpans.map(({ node, depth, row }) => {
							const isSelected =
								selectedSpanId === node.span.spanId;
							const isActive = node.endNs == null;
							const durationMs = node.endNs
								? nsToMs(node.endNs - node.startNs)
								: null;

							return (
								<button
									key={node.span.spanId}
									type="button"
									onClick={() =>
										setSelectedSpanId(
											isSelected
												? null
												: node.span.spanId,
										)
									}
									className={cn(
										"absolute left-0 right-0 flex items-center gap-2 px-3 py-1 text-left transition-colors",
										isSelected
											? "bg-primary/10"
											: "hover:bg-accent/50",
									)}
									style={{
										top: `${getRowTop(row)}px`,
										height: `${rowHeight}px`,
										paddingLeft: `${12 + depth * 12}px`,
									}}
								>
									{isActive && (
										<Icon
											icon={faSpinnerThird}
											className="h-3 w-3 animate-spin text-muted-foreground shrink-0"
										/>
									)}
									<span className="text-xs truncate flex-1">
										{node.span.name || "Unnamed span"}
									</span>
									{durationMs !== null && (
										<span className="text-[10px] text-muted-foreground shrink-0">
											{formatDuration(durationMs)}
										</span>
									)}
								</button>
							);
						})}
						{verticalGaps.map((gap) => (
							<div
								key={`gap-${gap.afterRow}`}
								className="absolute left-0 right-0 flex items-center gap-2 px-3"
								style={{
									top: `${getRowTop(gap.afterRow) + rowHeight + rowGap / 2}px`,
									height: `${gapMarkerHeight}px`,
								}}
							>
								<div className="h-px flex-1 bg-border" />
								<span className="text-[10px] text-muted-foreground">
									{formatGap(gap.gapMs)}
								</span>
								<div className="h-px flex-1 bg-border" />
							</div>
						))}
					</div>
				</div>

				<div
					ref={timelineRef}
					className="flex-1 overflow-auto"
					onScroll={handleScroll}
				>
					<div
						className="relative min-w-full"
						style={{ height: `${totalHeight + 32}px` }}
					>
						<TimelineScale
							totalDuration={compressedDuration}
							gaps={horizontalGaps}
							getCompressedPosition={getCompressedPosition}
						/>
						<div className="relative px-4">
							{flatSpans.map(({ node, depth, row }) => {
								const style = getSpanStyle(node);
								const isSelected =
									selectedSpanId === node.span.spanId;
								const durationMs = node.endNs
									? nsToMs(node.endNs - node.startNs)
									: null;
								const isActive = node.endNs == null;

								return (
									<div
										key={node.span.spanId}
										className="absolute"
										style={{
											top: `${getRowTop(row)}px`,
											left: style.left,
											width: style.width,
											height: `${rowHeight}px`,
											marginLeft: `${depth * 8}px`,
										}}
									>
										<button
											type="button"
											onClick={() =>
												setSelectedSpanId(
													isSelected
														? null
														: node.span.spanId,
												)
											}
											className={cn(
												"w-full h-full rounded border transition-all flex items-center px-2 gap-2 text-left overflow-hidden",
												isSelected
													? "bg-primary/20 border-primary ring-1 ring-primary"
													: "bg-card border-border hover:bg-accent/50",
											)}
										>
											{isActive && (
												<Icon
													icon={faSpinnerThird}
													className="h-3 w-3 animate-spin text-muted-foreground shrink-0"
												/>
											)}
											<span className="text-xs font-medium truncate flex-1">
												{node.span.name ||
													"Unnamed span"}
											</span>
											{durationMs !== null && (
												<span className="text-xs text-muted-foreground shrink-0">
													{formatDuration(durationMs)}
												</span>
											)}
										</button>
									</div>
								);
							})}
							{verticalGaps.map((gap) => (
								<div
									key={`timeline-gap-${gap.afterRow}`}
									className="absolute left-0 right-4 flex items-center"
									style={{
										top: `${getRowTop(gap.afterRow) + rowHeight + rowGap / 2}px`,
										height: `${gapMarkerHeight}px`,
									}}
								>
									<div className="h-px flex-1 bg-border/50 border-dashed" />
								</div>
							))}
							{horizontalGaps.map((gap) => {
								const startPercent = getCompressedPosition(
									gap.startNs,
								);
								const endPercent = getCompressedPosition(
									gap.endNs,
								);
								return (
									<div
										key={`h-gap-${gap.startNs.toString()}`}
										className="absolute top-0 bottom-0 flex items-center justify-center bg-muted/30 border-x border-dashed border-border/50"
										style={{
											left: `${startPercent}%`,
											width: `${endPercent - startPercent}%`,
										}}
									>
										<span className="text-[10px] text-muted-foreground bg-background/80 px-1 rounded whitespace-nowrap">
											{formatGap(gap.gapMs)}
										</span>
									</div>
								);
							})}
						</div>
					</div>
				</div>
			</div>

			{selectedNode ? (
				<TimelineDetailsPanel
					node={selectedNode}
					nowNs={nowNs}
					onClose={() => setSelectedSpanId(null)}
				/>
			) : null}

			{clamped ? (
				<div className="px-4 py-2 text-xs text-muted-foreground border-t">
					Results truncated at {limit} spans.
				</div>
			) : null}
		</div>
	);
}

function TimelineScale({
	totalDuration,
	gaps,
	getCompressedPosition,
}: {
	totalDuration: number;
	gaps: TimelineGap[];
	getCompressedPosition: (timeNs: bigint) => number;
}) {
	const ticks = useMemo(() => {
		if (totalDuration === 0) return [];

		const numTicks = 10;
		const tickInterval = totalDuration / numTicks;
		const result = [];

		for (let i = 0; i <= numTicks; i++) {
			const ms = tickInterval * i;
			result.push({
				position: `${(i / numTicks) * 100}%`,
				label: formatDuration(ms),
			});
		}

		return result;
	}, [totalDuration]);

	return (
		<div className="sticky top-0 z-10 h-8 border-b bg-background/95 backdrop-blur">
			<div className="relative h-full px-4">
				{ticks.map((tick) => (
					<div
						key={tick.position}
						className="absolute top-0 h-full flex flex-col items-center"
						style={{ left: tick.position }}
					>
						<div className="h-2 w-px bg-border" />
						<span className="text-[10px] text-muted-foreground mt-0.5">
							{tick.label}
						</span>
					</div>
				))}
				{gaps.map((gap) => {
					const startPercent = getCompressedPosition(gap.startNs);
					const endPercent = getCompressedPosition(gap.endNs);
					return (
						<div
							key={`scale-gap-${gap.startNs.toString()}`}
							className="absolute top-0 h-full bg-muted/50"
							style={{
								left: `${startPercent}%`,
								width: `${endPercent - startPercent}%`,
							}}
						/>
					);
				})}
			</div>
		</div>
	);
}

function TimelineDetailsPanel({
	node,
	nowNs,
	onClose,
}: {
	node: SpanNode;
	nowNs: bigint;
	onClose: () => void;
}) {
	const startMs = nsToMs(node.startNs);
	const endNs = node.endNs ?? nowNs;
	const durationMs = nsToMs(endNs - node.startNs);
	const details = buildSpanDetails(node.span);

	return (
		<div className="border-t bg-card max-h-64 overflow-y-auto">
			<div className="flex items-center justify-between px-4 py-2 border-b sticky top-0 bg-card">
				<div className="flex items-center gap-2 min-w-0">
					<span className="font-medium truncate">
						{node.span.name}
					</span>
					{node.endNs == null && (
						<Icon
							icon={faSpinnerThird}
							className="h-3 w-3 animate-spin text-muted-foreground"
						/>
					)}
				</div>
				<Button variant="ghost" size="sm" onClick={onClose}>
					Close
				</Button>
			</div>
			<div className="p-4 space-y-3">
				<div className="grid grid-cols-2 gap-4 text-sm">
					<div>
						<div className="text-xs text-muted-foreground mb-1">
							Start
						</div>
						<div className="font-mono text-xs">
							{format(new Date(startMs), "PPpp")}
						</div>
					</div>
					<div>
						<div className="text-xs text-muted-foreground mb-1">
							Duration
						</div>
						<div className="font-mono text-xs">
							{node.endNs
								? formatDuration(durationMs)
								: "In progress"}
						</div>
					</div>
				</div>
				{details ? (
					<div className="rounded-md border bg-muted/20 p-3">
						<div className="text-xs font-medium mb-2">
							Span details
						</div>
						<ActorObjectInspector data={details} />
					</div>
				) : null}
			</div>
		</div>
	);
}

function buildSpanDetails(span: OtlpSpan): Record<string, unknown> | null {
	const attributes = otlpAttributesToObject(span.attributes);
	const links = span.links?.map((link) => ({
		traceId: link.traceId,
		spanId: link.spanId,
		traceState: link.traceState,
		attributes: otlpAttributesToObject(link.attributes),
		droppedAttributesCount: link.droppedAttributesCount,
	}));
	const details: Record<string, unknown> = {};
	if (attributes && Object.keys(attributes).length > 0) {
		details.attributes = attributes;
	}
	if (span.status) {
		details.status = span.status;
	}
	if (links && links.length > 0) {
		details.links = links;
	}
	if (span.traceState) {
		details.traceState = span.traceState;
	}
	if (span.flags !== undefined) {
		details.flags = span.flags;
	}
	return Object.keys(details).length > 0 ? details : null;
}
