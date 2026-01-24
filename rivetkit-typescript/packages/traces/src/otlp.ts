import { decode as decodeCbor } from "cbor-x";

export interface OtlpAnyValue {
	stringValue?: string;
	boolValue?: boolean;
	intValue?: string;
	doubleValue?: number;
	bytesValue?: string;
	arrayValue?: { values: OtlpAnyValue[] };
	kvlistValue?: { values: OtlpKeyValue[] };
}

export interface OtlpKeyValue {
	key: string;
	value?: OtlpAnyValue;
}

export interface OtlpSpanStatus {
	code: number;
	message?: string;
}

export interface OtlpSpanEvent {
	timeUnixNano: string;
	name: string;
	attributes?: OtlpKeyValue[];
	droppedAttributesCount?: number;
}

export interface OtlpSpanLink {
	traceId: string;
	spanId: string;
	traceState?: string;
	attributes?: OtlpKeyValue[];
	droppedAttributesCount?: number;
}

export interface OtlpSpan {
	traceId: string;
	spanId: string;
	parentSpanId?: string;
	name: string;
	kind?: number;
	startTimeUnixNano: string;
	endTimeUnixNano?: string;
	attributes?: OtlpKeyValue[];
	droppedAttributesCount?: number;
	events?: OtlpSpanEvent[];
	droppedEventsCount?: number;
	links?: OtlpSpanLink[];
	droppedLinksCount?: number;
	status?: OtlpSpanStatus;
	traceState?: string;
	flags?: number;
}

export interface OtlpInstrumentationScope {
	name: string;
	version?: string;
	attributes?: OtlpKeyValue[];
	droppedAttributesCount?: number;
}

export interface OtlpScopeSpans {
	scope?: OtlpInstrumentationScope;
	spans: OtlpSpan[];
	schemaUrl?: string;
}

export interface OtlpResource {
	attributes?: OtlpKeyValue[];
	droppedAttributesCount?: number;
}

export interface OtlpResourceSpans {
	resource?: OtlpResource;
	scopeSpans: OtlpScopeSpans[];
	schemaUrl?: string;
}

export interface OtlpExportTraceServiceRequestJson {
	resourceSpans: OtlpResourceSpans[];
}

export function hexFromBytes(bytes: Uint8Array): string {
	let out = "";
	for (let i = 0; i < bytes.length; i++) {
		out += bytes[i].toString(16).padStart(2, "0");
	}
	return out;
}

export function base64FromBytes(bytes: Uint8Array): string {
	const bufferCtor = (globalThis as { Buffer?: { from: (data: Uint8Array) => { toString: (encoding: string) => string } } }).Buffer;
	if (bufferCtor) {
		return bufferCtor.from(bytes).toString("base64");
	}
	let binary = "";
	for (let i = 0; i < bytes.length; i++) {
		binary += String.fromCharCode(bytes[i]);
	}
	const btoaFn = (globalThis as { btoa?: (data: string) => string }).btoa;
	if (!btoaFn) {
		throw new Error("No base64 encoder available");
	}
	return btoaFn(binary);
}

export function anyValueFromCborBytes(bytes: Uint8Array): OtlpAnyValue {
	const value = decodeCbor(bytes) as unknown;
	return anyValueFromJs(value);
}

export function anyValueFromJs(value: unknown): OtlpAnyValue {
	if (value === null || value === undefined) {
		return { stringValue: "" };
	}
	if (typeof value === "string") {
		return { stringValue: value };
	}
	if (typeof value === "boolean") {
		return { boolValue: value };
	}
	if (typeof value === "number") {
		if (Number.isFinite(value) && Number.isInteger(value)) {
			return { intValue: value.toString() };
		}
		return { doubleValue: value };
	}
	if (typeof value === "bigint") {
		return { intValue: value.toString() };
	}
	if (value instanceof Uint8Array) {
		return { bytesValue: base64FromBytes(value) };
	}
	if (Array.isArray(value)) {
		return { arrayValue: { values: value.map((v) => anyValueFromJs(v)) } };
	}
	if (value instanceof Map) {
		const values: OtlpKeyValue[] = [];
		for (const [key, mapValue] of value.entries()) {
			if (typeof key !== "string") {
				continue;
			}
			values.push({ key, value: anyValueFromJs(mapValue) });
		}
		return { kvlistValue: { values } };
	}
	if (typeof value === "object") {
		const values: OtlpKeyValue[] = [];
		for (const [key, objectValue] of Object.entries(value as object)) {
			values.push({ key, value: anyValueFromJs(objectValue) });
		}
		return { kvlistValue: { values } };
	}

	return { stringValue: String(value) };
}
