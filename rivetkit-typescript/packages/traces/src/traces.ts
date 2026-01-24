import { AsyncLocalStorage } from "node:async_hooks";
import { Buffer } from "node:buffer";
import { randomBytes } from "node:crypto";
import { performance } from "node:perf_hooks";
import { decode as decodeCbor, encode as encodeCbor } from "cbor-x";
import { pack, unpack } from "fdb-tuple";
import {
	CHUNK_VERSIONED,
	CURRENT_VERSION,
	encodeRecord,
	type ActiveSpanRef,
	type Attributes,
	type Chunk,
	type KeyValue,
	type Record as TraceRecord,
	type RecordBody,
	type SpanEnd,
	type SpanEvent,
	type SpanId,
	type SpanLink,
	type SpanRecordKey,
	type SpanSnapshot,
	type SpanStart,
	type SpanStatus,
	SpanStatusCode,
	type SpanUpdate,
	type StringId,
	type TraceId,
} from "../schemas/versioned.js";
import {
	hexFromBytes,
	type OtlpExportTraceServiceRequestJson,
	type OtlpResource,
} from "./otlp.js";
import { readRangeWireToOtlp } from "./read-range.js";
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

// OTLP v1 JSON reference: https://opentelemetry.io/docs/specs/otlp/
// Span data model reference: https://opentelemetry.io/docs/specs/otel/trace/api/

const KEY_PREFIX = {
	DATA: 1,
};

const MAX_CHUNK_ID = 0xffff_ffff;
const AFTER_MAX_CHUNK_ID = 0x1_0000_0000;

const DEFAULT_BUCKET_SIZE_SEC = 3600;
const DEFAULT_TARGET_CHUNK_BYTES = 512 * 1024;
const DEFAULT_MAX_CHUNK_BYTES = 1024 * 1024;
const DEFAULT_MAX_CHUNK_AGE_MS = 5000;
const DEFAULT_SNAPSHOT_INTERVAL_MS = 300_000;
const DEFAULT_SNAPSHOT_BYTES_THRESHOLD = 256 * 1024;
const DEFAULT_MAX_READ_LIMIT = 10_000;
const DEFAULT_MAX_ACTIVE_SPANS = 10_000;

const SPAN_ID_BYTES = 8;
const TRACE_ID_BYTES = 16;

type AttributeMap = Map<string, unknown>;

type SpanState = {
	spanId: SpanId;
	traceId: TraceId;
	parentSpanId: SpanId | null;
	name: string;
	kind: number;
	traceState: string | null;
	flags: number;
	attributes: AttributeMap;
	droppedAttributesCount: number;
	links: LinkState[];
	droppedLinksCount: number;
	status: SpanStatus | null;
	startTimeUnixNs: bigint;
	depth: number;
	bytesSinceSnapshot: number;
	lastSnapshotMonoMs: number;
};

type LinkState = {
	traceId: TraceId;
	spanId: SpanId;
	traceState: string | null;
	attributes: AttributeMap;
	droppedAttributesCount: number;
};

type ChunkState = {
	bucketStartSec: number;
	chunkId: number;
	baseUnixNs: bigint;
	strings: string[];
	stringIds: Map<string, number>;
	records: TraceRecord[];
	sizeBytes: number;
	createdAtMonoMs: number;
};

type PendingChunk = {
	key: Uint8Array;
	bucketStartSec: number;
	chunkId: number;
	chunk: Chunk;
	bytes: Uint8Array;
	maxRecordNs: bigint;
};

const spanContext = new AsyncLocalStorage<SpanHandle | null>();

function spanKey(spanId: Uint8Array | SpanId): string {
	return hexFromBytes(normalizeBytes(spanId));
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
	const copy = new Uint8Array(bytes.byteLength);
	copy.set(bytes);
	return copy.buffer;
}

function toUint8Array(buffer: ArrayBuffer): Uint8Array {
	return new Uint8Array(buffer);
}

function normalizeBytes(input: Uint8Array | ArrayBuffer): Uint8Array {
	return input instanceof Uint8Array ? input : new Uint8Array(input);
}

