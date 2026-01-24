import { decode as decodeCbor } from "cbor-x";
import {
	CURRENT_VERSION,
	READ_RANGE_VERSIONED,
	type Attributes,
	type Chunk,
	type ReadRangeWire,
	type Record as TraceRecord,
	type RecordBody,
	type SpanId,
	type SpanLink,
	type SpanSnapshot,
	type SpanStart,
	type SpanStatus,
	SpanStatusCode,
	type TraceId,
} from "../schemas/versioned.js";
import {
	anyValueFromJs,
	hexFromBytes,
	type OtlpExportTraceServiceRequestJson,
	type OtlpKeyValue,
	type OtlpResource,
	type OtlpSpan,
	type OtlpSpanEvent,
	type OtlpSpanLink,
	type OtlpSpanStatus,
} from "./otlp.js";

type AttributeMap = Map<string, unknown>;

type LinkState = {
	traceId: TraceId;
	spanId: SpanId;
	traceState: string | null;
	attributes: AttributeMap;
	droppedAttributesCount: number;
};

type SpanEventState = {
	name: string;
	timeUnixNs: bigint;
	attributes: AttributeMap;
	droppedAttributesCount: number;
};

type SpanBuilder = {
	traceId: TraceId;
	spanId: SpanId;
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
	endTimeUnixNs: bigint | null;
	events: SpanEventState[];
};

type RecordEntry = {
	record: TraceRecord;
	strings: readonly string[];
	absNs: bigint;
	sequence: number;
};

type BaseRecordEntry = {
	record: TraceRecord;
	strings: readonly string[];
	absNs: bigint;
};

function toUint8Array(buffer: ArrayBuffer): Uint8Array {
	return new Uint8Array(buffer);
}

function normalizeBytes(input: Uint8Array | ArrayBuffer): Uint8Array {
	return input instanceof Uint8Array ? input : new Uint8Array(input);
}

function spanKey(spanId: Uint8Array | SpanId): string {
	return hexFromBytes(normalizeBytes(spanId));
}

export function encodeReadRangeWire(wire: ReadRangeWire): Uint8Array {
	return READ_RANGE_VERSIONED.serializeWithEmbeddedVersion(
		wire,
		CURRENT_VERSION,
	);
}

export function decodeReadRangeWire(bytes: Uint8Array): ReadRangeWire {
	return READ_RANGE_VERSIONED.deserializeWithEmbeddedVersion(bytes);
}

export function readRangeWireToOtlp(
	wire: ReadRangeWire,
	resource?: OtlpResource,
): { otlp: OtlpExportTraceServiceRequestJson; clamped: boolean } {
	const startMs =
		typeof wire.startTimeMs === "bigint"
			? wire.startTimeMs
			: BigInt(Math.floor(wire.startTimeMs));
	const endMs =
		typeof wire.endTimeMs === "bigint"
			? wire.endTimeMs
			: BigInt(Math.floor(wire.endTimeMs));
	const limit =
		typeof wire.limit === "bigint" ? Number(wire.limit) : wire.limit;

	if (limit <= 0 || endMs <= startMs) {
		return { otlp: emptyExport(resource), clamped: wire.clamped };
	}

	const startNs = startMs * 1_000_000n;
	const endNs = endMs * 1_000_000n;
	const baseRecords = buildBaseRecordMap(wire.baseChunks);
	const sequenceRef = { value: 0 };
	const records: RecordEntry[] = [];
	for (const chunk of wire.chunks) {
		collectRecordEntries(records, chunk, startNs, endNs, sequenceRef);
	}

	records.sort((a, b) => {
		if (a.absNs < b.absNs) return -1;
		if (a.absNs > b.absNs) return 1;
		return a.sequence - b.sequence;
	});

	const { spans, reachedSpanLimit } = buildSpansFromRecords(
		records,
		baseRecords,
		limit,
	);
	const exported = spans.map(toOtlpSpan);
	return {
		otlp: buildExport(exported, resource),
		clamped: wire.clamped || reachedSpanLimit,
	};
}

function collectRecordEntries(
	collector: RecordEntry[],
	chunk: Chunk,
	startNs: bigint,
	endNs: bigint,
	sequenceRef: { value: number },
): void {
	for (const record of chunk.records) {
		const absNs = chunk.baseUnixNs + record.timeOffsetNs;
		if (absNs < startNs || absNs >= endNs) {
			continue;
		}
		collector.push({
			record,
			strings: chunk.strings,
			absNs,
			sequence: sequenceRef.value++,
		});
	}
}

