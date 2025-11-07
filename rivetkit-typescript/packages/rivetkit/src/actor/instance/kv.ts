export const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
	CONN_PREFIX: Uint8Array.from([2]), // Prefix for connection keys
	INSPECTOR_TOKEN: Uint8Array.from([3]), // Inspector token key
};

// Helper to create a connection key
export function makeConnKey(connId: string): Uint8Array {
	const encoder = new TextEncoder();
	const connIdBytes = encoder.encode(connId);
	const key = new Uint8Array(KEYS.CONN_PREFIX.length + connIdBytes.length);
	key.set(KEYS.CONN_PREFIX, 0);
	key.set(connIdBytes, KEYS.CONN_PREFIX.length);
	return key;
}
