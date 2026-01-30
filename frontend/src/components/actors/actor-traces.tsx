import { faChevronDown, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import {
	useMemo,
	useState,
	type ReactElement,
} from "react";
import type { DateRange } from "../datepicker";
import { RangeDatePicker } from "../datepicker";
import { Button } from "../ui/button";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import { cn } from "../lib/utils";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useActorInspector } from "./actor-inspector-context";
import type { ActorId } from "./queries";
import { readRangeWireToOtlp } from "@rivetkit/traces/reader";
import type {
	OtlpAnyValue,
	OtlpExportTraceServiceRequestJson,
	OtlpKeyValue,
	OtlpSpan,
	OtlpSpanEvent,
} from "@rivetkit/traces";

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

type SpanNode = {
	span: OtlpSpan;
	startNs: bigint;
	endNs: bigint | null;
	children: SpanNode[];
	events: OtlpSpanEvent[];
};

type TraceItem =
	| { type: "span"; node: SpanNode; timeNs: bigint }
	| { type: "event"; event: OtlpSpanEvent; timeNs: bigint };

export function ActorTraces({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();
	const [isLive, setIsLive] = useState(true);
	const [presetMs, setPresetMs] = useState(DEFAULT_PRESET_MS);
	const [customRange, setCustomRange] = useState<DateRange | undefined>(() => {
		const now = Date.now();
		return {
			from: new Date(now - DEFAULT_PRESET_MS),
			to: new Date(now),
		};
	});

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
				: customRange?.from?.getTime() ?? now - presetMs;
			const rangeEndMs = isLive
				? now
				: customRange?.to?.getTime() ?? now;
			const startMs = Math.min(rangeStartMs, rangeEndMs);
			const endMs = Math.max(rangeStartMs, rangeEndMs);
			return inspector.api.getTraces({
				startMs,
				endMs,
				limit: DEFAULT_LIMIT,
			});
		},
		enabled: inspector.isInspectorAvailable && inspector.features.traces.supported,
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
			</div>

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

function extractSpans(
	otlp: OtlpExportTraceServiceRequestJson,
): OtlpSpan[] {
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
		return value.arrayValue.values.map((item) =>
			otlpAnyValueToJs(item),
		);
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
