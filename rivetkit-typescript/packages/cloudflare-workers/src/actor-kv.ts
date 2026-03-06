export function kvGet(sql: SqlStorage, key: Uint8Array): Uint8Array | null {
	const cursor = sql.exec(
		"SELECT value FROM _rivetkit_kv_storage WHERE key = ?",
		key,
	);
	const result = cursor.raw().next();

	if (!result.done && result.value) {
		return toUint8Array(result.value[0]);
	}
	return null;
}

export function kvPut(
	sql: SqlStorage,
	key: Uint8Array,
	value: Uint8Array,
): void {
	sql.exec(
		"INSERT OR REPLACE INTO _rivetkit_kv_storage (key, value) VALUES (?, ?)",
		key,
		value,
	);
}

export function kvDelete(sql: SqlStorage, key: Uint8Array): void {
	sql.exec("DELETE FROM _rivetkit_kv_storage WHERE key = ?", key);
}

export function kvDeleteRange(
	sql: SqlStorage,
	start: Uint8Array,
	end: Uint8Array,
): void {
	sql.exec(
		"DELETE FROM _rivetkit_kv_storage WHERE key >= ? AND key < ?",
		start,
		end,
	);
}

export function kvListPrefix(
	sql: SqlStorage,
	prefix: Uint8Array,
	options?: {
		reverse?: boolean;
		limit?: number;
	},
): [Uint8Array, Uint8Array][] {
	const upperBound = computePrefixUpperBound(prefix);
	if (upperBound) {
		return kvListRange(sql, prefix, upperBound, options);
	}

	const direction = options?.reverse ? "DESC" : "ASC";
	const query =
		options?.limit !== undefined
			? `SELECT key, value FROM _rivetkit_kv_storage WHERE key >= ? ORDER BY key ${direction} LIMIT ?`
			: `SELECT key, value FROM _rivetkit_kv_storage WHERE key >= ? ORDER BY key ${direction}`;
	const cursor =
		options?.limit !== undefined
			? sql.exec(query, prefix, options.limit)
			: sql.exec(query, prefix);
	return readEntries(cursor);
}

export function kvListRange(
	sql: SqlStorage,
	start: Uint8Array,
	end: Uint8Array,
	options?: {
		reverse?: boolean;
		limit?: number;
	},
): [Uint8Array, Uint8Array][] {
	const direction = options?.reverse ? "DESC" : "ASC";
	const query =
		options?.limit !== undefined
			? `SELECT key, value FROM _rivetkit_kv_storage WHERE key >= ? AND key < ? ORDER BY key ${direction} LIMIT ?`
			: `SELECT key, value FROM _rivetkit_kv_storage WHERE key >= ? AND key < ? ORDER BY key ${direction}`;
	const cursor =
		options?.limit !== undefined
			? sql.exec(query, start, end, options.limit)
			: sql.exec(query, start, end);
	return readEntries(cursor);
}

function toUint8Array(
	value: string | number | ArrayBuffer | Uint8Array | null,
): Uint8Array {
	if (value instanceof Uint8Array) {
		return value;
	}
	if (value instanceof ArrayBuffer) {
		return new Uint8Array(value);
	}
	throw new Error(
		`Unexpected SQL value type: ${typeof value} (${value?.constructor?.name})`,
	);
}

function readEntries(
	cursor: ReturnType<SqlStorage["exec"]>,
): [Uint8Array, Uint8Array][] {
	const entries: [Uint8Array, Uint8Array][] = [];
	for (const row of cursor.raw()) {
		entries.push([toUint8Array(row[0]), toUint8Array(row[1])]);
	}
	return entries;
}

function computePrefixUpperBound(prefix: Uint8Array): Uint8Array | null {
	const upperBound = prefix.slice();
	for (let i = upperBound.length - 1; i >= 0; i--) {
		if (upperBound[i] !== 0xff) {
			upperBound[i]++;
			return upperBound.slice(0, i + 1);
		}
	}
	return null;
}