function buildBaseRecordMap(
	chunks: readonly Chunk[],
): Map<string, BaseRecordEntry> {
	const map = new Map<string, BaseRecordEntry>();
	for (const chunk of chunks) {
		for (const record of chunk.records) {
			const absNs = chunk.baseUnixNs + record.timeOffsetNs;
			map.set(spanKey(recordSpanId(record.body)), {
				record,
				strings: chunk.strings,
				absNs,
			});
		}
	}
	return map;
}

function buildSpansFromRecords(
	records: RecordEntry[],
	baseRecords: Map<string, BaseRecordEntry>,
	limit: number,
): { spans: SpanBuilder[]; reachedSpanLimit: boolean } {
	const spans = new Map<string, SpanBuilder>();
	let reachedSpanLimit = false;

	for (const entry of records) {
		const body = entry.record.body;
		const id = recordSpanId(body);
		const key = spanKey(id);
		let span = spans.get(key);
		if (!span) {
			if (spans.size >= limit) {
				reachedSpanLimit = true;
				continue;
			}
			const baseRecord = baseRecords.get(key);
			if (baseRecord) {
				span = initSpanFromBaseRecord(
					baseRecord.record.body,
					baseRecord.strings,
					baseRecord.absNs,
				);
			}
			if (!span) {
				span = initSpanFromRecord(
					body,
					entry.absNs,
					entry.strings,
				);
			}
			if (!span) {
				continue;
			}
			spans.set(key, span);
		}
		applyRecord(span, body, entry.absNs, entry.strings);
	}

	return { spans: Array.from(spans.values()), reachedSpanLimit };
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

function initSpanFromBaseRecord(
	base: RecordBody,
	strings: readonly string[],
	absNs: bigint,
): SpanBuilder | undefined {
	switch (base.tag) {
		case "SpanStart":
			return initSpanFromStart(base.val, absNs, strings);
		case "SpanSnapshot":
			return initSpanFromSnapshot(base.val, strings);
		default:
			return undefined;
	}
}

function initSpanFromRecord(
	body: RecordBody,
	absNs: bigint,
	strings: readonly string[],
): SpanBuilder | undefined {
	switch (body.tag) {
		case "SpanStart":
			return initSpanFromStart(body.val, absNs, strings);
		case "SpanSnapshot":
			return initSpanFromSnapshot(body.val, strings);
		default:
			return undefined;
	}
}

function initSpanFromStart(
	start: SpanStart,
	absNs: bigint | null,
	strings: readonly string[],
): SpanBuilder {
	return {
		traceId: start.traceId,
		spanId: start.spanId,
		parentSpanId: start.parentSpanId,
		name: strings[start.name] ?? "",
		kind: start.kind,
		traceState: start.traceState,
		flags: start.flags,
		attributes: decodeAttributeList(start.attributes, strings),
		droppedAttributesCount: start.droppedAttributesCount,
		links: decodeLinks(start.links, strings),
		droppedLinksCount: start.droppedLinksCount,
		status: null,
		startTimeUnixNs: absNs ?? 0n,
		endTimeUnixNs: null,
		events: [],
	};
}

function initSpanFromSnapshot(
	snapshot: SpanSnapshot,
	strings: readonly string[],
): SpanBuilder {
	return {
		traceId: snapshot.traceId,
		spanId: snapshot.spanId,
		parentSpanId: snapshot.parentSpanId,
		name: strings[snapshot.name] ?? "",
		kind: snapshot.kind,
		traceState: snapshot.traceState,
		flags: snapshot.flags,
		attributes: decodeAttributeList(snapshot.attributes, strings),
		droppedAttributesCount: snapshot.droppedAttributesCount,
		links: decodeLinks(snapshot.links, strings),
		droppedLinksCount: snapshot.droppedLinksCount,
		status: snapshot.status,
		startTimeUnixNs: snapshot.startTimeUnixNs,
		endTimeUnixNs: null,
		events: [],
	};
}

function applyRecord(
	span: SpanBuilder,
	body: RecordBody,
	absNs: bigint,
	strings: readonly string[],
): void {
	switch (body.tag) {
		case "SpanStart":
			if (span.startTimeUnixNs === 0n) {
				span.startTimeUnixNs = absNs;
			}
			return;
		case "SpanSnapshot":
			span.traceId = body.val.traceId;
			span.parentSpanId = body.val.parentSpanId;
			span.name = strings[body.val.name] ?? "";
			span.kind = body.val.kind;
			span.traceState = body.val.traceState;
			span.flags = body.val.flags;
			span.attributes = decodeAttributeList(
				body.val.attributes,
				strings,
			);
			span.droppedAttributesCount = body.val.droppedAttributesCount;
			span.links = decodeLinks(body.val.links, strings);
			span.droppedLinksCount = body.val.droppedLinksCount;
			span.status = body.val.status;
			span.startTimeUnixNs = body.val.startTimeUnixNs;
			return;
		case "SpanUpdate":
			applyAttributes(span.attributes, body.val.attributes, strings);
			span.droppedAttributesCount += body.val.droppedAttributesCount;
			if (body.val.status) {
				span.status = body.val.status;
			}
			return;
		case "SpanEvent":
			span.events.push({
				name: strings[body.val.name] ?? "",
				timeUnixNs: absNs,
				attributes: decodeAttributeList(
					body.val.attributes,
					strings,
				),
				droppedAttributesCount: body.val.droppedAttributesCount,
			});
			return;
		case "SpanEnd":
			span.endTimeUnixNs = absNs;
			if (body.val.status) {
				span.status = body.val.status;
			}
			return;
	}
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

function applyAttributes(
	map: AttributeMap,
	attributes: Attributes,
	strings: readonly string[],
): void {
	for (const kv of attributes) {
		const key = strings[kv.key] ?? "";
		try {
			map.set(key, decodeCbor(toUint8Array(kv.value)) as unknown);
		} catch {
			continue;
		}
	}
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

function toOtlpSpan(span: SpanBuilder): OtlpSpan {
	const attributes = mapToOtlpAttributes(span.attributes);
	const events = span.events.map((event) => toOtlpEvent(event));
	const links = span.links.map((link) => toOtlpLink(link));
	const status = span.status ? toOtlpStatus(span.status) : undefined;
	return {
		traceId: hexFromBytes(normalizeBytes(span.traceId)),
		spanId: hexFromBytes(normalizeBytes(span.spanId)),
		parentSpanId: span.parentSpanId
			? hexFromBytes(normalizeBytes(span.parentSpanId))
			: undefined,
		name: span.name,
		kind: span.kind,
		traceState: span.traceState ?? undefined,
		flags: span.flags || undefined,
		startTimeUnixNano: span.startTimeUnixNs.toString(),
		endTimeUnixNano: span.endTimeUnixNs
			? span.endTimeUnixNs.toString()
			: undefined,
		attributes: attributes.length > 0 ? attributes : undefined,
		droppedAttributesCount: span.droppedAttributesCount || undefined,
		events: events.length > 0 ? events : undefined,
		links: links.length > 0 ? links : undefined,
		droppedLinksCount: span.droppedLinksCount || undefined,
		status,
	};
}

function toOtlpEvent(event: SpanEventState): OtlpSpanEvent {
	const attributes = mapToOtlpAttributes(event.attributes);
	return {
		timeUnixNano: event.timeUnixNs.toString(),
		name: event.name,
		attributes: attributes.length > 0 ? attributes : undefined,
		droppedAttributesCount: event.droppedAttributesCount || undefined,
	};
}

function toOtlpLink(link: LinkState): OtlpSpanLink {
	const attributes = mapToOtlpAttributes(link.attributes);
	return {
		traceId: hexFromBytes(normalizeBytes(link.traceId)),
		spanId: hexFromBytes(normalizeBytes(link.spanId)),
		traceState: link.traceState ?? undefined,
		attributes: attributes.length > 0 ? attributes : undefined,
		droppedAttributesCount: link.droppedAttributesCount || undefined,
	};
}

function toOtlpStatus(status: SpanStatus): OtlpSpanStatus {
	const code =
		status.code === SpanStatusCode.OK
			? 1
			: status.code === SpanStatusCode.ERROR
				? 2
				: 0;
	return {
		code,
		message: status.message ?? undefined,
	};
}

function mapToOtlpAttributes(map: AttributeMap): OtlpKeyValue[] {
	const list: OtlpKeyValue[] = [];
	for (const [key, value] of map.entries()) {
		if (value === undefined || typeof value === "function") {
			continue;
		}
		if (typeof value === "symbol") {
			continue;
		}
		list.push({ key, value: anyValueFromJs(value) });
	}
	return list;
}

function emptyExport(
	resourceValue?: OtlpResource,
): OtlpExportTraceServiceRequestJson {
	return buildExport([], resourceValue);
}

function buildExport(
	spans: OtlpSpan[],
	resourceValue?: OtlpResource,
): OtlpExportTraceServiceRequestJson {
	return {
		resourceSpans: [
			{
				resource: resourceValue,
				scopeSpans: [{ spans }],
			},
		],
	};
}
