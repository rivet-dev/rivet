import { z } from "zod/v4";
import type { VersionedDataHandler } from "vbare";
import { serializeWithEncoding } from "@/serde";
import { assertUnreachable } from "./utils";

/** Data that can be deserialized. */
export type InputData = string | Buffer | Blob | ArrayBufferLike | Uint8Array;

/** Data that's been serialized. */
export type OutputData = string | Uint8Array;

const JSON_COMPAT_BIGINT = "$BigInt";
const JSON_COMPAT_ARRAY_BUFFER = "$ArrayBuffer";
const JSON_COMPAT_UINT8_ARRAY = "$Uint8Array";
const JSON_COMPAT_UNDEFINED = "$Undefined";

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

function isPlainObject(value: unknown): value is Record<string, unknown> {
	if (value === null || typeof value !== "object") {
		return false;
	}

	const proto = Object.getPrototypeOf(value);
	return proto === Object.prototype || proto === null;
}

export function encodeJsonCompatValue(input: any): any {
	if (input === undefined) {
		return [JSON_COMPAT_UNDEFINED, 0];
	}
	if (typeof input === "bigint") {
		return [JSON_COMPAT_BIGINT, input.toString()];
	}
	if (input instanceof ArrayBuffer) {
		return [JSON_COMPAT_ARRAY_BUFFER, base64EncodeArrayBuffer(input)];
	}
	if (input instanceof Uint8Array) {
		return [JSON_COMPAT_UINT8_ARRAY, base64EncodeUint8Array(input)];
	}
	if (Array.isArray(input)) {
		const encoded = input.map((value) => encodeJsonCompatValue(value));
		if (
			encoded.length === 2 &&
			typeof encoded[0] === "string" &&
			encoded[0].startsWith("$")
		) {
			return ["$" + encoded[0], encoded[1]];
		}
		return encoded;
	}
	if (isPlainObject(input)) {
		const encoded: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(input)) {
			encoded[key] = encodeJsonCompatValue(value);
		}
		return encoded;
	}
	return input;
}

export function reviveJsonCompatValue(input: any): any {
	if (Array.isArray(input)) {
		if (
			input.length === 2 &&
			typeof input[0] === "string" &&
			input[0].startsWith("$")
		) {
			if (input[0] === JSON_COMPAT_BIGINT) {
				return BigInt(input[1]);
			}
			if (input[0] === JSON_COMPAT_ARRAY_BUFFER) {
				return base64DecodeToArrayBuffer(input[1]);
			}
			if (input[0] === JSON_COMPAT_UINT8_ARRAY) {
				return base64DecodeToUint8Array(input[1]);
			}
			if (input[0] === JSON_COMPAT_UNDEFINED) {
				return undefined;
			}
			if (input[0].startsWith("$$")) {
				return [input[0].substring(1), reviveJsonCompatValue(input[1])];
			}
			throw new Error(
				`Unknown JSON encoding type: ${input[0]}. This may indicate corrupted data or a version mismatch.`,
			);
		}
		return input.map((value) => reviveJsonCompatValue(value));
	}
	if (isPlainObject(input)) {
		const decoded: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(input)) {
			decoded[key] = reviveJsonCompatValue(value);
		}
		return decoded;
	}
	return input;
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
			return [JSON_COMPAT_BIGINT, value.toString()];
		}
		if (value instanceof ArrayBuffer) {
			return [
				JSON_COMPAT_ARRAY_BUFFER,
				base64EncodeArrayBuffer(value),
			];
		}
		if (value instanceof Uint8Array) {
			return [
				JSON_COMPAT_UINT8_ARRAY,
				base64EncodeUint8Array(value),
			];
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
	return reviveJsonCompatValue(JSON.parse(input));
}
