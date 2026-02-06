"use client";

import {
	faMagnifyingGlassMinus,
	faMagnifyingGlassPlus,
	faMinusLarge,
	Icon,
} from "@rivet-gg/icons";
import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button, cn, formatDuration } from "@/components";
import type { FlattenedSpan, SpanNode, TimeGap, TimeSegment } from "./types";

interface TimelineViewProps {
	spans: SpanNode[];
	selectedSpanId: string | null;
	selectedEventIndex: number | null;
	onSelectSpan: (spanId: string | null) => void;
	onSelectEvent: (spanId: string, eventIndex: number) => void;
}

// Gap detection threshold - gaps larger than this will be collapsed
const GAP_THRESHOLD_MS = 50; // 50ms minimum gap to collapse
const GAP_FIXED_WIDTH_PX = 48; // Fixed pixel width for all gaps
const MIN_ACTIVE_SEGMENT_WIDTH_PX = 8; // Minimum pixel width for active segments

// Flatten span tree for rendering
function flattenSpans(spans: SpanNode[], depth = 0): FlattenedSpan[] {
	const result: FlattenedSpan[] = [];
	let row = 0;

	function traverse(span: SpanNode, d: number) {
		result.push({ span, depth: d, row: row++ });
		span.children.forEach((child) => traverse(child, d + 1));
	}

	spans.forEach((span) => traverse(span, depth));
	return result;
}

// Get all spans flattened (for timeline calculations)
function getAllSpans(spans: SpanNode[]): SpanNode[] {
	const result: SpanNode[] = [];

	function traverse(span: SpanNode) {
		result.push(span);
		span.children.forEach(traverse);
	}

	spans.forEach(traverse);
	return result;
}

// Detect gaps in the timeline and create segments
function detectTimeSegments(
	allSpans: SpanNode[],
	minTime: bigint,
	maxTime: bigint,
): { segments: TimeSegment[]; gaps: TimeGap[] } {
	if (allSpans.length === 0) {
		return { segments: [], gaps: [] };
	}

	// Collect all time intervals from spans and events
	const intervals: { start: bigint; end: bigint }[] = [];

	allSpans.forEach((span) => {
		intervals.push({
			start: span.startNs,
			end: span.endNs ?? BigInt(Date.now()) * 1_000_000n,
		});

		// Also include events
		span.events?.forEach((event) => {
			const eventTime = BigInt(event.timeUnixNano);
			intervals.push({
				start: eventTime,
				end: eventTime,
			});
		});
	});

	// Sort intervals by start time
	intervals.sort((a, b) =>
		a.start < b.start ? -1 : a.start > b.start ? 1 : 0,
	);

	// Merge overlapping intervals
	const mergedIntervals: { start: bigint; end: bigint }[] = [];
	let current = { ...intervals[0] };

	for (let i = 1; i < intervals.length; i++) {
		const next = intervals[i];
		// Add small buffer to connect nearby spans (10ms)
		const buffer = 10n * 1_000_000n; // 10ms in nanos
		if (next.start <= current.end + buffer) {
			// Overlapping or close, extend current
			current.end = next.end > current.end ? next.end : current.end;
		} else {
			mergedIntervals.push(current);
			current = { ...next };
		}
	}
	mergedIntervals.push(current);

	// Build segments and gaps
	const segments: TimeSegment[] = [];
	const gaps: TimeGap[] = [];

	// Add segment from minTime to first interval if there's a gap
	if (mergedIntervals[0].start > minTime) {
		const gapDurationMs =
			Number(mergedIntervals[0].start - minTime) / 1_000_000;
		if (gapDurationMs > GAP_THRESHOLD_MS) {
			gaps.push({
				startTime: minTime,
				endTime: mergedIntervals[0].start,
				durationMs: gapDurationMs,
			});
			segments.push({
				type: "gap",
				startTime: minTime,
				endTime: mergedIntervals[0].start,
				originalDuration: gapDurationMs,
				displayDuration: 0, // Not used for gaps - we use fixed pixel width
			});
		}
	}

	// Process merged intervals
	for (let i = 0; i < mergedIntervals.length; i++) {
		const interval = mergedIntervals[i];

		// Add active segment
		const activeDuration =
			Number(interval.end - interval.start) / 1_000_000;
		segments.push({
			type: "active",
			startTime: interval.start,
			endTime: interval.end,
			originalDuration: activeDuration,
			displayDuration: activeDuration,
		});

		// Check for gap before next interval
		if (i < mergedIntervals.length - 1) {
			const nextInterval = mergedIntervals[i + 1];
			const gapDurationMs =
				Number(nextInterval.start - interval.end) / 1_000_000;

			if (gapDurationMs > GAP_THRESHOLD_MS) {
				gaps.push({
					startTime: interval.end,
					endTime: nextInterval.start,
					durationMs: gapDurationMs,
				});
				segments.push({
					type: "gap",
					startTime: interval.end,
					endTime: nextInterval.start,
					originalDuration: gapDurationMs,
					displayDuration: 0, // Not used for gaps - we use fixed pixel width
				});
			}
		}
	}

	// Add segment from last interval to maxTime if there's a gap
	const lastInterval = mergedIntervals[mergedIntervals.length - 1];
	if (lastInterval.end < maxTime) {
		const gapDurationMs = Number(maxTime - lastInterval.end) / 1_000_000;
		if (gapDurationMs > GAP_THRESHOLD_MS) {
			gaps.push({
				startTime: lastInterval.end,
				endTime: maxTime,
				durationMs: gapDurationMs,
			});
			segments.push({
				type: "gap",
				startTime: lastInterval.end,
				endTime: maxTime,
				originalDuration: gapDurationMs,
				displayDuration: 0, // Not used for gaps - we use fixed pixel width
			});
		}
	}

	return { segments, gaps };
}

