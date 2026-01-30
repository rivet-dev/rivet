// Browser stub for @rivetkit/traces
// This file is used as the browser entry point to prevent Node.js-specific code from being bundled

import type {
	OtlpAnyValue,
	OtlpExportTraceServiceRequestJson,
	OtlpInstrumentationScope,
	OtlpKeyValue,
	OtlpResource,
	OtlpResourceSpans,
	OtlpScopeSpans,
	OtlpSpan,
	OtlpSpanEvent,
	OtlpSpanLink,
	OtlpSpanStatus,
} from "./otlp.js";
import type {
	EndSpanOptions,
	EventOptions,
	ReadRangeOptions,
	ReadRangeResult,
	ReadRangeWire,
	SpanHandle,
	SpanStatusInput,
	StartSpanOptions,
	Traces,
	TracesDriver,
	TracesOptions,
	UpdateSpanOptions,
} from "./types.js";

function notSupported(name: string): never {
	throw new Error(
		`@rivetkit/traces: ${name} is not supported in the browser. Traces are only available on the server.`,
	);
}

export function createTraces(
	_options: TracesOptions<OtlpResource>,
): Traces<OtlpExportTraceServiceRequestJson> {
	notSupported("createTraces");
}

export function encodeReadRangeWire(_wire: ReadRangeWire): Uint8Array {
	notSupported("encodeReadRangeWire");
}

export function decodeReadRangeWire(_bytes: Uint8Array): ReadRangeWire {
	notSupported("decodeReadRangeWire");
}

export function readRangeWireToOtlp(
	_wire: ReadRangeWire,
	_resource?: OtlpResource,
): { otlp: OtlpExportTraceServiceRequestJson; clamped: boolean } {
	notSupported("readRangeWireToOtlp");
}

// Re-export types (these are safe for browsers)
export type {
	EndSpanOptions,
	EventOptions,
	ReadRangeOptions,
	ReadRangeResult,
	ReadRangeWire,
	SpanHandle,
	SpanStatusInput,
	StartSpanOptions,
	Traces,
	TracesDriver,
	TracesOptions,
	UpdateSpanOptions,
	OtlpAnyValue,
	OtlpExportTraceServiceRequestJson,
	OtlpInstrumentationScope,
	OtlpKeyValue,
	OtlpResource,
	OtlpResourceSpans,
	OtlpScopeSpans,
	OtlpSpan,
	OtlpSpanEvent,
	OtlpSpanLink,
	OtlpSpanStatus,
};
