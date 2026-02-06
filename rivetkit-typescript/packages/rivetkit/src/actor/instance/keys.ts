export const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
	CONN_PREFIX: Uint8Array.from([2]), // Prefix for connection keys
	INSPECTOR_TOKEN: Uint8Array.from([3]), // Inspector token key
	KV: Uint8Array.from([4]), // Prefix for user-facing KV storage
	QUEUE_PREFIX: Uint8Array.from([5]), // Prefix for queue message keys
	QUEUE_METADATA: Uint8Array.from([6]), // Queue metadata key
	WORKFLOW_PREFIX: Uint8Array.from([7]), // Prefix for workflow storage
	TRACES_PREFIX: Uint8Array.from([8]), // Prefix for traces storage
	SQLITE_PREFIX: Uint8Array.from([9]), // Prefix for SQLite VFS data
};

const QUEUE_ID_BYTES = 8;

// Helper to create a prefixed key for user-facing KV storage
export function makePrefixedKey(key: Uint8Array): Uint8Array {
	const prefixed = new Uint8Array(KEYS.KV.length + key.length);
	prefixed.set(KEYS.KV, 0);
	prefixed.set(key, KEYS.KV.length);
	return prefixed;
}

// Helper to remove the prefix from a key
export function removePrefixFromKey(prefixedKey: Uint8Array): Uint8Array {
	return prefixedKey.slice(KEYS.KV.length);
}

export function makeWorkflowKey(key: Uint8Array): Uint8Array {
	const prefixed = new Uint8Array(KEYS.WORKFLOW_PREFIX.length + key.length);
	prefixed.set(KEYS.WORKFLOW_PREFIX, 0);
	prefixed.set(key, KEYS.WORKFLOW_PREFIX.length);
	return prefixed;
}

export function makeTracesKey(key: Uint8Array): Uint8Array {
	const prefixed = new Uint8Array(KEYS.TRACES_PREFIX.length + key.length);
	prefixed.set(KEYS.TRACES_PREFIX, 0);
	prefixed.set(key, KEYS.TRACES_PREFIX.length);
	return prefixed;
}

// Helper to create a connection key
export function makeConnKey(connId: string): Uint8Array {
	const encoder = new TextEncoder();
	const connIdBytes = encoder.encode(connId);
	const key = new Uint8Array(KEYS.CONN_PREFIX.length + connIdBytes.length);
	key.set(KEYS.CONN_PREFIX, 0);
	key.set(connIdBytes, KEYS.CONN_PREFIX.length);
	return key;
}

// Helper to create a queue message key
export function makeQueueMessageKey(id: bigint): Uint8Array {
	const key = new Uint8Array(KEYS.QUEUE_PREFIX.length + QUEUE_ID_BYTES);
	key.set(KEYS.QUEUE_PREFIX, 0);
	const view = new DataView(key.buffer, key.byteOffset, key.byteLength);
	view.setBigUint64(KEYS.QUEUE_PREFIX.length, id, false);
	return key;
}

// Helper to decode a queue message key
export function decodeQueueMessageKey(key: Uint8Array): bigint {
	const offset = KEYS.QUEUE_PREFIX.length;
	if (key.length < offset + QUEUE_ID_BYTES) {
		throw new Error("Queue key is too short");
	}
	const view = new DataView(
		key.buffer,
		key.byteOffset + offset,
		QUEUE_ID_BYTES,
	);
	return view.getBigUint64(0, false);
}
