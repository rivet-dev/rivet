import * as cbor from "cbor-x";
import invariant from "invariant";
import type { z } from "zod";
import { assertUnreachable } from "@/common/utils";
import type { VersionedDataHandler } from "@/common/versioned-data";
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

export function serializeWithEncoding<T, TJson = T>(
	encoding: Encoding,
	value: T | TJson,
	versionedDataHandler: VersionedDataHandler<T> | undefined,
	zodSchema: z.ZodType<TJson>,
): Uint8Array | string {
	if (encoding === "json") {
		const validated = zodSchema.parse(value);
		return jsonStringifyCompat(validated);
	} else if (encoding === "cbor") {
		return cbor.encode(value);
	} else if (encoding === "bare") {
		if (!versionedDataHandler) {
			throw new Error(
				"VersionedDataHandler is required for 'bare' encoding",
			);
		}
		return versionedDataHandler.serializeWithEmbeddedVersion(value as T);
	} else {
		assertUnreachable(encoding);
	}
}

export function deserializeWithEncoding<T, TJson = T>(
	encoding: Encoding,
	buffer: Uint8Array | string,
	versionedDataHandler: VersionedDataHandler<T> | undefined,
	zodSchema: z.ZodType<TJson>,
): T | TJson {
	if (encoding === "json") {
		let parsed: unknown;
		if (typeof buffer === "string") {
			parsed = jsonParseCompat(buffer);
		} else {
			const decoder = new TextDecoder("utf-8");
			const jsonString = decoder.decode(buffer);
			parsed = jsonParseCompat(jsonString);
		}
		return zodSchema.parse(parsed);
	} else if (encoding === "cbor") {
		invariant(
			typeof buffer !== "string",
			"buffer cannot be string for cbor encoding",
		);
		return cbor.decode(buffer);
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
		return versionedDataHandler.deserializeWithEmbeddedVersion(buffer);
	} else {
		assertUnreachable(encoding);
	}
}
