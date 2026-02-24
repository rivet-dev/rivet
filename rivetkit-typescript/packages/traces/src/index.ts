export {
	createTraces,
} from "./traces.js";
export { createNoopTraces } from "./noop.js";
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
} from "./types.js";
export type {
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
