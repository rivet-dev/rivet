import type { ReadRangeWire as SchemaReadRangeWire } from "../schemas/versioned.js";

export interface TracesDriver {
	get(key: Uint8Array): Promise<Uint8Array | null>;
	set(key: Uint8Array, value: Uint8Array): Promise<void>;
	delete(key: Uint8Array): Promise<void>;
	deletePrefix(prefix: Uint8Array): Promise<void>;
	list(prefix: Uint8Array): Promise<Array<{ key: Uint8Array; value: Uint8Array }>>;
	listRange(
		start: Uint8Array,
		end: Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<{ key: Uint8Array; value: Uint8Array }>>;
	batch(writes: Array<{ key: Uint8Array; value: Uint8Array }>): Promise<void>;
}

export interface SpanHandle {
	spanId: Uint8Array;
	traceId: Uint8Array;
	isActive(): boolean;
}

export interface StartSpanOptions {
	parent?: SpanHandle;
	attributes?: Record<string, unknown>;
	links?: Array<{
		traceId: Uint8Array;
		spanId: Uint8Array;
		traceState?: string;
		attributes?: Record<string, unknown>;
	}>;
	kind?: number;
	traceState?: string;
	flags?: number;
}

export interface UpdateSpanOptions {
	attributes?: Record<string, unknown>;
	status?: SpanStatusInput;
}

export interface SpanStatusInput {
	code: "UNSET" | "OK" | "ERROR";
	message?: string;
}

export interface EndSpanOptions {
	status?: SpanStatusInput;
}

export interface EventOptions {
	attributes?: Record<string, unknown>;
	timeUnixMs?: number;
}

export interface ReadRangeOptions {
	startMs: number;
	endMs: number;
	limit: number;
}

export type ReadRangeWire = SchemaReadRangeWire;

export interface ReadRangeResult<TExport> {
	otlp: TExport;
	clamped: boolean;
}

export interface TracesOptions<TResource> {
	driver: TracesDriver;
	resource?: TResource;
	bucketSizeSec?: number;
	targetChunkBytes?: number;
	maxChunkBytes?: number;
	maxChunkAgeMs?: number;
	snapshotIntervalMs?: number;
	snapshotBytesThreshold?: number;
	maxActiveSpans?: number;
	maxReadLimit?: number;
}

export interface Traces<TExport> {
	startSpan(name: string, options?: StartSpanOptions): SpanHandle;
	updateSpan(handle: SpanHandle, options: UpdateSpanOptions): void;
	setAttributes(handle: SpanHandle, attributes: Record<string, unknown>): void;
	setStatus(handle: SpanHandle, status: SpanStatusInput): void;
	endSpan(handle: SpanHandle, options?: EndSpanOptions): void;
	emitEvent(handle: SpanHandle, name: string, options?: EventOptions): void;
	withSpan<T>(handle: SpanHandle, fn: () => T): T;
	getCurrentSpan(): SpanHandle | null;
	flush(): Promise<boolean>;
	readRange(options: ReadRangeOptions): Promise<ReadRangeResult<TExport>>;
	readRangeWire(options: ReadRangeOptions): Promise<ReadRangeWire>;
}
