import type { SpanNode } from "./types";

// Helper to check if a span is in progress
export function isSpanInProgress(span: SpanNode): boolean {
	return span.endNs === undefined;
}

// Helper to calculate span duration in ms
export function getSpanDurationMs(span: SpanNode): number | null {
	if (!span.endTimeUnixNano) return null;
	return (span.endTimeUnixNano - span.startTimeUnixNano) / 1_000_000;
}

// Format duration for display
export function formatDuration(ms: number): string {
	if (ms < 1000) {
		return `${ms.toFixed(0)}ms`;
	}
	return `${(ms / 1000).toFixed(1)}s`;
}

// Format nanoseconds timestamp to time string
export function formatTimestamp(nanos: number): string {
	const date = new Date(nanos / 1_000_000);
	return date.toLocaleTimeString("en-US", {
		hour: "numeric",
		minute: "2-digit",
		hour12: true,
	});
}

export function buildSpanTree(spans: SpanNode[]): SpanNode[] {
	const spanMap = new Map<string, SpanNode>();
	const roots: SpanNode[] = [];

	// First pass: create nodes
	spans.forEach((span) => {
		spanMap.set(span.spanId, { ...span, children: [] });
	});

	// Second pass: build tree
	spans.forEach((span) => {
		const node = spanMap.get(span.spanId)!;
		if (span.parentSpanId && spanMap.has(span.parentSpanId)) {
			spanMap.get(span.parentSpanId)!.children.push(node);
		} else {
			roots.push(node);
		}
	});

	// Sort by start time
	const sortByStartTime = (a: SpanNode, b: SpanNode) =>
		a.startTimeUnixNano - b.startTimeUnixNano;

	roots.sort(sortByStartTime);
	spanMap.forEach((node) => node.children.sort(sortByStartTime));

	return roots;
}

// Get total event count for a span (including children)
export function getTotalEventCount(span: SpanNode): number {
	const ownEvents = span.events?.length ?? 0;
	const childEvents = span.children.reduce(
		(sum, child) => sum + getTotalEventCount(child),
		0,
	);
	return ownEvents + childEvents + span.children.length;
}
