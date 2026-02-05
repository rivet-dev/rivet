import {
	CURRENT_VERSION,
	READ_RANGE_VERSIONED,
	type ReadRangeWire,
} from "../schemas/versioned.js";

export type { ReadRangeWire };
export type { ReadRangeOptions } from "./types.js";

export function encodeReadRangeWire(wire: ReadRangeWire): Uint8Array {
	return READ_RANGE_VERSIONED.serializeWithEmbeddedVersion(
		wire,
		CURRENT_VERSION,
	);
}

export function decodeReadRangeWire(bytes: Uint8Array): ReadRangeWire {
	return READ_RANGE_VERSIONED.deserializeWithEmbeddedVersion(bytes);
}
