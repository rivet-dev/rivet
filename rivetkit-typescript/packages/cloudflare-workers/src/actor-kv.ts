// export function kvGet(sql: SqlStorage, key: Uint8Array): Uint8Array | null {
// 	const cursor = sql.exec(
// 		"SELECT value FROM _rivetkit_kv_storage WHERE key = ?",
// 		key,
// 	);
// 	const result = cursor.raw().next();
//
// 	if (!result.done && result.value) {
// 		return toUint8Array(result.value[0]);
// 	}
// 	return null;
// }
//
// export function kvPut(
// 	sql: SqlStorage,
// 	key: Uint8Array,
// 	value: Uint8Array,
// ): void {
// 	sql.exec(
// 		"INSERT OR REPLACE INTO _rivetkit_kv_storage (key, value) VALUES (?, ?)",
// 		key,
// 		value,
// 	);
// }
//
// export function kvDelete(sql: SqlStorage, key: Uint8Array): void {
// 	sql.exec("DELETE FROM _rivetkit_kv_storage WHERE key = ?", key);
// }
//
// export function kvListPrefix(
// 	sql: SqlStorage,
// 	prefix: Uint8Array,
// ): [Uint8Array, Uint8Array][] {
// 	const cursor = sql.exec("SELECT key, value FROM _rivetkit_kv_storage");
// 	const entries: [Uint8Array, Uint8Array][] = [];
//
// 	for (const row of cursor.raw()) {
// 		const key = toUint8Array(row[0]);
// 		const value = toUint8Array(row[1]);
//
// 		// Check if key starts with prefix
// 		if (hasPrefix(key, prefix)) {
// 			entries.push([key, value]);
// 		}
// 	}
//
// 	return entries;
// }
//
// // Helper function to convert SqlStorageValue to Uint8Array
// function toUint8Array(
// 	value: string | number | ArrayBuffer | Uint8Array | null,
// ): Uint8Array {
// 	if (value instanceof Uint8Array) {
// 		return value;
// 	}
// 	if (value instanceof ArrayBuffer) {
// 		return new Uint8Array(value);
// 	}
// 	throw new Error(
// 		`Unexpected SQL value type: ${typeof value} (${value?.constructor?.name})`,
// 	);
// }
//
// function hasPrefix(arr: Uint8Array, prefix: Uint8Array): boolean {
// 	if (prefix.length > arr.length) return false;
// 	for (let i = 0; i < prefix.length; i++) {
// 		if (arr[i] !== prefix[i]) return false;
// 	}
// 	return true;
// }
