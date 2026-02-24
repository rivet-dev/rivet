/**
 * Binary key encoding/decoding using fdb-tuple.
 * All keys are encoded as tuples with integer prefixes for proper sorting.
 */

import * as tuple from "fdb-tuple";
import type { Location, LoopIterationMarker, PathSegment } from "./types.js";

// === Key Prefixes ===
// Using integers for compact encoding and proper sorting

export const KEY_PREFIX = {
	NAMES: 1, // Name registry: [1, index]
	HISTORY: 2, // History entries: [2, ...locationSegments]
	WORKFLOW: 3, // Workflow metadata: [3, field]
	ENTRY_METADATA: 4, // Entry metadata: [4, entryId]
} as const;

// Workflow metadata field identifiers
export const WORKFLOW_FIELD = {
	STATE: 1,
	OUTPUT: 2,
	ERROR: 3,
	VERSION: 4,
	INPUT: 5,
} as const;

// === Type Definitions ===

// fdb-tuple's TupleItem type - we use a subset
type TupleItem = string | number | boolean | null | TupleItem[];

// === Location Segment Encoding ===

/**
 * Convert a path segment to tuple elements.
 * - NameIndex (number) → just the number
 * - LoopIterationMarker → nested tuple [loopIdx, iteration]
 */
function segmentToTuple(segment: PathSegment): TupleItem {
	if (typeof segment === "number") {
		return segment;
	}
	// LoopIterationMarker
	return [segment.loop, segment.iteration];
}

/**
 * Convert tuple elements back to a path segment.
 */
function tupleToSegment(element: TupleItem): PathSegment {
	if (typeof element === "number") {
		return element;
	}
	if (Array.isArray(element) && element.length === 2) {
		const [loop, iteration] = element;
		if (typeof loop === "number" && typeof iteration === "number") {
			return { loop, iteration } as LoopIterationMarker;
		}
	}
	throw new Error(
		`Invalid path segment tuple element: ${JSON.stringify(element)}`,
	);
}

/**
 * Convert a location to tuple elements.
 */
function locationToTupleElements(location: Location): TupleItem[] {
	return location.map(segmentToTuple);
}

/**
 * Convert tuple elements back to a location.
 */
function tupleElementsToLocation(elements: TupleItem[]): Location {
	return elements.map(tupleToSegment);
}

// === Helper Functions ===

/**
 * Convert Buffer to Uint8Array.
 */
function bufferToUint8Array(buf: Buffer): Uint8Array {
	return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
}

/**
 * Convert Uint8Array to Buffer.
 */
function uint8ArrayToBuffer(arr: Uint8Array): Buffer {
	return Buffer.from(arr.buffer, arr.byteOffset, arr.byteLength);
}

/**
 * Pack tuple items and return as Uint8Array.
 */
function pack(items: TupleItem | TupleItem[]): Uint8Array {
	const buf = tuple.pack(items);
	return bufferToUint8Array(buf);
}

/**
 * Unpack a Uint8Array and return tuple items.
 */
function unpack(data: Uint8Array): TupleItem[] {
	const buf = uint8ArrayToBuffer(data);
	return tuple.unpack(buf) as TupleItem[];
}

// === Key Builders ===

/**
 * Build a key for the name registry.
 * Key: [1, index]
 */
export function buildNameKey(index: number): Uint8Array {
	return pack([KEY_PREFIX.NAMES, index]);
}

/**
 * Build a prefix for listing all names.
 * Prefix: [1]
 */
export function buildNamePrefix(): Uint8Array {
	return pack([KEY_PREFIX.NAMES]);
}

/**
 * Build a key for a history entry.
 * Key: [2, ...locationSegments]
 */
export function buildHistoryKey(location: Location): Uint8Array {
	return pack([KEY_PREFIX.HISTORY, ...locationToTupleElements(location)]);
}

/**
 * Build a prefix for listing history entries under a location.
 * Prefix: [2, ...locationSegments]
 */
