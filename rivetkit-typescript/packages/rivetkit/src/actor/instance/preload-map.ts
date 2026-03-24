/**
 * Input types matching the runner protocol PreloadedKv structure.
 * Defined locally to avoid a dependency on the runner-protocol package.
 */
export interface PreloadedKvInput {
	readonly entries: readonly {
		readonly key: ArrayBufferLike;
		readonly value: ArrayBufferLike;
	}[];
	readonly requestedGetKeys: readonly ArrayBufferLike[];
	readonly requestedPrefixes: readonly ArrayBufferLike[];
}

/**
 * Sorted array of [key, value] pairs for binary search lookups.
 * Used for prefix-based preloaded data (SQLite, connections, workflows).
 */
export type PreloadedEntries = [Uint8Array, Uint8Array][];

/**
 * Preloaded KV lookup supporting exact key lookup and prefix listing.
 *
 * Three-way return on get():
 * - Uint8Array: key was preloaded and exists with this value
 * - null: key was requested but does not exist in storage
 * - undefined: key was not preloaded, caller should fall back to KV read
 *
 * listPrefix():
 * - [Uint8Array, Uint8Array][]: prefix was preloaded, these are the entries
 * - undefined: prefix was not preloaded, caller should fall back to KV list
 */
export interface PreloadMap {
	get(key: Uint8Array): Uint8Array | null | undefined;
	listPrefix(prefix: Uint8Array): [Uint8Array, Uint8Array][] | undefined;
}

/** Lexicographic comparison of two byte arrays. */
export function compareBytes(a: Uint8Array, b: Uint8Array): number {
	const len = Math.min(a.length, b.length);
	for (let i = 0; i < len; i++) {
		if (a[i] !== b[i]) return a[i] - b[i];
	}
	return a.length - b.length;
}

/** Binary search a sorted [key, value][] array. Returns the value if found, undefined otherwise. */
export function binarySearch(
	entries: PreloadedEntries,
	key: Uint8Array,
): Uint8Array | undefined {
	let lo = 0;
	let hi = entries.length - 1;
	while (lo <= hi) {
		const mid = (lo + hi) >>> 1;
		const cmp = compareBytes(entries[mid][0], key);
		if (cmp === 0) return entries[mid][1];
		if (cmp < 0) lo = mid + 1;
		else hi = mid - 1;
	}
	return undefined;
}

/**
 * Returns true if `key` starts with `prefix`.
 */
function hasPrefix(key: Uint8Array, prefix: Uint8Array): boolean {
	if (key.length < prefix.length) return false;
	for (let i = 0; i < prefix.length; i++) {
		if (key[i] !== prefix[i]) return false;
	}
	return true;
}

/**
 * Build a PreloadMap from protocol data.
 *
 * Returns undefined if the protocol data is undefined/null (no preloading).
 */
export function buildPreloadMap(
	preloadedKv: PreloadedKvInput | null | undefined,
): PreloadMap | undefined {
	if (preloadedKv == null) return undefined;

	// Convert ArrayBuffer entries to Uint8Array and sort by key for binary search.
	const sorted: PreloadedEntries = preloadedKv.entries
		.map(
			(entry) =>
				[
					new Uint8Array(entry.key),
					new Uint8Array(entry.value),
				] as [Uint8Array, Uint8Array],
		)
		.sort((a, b) => compareBytes(a[0], b[0]));

	// Build a set of requested get keys for three-way return semantics.
	// We use sorted array + binary search since Uint8Array can't be used as Map key.
	const requestedGetKeys: Uint8Array[] = preloadedKv.requestedGetKeys
		.map((k) => new Uint8Array(k))
		.sort(compareBytes);

	// Build a set of requested prefixes for listPrefix semantics.
	const requestedPrefixes: Uint8Array[] = preloadedKv.requestedPrefixes
		.map((k) => new Uint8Array(k))
		.sort(compareBytes);

	return {
		get(key: Uint8Array): Uint8Array | null | undefined {
			// Check if this key has a value in the preloaded entries.
			const value = binarySearch(sorted, key);
			if (value !== undefined) return value;

			// Check if this key was explicitly requested (meaning it was looked up but not found).
			if (binarySearchExists(requestedGetKeys, key)) return null;

			// Key was not preloaded at all.
			return undefined;
		},

		listPrefix(prefix: Uint8Array): [Uint8Array, Uint8Array][] | undefined {
			// Check if this prefix was requested.
			if (!binarySearchExists(requestedPrefixes, prefix)) {
				return undefined;
			}

			// Collect all entries matching this prefix.
			// Since entries are sorted, find the first match and scan forward.
			const result: [Uint8Array, Uint8Array][] = [];
			let lo = 0;
			let hi = sorted.length - 1;

			// Binary search to find the first entry >= prefix.
			while (lo <= hi) {
				const mid = (lo + hi) >>> 1;
				if (compareBytes(sorted[mid][0], prefix) < 0) {
					lo = mid + 1;
				} else {
					hi = mid - 1;
				}
			}

			// Scan forward from `lo` collecting all entries with the prefix.
			for (let i = lo; i < sorted.length; i++) {
				if (!hasPrefix(sorted[i][0], prefix)) break;
				result.push(sorted[i]);
			}

			return result;
		},
	};
}

/**
 * Binary search a sorted Uint8Array[] to check if a key exists.
 */
function binarySearchExists(
	sortedKeys: Uint8Array[],
	key: Uint8Array,
): boolean {
	let lo = 0;
	let hi = sortedKeys.length - 1;
	while (lo <= hi) {
		const mid = (lo + hi) >>> 1;
		const cmp = compareBytes(sortedKeys[mid], key);
		if (cmp === 0) return true;
		if (cmp < 0) lo = mid + 1;
		else hi = mid - 1;
	}
	return false;
}
