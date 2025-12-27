import * as cbor from "cbor-x";
import type { VersionedDataHandler } from "vbare";
import { z } from "zod";
import * as errors from "@/actor/errors";
import { serializeWithEncoding } from "@/serde";
import { loggerWithoutContext } from "../log";
import { assertUnreachable } from "../utils";

/** Data that can be deserialized. */
export type InputData = string | Buffer | Blob | ArrayBufferLike | Uint8Array;

/** Data that's been serialized. */
export type OutputData = string | Uint8Array;

export const EncodingSchema = z.enum(["json", "cbor", "bare"]);

/**
 * Encoding used to communicate between the client & actor.
 */
export type Encoding = z.infer<typeof EncodingSchema>;

/**
 * Helper class that helps serialize data without re-serializing for the same encoding.
 */
export class CachedSerializer<TBare, TJson, T = TBare> {
	#data: T;
	#cache = new Map<Encoding, OutputData>();
	#versionedDataHandler: VersionedDataHandler<TBare>;
	#version: number;
	#zodSchema: z.ZodType<TJson>;
	#toJson: (value: T) => TJson;
	#toBare: (value: T) => TBare;

	constructor(
		data: T,
		versionedDataHandler: VersionedDataHandler<TBare>,
		version: number,
		zodSchema: z.ZodType<TJson>,
		toJson: (value: T) => TJson,
		toBare: (value: T) => TBare,
	) {
		this.#data = data;
		this.#versionedDataHandler = versionedDataHandler;
		this.#version = version;
		this.#zodSchema = zodSchema;
		this.#toJson = toJson;
		this.#toBare = toBare;
	}

	public get rawData(): T {
		return this.#data;
	}

	public serialize(encoding: Encoding): OutputData {
		const cached = this.#cache.get(encoding);
		if (cached) {
			return cached;
		} else {
			const serialized = serializeWithEncoding(
				encoding,
				this.#data,
				this.#versionedDataHandler,
				this.#version,
				this.#zodSchema,
				this.#toJson,
				this.#toBare,
			);
			this.#cache.set(encoding, serialized);
			return serialized;
		}
	}
}

///**
// * Use `CachedSerializer` if serializing the same data repeatedly.
// */
//export function serialize<T>(value: T, encoding: Encoding): OutputData {
//	if (encoding === "json") {
//		return JSON.stringify(value);
//	} else if (encoding === "cbor") {
//		// TODO: Remove this hack, but cbor-x can't handle anything extra in data structures
//		const cleanValue = JSON.parse(JSON.stringify(value));
//		return cbor.encode(cleanValue);
//	} else {
//		assertUnreachable(encoding);
//	}
//}
//
//export async function deserialize(data: InputData, encoding: Encoding) {
//	if (encoding === "json") {
//		if (typeof data !== "string") {
//			logger().warn("received non-string for json parse");
//			throw new errors.MalformedMessage();
//		} else {
//			return JSON.parse(data);
//		}
//	} else if (encoding === "cbor") {
//		if (data instanceof Blob) {
//			const arrayBuffer = await data.arrayBuffer();
//			return cbor.decode(new Uint8Array(arrayBuffer));
//		} else if (data instanceof Uint8Array) {
//			return cbor.decode(data);
//		} else if (
//			data instanceof ArrayBuffer ||
//			data instanceof SharedArrayBuffer
//		) {
//			return cbor.decode(new Uint8Array(data));
//		} else {
//			logger().warn("received non-binary type for cbor parse");
//			throw new errors.MalformedMessage();
//		}
//	} else {
//		assertUnreachable(encoding);
//	}
//}

// TODO: Encode base 128
function base64EncodeUint8Array(uint8Array: Uint8Array): string {
	let binary = "";
	const len = uint8Array.byteLength;
	for (let i = 0; i < len; i++) {
		binary += String.fromCharCode(uint8Array[i]);
	}
	return btoa(binary);
}

function base64EncodeArrayBuffer(arrayBuffer: ArrayBuffer): string {
	const uint8Array = new Uint8Array(arrayBuffer);
	return base64EncodeUint8Array(uint8Array);
}

/** Converts data that was encoded to a string. Some formats (like SSE) don't support raw binary data. */
export function encodeDataToString(message: OutputData): string {
	if (typeof message === "string") {
		return message;
	} else if (message instanceof ArrayBuffer) {
		return base64EncodeArrayBuffer(message);
	} else if (message instanceof Uint8Array) {
		return base64EncodeUint8Array(message);
	} else {
		assertUnreachable(message);
	}
}

function base64DecodeToUint8Array(base64: string): Uint8Array {
	// Check if Buffer is available (Node.js)
	if (typeof Buffer !== "undefined") {
		return new Uint8Array(Buffer.from(base64, "base64"));
	}

	// Browser environment - use atob
	const binary = atob(base64);
	const len = binary.length;
	const bytes = new Uint8Array(len);
	for (let i = 0; i < len; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

function base64DecodeToArrayBuffer(base64: string): ArrayBuffer {
	return base64DecodeToUint8Array(base64).buffer as ArrayBuffer;
}

/** Stringifies with compat for values that BARE & CBOR supports. */
export function jsonStringifyCompat(input: any): string {
	return JSON.stringify(input, (_key, value) => {
		if (typeof value === "bigint") {
			return ["$BigInt", value.toString()];
		} else if (value instanceof ArrayBuffer) {
			return ["$ArrayBuffer", base64EncodeArrayBuffer(value)];
		} else if (value instanceof Uint8Array) {
			return ["$Uint8Array", base64EncodeUint8Array(value)];
		}

		// Escape user arrays that start with $ by prepending another $
		if (
			Array.isArray(value) &&
			value.length === 2 &&
			typeof value[0] === "string" &&
			value[0].startsWith("$")
		) {
			return ["$" + value[0], value[1]];
		}

		return value;
	});
}

/** Parses JSON with compat for values that BARE & CBOR supports. */
export function jsonParseCompat(input: string): any {
	return JSON.parse(input, (_key, value) => {
		// Handle arrays with $ prefix
		if (
			Array.isArray(value) &&
			value.length === 2 &&
			typeof value[0] === "string" &&
			value[0].startsWith("$")
		) {
			// Known special types
			if (value[0] === "$BigInt") {
				return BigInt(value[1]);
			} else if (value[0] === "$ArrayBuffer") {
				return base64DecodeToArrayBuffer(value[1]);
			} else if (value[0] === "$Uint8Array") {
				return base64DecodeToUint8Array(value[1]);
			}

			// Unescape user arrays that started with $ ($$foo -> $foo)
			if (value[0].startsWith("$$")) {
				return [value[0].substring(1), value[1]];
			}

			// Unknown type starting with $ - this is an error
			throw new Error(
				`Unknown JSON encoding type: ${value[0]}. This may indicate corrupted data or a version mismatch.`,
			);
		}

		return value;
	});
}
