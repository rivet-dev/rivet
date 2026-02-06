import type { OtlpSpan, OtlpSpanEvent } from "@rivetkit/traces";
import type { ReactNode } from "react";

export interface FlattenedSpan {
	span: SpanNode;
	depth: number;
	row: number;
}

export type SpanNode = {
	timeUnixNano: bigint;
	spanId: string | null;
	endTimeUnixNano: any;
	startTimeUnixNano: any;
	name: ReactNode;
	span: OtlpSpan;
	startNs: bigint;
	endNs: bigint | null;
	children: SpanNode[];
	events: OtlpSpanEvent[];
};

export type TraceItem =
	| { type: "span"; node: SpanNode; timeNs: bigint }
	| { type: "event"; event: OtlpSpanEvent; timeNs: bigint };

// Represents a time gap that was collapsed
export interface TimeGap {
	startTime: bigint; // nanos
	endTime: bigint; // nanos
	durationMs: number;
}

// Represents a time segment (either active or gap)
export interface TimeSegment {
	type: "active" | "gap";
	startTime: bigint; // nanos
	endTime: bigint; // nanos
	originalDuration: number; // ms - actual duration
	displayDuration: number; // ms - collapsed duration for gaps
}
