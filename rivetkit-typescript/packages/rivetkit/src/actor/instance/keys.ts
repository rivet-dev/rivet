export const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
	CONN_PREFIX: Uint8Array.from([2]), // Prefix for connection keys
	INSPECTOR_TOKEN: Uint8Array.from([3]), // Inspector token key
	KV: Uint8Array.from([4]), // Prefix for user-facing KV storage
	QUEUE_PREFIX: Uint8Array.from([5]), // Prefix for queue storage
	WORKFLOW_PREFIX: Uint8Array.from([6]), // Prefix for workflow storage
	TRACES_PREFIX: Uint8Array.from([7]), // Prefix for traces storage
	SQLITE_PREFIX: Uint8Array.from([8]), // Prefix for SQLite VFS data
};

export const STORAGE_VERSION = {
	QUEUE: 1,
	WORKFLOW: 1,
	TRACES: 1,
	SQLITE: 1,
} as const;

const STORAGE_VERSION_BYTES = {
	QUEUE: Uint8Array.from([STORAGE_VERSION.QUEUE]),
	WORKFLOW: Uint8Array.from([STORAGE_VERSION.WORKFLOW]),
	TRACES: Uint8Array.from([STORAGE_VERSION.TRACES]),
	SQLITE: Uint8Array.from([STORAGE_VERSION.SQLITE]),
} as const;

const QUEUE_NAMESPACE = {
	METADATA: Uint8Array.from([1]),
	MESSAGES: Uint8Array.from([2]),
} as const;

const QUEUE_ID_BYTES = 8;

function concatPrefix(prefix: Uint8Array, suffix: Uint8Array): Uint8Array {
	const merged = new Uint8Array(prefix.length + suffix.length);
	merged.set(prefix, 0);
	merged.set(suffix, prefix.length);
	return merged;
}

const QUEUE_STORAGE_PREFIX = concatPrefix(
	KEYS.QUEUE_PREFIX,
	STORAGE_VERSION_BYTES.QUEUE,
);
const QUEUE_METADATA_KEY = concatPrefix(
	QUEUE_STORAGE_PREFIX,
	QUEUE_NAMESPACE.METADATA,
);
const QUEUE_MESSAGES_PREFIX = concatPrefix(
	QUEUE_STORAGE_PREFIX,
	QUEUE_NAMESPACE.MESSAGES,
);
const WORKFLOW_STORAGE_PREFIX = concatPrefix(
	KEYS.WORKFLOW_PREFIX,
	STORAGE_VERSION_BYTES.WORKFLOW,
);
const TRACES_STORAGE_PREFIX = concatPrefix(
	KEYS.TRACES_PREFIX,
	STORAGE_VERSION_BYTES.TRACES,
);
const SQLITE_STORAGE_PREFIX = concatPrefix(
	KEYS.SQLITE_PREFIX,
	STORAGE_VERSION_BYTES.SQLITE,
);

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
	return concatPrefix(WORKFLOW_STORAGE_PREFIX, key);
}

export function makeTracesKey(key: Uint8Array): Uint8Array {
	return concatPrefix(TRACES_STORAGE_PREFIX, key);
}

export function workflowStoragePrefix(): Uint8Array {
	return Uint8Array.from(WORKFLOW_STORAGE_PREFIX);
}

export function tracesStoragePrefix(): Uint8Array {
	return Uint8Array.from(TRACES_STORAGE_PREFIX);
}

export function sqliteStoragePrefix(): Uint8Array {
	return Uint8Array.from(SQLITE_STORAGE_PREFIX);
}

export function queueStoragePrefix(): Uint8Array {
	return Uint8Array.from(QUEUE_STORAGE_PREFIX);
}

export function queueMetadataKey(): Uint8Array {
	return Uint8Array.from(QUEUE_METADATA_KEY);
}

export function queueMessagesPrefix(): Uint8Array {
	return Uint8Array.from(QUEUE_MESSAGES_PREFIX);
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
	const key = new Uint8Array(QUEUE_MESSAGES_PREFIX.length + QUEUE_ID_BYTES);
	key.set(QUEUE_MESSAGES_PREFIX, 0);
	const view = new DataView(key.buffer, key.byteOffset, key.byteLength);
	view.setBigUint64(QUEUE_MESSAGES_PREFIX.length, id, false);
	return key;
}

// Helper to decode a queue message key
export function decodeQueueMessageKey(key: Uint8Array): bigint {
	const offset = QUEUE_MESSAGES_PREFIX.length;
	if (key.length < offset + QUEUE_ID_BYTES) {
		throw new Error("Queue key is too short");
	}
	for (let i = 0; i < QUEUE_MESSAGES_PREFIX.length; i++) {
		if (key[i] !== QUEUE_MESSAGES_PREFIX[i]) {
			throw new Error("Queue key has invalid prefix");
		}
	}
	const view = new DataView(
		key.buffer,
		key.byteOffset + offset,
		QUEUE_ID_BYTES,
	);
	return view.getBigUint64(0, false);
}