export function buildHistoryPrefix(location: Location): Uint8Array {
	return pack([KEY_PREFIX.HISTORY, ...locationToTupleElements(location)]);
}

/**
 * Build a prefix for listing all history entries.
 * Prefix: [2]
 */
export function buildHistoryPrefixAll(): Uint8Array {
	return pack([KEY_PREFIX.HISTORY]);
}

/**
 * Build a key for workflow state.
 * Key: [3, 1]
 */
export function buildWorkflowStateKey(): Uint8Array {
	return pack([KEY_PREFIX.WORKFLOW, WORKFLOW_FIELD.STATE]);
}

/**
 * Build a key for workflow output.
 * Key: [3, 2]
 */
export function buildWorkflowOutputKey(): Uint8Array {
	return pack([KEY_PREFIX.WORKFLOW, WORKFLOW_FIELD.OUTPUT]);
}

/**
 * Build a key for workflow error.
 * Key: [3, 3]
 */
export function buildWorkflowErrorKey(): Uint8Array {
	return pack([KEY_PREFIX.WORKFLOW, WORKFLOW_FIELD.ERROR]);
}

/**
 * Build a key for workflow input.
 * Key: [3, 5]
 */
export function buildWorkflowInputKey(): Uint8Array {
	return pack([KEY_PREFIX.WORKFLOW, WORKFLOW_FIELD.INPUT]);
}

/**
 * Build a key for entry metadata.
 * Key: [4, entryId]
 */
export function buildEntryMetadataKey(entryId: string): Uint8Array {
	return pack([KEY_PREFIX.ENTRY_METADATA, entryId]);
}

/**
 * Build a prefix for listing all entry metadata.
 * Prefix: [4]
 */
export function buildEntryMetadataPrefix(): Uint8Array {
	return pack([KEY_PREFIX.ENTRY_METADATA]);
}

// === Key Parsers ===

/**
 * Parse a name key and return the index.
 * Key: [1, index] → index
 */
export function parseNameKey(key: Uint8Array): number {
	const elements = unpack(key);
	if (elements.length !== 2 || elements[0] !== KEY_PREFIX.NAMES) {
		throw new Error("Invalid name key");
	}
	return elements[1] as number;
}

/**
 * Parse a history key and return the location.
 * Key: [2, ...segments] → Location
 */
export function parseHistoryKey(key: Uint8Array): Location {
	const elements = unpack(key);
	if (elements.length < 1 || elements[0] !== KEY_PREFIX.HISTORY) {
		throw new Error("Invalid history key");
	}
	return tupleElementsToLocation(elements.slice(1));
}

/**
 * Parse an entry metadata key and return the entry ID.
 * Key: [4, entryId] → entryId
 */
export function parseEntryMetadataKey(key: Uint8Array): string {
	const elements = unpack(key);
	if (elements.length !== 2 || elements[0] !== KEY_PREFIX.ENTRY_METADATA) {
		throw new Error("Invalid entry metadata key");
	}
	return elements[1] as string;
}

// === Key Comparison Utilities ===

/**
 * Check if a key starts with a prefix.
 */
export function keyStartsWith(key: Uint8Array, prefix: Uint8Array): boolean {
	if (key.length < prefix.length) {
		return false;
	}
	for (let i = 0; i < prefix.length; i++) {
		if (key[i] !== prefix[i]) {
			return false;
		}
	}
	return true;
}

/**
 * Compare two keys lexicographically.
 * Returns negative if a < b, 0 if a === b, positive if a > b.
 */
export function compareKeys(a: Uint8Array, b: Uint8Array): number {
	const minLen = Math.min(a.length, b.length);
	for (let i = 0; i < minLen; i++) {
		if (a[i] !== b[i]) {
			return a[i] - b[i];
		}
	}
	return a.length - b.length;
}

/**
 * Convert a key to a hex string for debugging.
 */
export function keyToHex(key: Uint8Array): string {
	return Array.from(key)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("");
}
