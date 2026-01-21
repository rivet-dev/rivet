export const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
	CONN_PREFIX: Uint8Array.from([2]), // Prefix for connection keys
	INSPECTOR_TOKEN: Uint8Array.from([3]), // Inspector token key
	KV: Uint8Array.from([4]), // Prefix for user-facing KV storage
};

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

// Helper to create a connection key
export function makeConnKey(connId: string): Uint8Array {
	const encoder = new TextEncoder();
	const connIdBytes = encoder.encode(connId);
	const key = new Uint8Array(KEYS.CONN_PREFIX.length + connIdBytes.length);
	key.set(KEYS.CONN_PREFIX, 0);
	key.set(connIdBytes, KEYS.CONN_PREFIX.length);
	return key;
}