export function createTraces(
	options: TracesOptions<OtlpResource>,
): Traces<OtlpExportTraceServiceRequestJson> {
	const driver = options.driver;
	const bucketSizeSec = options.bucketSizeSec ?? DEFAULT_BUCKET_SIZE_SEC;
	const maxChunkBytes = options.maxChunkBytes ?? DEFAULT_MAX_CHUNK_BYTES;
	const targetChunkBytes = Math.min(
		options.targetChunkBytes ?? DEFAULT_TARGET_CHUNK_BYTES,
		maxChunkBytes,
	);
	const maxChunkAgeMs = options.maxChunkAgeMs ?? DEFAULT_MAX_CHUNK_AGE_MS;
	const snapshotIntervalMs =
		options.snapshotIntervalMs ?? DEFAULT_SNAPSHOT_INTERVAL_MS;
	const snapshotBytesThreshold =
		options.snapshotBytesThreshold ?? DEFAULT_SNAPSHOT_BYTES_THRESHOLD;
	const maxActiveSpans = options.maxActiveSpans ?? DEFAULT_MAX_ACTIVE_SPANS;
	const maxReadLimit = options.maxReadLimit ?? DEFAULT_MAX_READ_LIMIT;
	const resource = options.resource;

	const timeAnchor = {
		unixMs: Date.now(),
		monoMs: performance.now(),
	};

	const activeSpans = new Map<string, SpanState>();
	const activeSpanRefs = new Map<string, ActiveSpanRef>();
	const pendingChunks: PendingChunk[] = [];
	let writeChain = Promise.resolve();
	const bucketChunkCounters = new Map<number, number>();

	function nowUnixMs(): number {
		return timeAnchor.unixMs + (performance.now() - timeAnchor.monoMs);
	}

	function nowUnixNs(anchor: { unixMs: number; monoMs: number }): bigint {
		const unixMs = anchor.unixMs + (performance.now() - anchor.monoMs);
		const wholeMs = Math.floor(unixMs);
		const fracMs = unixMs - wholeMs;
		return BigInt(wholeMs) * 1_000_000n + BigInt(Math.floor(fracMs * 1_000_000));
	}

	function createChunkState(bucketStartSec: number): ChunkState {
		return {
			bucketStartSec,
			chunkId: nextChunkId(bucketStartSec),
			baseUnixNs: BigInt(bucketStartSec) * 1_000_000_000n,
			strings: [],
			stringIds: new Map(),
			records: [],
			sizeBytes: 0,
			createdAtMonoMs: performance.now(),
		};
	}

	function nextChunkId(bucketStartSec: number): number {
		const current = bucketChunkCounters.get(bucketStartSec) ?? 0;
		bucketChunkCounters.set(bucketStartSec, current + 1);
		return current;
	}

	const currentChunk = createChunkState(
		computeBucketStartSec(nowUnixNs(timeAnchor), bucketSizeSec),
	);

	function computeBucketStartSec(
		absoluteUnixNs: bigint,
		bucketSize: number,
	): number {
		const sec = absoluteUnixNs / 1_000_000_000n;
		const bucket = sec / BigInt(bucketSize);
		return Number(bucket * BigInt(bucketSize));
	}

	function internString(value: string): StringId {
		const existing = currentChunk.stringIds.get(value);
		if (existing !== undefined) {
			return existing;
		}
		const id = currentChunk.strings.length;
		currentChunk.strings.push(value);
		currentChunk.stringIds.set(value, id);
		return id;
	}

	function encodeAttributes(
		attributes?: Record<string, unknown>,
	): { attributes: Attributes; dropped: number } {
		const list: KeyValue[] = [];
		let dropped = 0;
		if (!attributes) {
			return { attributes: list, dropped };
		}
		for (const [key, value] of Object.entries(attributes)) {
			const sanitized = sanitizeAttributeValue(value);
			if (sanitized === undefined) {
				dropped++;
				continue;
			}
			try {
				const encoded = encodeCbor(sanitized);
				list.push({ key: internString(key), value: toArrayBuffer(encoded) });
			} catch {
				dropped++;
			}
		}
		return { attributes: list, dropped };
	}

	function sanitizeAttributeValue(value: unknown): unknown | undefined {
		if (value === undefined || typeof value === "function") {
			return undefined;
		}
		if (typeof value === "symbol") {
			return undefined;
		}
		if (value instanceof Map) {
			const obj: Record<string, unknown> = {};
			for (const [key, mapValue] of value.entries()) {
				if (typeof key !== "string") {
					return undefined;
				}
				const sanitized = sanitizeAttributeValue(mapValue);
				if (sanitized !== undefined) {
					obj[key] = sanitized;
				}
			}
			return obj;
		}
		if (Array.isArray(value)) {
			return value
				.map((entry) => sanitizeAttributeValue(entry))
				.filter((entry) => entry !== undefined);
		}
		return value;
	}

	function encodeLinks(
		links?: StartSpanOptions["links"],
	): { links: SpanLink[]; dropped: number } {
		const result: SpanLink[] = [];
		let dropped = 0;
		if (!links) {
			return { links: result, dropped };
		}
		for (const link of links) {
			const { attributes, dropped: droppedAttributes } = encodeAttributes(
				link.attributes,
			);
			result.push({
				traceId: toArrayBuffer(link.traceId),
				spanId: toArrayBuffer(link.spanId),
				traceState: link.traceState ?? null,
				attributes,
				droppedAttributesCount: droppedAttributes,
			});
		}
		return { links: result, dropped };
	}

	function createSpanStartRecord(
		spanId: SpanId,
		traceId: TraceId,
		name: string,
		options: StartSpanOptions | undefined,
		parentSpanId: SpanId | null,
	): SpanStart {
		const { attributes, dropped } = encodeAttributes(options?.attributes);
		const { links, dropped: droppedLinks } = encodeLinks(options?.links);
		return {
			traceId,
			spanId,
			parentSpanId,
			name: internString(name),
			kind: options?.kind ?? 0,
			traceState: options?.traceState ?? null,
			flags: options?.flags ?? 0,
			attributes,
			droppedAttributesCount: dropped,
			links,
			droppedLinksCount: droppedLinks,
		};
	}

	function createSpanUpdateRecord(
		spanId: SpanId,
		options: UpdateSpanOptions,
	): SpanUpdate {
		const { attributes, dropped } = encodeAttributes(options.attributes);
		return {
			spanId,
			attributes,
			droppedAttributesCount: dropped,
			status: options.status ? toBareStatus(options.status) : null,
		};
	}

	function createSpanEventRecord(
		spanId: SpanId,
		name: string,
		options: EventOptions | undefined,
	): SpanEvent {
		const { attributes, dropped } = encodeAttributes(options?.attributes);
		return {
			spanId,
			name: internString(name),
			attributes,
			droppedAttributesCount: dropped,
		};
	}

	function createSpanEndRecord(
		spanId: SpanId,
		options: EndSpanOptions | undefined,
	): SpanEnd {
		return {
			spanId,
			status: options?.status ? toBareStatus(options.status) : null,
		};
	}

	function createSpanSnapshotRecord(state: SpanState): SpanSnapshot {
		const { attributes, dropped } = encodeAttributeMap(state.attributes);
		const { links, dropped: droppedLinks } = encodeLinkState(state.links);
		return {
			traceId: state.traceId,
			spanId: state.spanId,
			parentSpanId: state.parentSpanId,
			name: internString(state.name),
			kind: state.kind,
			startTimeUnixNs: state.startTimeUnixNs,
			traceState: state.traceState,
			flags: state.flags,
			attributes,
			droppedAttributesCount: state.droppedAttributesCount + dropped,
			links,
			droppedLinksCount: state.droppedLinksCount + droppedLinks,
			status: state.status,
		};
	}

	function encodeAttributeMap(
		attributes: AttributeMap,
	): { attributes: Attributes; dropped: number } {
		const list: KeyValue[] = [];
		let dropped = 0;
		for (const [key, value] of attributes.entries()) {
			const sanitized = sanitizeAttributeValue(value);
			if (sanitized === undefined) {
				dropped++;
				continue;
			}
			try {
				const encoded = encodeCbor(sanitized);
				list.push({ key: internString(key), value: toArrayBuffer(encoded) });
			} catch {
				dropped++;
			}
		}
		return { attributes: list, dropped };
	}

	function buildAttributeMapFromInput(
		attributes?: Record<string, unknown>,
	): AttributeMap {
		const map = new Map<string, unknown>();
		if (!attributes) {
			return map;
		}
		for (const [key, value] of Object.entries(attributes)) {
			const sanitized = sanitizeAttributeValue(value);
			if (sanitized !== undefined) {
				map.set(key, sanitized);
			}
		}
		return map;
	}

	function decodeAttributeList(
		attributes: Attributes,
		strings: readonly string[],
	): AttributeMap {
		const map = new Map<string, unknown>();
		for (const kv of attributes) {
			const key = strings[kv.key] ?? "";
			try {
				map.set(key, decodeCbor(toUint8Array(kv.value)) as unknown);
			} catch {
				continue;
			}
		}
		return map;
	}

	function decodeLinks(
		links: readonly SpanLink[],
		strings: readonly string[],
	): LinkState[] {
		return links.map((link) => ({
			traceId: link.traceId,
			spanId: link.spanId,
			traceState: link.traceState,
			attributes: decodeAttributeList(link.attributes, strings),
			droppedAttributesCount: link.droppedAttributesCount,
		}));
	}

	function encodeLinkState(
		links: LinkState[],
	): { links: SpanLink[]; dropped: number } {
		const result: SpanLink[] = [];
		let dropped = 0;
		for (const link of links) {
			const { attributes, dropped: droppedAttributes } = encodeAttributeMap(
				link.attributes,
			);
			result.push({
				traceId: link.traceId,
				spanId: link.spanId,
				traceState: link.traceState,
				attributes,
				droppedAttributesCount: droppedAttributes,
			});
		}
		return { links: result, dropped };
	}

	function appendRecord(
		buildBody: () => RecordBody,
		providedTimeUnixMs?: number,
	): { recordIndex: number; encodedBytes: number; body: RecordBody } {
		const absoluteUnixNs =
			providedTimeUnixMs !== undefined
				? BigInt(Math.floor(providedTimeUnixMs)) * 1_000_000n
				: nowUnixNs(timeAnchor);
		const recordBucketStart = computeBucketStartSec(
			absoluteUnixNs,
			bucketSizeSec,
		);
		if (recordBucketStart !== currentChunk.bucketStartSec) {
			flushChunk();
			resetChunkState(recordBucketStart);
		}
		if (performance.now() - currentChunk.createdAtMonoMs >= maxChunkAgeMs) {
			flushChunk();
			resetChunkState(recordBucketStart);
		}
		let body = buildBody();
		const timeOffsetNs = absoluteUnixNs - currentChunk.baseUnixNs;
		let record: TraceRecord = { timeOffsetNs, body };
		let encodedRecord = encodeRecord(record);
		if (encodedRecord.length > maxChunkBytes) {
			throw new Error("Record exceeds maxChunkBytes");
		}
		if (currentChunk.sizeBytes + encodedRecord.length > targetChunkBytes) {
			flushChunk();
			resetChunkState(recordBucketStart);
			body = buildBody();
			record = { timeOffsetNs, body };
			encodedRecord = encodeRecord(record);
			if (encodedRecord.length > maxChunkBytes) {
				throw new Error("Record exceeds maxChunkBytes");
			}
		}
		currentChunk.records.push(record);
		currentChunk.sizeBytes += encodedRecord.length;
		const recordIndex = currentChunk.records.length - 1;
		return { recordIndex, encodedBytes: encodedRecord.length, body };
	}

	function flushChunk(): boolean {
		if (currentChunk.records.length === 0) {
			return false;
		}
		const chunk: Chunk = {
			baseUnixNs: currentChunk.baseUnixNs,
			strings: currentChunk.strings,
			records: currentChunk.records,
			activeSpans: Array.from(activeSpanRefs.values()),
		};
		const bytes = CHUNK_VERSIONED.serializeWithEmbeddedVersion(
			chunk,
			CURRENT_VERSION,
		);
		const key = buildChunkKey(currentChunk.bucketStartSec, currentChunk.chunkId);
		const maxRecordNs =
			chunk.records.length > 0
				? chunk.baseUnixNs +
					chunk.records[chunk.records.length - 1].timeOffsetNs
				: chunk.baseUnixNs;
		const pending: PendingChunk = {
			key,
			bucketStartSec: currentChunk.bucketStartSec,
			chunkId: currentChunk.chunkId,
			chunk,
			bytes,
			maxRecordNs,
		};
		pendingChunks.push(pending);
		enqueueWrite(pending);
		return true;
	}

	function enqueueWrite(pending: PendingChunk): void {
		writeChain = writeChain.then(async () => {
			await driver.set(pending.key, pending.bytes);
			const index = pendingChunks.indexOf(pending);
			if (index !== -1) {
				pendingChunks.splice(index, 1);
			}
		});
	}

	function resetChunkState(bucketStartSec: number): void {
		currentChunk.bucketStartSec = bucketStartSec;
		currentChunk.chunkId = nextChunkId(bucketStartSec);
		currentChunk.baseUnixNs = BigInt(bucketStartSec) * 1_000_000_000n;
		currentChunk.strings = [];
		currentChunk.stringIds = new Map();
		currentChunk.records = [];
		currentChunk.sizeBytes = 0;
		currentChunk.createdAtMonoMs = performance.now();
	}

	function enforceMaxActiveSpans(): void {
		if (activeSpans.size <= maxActiveSpans) {
			return;
		}
		const candidates = Array.from(activeSpans.values()).sort((a, b) => {
			if (a.depth !== b.depth) {
				return b.depth - a.depth;
			}
			if (a.startTimeUnixNs > b.startTimeUnixNs) {
				return -1;
			}
			if (a.startTimeUnixNs < b.startTimeUnixNs) {
				return 1;
			}
			return 0;
		});
		for (const span of candidates) {
			dropSpan(span.spanId);
			if (activeSpans.size <= maxActiveSpans) {
				break;
			}
		}
	}

	function dropSpan(spanId: SpanId | Uint8Array): void {
		const key = spanKey(spanId);
		activeSpans.delete(key);
		activeSpanRefs.delete(key);
	}

	function assertActive(handle: SpanHandle): void {
		if (!isActive(handle)) {
			throw new Error("Span handle is not active");
		}
	}

	function isActive(handle: SpanHandle): boolean {
		return activeSpans.has(spanKey(handle.spanId));
	}

	function startSpan(name: string, options?: StartSpanOptions): SpanHandle {
		const parent = options?.parent ?? getCurrentSpan();
		if (parent) {
			assertActive(parent);
		}
		const spanIdBytes = randomBytes(SPAN_ID_BYTES);
		const traceIdBytes = parent ? parent.traceId : randomBytes(TRACE_ID_BYTES);
		const spanId = toArrayBuffer(spanIdBytes);
		const traceId = toArrayBuffer(traceIdBytes);
		const parentSpanId = parent ? toArrayBuffer(parent.spanId) : null;
		const { recordIndex, encodedBytes, body } = appendRecord(() => ({
			tag: "SpanStart",
			val: createSpanStartRecord(
				spanId,
				traceId,
				name,
				options,
				parentSpanId,
			),
		}));
		const spanStart = body.val as SpanStart;
		const key = spanKey(spanId);
		const startKey: SpanRecordKey = {
			prefix: KEY_PREFIX.DATA,
			bucketStartSec: BigInt(currentChunk.bucketStartSec),
			chunkId: currentChunk.chunkId,
			recordIndex,
		};
		activeSpanRefs.set(key, {
			spanId,
			startKey,
			latestSnapshotKey: null,
		});
		const depth = computeSpanDepth(parentSpanId);
		activeSpans.set(key, {
			spanId,
			traceId,
			parentSpanId,
			name,
			kind: options?.kind ?? 0,
			traceState: options?.traceState ?? null,
			flags: options?.flags ?? 0,
			attributes: buildAttributeMapFromInput(options?.attributes),
			droppedAttributesCount: spanStart.droppedAttributesCount,
			links: decodeLinks(spanStart.links, currentChunk.strings),
			droppedLinksCount: spanStart.droppedLinksCount,
			status: null,
			startTimeUnixNs:
				currentChunk.baseUnixNs + currentChunk.records[recordIndex].timeOffsetNs,
			depth,
			bytesSinceSnapshot: encodedBytes,
			lastSnapshotMonoMs: performance.now(),
		});
		enforceMaxActiveSpans();
		return {
			spanId: spanIdBytes,
			traceId: traceIdBytes,
			isActive: () => activeSpans.has(key),
		};
	}

	function updateSpan(handle: SpanHandle, options: UpdateSpanOptions): void {
		if (!options.attributes && !options.status) {
			return;
		}
		assertActive(handle);
		const { encodedBytes, body } = appendRecord(() => ({
			tag: "SpanUpdate",
			val: createSpanUpdateRecord(toArrayBuffer(handle.spanId), options),
		}));
		const spanUpdate = body.val as SpanUpdate;
		const state = activeSpans.get(spanKey(handle.spanId));
		if (!state) {
			return;
		}
		if (options.attributes) {
			const updates = buildAttributeMapFromInput(options.attributes);
			for (const [key, value] of updates.entries()) {
				state.attributes.set(key, value);
			}
		}
		state.droppedAttributesCount += spanUpdate.droppedAttributesCount;
		if (options.status) {
			state.status = toBareStatus(options.status);
		}
		state.bytesSinceSnapshot += encodedBytes;
		maybeSnapshot(handle.spanId, state);
	}

	function setAttributes(
		handle: SpanHandle,
		attributes: Record<string, unknown>,
	): void {
		updateSpan(handle, { attributes });
	}

	function setStatus(handle: SpanHandle, status: SpanStatusInput): void {
		updateSpan(handle, { status });
	}

	function emitEvent(
		handle: SpanHandle,
		name: string,
		options?: EventOptions,
	): void {
		assertActive(handle);
		const { encodedBytes } = appendRecord(
			() => ({
				tag: "SpanEvent",
				val: createSpanEventRecord(toArrayBuffer(handle.spanId), name, options),
			}),
			options?.timeUnixMs,
		);
		const state = activeSpans.get(spanKey(handle.spanId));
		if (state) {
			state.bytesSinceSnapshot += encodedBytes;
			maybeSnapshot(handle.spanId, state);
		}
	}

	function endSpan(handle: SpanHandle, options?: EndSpanOptions): void {
		assertActive(handle);
		appendRecord(() => ({
			tag: "SpanEnd",
			val: createSpanEndRecord(toArrayBuffer(handle.spanId), options),
		}));
		dropSpan(handle.spanId);
	}

	function maybeSnapshot(spanId: SpanId | Uint8Array, state: SpanState): void {
		if (
			state.bytesSinceSnapshot < snapshotBytesThreshold &&
			performance.now() - state.lastSnapshotMonoMs < snapshotIntervalMs
		) {
			return;
		}
		const { recordIndex } = appendRecord(() => ({
			tag: "SpanSnapshot",
			val: createSpanSnapshotRecord(state),
		}));
		const key = spanKey(spanId);
		const ref = activeSpanRefs.get(key);
		if (ref) {
			activeSpanRefs.set(key, {
				...ref,
				latestSnapshotKey: {
				prefix: KEY_PREFIX.DATA,
				bucketStartSec: BigInt(currentChunk.bucketStartSec),
				chunkId: currentChunk.chunkId,
				recordIndex,
				},
			});
		}
		state.bytesSinceSnapshot = 0;
		state.lastSnapshotMonoMs = performance.now();
	}

	async function flush(): Promise<boolean> {
		const didFlush = flushChunk();
		if (didFlush) {
			resetChunkState(currentChunk.bucketStartSec);
		}
		await writeChain;
		return didFlush;
	}

	function withSpan<T>(handle: SpanHandle, fn: () => T): T {
		return spanContext.run(handle, fn);
	}

	function getCurrentSpan(): SpanHandle | null {
		const handle = spanContext.getStore() ?? null;
		if (!handle) {
			return null;
		}
		return isActive(handle) ? handle : null;
	}

	async function readRangeWire(
		options: ReadRangeOptions,
	): Promise<ReadRangeWire> {
		const startMs = Math.floor(options.startMs);
		const endMs = Math.floor(options.endMs);
		if (options.limit <= 0 || endMs <= startMs) {
			return {
				startTimeMs: BigInt(startMs),
				endTimeMs: BigInt(endMs),
				limit: 0,
				clamped: false,
				baseChunks: [],
				chunks: [],
			};
		}
		const limitWasClamped = options.limit > maxReadLimit;
		const limit = Math.min(options.limit, maxReadLimit);
		const startNs = BigInt(startMs) * 1_000_000n;
		const endNs = BigInt(endMs) * 1_000_000n;

		const previousChunk = await findPreviousChunk(startNs, bucketSizeSec);
		const activeRefs = previousChunk?.activeSpans ?? [];
		const baseChunks: Chunk[] = [];
		for (const ref of activeRefs) {
			const baseRecord = await loadBaseRecord(ref);
			if (!baseRecord) {
				continue;
			}
			const baseUnixNs =
				baseRecord.absNs - baseRecord.record.timeOffsetNs;
			baseChunks.push({
				baseUnixNs,
				strings: baseRecord.strings,
				records: [baseRecord.record],
				activeSpans: [],
			});
		}

		const chunks: Chunk[] = [];
		const diskChunks = await listRangeChunks(startNs, endNs, bucketSizeSec);
		for (const chunk of diskChunks) {
			const filtered = filterChunkRecords(chunk.chunk, startNs, endNs);
			if (filtered) {
				chunks.push(filtered);
			}
		}
		for (const pending of pendingChunks) {
			const filtered = filterChunkRecords(pending.chunk, startNs, endNs);
			if (filtered) {
				chunks.push(filtered);
			}
		}
		const currentFiltered = filterChunkRecords(
			currentChunkAsChunk(),
			startNs,
			endNs,
		);
		if (currentFiltered) {
			chunks.push(currentFiltered);
		}

		const reachedSpanLimit = countUniqueSpanIds(chunks, limit);
		return {
			startTimeMs: BigInt(startMs),
			endTimeMs: BigInt(endMs),
			limit,
			clamped: limitWasClamped || reachedSpanLimit,
			baseChunks,
			chunks,
		};
	}

	async function readRange(
		options: ReadRangeOptions,
	): Promise<ReadRangeResult<OtlpExportTraceServiceRequestJson>> {
		const wire = await readRangeWire(options);
		return readRangeWireToOtlp(wire, resource);
	}

	function filterChunkRecords(
		chunk: Chunk,
		startNs: bigint,
		endNs: bigint,
	): Chunk | null {
		const filtered: TraceRecord[] = [];
		for (const record of chunk.records) {
			const absNs = chunk.baseUnixNs + record.timeOffsetNs;
			if (absNs < startNs || absNs >= endNs) {
				continue;
			}
			filtered.push(record);
		}
		if (filtered.length === 0) {
			return null;
		}
		return {
			baseUnixNs: chunk.baseUnixNs,
			strings: chunk.strings,
			records: filtered,
			activeSpans: chunk.activeSpans,
		};
	}

	function countUniqueSpanIds(chunks: Chunk[], limit: number): boolean {
		if (limit <= 0) {
			return true;
		}
		const seen = new Set<string>();
		for (const chunk of chunks) {
			for (const record of chunk.records) {
				const key = spanKey(recordSpanId(record.body));
				if (seen.has(key)) {
					continue;
				}
				if (seen.size >= limit) {
					return true;
				}
				seen.add(key);
			}
		}
		return false;
	}

	function recordSpanId(body: RecordBody): SpanId {
		switch (body.tag) {
			case "SpanStart":
				return body.val.spanId;
			case "SpanEvent":
				return body.val.spanId;
			case "SpanUpdate":
				return body.val.spanId;
			case "SpanEnd":
				return body.val.spanId;
			case "SpanSnapshot":
				return body.val.spanId;
		}
	}

	function currentChunkAsChunk(): Chunk {
		return {
			baseUnixNs: currentChunk.baseUnixNs,
			strings: currentChunk.strings,
			records: currentChunk.records,
			activeSpans: Array.from(activeSpanRefs.values()),
		};
	}

	async function listRangeChunks(
		startNs: bigint,
		endNs: bigint,
		bucketSize: number,
	): Promise<Array<{ key: Uint8Array; chunk: Chunk }>> {
		const startBucket = computeBucketStartSec(startNs, bucketSize);
		const endBucket = computeBucketStartSec(endNs, bucketSize);
		const startKey = buildChunkKey(startBucket, 0);
		const endKey = buildChunkKey(endBucket + bucketSize, 0);
		const entries = await driver.listRange(startKey, endKey);
		const output: Array<{ key: Uint8Array; chunk: Chunk }> = [];
		for (const entry of entries) {
			const chunk = deserializeChunkSafe(entry.value);
			if (!chunk) {
				continue;
			}
			output.push({ key: entry.key, chunk });
		}
		return output;
	}

	async function findPreviousChunk(
		startNs: bigint,
		bucketSize: number,
	): Promise<Chunk | null> {
		const startBucket = computeBucketStartSec(startNs, bucketSize);
		let cursor = {
			bucketStartSec: startBucket,
			chunkId: AFTER_MAX_CHUNK_ID,
		};

		while (true) {
			const pendingCandidate = findLatestPendingBefore(cursor);
			const diskCandidate = await findLatestDiskBefore(cursor);
			const candidate = selectLatestCandidate(
				pendingCandidate,
				diskCandidate,
			);
			if (!candidate) {
				return null;
			}
			if (candidate.maxRecordNs < startNs) {
				return candidate.chunk;
			}
			cursor = {
				bucketStartSec: candidate.bucketStartSec,
				chunkId: candidate.chunkId,
			};
		}
	}

	function findLatestPendingBefore(cursor: {
		bucketStartSec: number;
		chunkId: number;
	}): PendingChunk | null {
		let best: PendingChunk | null = null;
		for (const pending of pendingChunks) {
			if (compareChunkKey(pending, cursor) >= 0) {
				continue;
			}
			if (!best || compareChunkKey(pending, best) > 0) {
				best = pending;
			}
		}
		return best;
	}

	async function findLatestDiskBefore(cursor: {
		bucketStartSec: number;
		chunkId: number;
	}): Promise<PendingChunk | null> {
		const startKey = buildChunkKey(0, 0);
		let endKey = buildChunkKey(cursor.bucketStartSec, cursor.chunkId);

		while (true) {
			const entries = await driver.listRange(startKey, endKey, {
				reverse: true,
				limit: 10,
			});
			if (entries.length === 0) {
				return null;
			}
			for (const entry of entries) {
				const chunk = deserializeChunkSafe(entry.value);
				if (!chunk) {
					endKey = entry.key;
					continue;
				}
				const { bucketStartSec, chunkId } = decodeChunkKey(entry.key);
				const maxRecordNs =
					chunk.records.length > 0
						? chunk.baseUnixNs +
							chunk.records[chunk.records.length - 1].timeOffsetNs
						: chunk.baseUnixNs;
				return {
					key: entry.key,
					bucketStartSec,
					chunkId,
					chunk,
					bytes: entry.value,
					maxRecordNs,
				};
			}
		}
	}

	function selectLatestCandidate(
		pending: PendingChunk | null,
		disk: PendingChunk | null,
	): PendingChunk | null {
		if (pending && disk) {
			return compareChunkKey(pending, disk) >= 0 ? pending : disk;
		}
		return pending ?? disk;
	}

	function compareChunkKey(
		a: { bucketStartSec: number; chunkId: number },
		b: { bucketStartSec: number; chunkId: number },
	): number {
		if (a.bucketStartSec !== b.bucketStartSec) {
			return a.bucketStartSec - b.bucketStartSec;
		}
		return a.chunkId - b.chunkId;
	}

	function decodeChunkKey(key: Uint8Array): {
		bucketStartSec: number;
		chunkId: number;
	} {
		const tuple = unpack(Buffer.from(key)) as [number, number, number];
		return {
			bucketStartSec: tuple[1],
			chunkId: tuple[2],
		};
	}

	function buildChunkKey(bucketStartSec: number, chunkId: number): Uint8Array {
		return pack([KEY_PREFIX.DATA, bucketStartSec, chunkId]);
	}

	function deserializeChunkSafe(bytes: Uint8Array): Chunk | null {
		try {
			return CHUNK_VERSIONED.deserializeWithEmbeddedVersion(bytes);
		} catch {
			return null;
		}
	}

	async function loadBaseRecord(
		ref: ActiveSpanRef,
	): Promise<
		{ record: TraceRecord; strings: readonly string[]; absNs: bigint } | null
	> {
		const key = ref.latestSnapshotKey ?? ref.startKey;
		const bucketStartSec = toNumber(key.bucketStartSec);
		const fromMemory = findChunkInMemory(bucketStartSec, key.chunkId);
		if (fromMemory) {
			const record = fromMemory.records[key.recordIndex];
			if (!record) {
				return null;
			}
			const absNs = fromMemory.baseUnixNs + record.timeOffsetNs;
			return { record, strings: fromMemory.strings, absNs };
		}
		const chunkKey = buildChunkKey(bucketStartSec, key.chunkId);
		const bytes = await driver.get(chunkKey);
		if (!bytes) {
			return null;
		}
		const chunk = deserializeChunkSafe(bytes);
		if (!chunk) {
			return null;
		}
		const record = chunk.records[key.recordIndex];
		if (!record) {
			return null;
		}
		const absNs = chunk.baseUnixNs + record.timeOffsetNs;
		return { record, strings: chunk.strings, absNs };
	}

	function findChunkInMemory(
		bucketStartSec: number,
		chunkId: number,
	): Chunk | null {
		if (
			currentChunk.bucketStartSec === bucketStartSec &&
			currentChunk.chunkId === chunkId
		) {
			return currentChunkAsChunk();
		}
		const pending = pendingChunks.find(
			(candidate) =>
				candidate.bucketStartSec === bucketStartSec &&
				candidate.chunkId === chunkId,
		);
		return pending?.chunk ?? null;
	}

	function toNumber(value: bigint): number {
		const asNumber = Number(value);
		if (!Number.isSafeInteger(asNumber)) {
			throw new Error("Value exceeds safe integer range");
		}
		return asNumber;
	}

	function computeSpanDepth(parentSpanId: SpanId | null): number {
		if (!parentSpanId) {
			return 0;
		}
		const parent = activeSpans.get(spanKey(parentSpanId));
		if (!parent) {
			return 0;
		}
		return parent.depth + 1;
	}

	function randomSpanId(): SpanId {
		return toArrayBuffer(randomBytes(SPAN_ID_BYTES));
	}

	function randomTraceId(): TraceId {
		return toArrayBuffer(randomBytes(TRACE_ID_BYTES));
	}

	function toBareStatus(status: SpanStatusInput): SpanStatus {
		return {
			code: toBareStatusCode(status.code),
			message: status.message ?? null,
		};
	}

	function toBareStatusCode(code: SpanStatusInput["code"]): SpanStatusCode {
		switch (code) {
			case "OK":
				return SpanStatusCode.OK;
			case "ERROR":
				return SpanStatusCode.ERROR;
			case "UNSET":
			default:
				return SpanStatusCode.UNSET;
		}
	}

	return {
		startSpan,
		updateSpan,
		setAttributes,
		setStatus,
		endSpan,
		emitEvent,
		withSpan,
		getCurrentSpan,
		flush,
		readRange,
		readRangeWire,
	};
}
