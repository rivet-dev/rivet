import { InvalidParams } from "@/actor/errors";

export const METADATA_KEY_REGEX = /^[a-z0-9._-]+$/;
export const METADATA_KEY_MAX_BYTES = 128;
export const METADATA_VALUE_MAX_BYTES = 4096;
export const METADATA_TOTAL_MAX_BYTES = 16 * 1024;
export const METADATA_LIST_MAX_KEYS = 16;

const UTF8_ENCODER = new TextEncoder();

export function validateMetadataProjectionKeys(keys: string[]): void {
	if (keys.length > METADATA_LIST_MAX_KEYS) {
		throw new InvalidParams(
			`a maximum of ${METADATA_LIST_MAX_KEYS} metadata keys is allowed`,
		);
	}

	for (const key of keys) {
		validateMetadataKey(key);
	}
}

export function validateMetadataPatchEntries(
	entries: [string, string | null][],
): void {
	if (entries.length === 0) {
		throw new InvalidParams("metadata patch cannot be empty");
	}

	for (const [key, value] of entries) {
		validateMetadataKey(key);

		if (
			value !== null &&
			UTF8_ENCODER.encode(value).length > METADATA_VALUE_MAX_BYTES
		) {
			throw new InvalidParams(
				`metadata value is too large for key '${key}' (max ${METADATA_VALUE_MAX_BYTES} bytes)`,
			);
		}
	}
}

export function validateMetadataMap(
	metadata: Record<string, string>,
): void {
	let totalSize = 0;

	for (const [key, value] of Object.entries(metadata)) {
		validateMetadataKey(key);

		if (UTF8_ENCODER.encode(value).length > METADATA_VALUE_MAX_BYTES) {
			throw new InvalidParams(
				`metadata value is too large for key '${key}' (max ${METADATA_VALUE_MAX_BYTES} bytes)`,
			);
		}

		totalSize +=
			UTF8_ENCODER.encode(key).length + UTF8_ENCODER.encode(value).length;
	}

	if (totalSize > METADATA_TOTAL_MAX_BYTES) {
		throw new InvalidParams(
			`metadata is too large (max ${METADATA_TOTAL_MAX_BYTES} bytes)`,
		);
	}
}

export function projectMetadata(
	metadata: Record<string, string>,
	keys: string[],
): Record<string, string> {
	const projected: Record<string, string> = {};

	for (const key of keys) {
		const value = metadata[key];
		if (value !== undefined) {
			projected[key] = value;
		}
	}

	return projected;
}

function validateMetadataKey(key: string): void {
	const keyBytes = UTF8_ENCODER.encode(key).length;

	if (keyBytes === 0) {
		throw new InvalidParams("metadata key cannot be empty");
	}

	if (keyBytes > METADATA_KEY_MAX_BYTES) {
		throw new InvalidParams(
			`metadata key is too large (max ${METADATA_KEY_MAX_BYTES} bytes)`,
		);
	}

	if (!METADATA_KEY_REGEX.test(key)) {
		throw new InvalidParams(`invalid metadata key: ${key}`);
	}
}