// Convert original time to display position (accounting for collapsed gaps with fixed pixel width)
function createTimeMapper(segments: TimeSegment[], timelineWidth: number) {
	if (segments.length === 0) {
		return {
			totalWidth: 0,
			mapTimeToPixel: () => 0,
		};
	}

	// Count gaps and calculate total active duration
	const gapCount = segments.filter((s) => s.type === "gap").length;
	const totalGapWidth = gapCount * GAP_FIXED_WIDTH_PX;
	const remainingWidth = Math.max(0, timelineWidth - totalGapWidth);

	const totalActiveDuration = segments
		.filter((s) => s.type === "active")
		.reduce((sum, seg) => sum + seg.originalDuration, 0);

	// Pixels per ms for active segments
	const pxPerMs =
		totalActiveDuration > 0 ? remainingWidth / totalActiveDuration : 0;

	// Build cumulative pixel positions for each segment
	const segmentPositions: {
		segment: TimeSegment;
		pxStart: number;
		pxEnd: number;
	}[] = [];
	let pxOffset = 0;

	for (const segment of segments) {
		let segmentWidth: number;
		if (segment.type === "gap") {
			segmentWidth = GAP_FIXED_WIDTH_PX;
		} else {
			// Ensure active segments have a minimum width
			segmentWidth = Math.max(
				segment.originalDuration * pxPerMs,
				MIN_ACTIVE_SEGMENT_WIDTH_PX,
			);
		}

		segmentPositions.push({
			segment,
			pxStart: pxOffset,
			pxEnd: pxOffset + segmentWidth,
		});
		pxOffset += segmentWidth;
	}

	// Map a time (in nanos) to a pixel position
	const mapTimeToPixel = (timeNanos: bigint): number => {
		for (const { segment, pxStart, pxEnd } of segmentPositions) {
			if (
				timeNanos >= segment.startTime &&
				timeNanos <= segment.endTime
			) {
				// Calculate progress within this segment
				const elapsed = Number(timeNanos - segment.startTime);
				const total = Number(segment.endTime - segment.startTime);
				const progress = total === 0 ? 0.5 : elapsed / total;
				return pxStart + progress * (pxEnd - pxStart);
			}
		}

		// If time is before first segment
		if (timeNanos < segmentPositions[0].segment.startTime) {
			return 0;
		}

		// If time is after last segment
		return timelineWidth;
	};

	return { totalWidth: pxOffset, mapTimeToPixel };
}

