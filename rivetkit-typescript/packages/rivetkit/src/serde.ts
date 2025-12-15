import * as cbor from "cbor-x";
import invariant from "invariant";
import type { z } from "zod";
import { assertUnreachable } from "@/common/utils";
import type { VersionedDataHandler } from "vbare";
import type { Encoding } from "@/mod";
import { jsonParseCompat, jsonStringifyCompat } from "./actor/protocol/serde";

export function uint8ArrayToBase64(uint8Array: Uint8Array): string {
	// Check if Buffer is available (Node.js)
	if (typeof Buffer !== "undefined") {
		return Buffer.from(uint8Array).toString("base64");
	}

	// Browser environment - use btoa
	let binary = "";
	const len = uint8Array.byteLength;
	for (let i = 0; i < len; i++) {
		binary += String.fromCharCode(uint8Array[i]);
	}
	return btoa(binary);
}

export function encodingIsBinary(encoding: Encoding): boolean {
	if (encoding === "json") {
		return false;
	} else if (encoding === "cbor" || encoding === "bare") {
		return true;
	} else {
		assertUnreachable(encoding);
	}
}

export function contentTypeForEncoding(encoding: Encoding): string {
	if (encoding === "json") {
		return "application/json";
	} else if (encoding === "cbor" || encoding === "bare") {
		return "application/octet-stream";
	} else {
		assertUnreachable(encoding);
	}
}

export function wsBinaryTypeForEncoding(
	encoding: Encoding,
): "arraybuffer" | "blob" {
	if (encoding === "json") {
		return "blob";
	} else if (encoding === "cbor" || encoding === "bare") {
		return "arraybuffer";
	} else {
		assertUnreachable(encoding);
	}
}

export function serializeWithEncoding<TBare, TJson, T = TBare>(
	encoding: Encoding,
	value: T,
	versionedDataHandler: VersionedDataHandler<TBare> | undefined,
	version: number | undefined,
	zodSchema: z.ZodType<TJson>,
	toJson: (value: T) => TJson,
	toBare: (value: T) => TBare,
): Uint8Array | string {
	if (encoding === "json") {
		const jsonValue = toJson(value);
		const validated = zodSchema.parse(jsonValue);
		return jsonStringifyCompat(validated);
	} else if (encoding === "cbor") {
		const jsonValue = toJson(value);
		const validated = zodSchema.parse(jsonValue);
		return cbor.encode(validated);
	} else if (encoding === "bare") {
		if (!versionedDataHandler) {
			throw new Error(
				"VersionedDataHandler is required for 'bare' encoding",
			);
		}
		if (version === undefined) {
			throw new Error("version is required for 'bare' encoding");
		}
		const bareValue = toBare(value);
		return versionedDataHandler.serializeWithEmbeddedVersion(
			bareValue,
			version,
		);
	} else {
		assertUnreachable(encoding);
	}
}

export function deserializeWithEncoding<TBare, TJson, T = TBare>(
	encoding: Encoding,
	buffer: Uint8Array | string,
	versionedDataHandler: VersionedDataHandler<TBare> | undefined,
	zodSchema: z.ZodType<TJson>,
	fromJson: (value: TJson) => T,
	fromBare: (value: TBare) => T,
): T {
	if (encoding === "json") {
		let parsed: unknown;
		if (typeof buffer === "string") {
			parsed = jsonParseCompat(buffer);
		} else {
			const decoder = new TextDecoder("utf-8");
			const jsonString = decoder.decode(buffer);
			parsed = jsonParseCompat(jsonString);
		}
		const validated = zodSchema.parse(parsed);
		return fromJson(validated);
	} else if (encoding === "cbor") {
		invariant(
			typeof buffer !== "string",
			"buffer cannot be string for cbor encoding",
		);
		// Decode CBOR to get JavaScript values (similar to JSON.parse)
		const decoded: unknown = cbor.decode(buffer);
		// Validate with Zod schema (CBOR produces same structure as JSON)
		const validated = zodSchema.parse(decoded);
		// CBOR decoding produces JS objects, use fromJson
		return fromJson(validated);
	} else if (encoding === "bare") {
		invariant(
			typeof buffer !== "string",
			"buffer cannot be string for bare encoding",
		);
		if (!versionedDataHandler) {
			throw new Error(
				"VersionedDataHandler is required for 'bare' encoding",
			);
		}
		const bareValue =
			versionedDataHandler.deserializeWithEmbeddedVersion(buffer);
		return fromBare(bareValue);
	} else {
		assertUnreachable(encoding);
	}
}
