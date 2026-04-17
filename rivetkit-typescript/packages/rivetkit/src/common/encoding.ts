import { z } from "zod/v4";
import type { VersionedDataHandler } from "vbare";
import { serializeWithEncoding } from "@/serde";
import { assertUnreachable } from "./utils";

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
		}

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

export async function inputDataToBuffer(
	data: InputData,
): Promise<Uint8Array | string> {
	if (typeof data === "string") {
		return data;
	}
	if (data instanceof Blob) {
		return new Uint8Array(await data.arrayBuffer());
	}
	if (data instanceof Uint8Array) {
		return data;
	}
	if (data instanceof ArrayBuffer || data instanceof SharedArrayBuffer) {
		return new Uint8Array(data);
	}
	throw new Error("Malformed message");
}

function base64EncodeUint8Array(uint8Array: Uint8Array): string {
	let binary = "";
	for (const value of uint8Array) {
		binary += String.fromCharCode(value);
	}
	return btoa(binary);
}

function base64EncodeArrayBuffer(arrayBuffer: ArrayBuffer): string {
	return base64EncodeUint8Array(new Uint8Array(arrayBuffer));
}

/** Converts data that was encoded to a string. Some formats do not support raw binary data. */
export function encodeDataToString(message: OutputData): string {
	if (typeof message === "string") {
		return message;
	}
	if (message instanceof Uint8Array) {
		return base64EncodeUint8Array(message);
	}
	assertUnreachable(message);
}

function base64DecodeToUint8Array(base64: string): Uint8Array {
	if (typeof Buffer !== "undefined") {
		return new Uint8Array(Buffer.from(base64, "base64"));
	}

	const binary = atob(base64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
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
		}
		if (value instanceof ArrayBuffer) {
			return ["$ArrayBuffer", base64EncodeArrayBuffer(value)];
		}
		if (value instanceof Uint8Array) {
			return ["$Uint8Array", base64EncodeUint8Array(value)];
		}
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
		if (
			Array.isArray(value) &&
			value.length === 2 &&
			typeof value[0] === "string" &&
			value[0].startsWith("$")
		) {
			if (value[0] === "$BigInt") {
				return BigInt(value[1]);
			}
			if (value[0] === "$ArrayBuffer") {
				return base64DecodeToArrayBuffer(value[1]);
			}
			if (value[0] === "$Uint8Array") {
				return base64DecodeToUint8Array(value[1]);
			}
			if (value[0].startsWith("$$")) {
				return [value[0].substring(1), value[1]];
			}
			throw new Error(
				`Unknown JSON encoding type: ${value[0]}. This may indicate corrupted data or a version mismatch.`,
			);
		}
		return value;
	});
}
