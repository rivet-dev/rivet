import * as bare from "@rivetkit/bare-ts";
import { createVersionedDataHandler } from "vbare";
import * as v1 from "../dist/schemas/v1";

export const CURRENT_VERSION = 1;

export type {
	ActiveSpanRef,
	Attributes,
	Chunk,
	KeyValue,
	ReadRangeWire,
	Record,
	RecordBody,
	SpanEnd,
	SpanEvent,
	SpanId,
	SpanLink,
	SpanRecordKey,
	SpanSnapshot,
	SpanStart,
	SpanStatus,
	SpanUpdate,
	StringId,
	TraceId,
} from "../dist/schemas/v1";

export { SpanStatusCode } from "../dist/schemas/v1";

export const CHUNK_VERSIONED = createVersionedDataHandler<v1.Chunk>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return decodeChunk(bytes);
			default:
				throw new Error(`Unknown Chunk version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return encodeChunk(data as v1.Chunk);
			default:
				throw new Error(`Unknown Chunk version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});

export const READ_RANGE_VERSIONED =
	createVersionedDataHandler<v1.ReadRangeWire>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeReadRangeWire(bytes);
				default:
					throw new Error(`Unknown ReadRangeWire version ${version}`);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 1:
					return v1.encodeReadRangeWire(data as v1.ReadRangeWire);
				default:
					throw new Error(`Unknown ReadRangeWire version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export { decodeReadRangeWire, encodeReadRangeWire } from "../dist/schemas/v1";

const recordConfig = bare.Config({});
const chunkConfig = bare.Config({});

export function encodeChunk(chunk: v1.Chunk): Uint8Array {
	const bc = new bare.ByteCursor(
		new Uint8Array(chunkConfig.initialBufferLength),
		chunkConfig,
	);
	v1.writeChunk(bc, chunk);
	return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset);
}

export function decodeChunk(bytes: Uint8Array): v1.Chunk {
	const bc = new bare.ByteCursor(bytes, chunkConfig);
	return v1.readChunk(bc);
}

export function encodeRecord(record: v1.Record): Uint8Array {
	const bc = new bare.ByteCursor(
		new Uint8Array(recordConfig.initialBufferLength),
		recordConfig,
	);
	v1.writeRecord(bc, record);
	return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset);
}