export function TimelineView({
	spans,
	selectedSpanId,
	selectedEventIndex,
	onSelectSpan,
	onSelectEvent,
}: TimelineViewProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [zoom, setZoom] = useState(1);
	const [isDragging, setIsDragging] = useState(false);
	const [dragStart, setDragStart] = useState({ x: 0, scrollLeft: 0 });

	const flattenedSpans = useMemo(() => flattenSpans(spans), [spans]);
	const allSpans = useMemo(() => getAllSpans(spans), [spans]);

	// Calculate timeline bounds
	const { minTime, maxTime } = useMemo(() => {
		if (allSpans.length === 0) {
			return { minTime: 0n, maxTime: 0n };
		}

		let min = allSpans[0].startNs;
		let max = allSpans[0].endNs ?? BigInt(Date.now()) * 1_000_000n;

		for (const span of allSpans) {
			if (span.startNs < min) min = span.startNs;
			const end = span.endNs ?? BigInt(Date.now()) * 1_000_000n;
			if (end > max) max = end;
		}

		return {
			minTime: min,
			maxTime: max,
		};
	}, [allSpans]);

	// Detect time gaps and create segments
	const { segments, gaps } = useMemo(
		() => detectTimeSegments(allSpans, minTime, maxTime),
		[allSpans, minTime, maxTime],
	);

	// Row height and spacing
	const rowHeight = 40;
	const rowGap = 8;
	const totalHeight = flattenedSpans.length * (rowHeight + rowGap);

	// Base width calculation
	const baseWidth = 1200;
	const timelineWidth = baseWidth * zoom;

	// Create time mapper for converting real time to pixel position
	const { totalWidth, mapTimeToPixel } = useMemo(
		() => createTimeMapper(segments, timelineWidth),
		[segments, timelineWidth],
	);

	// Calculate position and width for a span (using gap-aware mapping)
	const getSpanStyle = useCallback(
		(span: SpanNode) => {
			if (totalWidth === 0) return { left: "0px", width: "100%" };

			const startPx = mapTimeToPixel(span.startNs);
			const endTime = span.endNs ?? BigInt(Date.now()) * 1_000_000n;
			const endPx = mapTimeToPixel(endTime);
			const widthPx = endPx - startPx;

			return {
				left: `${startPx}px`,
				width: `${Math.max(widthPx, 4)}px`,
			};
		},
		[totalWidth, mapTimeToPixel],
	);

	// Calculate gap indicator positions (in pixels)
	const gapIndicators = useMemo(() => {
		return gaps.map((gap) => {
			const startPx = mapTimeToPixel(gap.startTime);
			const endPx = mapTimeToPixel(gap.endTime);
			const centerPx = (startPx + endPx) / 2;
			return {
				gap,
				centerPx,
				startPx,
				endPx,
			};
		});
	}, [gaps, mapTimeToPixel]);

	// Zoom controls
	const handleZoomIn = () => setZoom((z) => Math.min(z * 1.5, 10));
	const handleZoomOut = () => setZoom((z) => Math.max(z / 1.5, 0.5));
	const handleZoomReset = () => setZoom(1);

	// Pan handling
	const handleMouseDown = (e: React.MouseEvent) => {
		if (e.button !== 0) return;
		setIsDragging(true);
		setDragStart({
			x: e.clientX,
			scrollLeft: containerRef.current?.scrollLeft ?? 0,
		});
	};

	const handleMouseMove = useCallback(
		(e: MouseEvent) => {
			if (!isDragging || !containerRef.current) return;
			const dx = e.clientX - dragStart.x;
			containerRef.current.scrollLeft = dragStart.scrollLeft - dx;
		},
		[isDragging, dragStart],
	);

	const handleMouseUp = useCallback(() => {
		setIsDragging(false);
	}, []);

	useEffect(() => {
		if (isDragging) {
			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);
			return () => {
				window.removeEventListener("mousemove", handleMouseMove);
				window.removeEventListener("mouseup", handleMouseUp);
			};
		}
	}, [isDragging, handleMouseMove, handleMouseUp]);

	if (spans.length === 0) {
		return (
			<div className="flex-1 flex items-center justify-center text-muted-foreground">
				No traces to display
			</div>
		);
	}

	return (
		<div className="flex-1 flex flex-col bg-background relative">
			{/* Timeline content */}
			<div
				ref={containerRef}
				className={cn(
					"flex-1 overflow-x-auto overflow-y-auto",
					isDragging && "cursor-grabbing select-none",
				)}
				onMouseDown={handleMouseDown}
			>
				<div
					className="relative"
					style={{
						width: `${timelineWidth}px`,
						minWidth: "100%",
						height: `${totalHeight + 40}px`,
					}}
				>
					{/* Time scale header */}
					<div className="sticky top-0 z-10 h-8 border-b border-border bg-background/95 backdrop-blur">
						<TimeScale
							totalWidth={totalWidth}
							totalElapsedMs={
								Number(maxTime - minTime) / 1_000_000
							}
							gapIndicators={gapIndicators}
						/>
					</div>

					{/* Gap indicators - vertical stripes across the timeline */}
					{gapIndicators.map((indicator, idx) => (
						<div
							key={`gap-${idx}`}
							className="absolute top-8 bottom-0 flex items-center justify-center pointer-events-none z-5"
							style={{
								left: `${indicator.centerPx}px`,
								width: `${GAP_FIXED_WIDTH_PX}px`,
								transform: "translateX(-50%)",
							}}
						>
							{/* Striped background pattern */}
							<div
								className="absolute inset-0 opacity-30"
								style={{
									background: `repeating-linear-gradient(
                    -45deg,
                    transparent,
                    transparent 4px,
                    hsl(var(--muted)) 4px,
                    hsl(var(--muted)) 6px
                  )`,
								}}
							/>
						</div>
					))}

					{/* Spans */}
					<div className="relative pt-2 px-4">
						{flattenedSpans.map(({ span, depth, row }) => {
							const style = getSpanStyle(span);
							const isSelected = selectedSpanId === span.spanId;
							const durationMs = span.endTimeUnixNano
								? (span.endTimeUnixNano -
										span.startTimeUnixNano) /
									1_000_000
								: null;

							return (
								<div
									key={span.spanId}
									className="absolute"
									style={{
										top: `${row * (rowHeight + rowGap)}px`,
										left: style.left,
										width: style.width,
										height: `${rowHeight}px`,
										marginLeft: `${depth * 24}px`,
									}}
								>
									{/* Span bar */}
									<button
										onClick={() =>
											onSelectSpan(span.spanId)
										}
										className={cn(
											"w-full h-full rounded-md border transition-all flex items-center gap-2 text-left overflow-hidden",
											isSelected
												? "bg-chart-1/30 border-chart-1 ring-1 ring-chart-1"
												: "bg-card border-border hover:bg-accent/50",
										)}
									>
										<span className="text-xs font-medium truncate flex-1 text-foreground mx-2">
											{span.span.name}
										</span>
										{durationMs !== null && (
											<span className="text-xs text-muted-foreground shrink-0">
												{formatDuration(durationMs)}
											</span>
										)}
									</button>

									{/* Events as dots */}
									{span.events?.map((event, idx) => {
										// Calculate event position relative to span start
										const eventPx = mapTimeToPixel(
											BigInt(event.timeUnixNano),
										);
										const spanStartPx = mapTimeToPixel(
											span.startNs,
										);
										const relativePx =
											eventPx - spanStartPx;
										const isEventSelected =
											isSelected &&
											selectedEventIndex === idx;

										return (
											<button
												key={`${span.spanId}-event-${idx}`}
												onClick={(e) => {
													e.stopPropagation();
													onSelectEvent(
														span.spanId,
														idx,
													);
												}}
												className={cn(
													"absolute top-1/2 -translate-y-1/2 -translate-x-1/2 size-3 rounded-full border-2 transition-all z-10",
													isEventSelected
														? "bg-chart-1 border-chart-1 scale-125"
														: "bg-background border-muted-foreground hover:border-foreground hover:scale-110",
												)}
												style={{
													left: `${relativePx}px`,
												}}
												title={event.name}
											/>
										);
									})}
								</div>
							);
						})}
					</div>
				</div>
			</div>

			<div className="absolute bottom-4 right-4 flex items-center gap-1 bg-card border border-border rounded-lg p-1">
				<Button
					variant="ghost"
					size="icon"
					className="size-8"
					onClick={handleZoomOut}
				>
					<Icon icon={faMagnifyingGlassMinus} className="size-4" />
				</Button>
				<Button
					variant="ghost"
					size="icon"
					className="size-8"
					onClick={handleZoomReset}
				>
					<Icon icon={faMinusLarge} className="size-4" />
				</Button>
				<Button
					variant="ghost"
					size="icon"
					className="size-8"
					onClick={handleZoomIn}
				>
					<Icon icon={faMagnifyingGlassPlus} className="size-4" />
				</Button>
			</div>
		</div>
	);
}

