import type { SqliteRuntimeDatabase } from "./sqlite-runtime";

// Keep these limits in sync with engine/packages/pegboard/src/actor_kv/mod.rs.
const KV_MAX_KEY_SIZE = 2 * 1024;
const KV_MAX_VALUE_SIZE = 128 * 1024;
const KV_MAX_KEYS = 128;
const KV_MAX_PUT_PAYLOAD_SIZE = 976 * 1024;
const KV_MAX_STORAGE_SIZE = 10 * 1024 * 1024 * 1024;
const KV_KEY_WRAPPER_OVERHEAD_SIZE = 2;

export function estimateKvSize(db: SqliteRuntimeDatabase): number {
	const row = db.get<{ total: number | bigint | null }>(
		"SELECT COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) AS total FROM kv",
	);
	return row ? Number(row.total ?? 0) : 0;
}

export function validateKvKey(
	key: Uint8Array,
	keyLabel: "key" | "prefix key" = "key",
): void {
	if (key.byteLength + KV_KEY_WRAPPER_OVERHEAD_SIZE > KV_MAX_KEY_SIZE) {
		throw new Error(`${keyLabel} is too long (max 2048 bytes)`);
	}
}

export function validateKvKeys(keys: Uint8Array[]): void {
	if (keys.length > KV_MAX_KEYS) {
		throw new Error("a maximum of 128 keys is allowed");
	}

	for (const key of keys) {
		validateKvKey(key);
	}
}

export function validateKvEntries(
	entries: [Uint8Array, Uint8Array][],
	totalSize: number,
): void {
	if (entries.length > KV_MAX_KEYS) {
		throw new Error("A maximum of 128 key-value entries is allowed");
	}

	let payloadSize = 0;
	for (const [key, value] of entries) {
		payloadSize +=
			key.byteLength + KV_KEY_WRAPPER_OVERHEAD_SIZE + value.byteLength;
	}

	if (payloadSize > KV_MAX_PUT_PAYLOAD_SIZE) {
		throw new Error("total payload is too large (max 976 KiB)");
	}

	const storageRemaining = Math.max(0, KV_MAX_STORAGE_SIZE - totalSize);
	if (payloadSize > storageRemaining) {
		throw new Error(
			`not enough space left in storage (${storageRemaining} bytes remaining, current payload is ${payloadSize} bytes)`,
		);
	}

	for (const [key, value] of entries) {
		validateKvKey(key);
		if (value.byteLength > KV_MAX_VALUE_SIZE) {
			throw new Error(
				`value is too large (max ${KV_MAX_VALUE_SIZE / 1024} KiB)`,
			);
		}
	}
}
