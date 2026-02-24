import type { OtlpExportTraceServiceRequestJson } from "./otlp.js";
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
	UpdateSpanOptions,
} from "./types.js";

const U32_MAX = 0xffff_ffff;

const NOOP_SPAN: SpanHandle = {
	spanId: new Uint8Array(8),
	traceId: new Uint8Array(16),
	isActive: () => false,
};

function createEmptyOtlpExport(): OtlpExportTraceServiceRequestJson {
	return {
		resourceSpans: [
			{
				scopeSpans: [{ spans: [] }],
			},
		],
	};
}

/**
 * Implements the Traces contract without persisting or exporting trace data.
 */
export function createNoopTraces(): Traces<OtlpExportTraceServiceRequestJson> {
	return {
		startSpan(_name: string, _options?: StartSpanOptions): SpanHandle {
			return NOOP_SPAN;
		},
		updateSpan(_handle: SpanHandle, _options: UpdateSpanOptions): void {},
		setAttributes(
			_handle: SpanHandle,
			_attributes: Record<string, unknown>,
		): void {},
		setStatus(_handle: SpanHandle, _status: SpanStatusInput): void {},
		endSpan(_handle: SpanHandle, _options?: EndSpanOptions): void {},
		emitEvent(
			_handle: SpanHandle,
			_name: string,
			_options?: EventOptions,
		): void {},
		withSpan<T>(_handle: SpanHandle, fn: () => T): T {
			return fn();
		},
		getCurrentSpan(): SpanHandle | null {
			return null;
		},
		async flush(): Promise<boolean> {
			return false;
		},
		async readRange(
			_options: ReadRangeOptions,
		): Promise<ReadRangeResult<OtlpExportTraceServiceRequestJson>> {
			return {
				otlp: createEmptyOtlpExport(),
				clamped: false,
			};
		},
		async readRangeWire(options: ReadRangeOptions): Promise<ReadRangeWire> {
			return {
				startTimeMs: BigInt(options.startMs),
				endTimeMs: BigInt(options.endMs),
				limit: Math.max(0, Math.min(U32_MAX, Math.floor(options.limit))),
				clamped: false,
				baseChunks: [],
				chunks: [],
			};
		},
	};
}