interface GapIndicatorInfo {
	gap: TimeGap;
	centerPx: number;
	startPx: number;
	endPx: number;
}

function TimeScale({
	totalWidth,
	totalElapsedMs,
	gapIndicators,
}: {
	totalWidth: number;
	totalElapsedMs: number;
	gapIndicators: GapIndicatorInfo[];
}) {
	// Generate tick marks based on segments
	const ticks = useMemo(() => {
		if (totalWidth === 0) return [];

		const result: {
			position: string;
			label: string;
			isGapLabel?: boolean;
		}[] = [];

		// Add start tick
		result.push({
			position: "0px",
			label: "0ms",
		});

		// Add gap labels using the gapIndicators positions
		for (const indicator of gapIndicators) {
			result.push({
				position: `${indicator.centerPx}px`,
				label: formatGapDuration(indicator.gap.durationMs),
				isGapLabel: true,
			});
		}

		// Add end tick with total real elapsed time
		result.push({
			position: `${totalWidth}px`,
			label: formatDuration(totalElapsedMs),
		});

		return result;
	}, [totalWidth, totalElapsedMs, gapIndicators]);

	return (
		<div className="relative h-full">
			{ticks.map((tick, i) => (
				<div
					key={i}
					className={cn(
						"absolute top-0 h-full flex flex-col items-center",
						tick.isGapLabel && "z-10",
					)}
					style={{ left: tick.position }}
				>
					{tick.isGapLabel ? (
						<>
							<div
								className="h-full w-px bg-muted-foreground/30"
								style={{
									backgroundImage:
										"repeating-linear-gradient(to bottom, transparent, transparent 2px, hsl(var(--muted-foreground) / 0.3) 2px, hsl(var(--muted-foreground) / 0.3) 4px)",
								}}
							/>
							<span className="absolute top-1/2 -translate-y-1/2 text-[10px] text-muted-foreground bg-background/95 px-1.5 py-0.5 rounded border border-border whitespace-nowrap">
								{tick.label} skipped
							</span>
						</>
					) : (
						<>
							<div className="h-2 w-px bg-border" />
							<span className="text-[10px] text-muted-foreground mt-0.5">
								{tick.label}
							</span>
						</>
					)}
				</div>
			))}
		</div>
	);
}

// Format gap duration in a human-readable way
function formatGapDuration(ms: number): string {
	if (ms < 1000) {
		return `${Math.round(ms)}ms`;
	}
	if (ms < 60000) {
		return `${(ms / 1000).toFixed(1)}s`;
	}
	if (ms < 3600000) {
		const minutes = Math.floor(ms / 60000);
		const seconds = Math.floor((ms % 60000) / 1000);
		return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`;
	}
	const hours = Math.floor(ms / 3600000);
	const minutes = Math.floor((ms % 3600000) / 60000);
	return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
}
